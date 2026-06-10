#!/usr/bin/env bash
set -euo pipefail

OUTPUT_MD="docs/generated/UNSAFE_INVENTORY.md"
OUTPUT_CSV="docs/generated/UNSAFE_INVENTORY.csv"
STRICT=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-md)
      OUTPUT_MD="$2"
      shift 2
      ;;
    --output-csv)
      OUTPUT_CSV="$2"
      shift 2
      ;;
    --strict)
      STRICT=1
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

mkdir -p "$(dirname "$OUTPUT_MD")" "$(dirname "$OUTPUT_CSV")"

TMP_MATCHES="$(mktemp)"
trap 'rm -f "$TMP_MATCHES"' EXIT

SEARCH_COMMAND=""
if command -v rg >/dev/null 2>&1; then
  SEARCH_COMMAND="rg -n --glob '*.rs' --glob '!tests/unsafe_inventory_contract.rs' '\\bunsafe\\b' src tests benches fuzz"
  rg -n --glob '*.rs' --glob '!tests/unsafe_inventory_contract.rs' '\bunsafe\b' src tests benches fuzz > "$TMP_MATCHES" || true
else
  SEARCH_COMMAND="grep -RIn --include='*.rs' --exclude='unsafe_inventory_contract.rs' 'unsafe' src tests benches fuzz"
  grep -RIn --include='*.rs' --exclude='unsafe_inventory_contract.rs' 'unsafe' src tests benches fuzz > "$TMP_MATCHES" || true
fi
sort -t: -k1,1 -k2,2n "$TMP_MATCHES" -o "$TMP_MATCHES"

TOTAL=0
EXECUTABLE=0
NON_EXECUTABLE=0
UNKNOWN=0

{
  echo "path,line,kind,classification,text"

  while IFS=: read -r path line text; do
    [[ -z "${path}" ]] && continue
    TOTAL=$((TOTAL + 1))

    kind="non_executable"
    if [[ "$text" =~ unsafe[[:space:]]*\{ ]] || [[ "$text" =~ unsafe[[:space:]]+extern ]] || [[ "$text" =~ (^|[[:space:]])pub[[:space:]]+unsafe ]] || [[ "$text" =~ (^|[[:space:]])unsafe[[:space:]]+fn ]]; then
      kind="executable"
      EXECUTABLE=$((EXECUTABLE + 1))
    else
      NON_EXECUTABLE=$((NON_EXECUTABLE + 1))
    fi

    classification="unknown"
    case "$path" in
      src/jit.rs)
        if [[ "$kind" == "executable" ]]; then
          classification="jit_executable"
        else
          classification="jit_comment_or_doc"
        fi
        ;;
      src/vm.rs)
        if [[ "$kind" == "executable" ]]; then
          classification="vm_executable"
        else
          classification="vm_comment_or_doc"
        fi
        ;;
      tests/*)
        if [[ "$kind" == "executable" ]]; then
          classification="test_executable"
        else
          classification="test_comment_or_string"
        fi
        ;;
      src/*)
        if [[ "$kind" == "executable" ]]; then
          classification="src_executable_other"
        else
          classification="src_comment_or_string"
        fi
        ;;
      *)
        classification="other"
        ;;
    esac

    if [[ "$classification" == "unknown" ]]; then
      UNKNOWN=$((UNKNOWN + 1))
    fi

    escaped_text="${text//\"/\"\"}"
    printf '"%s",%s,"%s","%s","%s"\n' "$path" "$line" "$kind" "$classification" "$escaped_text"
  done < "$TMP_MATCHES"
} > "$OUTPUT_CSV"

if [[ "$STRICT" -eq 1 && "$UNKNOWN" -gt 0 ]]; then
  echo "Unclassified unsafe inventory rows: $UNKNOWN" >&2
  exit 1
fi

{
  echo "# Unsafe Inventory"
  echo
  echo "Generated: $(date +%Y-%m-%d)"
  echo "Command: $SEARCH_COMMAND"
  echo
  echo "## Summary"
  echo
  echo "- Total matches: $TOTAL"
  echo "- Executable matches: $EXECUTABLE"
  echo "- Non-executable matches: $NON_EXECUTABLE"
  echo "- Unknown classifications: $UNKNOWN"
  echo
  echo "## Rows"
  echo
  echo "| Path | Line | Kind | Classification | Text |"
  echo "| --- | ---: | --- | --- | --- |"
  while IFS=, read -r c_path c_line c_kind c_class c_text; do
    if [[ "$c_path" == "path" ]]; then
      continue
    fi
    path="${c_path#\"}"; path="${path%\"}"
    kind="${c_kind#\"}"; kind="${kind%\"}"
    class="${c_class#\"}"; class="${class%\"}"
    text="${c_text#\"}"; text="${text%\"}"
    text="${text//|/\\|}"
    echo "| $path | $c_line | $kind | $class | $text |"
  done < "$OUTPUT_CSV"
} > "$OUTPUT_MD"

echo "Generated $OUTPUT_MD and $OUTPUT_CSV"
