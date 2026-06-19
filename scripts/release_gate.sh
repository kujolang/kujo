#!/usr/bin/env bash
set -euo pipefail

mode="${KUJO_RELEASE_GATE_MODE:-full}"

usage() {
  cat <<'EOF'
Usage: bash scripts/release_gate.sh [--full|--minimal]

Modes:
  --full     Run the complete release gate (default).
  --minimal  Run a fast smoke gate suitable for quick CI/local validation.

Environment:
  KUJO_ENABLE_SOCKET_TESTS=1        Include socket-bound serve integration tests.
  KUJO_RELEASE_GATE_RUN_BENCH=1     Run benchmark smoke command in full mode.
  KUJO_RELEASE_GATE_MODE=full|minimal
EOF
}

if [[ "$#" -gt 1 ]]; then
  usage
  exit 2
fi

if [[ "$#" -eq 1 ]]; then
  case "$1" in
    --full)
      mode="full"
      ;;
    --minimal)
      mode="minimal"
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
fi

run_cmd() {
  echo ""
  echo "+ $*"
  "$@"
}

run_optional_cmd() {
  local tool="$1"
  shift

  if command -v "${tool}" >/dev/null 2>&1; then
    run_cmd "$@"
  else
    echo ""
    echo "- Skipping optional command: $* (missing ${tool})"
  fi
}

echo "Release gate mode: ${mode}"

if [[ "${mode}" == "full" ]]; then
  run_cmd bash scripts/repo_hygiene_audit.sh
  run_cmd cargo fmt --check
  run_cmd cargo clippy --all-targets --all-features -- -D warnings
  run_cmd cargo test
  run_cmd cargo test --test native_api_security_boundaries
  run_cmd cargo test --test package_module_workflow_integration
  run_cmd cargo test --test vm_interpreter_parity_surfaces

  if [[ "${KUJO_ENABLE_SOCKET_TESTS:-0}" == "1" ]]; then
    run_cmd cargo test --test serve_command_integration
  else
    echo ""
    echo "- Skipping socket-bound serve integration tests (set KUJO_ENABLE_SOCKET_TESTS=1 to enable)"
  fi

  run_cmd cargo run -- test

  if [[ "${KUJO_RELEASE_GATE_RUN_BENCH:-0}" == "1" ]]; then
    run_cmd cargo run -- bench examples/benchmarks
  else
    echo ""
    echo "- Skipping benchmark smoke (set KUJO_RELEASE_GATE_RUN_BENCH=1 to enable)"
  fi

  # RUSTSEC-2023-0071 currently has no fixed rsa upgrade. Keep the
  # exception explicit so new vulnerability advisories still fail the gate.
  run_optional_cmd cargo-audit cargo audit --ignore RUSTSEC-2023-0071
  run_optional_cmd cargo-deny cargo deny check
else
  run_cmd bash scripts/repo_hygiene_audit.sh
  run_cmd cargo test --lib -- --test-threads=1
  run_cmd cargo test --test vm_interpreter_parity_surfaces
  run_cmd cargo run -- test --help
fi
