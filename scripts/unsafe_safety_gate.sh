#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/unsafe_safety_gate.sh [options]

Runs the unsafe hardening verification gate.

Options:
  --dry-run     Print commands without executing them.
  --with-miri   Also run an optional nightly Miri probe test.
  --help        Show this help message.

Base gate commands:
  1) bash scripts/generate_unsafe_inventory.sh
  2) cargo test --test unsafe_inventory_contract
  3) cargo test --test vm_interpreter_parity_surfaces

Optional Miri probe:
  cargo +nightly miri test --test vm_interpreter_parity_surfaces vm_and_interpreter_resolve_defined_identifiers

Failure modes:
  - exits 2 for unsupported flags/arguments
  - exits 3 when --with-miri is requested but nightly+miri prerequisites are missing
EOF
}

DRY_RUN=false
WITH_MIRI=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --with-miri)
      WITH_MIRI=true
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "unsupported argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

run_cmd() {
  local cmd=("$@")
  if [[ "$DRY_RUN" == "true" ]]; then
    echo "[dry-run] ${cmd[*]}"
    return 0
  fi
  "${cmd[@]}"
}

require_miri_prereqs() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required for --with-miri" >&2
    exit 3
  fi
  if ! cargo +nightly --version >/dev/null 2>&1; then
    echo "nightly toolchain is required for --with-miri (install via: rustup toolchain install nightly)" >&2
    exit 3
  fi
  if ! cargo +nightly miri --version >/dev/null 2>&1; then
    echo "miri is required for --with-miri (install via: rustup component add miri --toolchain nightly)" >&2
    exit 3
  fi
}

run_cmd bash scripts/generate_unsafe_inventory.sh
run_cmd cargo test --test unsafe_inventory_contract
run_cmd cargo test --test vm_interpreter_parity_surfaces

if [[ "$WITH_MIRI" == "true" ]]; then
  if [[ "$DRY_RUN" != "true" ]]; then
    require_miri_prereqs
  fi
  run_cmd cargo +nightly miri test --test vm_interpreter_parity_surfaces vm_and_interpreter_resolve_defined_identifiers
fi
