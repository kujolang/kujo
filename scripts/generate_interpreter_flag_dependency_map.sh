#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_PATH="${1:-$ROOT/docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md}"
TMP_MATCHES="$(mktemp)"
TMP_ROWS="$(mktemp)"
trap 'rm -f "$TMP_MATCHES" "$TMP_ROWS"' EXIT

cd "$ROOT"

SEARCH_COMMAND=""
if command -v rg >/dev/null 2>&1; then
    SEARCH_COMMAND='rg -n -- "--interpreter" src tests docs README.md ROADMAP.md examples notes .github'
    rg -n -- "--interpreter" src tests docs README.md ROADMAP.md examples notes .github \
        | rg -v '^docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md:' >"$TMP_MATCHES"
else
    SEARCH_COMMAND='grep -RIn -- "--interpreter" src tests docs README.md ROADMAP.md examples notes .github'
    grep -RIn -- "--interpreter" src tests docs README.md ROADMAP.md examples notes .github 2>/dev/null \
        | grep -v '^docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md:' >"$TMP_MATCHES"
fi

if [[ ! -s "$TMP_MATCHES" ]]; then
    echo "error: no --interpreter references found in expected search targets" >&2
    exit 1
fi

awk -F: '
function classify(path,    category, tags) {
    if (path == "src/parser.rs") {
        category = "cli-harness"
        tags = "harness-legacy,parity-gap"
    } else if (path == "tests/native_api_security_boundaries.rs" || path == "tests/runtime_security.rs") {
        category = "integration-test"
        tags = "security-test-choice"
    } else if (path == "tests/docs_examples.rs") {
        category = "integration-test"
        tags = "docs-smoke,harness-legacy"
    } else if (path == "tests/diagnostics_golden.rs") {
        category = "integration-test"
        tags = "diagnostics-diff,harness-legacy"
    } else if (path == "tests/package_module_workflow_integration.rs") {
        category = "integration-test"
        tags = "harness-legacy,package-workflow"
    } else if (path ~ /^tests\//) {
        category = "integration-test"
        tags = "harness-legacy"
    } else if (path == "README.md" || path == "ROADMAP.md" || path ~ /^docs\//) {
        category = "documentation"
        tags = "docs-contract"
    } else if (path ~ /^examples\//) {
        category = "example-doc"
        tags = "benchmark-baseline"
    } else if (path ~ /^notes\//) {
        category = "notes-history"
        tags = "archive-note"
    } else {
        category = "other"
        tags = "manual-review"
    }
    return category "|" tags
}

{
    path = $1
    line = $2
    split(classify(path), parts, "|")
    category[path] = parts[1]
    tags[path] = parts[2]
    count[path] += 1
    if (lines[path] == "") {
        lines[path] = line
    } else {
        lines[path] = lines[path] "," line
    }
}

END {
    for (path in count) {
        printf "%s\t%s\t%s\t%d\t%s\n", path, category[path], tags[path], count[path], lines[path]
    }
}
' "$TMP_MATCHES" | LC_ALL=C sort -t$'\t' -k1,1 >"$TMP_ROWS"

parity_gap_count="$(awk -F'\t' '$3 ~ /(^|,)parity-gap($|,)/ {count++} END {print count + 0}' "$TMP_ROWS")"
parity_gap_rows="$(awk -F'\t' '$3 ~ /(^|,)parity-gap($|,)/ {printf "- `%s` (%s)\n", $1, $3}' "$TMP_ROWS")"

mkdir -p "$(dirname "$OUTPUT_PATH")"

{
    echo "# Interpreter Flag Dependency Map"
    echo
    echo "- Generated: $(date '+%Y-%m-%d %H:%M:%S %Z')"
    echo "- Command: \`${SEARCH_COMMAND}\`"
    echo
    echo "Reason tags:"
    echo "- \`harness-legacy\`: Existing harness behavior still forces interpreter mode."
    echo "- \`parity-gap\`: Runtime path currently depends on an explicitly tracked interpreter/VM parity or output-contract gap."
    echo "- \`security-test-choice\`: Security-boundary regression intentionally exercises interpreter path."
    echo "- \`diagnostics-diff\`: Diagnostic contract coverage currently pins interpreter output shape."
    echo "- \`docs-smoke\`: Docs/example smoke harness runs interpreter as canonical execution path."
    echo "- \`package-workflow\`: Package/module workflow integration still validated via interpreter runs."
    echo "- \`docs-contract\`: User-facing docs explicitly describe interpreter mode behavior."
    echo "- \`benchmark-baseline\`: Example/benchmark docs keep interpreter as baseline comparator."
    echo "- \`archive-note\`: Historical field notes mentioning interpreter usage."
    echo
    echo "| File | Category | Reason Tags | Usage Count | Line References |"
    echo "| --- | --- | --- | --- | --- |"
    while IFS=$'\t' read -r path category tags count line_refs; do
        printf '| `%s` | %s | `%s` | %s | %s |\n' "$path" "$category" "$tags" "$count" "$line_refs"
    done <"$TMP_ROWS"
    echo
    echo "## V1U-RUN-005: Parity-Gap Coverage Status"
    echo
    echo "- Current \`parity-gap\` tagged entries: ${parity_gap_count}"
    if [[ "$parity_gap_count" -gt 0 ]]; then
        echo "- Tagged surfaces:"
        printf "%s\n" "$parity_gap_rows"
        echo "- Coverage expectation: each tagged surface must have parity tests or explicit documented divergence."
        echo "- Current closure evidence paths:"
        echo "  - \`tests/cli_contracts.rs\` (bounded runtime fallback contracts)"
        echo "  - \`tests/vm_interpreter_parity_surfaces.rs\` (generator divergence contract)"
        echo "  - \`README.md\` and \`docs/VM_INTERPRETER_PARITY_MATRIX.md\` (canonical divergence docs)"
    else
        echo "- No current \`parity-gap\` tags remain in tracked interpreter-flag surfaces."
    fi
    cat <<'EOF'

## V1U-RUN-002: `kujo test` Runtime Strategy Status

Current state (`src/parser.rs::run_all_tests`): `kujo test` supports explicit runtime strategy selection via `--runtime dual|vm|interpreter` (default `dual`), with VM-primary execution and bounded interpreter fallback in dual mode.

Current rationale:

- Snapshot corpus compatibility still matters because many `tests/*.out` files were created under interpreter-first historical behavior.
- Runtime-path drift remains measurable for part of the legacy fixture corpus, but the harness is no longer blanket interpreter-pinned.
- Command-level runtime strategy behavior is tracked in `docs/VM_INTERPRETER_PARITY_MATRIX.md`.

Import-reliability clarification:

- Dotted and flat module imports are supported in both VM and interpreter runtime paths.
- `--interpreter` is not required for ordinary multi-module import layouts; it remains an explicit fallback/debug mode while fixture parity burn-down continues.

VM-first practical recommendations:

- Use `kujo run <file>` as the default VM-first path for ordinary modular projects.
- Use `kujo test --runtime dual` for compatibility sweeps where fallback visibility matters.
- Use `kujo test --runtime vm` for strict migration/parity gating.
- Use `--interpreter` only for explicit compatibility/debug isolation.
EOF
} >"$OUTPUT_PATH"

echo "generated interpreter dependency map: ${OUTPUT_PATH#$ROOT/}"
