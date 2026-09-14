#!/usr/bin/env bash
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
BASELINE_SHA=1a7b6ef0eacd4918912ef58cfb5ec8533bff9e91
CURRENT_SHA=1395f49ff794707380728df3b80ed363788461c0
OUTPUT_DIR=${1:-$ROOT/benchmarks/results/hardening-2026-09}
EVAL_ROOT=${KUJO_EVAL_ROOT:-$(dirname "$ROOT")/eval}
WORK_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/kujo-hardening-eval.XXXXXX")
BASELINE_ROOT=$WORK_ROOT/baseline
CURRENT_ROOT=$WORK_ROOT/current

cleanup() {
    git -C "$ROOT" worktree remove --force "$BASELINE_ROOT" >/dev/null 2>&1 || true
    git -C "$ROOT" worktree remove --force "$CURRENT_ROOT" >/dev/null 2>&1 || true
}
trap cleanup EXIT

for command in git jq hyperfine rg cargo; do
    command -v "$command" >/dev/null || {
        echo "missing required command: $command" >&2
        exit 2
    }
done

test -f "$EVAL_ROOT/main.kujo" || {
    echo "Kujo Eval not found at $EVAL_ROOT; set KUJO_EVAL_ROOT" >&2
    exit 2
}

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR=$(cd "$OUTPUT_DIR" && pwd)
git -C "$ROOT" worktree add --detach "$BASELINE_ROOT" "$BASELINE_SHA" >/dev/null
git -C "$ROOT" worktree add --detach "$CURRENT_ROOT" "$CURRENT_SHA" >/dev/null

if ! git -C "$ROOT" diff --quiet "$BASELINE_SHA..$CURRENT_SHA" -- src modules config schemas tools install.sh Cargo.toml Cargo.lock; then
    echo "runtime inputs differ; build and benchmark each checkout independently" >&2
    exit 1
fi

mkdir -p "$BASELINE_ROOT/benchmarks" "$CURRENT_ROOT/benchmarks"
cp -R "$ROOT/benchmarks/hardening-2026-09" "$BASELINE_ROOT/benchmarks/"
cp -R "$ROOT/benchmarks/hardening-2026-09" "$CURRENT_ROOT/benchmarks/"

if [[ -n ${KUJO_HARDENING_BIN:-} ]]; then
    KUJO_BIN=$KUJO_HARDENING_BIN
else
    cargo build --release --bin kujo --manifest-path "$BASELINE_ROOT/Cargo.toml" -j "${CARGO_BUILD_JOBS:-2}"
    KUJO_BIN=$BASELINE_ROOT/target/release/kujo
fi
KUJO_BIN=$(cd "$(dirname "$KUJO_BIN")" && pwd)/$(basename "$KUJO_BIN")
"$KUJO_BIN" --version | rg -q '^kujo 1\.4\.0$'

(cd "$BASELINE_ROOT" && KUJO_BIN=$KUJO_BIN "$KUJO_BIN" run "$EVAL_ROOT/main.kujo" run benchmarks/hardening-2026-09/eval.json --output-dir "$OUTPUT_DIR/eval-baseline" --json > "$OUTPUT_DIR/eval-baseline.log") || true
(cd "$CURRENT_ROOT" && KUJO_BIN=$KUJO_BIN "$KUJO_BIN" run "$EVAL_ROOT/main.kujo" run benchmarks/hardening-2026-09/eval.json --output-dir "$OUTPUT_DIR/eval-current" --json > "$OUTPUT_DIR/eval-current.log")

"$KUJO_BIN" run "$ROOT/benchmarks/hardening-2026-09/context_metrics.kujo" -- "$BASELINE_ROOT/README.md" "$BASELINE_ROOT/INSTALLATION.md" "$BASELINE_ROOT/ROADMAP.md" > "$OUTPUT_DIR/context-baseline.json"
"$KUJO_BIN" run "$ROOT/benchmarks/hardening-2026-09/context_metrics.kujo" -- "$CURRENT_ROOT/README.md" "$CURRENT_ROOT/INSTALLATION.md" "$CURRENT_ROOT/ROADMAP.md" > "$OUTPUT_DIR/context-current.json"

run_pair() {
    local name=$1
    local runs=$2
    local baseline_command=$3
    local current_command=$4
    shift 4
    hyperfine --warmup 3 --runs "$runs" --export-json "$OUTPUT_DIR/runtime-$name.json" "$@" \
        --command-name baseline "$baseline_command" \
        --command-name current "$current_command"
}

run_pair minimal 30 "cd '$BASELINE_ROOT' && '$KUJO_BIN' --version" "cd '$CURRENT_ROOT' && '$KUJO_BIN' --version"
run_pair typical 30 "cd '$BASELINE_ROOT' && '$KUJO_BIN' run examples/string_interpolation.kujo" "cd '$CURRENT_ROOT' && '$KUJO_BIN' run examples/string_interpolation.kujo"
run_pair large 20 "cd '$BASELINE_ROOT' && '$KUJO_BIN' run benchmarks/hardening-2026-09/scale_workload.kujo -- 50000" "cd '$CURRENT_ROOT' && '$KUJO_BIN' run benchmarks/hardening-2026-09/scale_workload.kujo -- 50000"
run_pair stress 10 "cd '$BASELINE_ROOT' && '$KUJO_BIN' run benchmarks/hardening-2026-09/scale_workload.kujo -- 100000" "cd '$CURRENT_ROOT' && '$KUJO_BIN' run benchmarks/hardening-2026-09/scale_workload.kujo -- 100000"
run_pair failure 30 "cd '$BASELINE_ROOT' && '$KUJO_BIN' run benchmarks/hardening-2026-09/invalid_source.kujo" "cd '$CURRENT_ROOT' && '$KUJO_BIN' run benchmarks/hardening-2026-09/invalid_source.kujo" --ignore-failure

for iterations in 100 1000 10000 50000 100000; do
    run_pair "scale-$iterations" 10 \
        "cd '$BASELINE_ROOT' && '$KUJO_BIN' run benchmarks/hardening-2026-09/scale_workload.kujo -- $iterations" \
        "cd '$CURRENT_ROOT' && '$KUJO_BIN' run benchmarks/hardening-2026-09/scale_workload.kujo -- $iterations"
done

hyperfine --warmup 5 --runs 50 --export-json "$OUTPUT_DIR/search-latency.json" \
    --command-name baseline "cd '$BASELINE_ROOT' && rg -n -i 'security|untrusted|sandbox|allow-' README.md INSTALLATION.md ROADMAP.md" \
    --command-name current "cd '$CURRENT_ROOT' && rg -n -i 'security|untrusted|sandbox|allow-' README.md INSTALLATION.md ROADMAP.md"

: > "$OUTPUT_DIR/memory-baseline.txt"
: > "$OUTPUT_DIR/memory-current.txt"
for _ in $(seq 1 10); do
    (cd "$BASELINE_ROOT" && /usr/bin/time -lp "$KUJO_BIN" run benchmarks/hardening-2026-09/scale_workload.kujo -- 100000 >/dev/null) 2>> "$OUTPUT_DIR/memory-baseline.txt"
    (cd "$CURRENT_ROOT" && /usr/bin/time -lp "$KUJO_BIN" run benchmarks/hardening-2026-09/scale_workload.kujo -- 100000 >/dev/null) 2>> "$OUTPUT_DIR/memory-current.txt"
done

{
    uname -a
    rustc --version
    cargo --version
    "$KUJO_BIN" --version
    sysctl -n machdep.cpu.brand_string hw.ncpu hw.memsize 2>/dev/null || true
} > "$OUTPUT_DIR/environment.txt"

for tree in baseline current; do
    checkout=$WORK_ROOT/$tree
    printf '%s\n' "$tree"
    wc -l -w -c "$checkout/README.md" "$checkout/INSTALLATION.md" "$checkout/ROADMAP.md"
    for query in 'install|package|registry' 'security|untrusted|sandbox|allow-' 'release|version|stable'; do
        bytes=$(cd "$checkout" && rg -n -i "$query" README.md INSTALLATION.md ROADMAP.md | wc -c | tr -d ' ')
        lines=$(cd "$checkout" && rg -n -i "$query" README.md INSTALLATION.md ROADMAP.md | wc -l | tr -d ' ')
        printf '%s lines=%s bytes=%s\n' "$query" "$lines" "$bytes"
    done
done > "$OUTPUT_DIR/document-metrics.txt"

{
    for tree in baseline current; do
        checkout=$WORK_ROOT/$tree
        printf '%s\n' "$tree"
        printf 'tracked_files '
        git -C "$checkout" ls-files | wc -l
        printf 'rust_source_lines '
        git -C "$checkout" ls-files 'src/*.rs' 'src/**/*.rs' | sed "s#^#$checkout/#" | xargs wc -l | tail -1
        printf 'rust_test_lines '
        git -C "$checkout" ls-files 'tests/*.rs' | sed "s#^#$checkout/#" | xargs wc -l | tail -1
        printf 'direct_dependencies '
        cargo metadata --locked --format-version 1 --no-deps --manifest-path "$checkout/Cargo.toml" | jq '.packages[0].dependencies | length'
        printf 'lockfile_packages '
        awk '/^\[\[package\]\]/{n++} END{print n}' "$checkout/Cargo.lock"
        shasum -a 256 "$checkout/Cargo.toml" "$checkout/Cargo.lock"
    done
} > "$OUTPUT_DIR/source-metrics.txt"

echo "results written to $OUTPUT_DIR"
