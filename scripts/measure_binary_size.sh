#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/measure_binary_size.sh [options]

Builds Kujo binaries and reports deterministic size evidence.

Options:
  --dry-run        Print commands without executing them.
  --metadata-only  Print host/toolchain metadata and exit.
  --help           Show this help message.

Reported artifacts:
  - target/debug/kujo
  - target/release/kujo
  - stripped release copy (when `strip` is available)
EOF
}

DRY_RUN=false
METADATA_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --metadata-only)
      METADATA_ONLY=true
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

print_metadata() {
  echo "timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "host: $(uname -a)"
  echo "rustc: $(rustc -V)"
  echo "cargo: $(cargo -V)"
  echo "rustc_verbose:"
  rustc -Vv | sed 's/^/  /'
}

print_metadata

if [[ "$METADATA_ONLY" == "true" ]]; then
  exit 0
fi

run_cmd cargo build
run_cmd cargo build --release

if [[ "$DRY_RUN" == "true" ]]; then
  echo "[dry-run] wc -c target/debug/kujo target/release/kujo"
  if command -v strip >/dev/null 2>&1; then
    echo "[dry-run] cp target/release/kujo target/release/kujo.stripped && strip target/release/kujo.stripped && wc -c target/release/kujo.stripped"
  else
    echo "[dry-run] strip unavailable; skipping stripped size"
  fi
  exit 0
fi

if [[ ! -f target/debug/kujo ]] || [[ ! -f target/release/kujo ]]; then
  echo "expected binaries missing after build" >&2
  exit 3
fi

debug_bytes=$(wc -c < target/debug/kujo | tr -d ' ')
release_bytes=$(wc -c < target/release/kujo | tr -d ' ')

echo "binary_size_bytes:"
echo "  debug: ${debug_bytes}"
echo "  release: ${release_bytes}"

if command -v strip >/dev/null 2>&1; then
  cp target/release/kujo target/release/kujo.stripped
  strip target/release/kujo.stripped
  stripped_bytes=$(wc -c < target/release/kujo.stripped | tr -d ' ')
  rm -f target/release/kujo.stripped
  echo "  release_stripped: ${stripped_bytes}"
else
  echo "  release_stripped: unavailable (strip not installed)"
fi
