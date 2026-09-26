#!/usr/bin/env bash
# Repeated verification, not retries: the first failure fails the entire gate.
set -euo pipefail
cd "$(dirname "$0")/.."
iterations="${1:-50}"
case "$iterations" in ''|*[!0-9]*) echo 'iterations must be an integer from 1 to 200' >&2; exit 2 ;; esac
if [ "$iterations" -lt 1 ] || [ "$iterations" -gt 200 ]; then
  echo 'iterations must be an integer from 1 to 200' >&2
  exit 2
fi
evidence="${KUJO_NETWORK_STRESS_EVIDENCE:-$(mktemp -d "${TMPDIR:-/tmp}/kujo-network-stress.XXXXXX")}"
mkdir -p "$evidence" target/debug/deps
{
  git rev-parse HEAD
  uname -a
  rustc --version
  printf 'iterations=%s; test_threads=2; deadlines=unchanged\n' "$iterations"
} > "$evidence/environment.txt"
printf 'Evidence: %s\n' "$evidence"
trap 'code=$?; printf "exit=%s\n" "$code" > "$evidence/result.txt"' EXIT
rustc --edition=2021 -D warnings --test scripts/loopback_network_probe.rs \
  -o target/debug/deps/kujo_host_network_probe
cargo test --locked --test docgen_universal --no-run --message-format=json \
  > "$evidence/build.jsonl"
# Cargo JSON is build metadata; Python is used only by this maintenance script.
executable="$(python3 - "$evidence/build.jsonl" <<'PY'
import json
import sys
from pathlib import Path
matches = []
for line in Path(sys.argv[1]).read_text().splitlines():
    item = json.loads(line)
    if item.get('reason') == 'compiler-artifact' and item.get('target', {}).get('name') == 'docgen_universal' and item.get('executable'):
        matches.append(item['executable'])
if len(matches) != 1:
    sys.exit('Expected exactly one Docgen test executable')
print(matches[0])
PY
)"
for ((round=1; round<=iterations; round++)); do
  printf 'Round %s/%s\n' "$round" "$iterations"
  if ! target/debug/deps/kujo_host_network_probe --test-threads=2 > "$evidence/host-$round.log" 2>&1; then
    cat "$evidence/host-$round.log"
    exit 1
  fi
  if ! "$executable" --test-threads=2 > "$evidence/docgen-$round.log" 2>&1; then
    cat "$evidence/docgen-$round.log"
    exit 1
  fi
done
printf 'PASS: %s consecutive host and full Docgen suites\n' "$iterations"
