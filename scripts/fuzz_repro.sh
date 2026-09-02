#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/fuzz_repro.sh --artifact <path> [--target <name>] [--dry-run] [--check-prereqs]

Replay a fuzz crash artifact against a cargo-fuzz target.

Options:
  --artifact <path>        Path to a crash artifact file to replay (required).
  --target <name>          Fuzz target name (for example: lexer, parser, xml_bounded). If omitted, infer from .../artifacts/<target>/...
  --dry-run                Print the replay command and exit without running cargo-fuzz.
  --check-prereqs          Validate prerequisites before replay.
  -h, --help               Show this help.

Examples:
  scripts/fuzz_repro.sh --target lexer --artifact fuzz/artifacts/lexer/crash-123
  scripts/fuzz_repro.sh --artifact fuzz/artifacts/parser/crash-abc
  scripts/fuzz_repro.sh --target parser --artifact tests/fixtures/fuzz/synthetic_crash_input.kujo --dry-run
EOF
}

TARGET=""
ARTIFACT=""
DRY_RUN=0
CHECK_PREREQS=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      if [[ $# -lt 2 ]]; then
        echo "error: --target requires a value" >&2
        usage
        exit 2
      fi
      TARGET="$2"
      shift 2
      ;;
    --artifact)
      if [[ $# -lt 2 ]]; then
        echo "error: --artifact requires a value" >&2
        usage
        exit 2
      fi
      ARTIFACT="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --check-prereqs)
      CHECK_PREREQS=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unexpected argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$ARTIFACT" ]]; then
  echo "error: --artifact is required" >&2
  usage
  exit 2
fi

infer_target_from_artifact() {
  local artifact_path="$1"
  local IFS='/'
  read -r -a parts <<< "$artifact_path"
  local i=0
  while [[ $i -lt ${#parts[@]} ]]; do
    if [[ "${parts[$i]}" == "artifacts" ]]; then
      local next=$((i + 1))
      if [[ $next -lt ${#parts[@]} && -n "${parts[$next]}" ]]; then
        echo "${parts[$next]}"
        return 0
      fi
    fi
    i=$((i + 1))
  done
  return 1
}

if [[ -z "$TARGET" ]]; then
  if inferred_target="$(infer_target_from_artifact "$ARTIFACT")"; then
    TARGET="$inferred_target"
    echo "[info] inferred fuzz target '$TARGET' from artifact path"
  else
    echo "error: --target is required when target cannot be inferred from artifact path" >&2
    exit 2
  fi
fi

if [[ ! "$TARGET" =~ ^[A-Za-z0-9_-]+$ ]]; then
  echo "error: --target must match [A-Za-z0-9_-], got '$TARGET'" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ ! -f "$ARTIFACT" ]]; then
  echo "error: artifact file not found: $ARTIFACT" >&2
  exit 2
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

check_prereqs() {
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
}

if [[ "$CHECK_PREREQS" -eq 1 ]]; then
  check_prereqs
fi

if [[ "$missing" -ne 0 ]]; then
  echo "fuzz-repro prerequisite check failed" >&2
  exit 1
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "[dry-run] cargo +nightly fuzz run $TARGET $ARTIFACT"
  exit 0
fi

echo "[run] cargo +nightly fuzz run $TARGET $ARTIFACT"
cargo +nightly fuzz run "$TARGET" "$ARTIFACT"

echo "fuzz-repro run completed"
