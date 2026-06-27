#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TESTS_DIR="tests"
OUTPUT_MD="docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md"
OUTPUT_CSV="docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv"
RUNNER=""
MAX_FIXTURES=""
STRICT_MODE=0
VM_COVERAGE_TARGET_PERCENT="70.0"
# Keep the inventory from hanging forever on pathological fixtures, but allow
# slow normal fixtures enough time to finish consistently before they are
# classified as harness debt.
FIXTURE_TIMEOUT_SECONDS="${KUJO_MISMATCH_FIXTURE_TIMEOUT_SECONDS:-15}"
CRYPTO_FIXTURE_TIMEOUT_SECONDS="${KUJO_MISMATCH_CRYPTO_FIXTURE_TIMEOUT_SECONDS:-240}"

resolve_output_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s\n' "$path"
  else
    printf '%s/%s\n' "$ROOT" "$path"
  fi
}

usage() {
  cat <<'USAGE'
Usage: bash scripts/generate_vm_runtime_mismatch_inventory.sh [options]

Generate deterministic VM-vs-interpreter runtime mismatch inventory for Kujo test fixtures.

Options:
  --tests-dir <path>      Fixture directory root (default: tests)
  --output-md <path>      Markdown output path (default: docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md)
  --output-csv <path>     CSV output path (default: docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv)
  --runner <path>         Kujo binary path to use (default: auto-build target/debug/kujo)
  --max-fixtures <count>  Optional fixture cap for quick smoke runs
  --strict                Exit non-zero if any mismatch row lacks required classification fields
  -h, --help              Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tests-dir)
      TESTS_DIR="${2:?missing value for --tests-dir}"
      shift 2
      ;;
    --output-md)
      OUTPUT_MD="${2:?missing value for --output-md}"
      shift 2
      ;;
    --output-csv)
      OUTPUT_CSV="${2:?missing value for --output-csv}"
      shift 2
      ;;
    --runner)
      RUNNER="${2:?missing value for --runner}"
      shift 2
      ;;
    --max-fixtures)
      MAX_FIXTURES="${2:?missing value for --max-fixtures}"
      shift 2
      ;;
    --strict)
      STRICT_MODE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! -d "$ROOT/$TESTS_DIR" ]]; then
  echo "error: tests directory not found: $TESTS_DIR" >&2
  exit 1
fi

if [[ -z "$RUNNER" ]]; then
  (cd "$ROOT" && cargo build --quiet)
  RUNNER="$ROOT/target/debug/kujo"
elif [[ "$RUNNER" != /* ]]; then
  RUNNER="$ROOT/$RUNNER"
fi

if [[ ! -x "$RUNNER" ]]; then
  echo "error: runner binary is not executable: $RUNNER" >&2
  exit 1
fi

trim_file() {
  local path="$1"
  sed -e 's/[[:space:]]*$//' "$path" | sed -e '${/^$/d;}' | sed -e 's/\[KUJOVM001\]/[RUFVM001]/g'
}

run_with_timeout() {
  local timeout_seconds="$1"
  shift

  if command -v timeout >/dev/null 2>&1; then
    timeout --preserve-status --kill-after=5s "${timeout_seconds}s" "$@"
    return $?
  fi

  if command -v python3 >/dev/null 2>&1; then
    python3 - "$timeout_seconds" "$@" <<'PY'
import subprocess
import sys

timeout = float(sys.argv[1])
cmd = sys.argv[2:]

try:
    completed = subprocess.run(
        cmd,
        stdout=sys.stdout.buffer,
        stderr=subprocess.DEVNULL,
        timeout=timeout,
        check=False,
    )
    sys.exit(completed.returncode)
except subprocess.TimeoutExpired:
    sys.exit(124)
PY
    return $?
  fi

  "$@"
}

run_fixture() {
  local fixture="$1"
  local runtime="$2"
  local output_file="$3"
  local status_file="$4"
  local timeout_seconds="$FIXTURE_TIMEOUT_SECONDS"
  local stdout_file="${output_file}.stdout"
  local stderr_file="${output_file}.stderr"
  if [[ "$fixture" == *"stdlib_crypto_test.kujo" ]]; then
    timeout_seconds="$CRYPTO_FIXTURE_TIMEOUT_SECONDS"
  fi
  local -a cmd=("$RUNNER" run "$fixture")

  if [[ "$runtime" == "interpreter" ]]; then
    cmd+=("--interpreter")
  fi

  if run_with_timeout "$timeout_seconds" "${cmd[@]}" >"$stdout_file" 2>"$stderr_file"; then
    echo "0" > "$status_file"
  else
    echo "$?" > "$status_file"
  fi
  if [[ -s "$stdout_file" ]]; then
    cp "$stdout_file" "$output_file"
  elif [[ -s "$stderr_file" ]]; then
    cp "$stderr_file" "$output_file"
  else
    : > "$output_file"
  fi
}

cleanup_fixture_side_effects() {
  rm -f "$ROOT/blocked.txt"
}

classify_delta() {
  local vm_match="$1"
  local interpreter_match="$2"
  local vm_output="$3"
  local interpreter_output="$4"

  if [[ "$vm_match" == "yes" && "$interpreter_match" == "yes" ]]; then
    echo "both_match_snapshot"
  elif [[ "$vm_match" == "no" && "$interpreter_match" == "yes" ]]; then
    echo "vm_only_mismatch"
  elif [[ "$vm_match" == "yes" && "$interpreter_match" == "no" ]]; then
    echo "interpreter_only_mismatch"
  elif [[ "$vm_output" == "$interpreter_output" ]]; then
    echo "both_mismatch_same_output"
  else
    echo "both_mismatch_different_output"
  fi
}

classify_mismatch_cause() {
  local fixture="$1"
  local delta_type="$2"
  local vm_exit="$3"
  local interpreter_exit="$4"

  if [[ "$delta_type" == "both_match_snapshot" ]]; then
    echo "none|n/a|P4|snapshot matches in both runtimes"
    return
  fi

  if [[ "$delta_type" == "vm_only_mismatch" || "$delta_type" == "interpreter_only_mismatch" ]]; then
    echo "runtime-parity-bug|runtime-owner|P0|runtime-path mismatch against snapshot indicates parity defect or runtime-specific contract drift"
    return
  fi

  if [[ "$delta_type" == "both_mismatch_same_output" ]]; then
    echo "stale-snapshot-expectation|docs-owner|P1|both runtimes agree on output but snapshot expectation diverges"
    return
  fi

  if [[ "$vm_exit" == "3" && "$interpreter_exit" == "3" ]]; then
    echo "parser-invalid-fixture|language-owner|P1|both runtimes fail with parser diagnostics and fixture/snapshot contract should be refreshed"
    return
  fi

  if [[ "$delta_type" == "both_mismatch_different_output" ]]; then
    echo "runtime-parity-bug|runtime-owner|P0|both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations"
    return
  fi

  if [[ "$vm_exit" == "124" || "$interpreter_exit" == "124" || "$vm_exit" == "137" || "$interpreter_exit" == "137" ]]; then
    echo "harness-debt|harness-owner|P2|fixture exceeded inventory generation timeout and needs a dedicated harness follow-up"
    return
  fi

  if [[ "$fixture" == *"generators_test.kujo"* ]]; then
    echo "intentional-divergence|runtime-owner|P2|known generator-surface divergence should be documented and contract-locked"
    return
  fi
  echo "runtime-parity-bug|runtime-owner|P0|both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations"
}

if command -v rg >/dev/null 2>&1; then
  fixtures=$(cd "$ROOT" && rg --files "$TESTS_DIR" -g '*.kujo' | sort)
else
  fixtures=$(cd "$ROOT" && find "$TESTS_DIR" -type f -name '*.kujo' | sort)
fi
if [[ -n "$MAX_FIXTURES" ]]; then
  fixtures=$(printf '%s\n' "$fixtures" | head -n "$MAX_FIXTURES")
fi

OUTPUT_MD_PATH="$(resolve_output_path "$OUTPUT_MD")"
OUTPUT_CSV_PATH="$(resolve_output_path "$OUTPUT_CSV")"
mkdir -p "$(dirname "$OUTPUT_MD_PATH")"
mkdir -p "$(dirname "$OUTPUT_CSV_PATH")"

{
  echo "# VM Runtime Mismatch Inventory"
  echo
  echo "Generated: $(date +%Y-%m-%d)"
  echo "Runner: \`$RUNNER\`"
  echo "Fixture root: \`$TESTS_DIR\`"
  echo
  echo "| Fixture | VM Exit | Interpreter Exit | VM Matches Snapshot | Interpreter Matches Snapshot | Delta Type | Mismatch Bucket | Owner | Priority | Rationale |"
  echo "| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- |"
} > "$OUTPUT_MD_PATH"

echo "fixture,vm_exit,interpreter_exit,vm_matches_snapshot,interpreter_matches_snapshot,delta_type,mismatch_bucket,bucket_owner,priority,rationale" > "$OUTPUT_CSV_PATH"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

count=0
vm_only_mismatch=0
interpreter_only_mismatch=0
both_mismatch=0
both_match=0
bucket_parser_invalid=0
bucket_stale_snapshot=0
bucket_runtime_parity=0
bucket_intentional_divergence=0
bucket_harness_debt=0
classification_errors=0

while IFS= read -r fixture; do
  [[ -z "$fixture" ]] && continue
  count=$((count + 1))

  expected_path="$ROOT/${fixture%.kujo}.out"
  expected=""
  if [[ -f "$expected_path" ]]; then
    expected="$(trim_file "$expected_path")"
  fi

  vm_out_file="$tmp_dir/vm_${count}.txt"
  vm_status_file="$tmp_dir/vm_${count}.status"
  int_out_file="$tmp_dir/int_${count}.txt"
  int_status_file="$tmp_dir/int_${count}.status"

  run_fixture "$ROOT/$fixture" "vm" "$vm_out_file" "$vm_status_file"
  run_fixture "$ROOT/$fixture" "interpreter" "$int_out_file" "$int_status_file"
  cleanup_fixture_side_effects

  vm_output="$(trim_file "$vm_out_file")"
  interpreter_output="$(trim_file "$int_out_file")"
  vm_exit="$(cat "$vm_status_file")"
  interpreter_exit="$(cat "$int_status_file")"

  vm_match="no"
  interpreter_match="no"
  if [[ "$vm_output" == "$expected" ]]; then
    vm_match="yes"
  fi
  if [[ "$interpreter_output" == "$expected" ]]; then
    interpreter_match="yes"
  fi

  delta_type="$(classify_delta "$vm_match" "$interpreter_match" "$vm_output" "$interpreter_output")"
  cause_payload="$(classify_mismatch_cause "$fixture" "$delta_type" "$vm_exit" "$interpreter_exit")"
  mismatch_bucket="${cause_payload%%|*}"
  remaining="${cause_payload#*|}"
  mismatch_owner="${remaining%%|*}"
  remaining="${remaining#*|}"
  mismatch_priority="${remaining%%|*}"
  mismatch_rationale="${remaining#*|}"

  case "$delta_type" in
    both_match_snapshot) both_match=$((both_match + 1)) ;;
    vm_only_mismatch) vm_only_mismatch=$((vm_only_mismatch + 1)) ;;
    interpreter_only_mismatch) interpreter_only_mismatch=$((interpreter_only_mismatch + 1)) ;;
    *) both_mismatch=$((both_mismatch + 1)) ;;
  esac
  case "$mismatch_bucket" in
    parser-invalid-fixture) bucket_parser_invalid=$((bucket_parser_invalid + 1)) ;;
    stale-snapshot-expectation) bucket_stale_snapshot=$((bucket_stale_snapshot + 1)) ;;
    runtime-parity-bug) bucket_runtime_parity=$((bucket_runtime_parity + 1)) ;;
    intentional-divergence) bucket_intentional_divergence=$((bucket_intentional_divergence + 1)) ;;
    harness-debt) bucket_harness_debt=$((bucket_harness_debt + 1)) ;;
  esac

  if [[ "$delta_type" != "both_match_snapshot" ]]; then
    if [[ "$mismatch_bucket" == "none" || "$mismatch_owner" == "n/a" || "$mismatch_priority" == "P4" ]]; then
      classification_errors=$((classification_errors + 1))
    fi
  fi

  echo "| \`$fixture\` | $vm_exit | $interpreter_exit | $vm_match | $interpreter_match | \`$delta_type\` | \`$mismatch_bucket\` | $mismatch_owner | \`$mismatch_priority\` | $mismatch_rationale |" >> "$OUTPUT_MD_PATH"
  printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$fixture" \
    "$vm_exit" \
    "$interpreter_exit" \
    "$vm_match" \
    "$interpreter_match" \
    "$delta_type" \
    "$mismatch_bucket" \
    "$mismatch_owner" \
    "$mismatch_priority" \
    "$(printf '%s' "$mismatch_rationale" | sed 's/,/;/g')" >> "$OUTPUT_CSV_PATH"
done <<< "$fixtures"

vm_matches_snapshot_total=$((both_match + interpreter_only_mismatch))
vm_match_percent="$(awk -v matches="$vm_matches_snapshot_total" -v total="$count" 'BEGIN { if (total == 0) { printf "0.0" } else { printf "%.1f", (matches * 100.0) / total } }')"
vm_target_met="$(awk -v pct="$vm_match_percent" -v target="$VM_COVERAGE_TARGET_PERCENT" 'BEGIN { if (pct + 0 >= target + 0) print "PASS"; else print "FAIL"; }')"

{
  echo
  echo "Summary: \`$count\` fixtures scanned"
  echo "- both match snapshot: \`$both_match\`"
  echo "- VM-only mismatch: \`$vm_only_mismatch\`"
  echo "- interpreter-only mismatch: \`$interpreter_only_mismatch\`"
  echo "- both mismatch: \`$both_mismatch\`"
  echo
  echo "Mismatch classification totals (priority order):"
  echo "- P0 runtime-parity-bug (\`runtime-owner\`): \`$bucket_runtime_parity\`"
  echo "- P1 stale-snapshot-expectation (\`docs-owner\`): \`$bucket_stale_snapshot\`"
  echo "- P1 parser-invalid-fixture (\`language-owner\`): \`$bucket_parser_invalid\`"
  echo "- P2 harness-debt (\`harness-owner\`): \`$bucket_harness_debt\`"
  echo "- P2 intentional-divergence (\`runtime-owner\`): \`$bucket_intentional_divergence\`"
  echo
  echo "VM coverage gate:"
  echo "- metric: \`vm_matches_snapshot / fixtures_scanned\`"
  echo "- vm_matches_snapshot: \`$vm_matches_snapshot_total/$count\` (\`$vm_match_percent%\`)"
  echo "- target threshold: \`$VM_COVERAGE_TARGET_PERCENT%\`"
  echo "- gate status: \`$vm_target_met\`"
} >> "$OUTPUT_MD_PATH"

echo "generated inventory: $OUTPUT_MD"
echo "generated inventory csv: $OUTPUT_CSV"

if [[ "$STRICT_MODE" -eq 1 && "$classification_errors" -gt 0 ]]; then
  echo "error: $classification_errors mismatch rows were missing required classification fields" >&2
  exit 1
fi
