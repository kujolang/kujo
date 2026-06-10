#!/usr/bin/env bash
set -euo pipefail

SOURCE_ROOT="src"
OUTPUT_MD="docs/generated/V1_CODE_TODO_TRIAGE.md"
OUTPUT_CSV="docs/generated/V1_CODE_TODO_TRIAGE.csv"
STRICT_MODE=0

usage() {
  cat <<'USAGE'
Usage: bash scripts/generate_v1_code_todo_triage.sh [options]

Options:
  --source-root <path>   Source tree to scan (default: src)
  --output-md <path>     Markdown output path (default: docs/generated/V1_CODE_TODO_TRIAGE.md)
  --output-csv <path>    CSV output path (default: docs/generated/V1_CODE_TODO_TRIAGE.csv)
  --strict               Exit non-zero if any item remains unclassified
  -h, --help             Show this help text
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-root)
      SOURCE_ROOT="${2:?missing value for --source-root}"
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

if command -v rg >/dev/null 2>&1; then
  matches=$(rg -n --no-heading "TODO|FIXME|HACK" "$SOURCE_ROOT" -g '*.rs' | sort || true)
else
  matches=$(grep -RIn --include='*.rs' -E "TODO|FIXME|HACK" "$SOURCE_ROOT" | sort || true)
fi

mkdir -p "$(dirname "$OUTPUT_MD")"
mkdir -p "$(dirname "$OUTPUT_CSV")"

classify_item() {
  local file="$1"
  local text="$2"
  local lower
  lower=$(printf '%s' "$text" | tr '[:upper:]' '[:lower:]')

  local severity owner bucket reason scope
  severity=""
  owner=""
  bucket=""
  reason=""
  scope="production"

  if [[ "$file" == *"/type_checker.rs" ]]; then
    severity="medium"
    owner="typing-owner"
    bucket="post-v1"
    reason="optional typing/type-inference backlog outside runtime enforcement path"
  elif [[ "$file" == *"/compiler.rs" ]]; then
    severity="high"
    owner="compiler-owner"
    bucket="v1"
    reason="compiler/runtime execution-path TODO impacting VM behavior or diagnostics"
  elif [[ "$file" == *"/vm.rs" ]]; then
    severity="high"
    owner="vm-owner"
    bucket="v1"
    reason="default runtime execution-path TODO affecting closure/generator correctness"
  elif [[ "$file" == *"/interpreter/native_functions/async_ops.rs" ]]; then
    severity="high"
    owner="runtime-async-owner"
    bucket="v1"
    reason="async native runtime TODO in script-facing execution flow"
  elif [[ "$file" == *"/interpreter/mod.rs" ]]; then
    severity="medium"
    owner="interpreter-owner"
    bucket="v1"
    reason="interpreter behavior TODO in script-facing runtime path"
  elif [[ "$file" == *"/jit.rs" ]]; then
    severity="low"
    owner="jit-owner"
    bucket="post-v1"
    reason="experimental JIT backlog outside default release-critical runtime path"
  elif [[ "$file" == *"/benchmarks/profiler.rs" ]]; then
    severity="low"
    owner="perf-owner"
    bucket="post-v1"
    reason="benchmark/profiler integration backlog outside production execution path"
    scope="non-production"
  fi

  if [[ "$lower" == *"spawnthread opcode"* || "$lower" == *"full closure upvalue"* || "$lower" == *"full generator state restoration"* ]]; then
    severity="high"
    bucket="v1"
  fi

  if [[ "$severity" == "" || "$owner" == "" || "$bucket" == "" ]]; then
    severity="unknown"
    owner="unassigned"
    bucket="unclassified"
    reason="classification rule missing"
  fi

  printf '%s|%s|%s|%s|%s\n' "$severity" "$owner" "$bucket" "$reason" "$scope"
}

echo "id,file,line,marker,summary,severity,owner,target_release_bucket,scope,rationale" > "$OUTPUT_CSV"

{
  echo "# V1 Code TODO/FIXME/HACK Triage"
  echo
  echo "Generated: $(date +%Y-%m-%d)"
  echo "Source root: \`$SOURCE_ROOT\`"
  echo
  echo "| ID | File | Line | Marker | Summary | Severity | Owner | Target Release Bucket | Scope | Rationale |"
  echo "| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- |"
} > "$OUTPUT_MD"

count=0
unclassified=0
while IFS= read -r match; do
  [[ -z "$match" ]] && continue
  IFS=':' read -r file line text <<< "$match"
  marker="TODO"
  if [[ "$text" == *"FIXME"* ]]; then
    marker="FIXME"
  elif [[ "$text" == *"HACK"* ]]; then
    marker="HACK"
  fi

  class_row=$(classify_item "$file" "$text")
  severity=${class_row%%|*}
  rest=${class_row#*|}
  owner=${rest%%|*}
  rest=${rest#*|}
  bucket=${rest%%|*}
  rest=${rest#*|}
  reason=${rest%%|*}
  scope=${rest##*|}

  if [[ "$bucket" == "unclassified" ]]; then
    unclassified=$((unclassified + 1))
  fi

  count=$((count + 1))
  id=$(printf 'V1TODO-%03d' "$count")
  summary=$(printf '%s' "$text" | sed 's/|/\\|/g')

  echo "| $id | \`$file\` | $line | $marker | $summary | $severity | $owner | $bucket | $scope | $reason |" >> "$OUTPUT_MD"
  printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$id" "$file" "$line" "$marker" "$(printf '%s' "$text" | tr '\n' ' ' | sed 's/,/;/g')" \
    "$severity" "$owner" "$bucket" "$scope" "$(printf '%s' "$reason" | sed 's/,/;/g')" >> "$OUTPUT_CSV"
done <<< "$matches"

{
  echo
  echo "Summary: \`$count\` markers triaged, \`$unclassified\` unclassified."
} >> "$OUTPUT_MD"

if [[ "$STRICT_MODE" -eq 1 && "$unclassified" -gt 0 ]]; then
  echo "error: Unclassified TODO/FIXME/HACK items found ($unclassified)." >&2
  exit 1
fi

echo "Generated $OUTPUT_MD and $OUTPUT_CSV"
