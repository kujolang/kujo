#!/usr/bin/env bash
# Reuse an optimized build's dependencies without rebuilding the entire runtime as a test dependency.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DEPS_DIR="$ROOT_DIR/target/release/deps"
KUJO_BIN="${KUJO_BIN:-$ROOT_DIR/target/release/kujo}"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "$TEST_DIR"' EXIT
select_rlib() {
    local name="$1" candidate selected=""
    for candidate in "$DEPS_DIR"/lib"$name"-*.rlib; do
        [[ -f "$candidate" ]] || continue
        if [[ -z "$selected" || "$candidate" -nt "$selected" ]]; then
            selected="$candidate"
        fi
    done
    [[ -n "$selected" ]] || { echo "Build release dependencies first (cargo build --release)." >&2; return 1; }
    printf '%s' "$selected"
}
CARGO_BIN_EXE_kujo="$KUJO_BIN" rustc --test --edition=2021 -C lto=thin \
    "$ROOT_DIR/tests/installed_tool_imports.rs" -L "dependency=$DEPS_DIR" \
    --extern "tempfile=$(select_rlib tempfile)" --extern "serde_json=$(select_rlib serde_json)" \
    -o "$TEST_DIR/import-tests"
"$TEST_DIR/import-tests" --nocapture
