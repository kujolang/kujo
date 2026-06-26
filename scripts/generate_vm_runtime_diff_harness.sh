#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TESTS_DIR="tests"
OUTPUT_MD="docs/generated/VM_RUNTIME_DIFF_HARNESS.md"
OUTPUT_CSV="docs/generated/VM_RUNTIME_DIFF_HARNESS.csv"
RUNNER=""
MAX_FIXTURES=""
SELF_CHECK_ONLY=0

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
Usage: bash scripts/generate_vm_runtime_diff_harness.sh [options]

Generate runtime diff comparison artifacts for VM vs interpreter execution with
explicit output normalization.

Options:
  --tests-dir <path>                 Fixture directory root (default: tests)
  --output-md <path>                 Markdown output path (default: docs/generated/VM_RUNTIME_DIFF_HARNESS.md)
  --output-csv <path>                CSV output path (default: docs/generated/VM_RUNTIME_DIFF_HARNESS.csv)
  --runner <path>                    Kujo binary path (default: auto-build target/debug/kujo)
  --max-fixtures <count>             Optional fixture cap for faster local smoke runs
  --normalization-self-check-only    Run normalization self-check and exit
  -h, --help                         Show this help text
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
    --normalization-self-check-only)
      SELF_CHECK_ONLY=1
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

normalize_output() {
  local input_path="$1"
  sed -E \
    -e 's/^\[KUJO[A-Z0-9]+\] \[[^]]+\] Runtime Error: /Runtime Error: /' \
    -e 's/^\[KUJO[A-Z0-9]+\] \[[^]]+\] /[KUJO] /' \
    -e 's/[[:space:]]+$//' \
    "$input_path" \
  | sed -e '${/^$/d;}'
}

normalization_self_check() {
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN

  cat > "$tmp_dir/vm.txt" <<'VM'
[KUJOVM001] [vm] Runtime Error: Cannot call non-function
  --> 0:0
VM

  cat > "$tmp_dir/interpreter.txt" <<'INTERP'
[KUJORUN001] [runtime] Runtime Error: Cannot call non-function
  --> 0:0
INTERP

  local vm_norm
  local interp_norm
  vm_norm="$(normalize_output "$tmp_dir/vm.txt")"
  interp_norm="$(normalize_output "$tmp_dir/interpreter.txt")"

  if [[ "$vm_norm" != "$interp_norm" ]]; then
    echo "error: normalization self-check failed" >&2
    echo "vm_norm=$vm_norm" >&2
    echo "interp_norm=$interp_norm" >&2
    return 1
  fi

  echo "normalization self-check: ok"
}

if [[ "$SELF_CHECK_ONLY" -eq 1 ]]; then
  normalization_self_check
  exit 0
fi

normalization_self_check

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

run_fixture() {
  local fixture="$1"
  local runtime="$2"
  local output_file="$3"
  local status_file="$4"

  if [[ "$runtime" == "interpreter" ]]; then
    if "$RUNNER" run "$fixture" --interpreter >"$output_file" 2>/dev/null; then
      echo "0" > "$status_file"
    else
      echo "$?" > "$status_file"
    fi
  else
    if "$RUNNER" run "$fixture" >"$output_file" 2>/dev/null; then
      echo "0" > "$status_file"
    else
      echo "$?" > "$status_file"
    fi
  fi
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
  echo "# VM Runtime Diff Harness"
  echo
  echo "Generated: $(date +%Y-%m-%d)"
  echo "Runner: \`$RUNNER\`"
  echo "Fixture root: \`$TESTS_DIR\`"
  echo
  echo "| Fixture | VM Exit | Interpreter Exit | Raw Equal | Normalized Equal | Diff Class |"
  echo "| --- | ---: | ---: | --- | --- | --- |"
} > "$OUTPUT_MD_PATH"

echo "fixture,vm_exit,interpreter_exit,raw_equal,normalized_equal,diff_class" > "$OUTPUT_CSV_PATH"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

count=0
raw_equal_count=0
normalized_equal_count=0
normalized_noise_only_count=0
semantic_drift_count=0

while IFS= read -r fixture; do
  [[ -z "$fixture" ]] && continue
  count=$((count + 1))

  vm_out_file="$tmp_dir/vm_${count}.txt"
  vm_status_file="$tmp_dir/vm_${count}.status"
  int_out_file="$tmp_dir/int_${count}.txt"
  int_status_file="$tmp_dir/int_${count}.status"

  run_fixture "$ROOT/$fixture" "vm" "$vm_out_file" "$vm_status_file"
  run_fixture "$ROOT/$fixture" "interpreter" "$int_out_file" "$int_status_file"

  vm_raw="$(cat "$vm_out_file")"
  interp_raw="$(cat "$int_out_file")"
  vm_norm="$(normalize_output "$vm_out_file")"
  interp_norm="$(normalize_output "$int_out_file")"
  vm_exit="$(cat "$vm_status_file")"
  interp_exit="$(cat "$int_status_file")"

  raw_equal="no"
  normalized_equal="no"
  if [[ "$vm_raw" == "$interp_raw" ]]; then
    raw_equal="yes"
    raw_equal_count=$((raw_equal_count + 1))
  fi
  if [[ "$vm_norm" == "$interp_norm" ]]; then
    normalized_equal="yes"
    normalized_equal_count=$((normalized_equal_count + 1))
  fi

  diff_class="semantic_drift"
  if [[ "$raw_equal" == "yes" ]]; then
    diff_class="raw_equal"
  elif [[ "$normalized_equal" == "yes" ]]; then
    diff_class="normalized_noise_only"
  fi

  if [[ "$diff_class" == "normalized_noise_only" ]]; then
    normalized_noise_only_count=$((normalized_noise_only_count + 1))
  elif [[ "$diff_class" == "semantic_drift" ]]; then
    semantic_drift_count=$((semantic_drift_count + 1))
  fi

  echo "| \`$fixture\` | $vm_exit | $interp_exit | $raw_equal | $normalized_equal | \`$diff_class\` |" >> "$OUTPUT_MD_PATH"
  printf '%s,%s,%s,%s,%s,%s\n' "$fixture" "$vm_exit" "$interp_exit" "$raw_equal" "$normalized_equal" "$diff_class" >> "$OUTPUT_CSV_PATH"
done <<< "$fixtures"

{
  echo
  echo "Summary: \`$count\` fixtures compared"
  echo "- raw equal: \`$raw_equal_count\`"
  echo "- normalized equal: \`$normalized_equal_count\`"
  echo "- normalized noise only: \`$normalized_noise_only_count\`"
  echo "- semantic drift: \`$semantic_drift_count\`"
} >> "$OUTPUT_MD_PATH"

echo "generated runtime diff harness: $OUTPUT_MD"
echo "generated runtime diff harness csv: $OUTPUT_CSV"
