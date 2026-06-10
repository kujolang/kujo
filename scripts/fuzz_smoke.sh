#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/fuzz_smoke.sh [--check-prereqs] [--max-total-time <seconds>] [target...]

Runs bounded cargo-fuzz smoke targets (default: lexer parser) with prerequisite checks.

Options:
  --check-prereqs          Validate toolchain prerequisites only; do not run fuzz targets.
  --max-total-time <secs>  libFuzzer max_total_time per target (default: 20).
  -h, --help               Show this help.

Examples:
  scripts/fuzz_smoke.sh --check-prereqs
  scripts/fuzz_smoke.sh
  scripts/fuzz_smoke.sh --max-total-time 30 lexer parser
EOF
}

CHECK_PREREQS_ONLY=0
MAX_TOTAL_TIME=20
declare -a TARGETS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-prereqs)
      CHECK_PREREQS_ONLY=1
      shift
      ;;
    --max-total-time)
      if [[ $# -lt 2 ]]; then
        echo "error: --max-total-time requires a value" >&2
        usage
        exit 2
      fi
      MAX_TOTAL_TIME="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      TARGETS+=("$1")
      shift
      ;;
  esac
done

if [[ ${#TARGETS[@]} -eq 0 ]]; then
  TARGETS=(lexer parser)
fi

if ! [[ "$MAX_TOTAL_TIME" =~ ^[1-9][0-9]*$ ]]; then
  echo "error: --max-total-time must be a positive integer, got '$MAX_TOTAL_TIME'" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

missing=0

check_cmd() {
  local cmd="$1"
  local install_hint="$2"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[missing] required command '$cmd' not found. $install_hint" >&2
    missing=1
  else
    echo "[ok] found command '$cmd'"
  fi
}

check_cmd cargo "Install Rust toolchain: https://rustup.rs/"
check_cmd rustup "Install Rust toolchain manager: https://rustup.rs/"
check_cmd clang++ "Install a C++ compiler and headers (Xcode CLT or LLVM package)."

if command -v cargo >/dev/null 2>&1; then
  if cargo +nightly --version >/dev/null 2>&1; then
    echo "[ok] nightly toolchain available via 'cargo +nightly'"
  else
    echo "[missing] nightly Rust toolchain not available. Run: rustup toolchain install nightly --profile minimal" >&2
    missing=1
  fi

  if cargo fuzz --help >/dev/null 2>&1; then
    echo "[ok] cargo-fuzz is installed"
  else
    echo "[missing] cargo-fuzz is not installed. Run: cargo +stable install cargo-fuzz --locked" >&2
    missing=1
  fi
fi

if [[ "$missing" -ne 0 ]]; then
  echo "fuzz-smoke prerequisite check failed" >&2
  exit 1
fi

if [[ "$CHECK_PREREQS_ONLY" -eq 1 ]]; then
  echo "fuzz-smoke prerequisite check passed"
  exit 0
fi

for target in "${TARGETS[@]}"; do
  echo "[run] cargo +nightly fuzz run $target -- -max_total_time=$MAX_TOTAL_TIME"
  cargo +nightly fuzz run "$target" -- -max_total_time="$MAX_TOTAL_TIME"
done

echo "fuzz-smoke run completed"
