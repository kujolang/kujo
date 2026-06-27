#!/usr/bin/env bash
set -euo pipefail

mode="minimal"
dry_run=0

usage() {
  cat <<'EOF'
Usage: bash scripts/enterprise_verify.sh [--minimal|--full] [--dry-run]

Runs the enterprise-readiness verification wrapper for Kujo's current
AI-native release-candidate branch. The wrapper delegates core release checks
to existing gates, then adds AI replay, security docs, README, and showcase
checks that make the product-readiness story easy to repeat.

Modes:
  --minimal  Fast local/CI confidence gate (default).
  --full     Full release-style matrix plus enterprise presentation checks.

Options:
  --dry-run  Print commands without executing them.
  --help     Show this help text.

Environment:
  KUJO_AI_REPLAY_MODE=strict is set for replay showcase commands.
EOF
}

run_cmd() {
  if [[ "${dry_run}" == "1" ]]; then
    echo "[dry-run] $*"
  else
    echo ""
    echo "+ $*"
    "$@"
  fi
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --minimal)
      mode="minimal"
      shift
      ;;
    --full)
      mode="full"
      shift
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --help|-h)
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

echo "Enterprise verify mode: ${mode}"

if [[ "${mode}" == "full" ]]; then
  run_cmd cargo fmt --check
  run_cmd cargo check
  run_cmd cargo test
  run_cmd cargo test --test docs_examples
  run_cmd cargo test --test readme_contracts
  run_cmd cargo test --test cli_contracts
  run_cmd cargo test --test cli_json_contracts
  run_cmd cargo test --test diagnostics_golden
  run_cmd cargo run -- test --runtime vm
  run_cmd cargo run -- test --runtime dual
  run_cmd bash scripts/release_gate.sh --full
else
  run_cmd bash scripts/repo_hygiene_audit.sh
  run_cmd cargo test --test readme_contracts
  run_cmd cargo test --test docs_policy_consistency_contract
  run_cmd cargo test --test enterprise_verify_contract
  run_cmd cargo test --test ai_replay_hermeticity_contract
  run_cmd cargo test --test docs_examples
fi

run_cmd cargo run -- check examples/ai_enterprise_replay_showcase.kujo

if [[ "${dry_run}" == "1" ]]; then
  echo "[dry-run] KUJO_AI_REPLAY=tests/fixtures/ai_cassettes KUJO_AI_REPLAY_MODE=strict cargo run -- run examples/ai_enterprise_replay_showcase.kujo"
else
  echo ""
  echo "+ KUJO_AI_REPLAY=tests/fixtures/ai_cassettes KUJO_AI_REPLAY_MODE=strict cargo run -- run examples/ai_enterprise_replay_showcase.kujo"
  KUJO_AI_REPLAY=tests/fixtures/ai_cassettes \
    KUJO_AI_REPLAY_MODE=strict \
    cargo run -- run examples/ai_enterprise_replay_showcase.kujo
fi

run_cmd bash scripts/repo_hygiene_audit.sh
