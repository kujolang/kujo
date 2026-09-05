#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/fuzz_smoke.sh [--check-prereqs] [--max-total-time <seconds>] [--run-root <directory>] [target...]

Runs bounded cargo-fuzz smoke targets (default: lexer parser xml_bounded gzip_bounded
zip_single_bounded) with prerequisite checks.

Options:
  --check-prereqs          Validate toolchain prerequisites only; do not run fuzz targets.
  --max-total-time <secs>  libFuzzer max_total_time per target (default: 20).
  --run-root <directory>   Copy seeds and write corpus/artifacts below this directory.
  -h, --help               Show this help.

Examples:
  scripts/fuzz_smoke.sh --check-prereqs
  scripts/fuzz_smoke.sh
  scripts/fuzz_smoke.sh --max-total-time 30 lexer parser xml_bounded gzip_bounded zip_single_bounded
EOF
}

CHECK_PREREQS_ONLY=0
MAX_TOTAL_TIME=20
RUN_ROOT=""
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
    --run-root)
      if [[ $# -lt 2 || -z "$2" ]]; then
        echo "error: --run-root requires a directory" >&2
        usage
        exit 2
      fi
      RUN_ROOT="$2"
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
  TARGETS=(lexer parser xml_bounded gzip_bounded zip_single_bounded)
fi

if ! [[ "$MAX_TOTAL_TIME" =~ ^[1-9][0-9]*$ ]]; then
  echo "error: --max-total-time must be a positive integer, got '$MAX_TOTAL_TIME'" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Some macOS Command Line Tools releases provide clang++ without the libc++
# headers required to compile libFuzzer. Prefer a keg-only Homebrew LLVM when
# it is installed, while preserving an explicit operator CXX override.
if [[ -z "${CXX:-}" && -x /usr/local/opt/llvm/bin/clang++ ]]; then
  export CXX=/usr/local/opt/llvm/bin/clang++
elif [[ -z "${CXX:-}" && -x /opt/homebrew/opt/llvm/bin/clang++ ]]; then
  export CXX=/opt/homebrew/opt/llvm/bin/clang++
fi

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
if [[ -n "${CXX:-}" ]]; then
  check_cmd "$CXX" "Install a C++ compiler and headers (Xcode CLT or LLVM package)."
  echo "[ok] cargo-fuzz CXX=$CXX"
else
  check_cmd clang++ "Install a C++ compiler and headers (Xcode CLT or LLVM package)."
fi

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
  if [[ -n "$RUN_ROOT" ]]; then
    corpus_dir="$RUN_ROOT/corpus/$target"
    artifact_dir="$RUN_ROOT/artifacts/$target"
    mkdir -p "$corpus_dir" "$artifact_dir"
    if [[ -d "$repo_root/fuzz/corpus/$target" ]]; then
      cp -R "$repo_root/fuzz/corpus/$target/." "$corpus_dir/"
    fi
    echo "[evidence] corpus=$corpus_dir artifacts=$artifact_dir"
    cargo +nightly fuzz run "$target" "$corpus_dir" -- \
      -max_total_time="$MAX_TOTAL_TIME" \
      -print_final_stats=1 \
      -artifact_prefix="$artifact_dir/"
  else
    cargo +nightly fuzz run "$target" -- -max_total_time="$MAX_TOTAL_TIME"
  fi
done

echo "fuzz-smoke run completed"
