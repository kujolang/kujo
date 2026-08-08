#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: bash scripts/generate_pre_v1_unresolved_inventory.sh [CHECKLIST_PATH] [OUTPUT_MD_PATH]

Generates an auditable unresolved-item inventory from the pre-v1 master checklist.
Outputs:
  - markdown table at OUTPUT_MD_PATH (default: docs/generated/PRE_V1_UNRESOLVED_INVENTORY.md)
  - CSV table at the same path with .csv extension

Exit codes:
  2: checklist contains an item ID without a source mapping
  3: checklist contains duplicate item IDs
  4: checklist contains no V1U checklist items
EOF
}

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHECKLIST_PATH="${1:-$ROOT/docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md}"
OUTPUT_MD_PATH="${2:-$ROOT/docs/generated/PRE_V1_UNRESOLVED_INVENTORY.md}"
OUTPUT_CSV_PATH="${OUTPUT_MD_PATH%.md}.csv"

if [[ "${CHECKLIST_PATH}" == "--help" || "${CHECKLIST_PATH}" == "-h" ]]; then
    usage
    exit 0
fi

owner_for_item() {
    local item_id="$1"
    case "$item_id" in
        V1U-RES-*) echo "release-audit" ;;
        V1U-OPEN-*) echo "release-owner" ;;
        V1U-GATE-*) echo "release-engineering" ;;
        V1U-RUN-*) echo "runtime-parity" ;;
        V1U-DOC-*) echo "docs-owner" ;;
        V1U-DG-*) echo "docgen-owner" ;;
        V1U-CODE-*) echo "runtime-owner" ;;
        V1U-FINAL-*) echo "release-owner" ;;
        *) echo "unassigned" ;;
    esac
}

classification_for_item() {
    local item_id="$1"
    local item_status="$2"
    local classification=""
    local rationale=""

    case "$item_id" in
        V1U-RES-002|V1U-RES-003|V1U-GATE-001|V1U-GATE-002|V1U-GATE-003|V1U-GATE-004|V1U-RUN-001|V1U-RUN-002|V1U-RUN-003|V1U-RUN-004|V1U-RUN-005|V1U-RUN-006|V1U-CODE-002|V1U-FINAL-001|V1U-FINAL-002|V1U-FINAL-003)
            classification="v1-blocker"
            rationale="Directly required to produce deterministic release behavior, runtime parity confidence, or final v1 release evidence."
            ;;
        V1U-OPEN-001|V1U-OPEN-004|V1U-DOC-001|V1U-DOC-002|V1U-DOC-003|V1U-DOC-004|V1U-DG-001|V1U-DG-002|V1U-DG-003|V1U-CODE-001|V1U-CODE-003)
            classification="v1-should-fix"
            rationale="High-value pre-v1 clarity or quality item that should be completed before release unless explicitly deferred with rationale."
            ;;
        V1U-OPEN-002|V1U-OPEN-003)
            classification="post-v1"
            rationale="Tag-time release-publication tasks that cannot be fully closed until the final v1 release event."
            ;;
        *)
            if [[ "$item_status" == "checked" ]]; then
                echo "archive|Already completed in the master checklist; retained for audit traceability."
                return 0
            fi
            return 1
            ;;
    esac

    if [[ "$item_status" == "checked" ]]; then
        classification="archive"
        rationale="Already completed in the master checklist; retained for audit traceability. Historical rationale: ${rationale}"
    fi

    echo "${classification}|${rationale}"
}

source_refs_for_item() {
    local item_id="$1"
    case "$item_id" in
        V1U-RES-*)
            echo "docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md;docs/PRE_V1_ACTION_CHECKLIST.md;ROADMAP.md;docs/RELEASE_PROCESS.md;docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md;docs/UNFINISHED_AND_MVP_AUDIT.md;docs/VM_INTERPRETER_PARITY_MATRIX.md;README.md;notes/2026-05-20_09-07_prev1-rel-001-rc-gate-evidence.md"
            ;;
        V1U-OPEN-001)
            echo "docs/PRE_V1_ACTION_CHECKLIST.md;notes/2026-05-12_23-23_NO-ROADMAP_named-nested-closure-capture-parity.md"
            ;;
        V1U-OPEN-002)
            echo "ROADMAP.md;docs/UNFINISHED_AND_MVP_AUDIT.md"
            ;;
        V1U-OPEN-003)
            echo "docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md;docs/RELEASE_PROCESS.md"
            ;;
        V1U-OPEN-004)
            echo "ROADMAP.md;docs/DOCGEN.md"
            ;;
        V1U-GATE-*)
            echo "scripts/release_candidate_gate.sh;scripts/release_gate.sh;docs/RELEASE_PROCESS.md;notes/2026-05-20_09-07_prev1-rel-001-rc-gate-evidence.md"
            ;;
        V1U-RUN-*)
            echo "README.md;docs/VM_INTERPRETER_PARITY_MATRIX.md;src/main.rs;src/parser.rs;tests/vm_interpreter_parity_surfaces.rs"
            ;;
        V1U-DOC-001)
            echo "docs/ARCHITECTURE.md;README.md;docs/LANGUAGE_SPEC.md"
            ;;
        V1U-DOC-002)
            echo "README.md;docs/V1_SCOPE.md;docs/LANGUAGE_SPEC.md;docs/KUJO_FEATURE_INVENTORY.md;docs/UNFINISHED_AND_MVP_AUDIT.md"
            ;;
        V1U-DOC-003)
            echo "docs/STANDARD_LIBRARY_REFERENCE.md;docs/V1_SCOPE.md;docs/STANDARD_LIBRARY.md"
            ;;
        V1U-DOC-004)
            echo "tests/readme_contracts.rs;tests/language_spec_contracts.rs;tests/release_process_docs_contract.rs;tests/v1_scope_docs_alignment.rs"
            ;;
        V1U-DG-*)
            echo "ROADMAP.md;docs/DOCGEN.md;tests/docgen_universal.rs"
            ;;
        V1U-CODE-*)
            echo "src/interpreter.rs;src/vm.rs;src/compiler.rs;src/builtins.rs;docs/V1_SCOPE.md"
            ;;
        V1U-FINAL-*)
            echo "docs/RELEASE_PROCESS.md;docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md;scripts/release_candidate_gate.sh;scripts/release_gate.sh"
            ;;
        *)
            return 1
            ;;
    esac
}

latest_touched_date_for_refs() {
    local refs_csv="$1"
    local latest_date=""

    IFS=';' read -r -a refs <<<"$refs_csv"
    for ref in "${refs[@]}"; do
        local file_date=""
        if [[ -f "$ROOT/$ref" ]]; then
            file_date="$(git -C "$ROOT" log -1 --format=%cs -- "$ref" 2>/dev/null || true)"
            if [[ -z "$file_date" ]]; then
                file_date="$(date '+%Y-%m-%d')"
            fi
            if [[ -z "$latest_date" || "$file_date" > "$latest_date" ]]; then
                latest_date="$file_date"
            fi
        fi
    done

    if [[ -z "$latest_date" ]]; then
        latest_date="unknown"
    fi

    echo "$latest_date"
}

if [[ ! -f "$CHECKLIST_PATH" ]]; then
    echo "error: checklist file not found: $CHECKLIST_PATH" >&2
    exit 1
fi

declare -a item_statuses
declare -a item_ids
declare -a item_titles
item_count=0

while IFS= read -r line; do
    if [[ "$line" =~ ^-\ \[([[:space:]xX])\]\ \*\*(V1U-[A-Z]+-[0-9]{3})\*\*:\ (.+)$ ]]; then
        raw_state="${BASH_REMATCH[1]}"
        item_id="${BASH_REMATCH[2]}"
        item_title="${BASH_REMATCH[3]}"

        for existing_id in "${item_ids[@]-}"; do
            if [[ "$existing_id" == "$item_id" ]]; then
                echo "error: duplicate checklist item id encountered: $item_id" >&2
                exit 3
            fi
        done

        item_state="unchecked"
        if [[ "$raw_state" != " " ]]; then
            item_state="checked"
        fi

        item_statuses+=("$item_state")
        item_ids+=("$item_id")
        item_titles+=("$item_title")
        item_count=$((item_count + 1))
    fi
done < "$CHECKLIST_PATH"

if [[ "$item_count" -eq 0 ]]; then
    echo "error: no V1U checklist items found in $CHECKLIST_PATH" >&2
    exit 4
fi

mkdir -p "$(dirname "$OUTPUT_MD_PATH")"

{
    echo "# Pre-v1 Unresolved Item Inventory"
    echo
    echo "- Generated: $(date '+%Y-%m-%d %H:%M:%S %Z')"
    echo "- Source checklist: ${CHECKLIST_PATH#$ROOT/}"
    echo "- Total tracked items: ${item_count}"
    echo
    echo "| Item ID | Status | Classification | Rationale | Summary | Source References | Last Touched | Current Owner |"
    echo "| --- | --- | --- | --- | --- | --- | --- | --- |"
} > "$OUTPUT_MD_PATH"

echo "item_id,status,classification,rationale,summary,source_references,last_touched,current_owner" > "$OUTPUT_CSV_PATH"

for ((idx = 0; idx < item_count; idx++)); do
    item_id="${item_ids[$idx]}"
    item_status="${item_statuses[$idx]}"
    item_title="${item_titles[$idx]}"
    owner="$(owner_for_item "$item_id")"

    if ! source_refs="$(source_refs_for_item "$item_id")"; then
        echo "error: no source mapping configured for checklist item id: $item_id" >&2
        exit 2
    fi

    last_touched="$(latest_touched_date_for_refs "$source_refs")"
    source_refs_pretty="${source_refs//;/, }"

    if ! classification_payload="$(classification_for_item "$item_id" "$item_status")"; then
        echo "error: no classification mapping configured for checklist item id: $item_id" >&2
        exit 2
    fi
    classification="${classification_payload%%|*}"
    rationale="${classification_payload#*|}"

    printf '| `%s` | %s | `%s` | %s | %s | %s | %s | %s |\n' \
        "$item_id" \
        "$item_status" \
        "$classification" \
        "$rationale" \
        "$item_title" \
        "$source_refs_pretty" \
        "$last_touched" \
        "$owner" >> "$OUTPUT_MD_PATH"

    csv_summary="${item_title//\"/\"\"}"
    csv_sources="${source_refs_pretty//\"/\"\"}"
    csv_rationale="${rationale//\"/\"\"}"
    printf '"%s","%s","%s","%s","%s","%s","%s","%s"\n' \
        "$item_id" \
        "$item_status" \
        "$classification" \
        "$csv_rationale" \
        "$csv_summary" \
        "$csv_sources" \
        "$last_touched" \
        "$owner" >> "$OUTPUT_CSV_PATH"
done

echo "generated inventory: ${OUTPUT_MD_PATH#$ROOT/}"
echo "generated inventory csv: ${OUTPUT_CSV_PATH#$ROOT/}"
