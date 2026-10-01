#!/usr/bin/env bash
set -euo pipefail

SUITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KUJO_REPO="$(cd "$SUITE_DIR/../../.." && pwd)"
RESULTS_ROOT="$KUJO_REPO/benchmarks/cross-language/results"
RUN_ID="kujo-go-runtime-$(date -u +%Y%m%dT%H%M%SZ)"
RESULTS_DIR="${RESULTS_DIR:-$RESULTS_ROOT/$RUN_ID}"
BIN_DIR="$RESULTS_DIR/bin"

BENCH_WARMUP="${BENCH_WARMUP:-1}"
BENCH_RUNS="${BENCH_RUNS:-5}"
STARTUP_WARMUP="${STARTUP_WARMUP:-3}"
STARTUP_RUNS="${STARTUP_RUNS:-15}"

for command_name in cargo go hyperfine jq git; do
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "missing required command: $command_name" >&2
        exit 1
    fi
done

sha256_file() {
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    elif command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        echo "missing required command: shasum or sha256sum" >&2
        return 1
    fi
}

mkdir -p "$BIN_DIR"

KUJO_COMMIT="$(git -C "$KUJO_REPO" rev-parse HEAD)"
KUJO_DIRTY="false"
if [[ -n "$(git -C "$KUJO_REPO" status --porcelain --untracked-files=no)" ]]; then
    KUJO_DIRTY="true"
fi

if [[ "$KUJO_DIRTY" == "true" && "${ALLOW_DIRTY_KUJO:-0}" != "1" ]]; then
    echo "Kujo tracked files are dirty; commit them or set ALLOW_DIRTY_KUJO=1" >&2
    exit 1
fi

BUILD_TIMING="$RESULTS_DIR/kujo-release-build.time"
(
    cd "$KUJO_REPO"
    /usr/bin/time -p cargo build --release --locked --bin kujo
) 2>"$BUILD_TIMING"

cp "$KUJO_REPO/target/release/kujo" "$BIN_DIR/kujo"
KUJO_BIN="$BIN_DIR/kujo"

workloads=(startup prime_count integer_mix)
for workload in "${workloads[@]}"; do
    go build -trimpath -o "$BIN_DIR/$workload-go" "$SUITE_DIR/$workload.go"
done

CORRECTNESS_TSV="$RESULTS_DIR/correctness.tsv"
printf 'workload\tkujo_vm_output\tkujo_jit_output\tgo_output\texpected\tstatus\n' >"$CORRECTNESS_TSV"

for workload in "${workloads[@]}"; do
    kujo_vm_output="$($KUJO_BIN run "$SUITE_DIR/$workload.kujo")"
    kujo_jit_output="$($KUJO_BIN run --jit "$SUITE_DIR/$workload.kujo" 2>/dev/null)"
    go_output="$($BIN_DIR/$workload-go)"

    expected="$(<"$SUITE_DIR/expected/$workload.txt")"

    status="pass"
    if [[ "$kujo_vm_output" != "$expected" || "$kujo_jit_output" != "$expected" || "$go_output" != "$expected" ]]; then
        status="fail"
    fi

    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$workload" "$kujo_vm_output" "$kujo_jit_output" "$go_output" "$expected" "$status" \
        >>"$CORRECTNESS_TSV"

    if [[ "$status" != "pass" ]]; then
        echo "correctness check failed for $workload" >&2
        exit 1
    fi
done

hyperfine \
    --warmup "$STARTUP_WARMUP" \
    --runs "$STARTUP_RUNS" \
    --command-name "Kujo startup" "$KUJO_BIN run $SUITE_DIR/startup.kujo" \
    --command-name "Kujo JIT startup" "$KUJO_BIN run --jit $SUITE_DIR/startup.kujo 2>/dev/null" \
    --command-name "Go startup" "$BIN_DIR/startup-go" \
    --export-json "$RESULTS_DIR/startup.json" \
    --export-markdown "$RESULTS_DIR/startup.md"

for workload in prime_count integer_mix; do
    compile_output="$BIN_DIR/$workload-compile-go"

    hyperfine \
        --warmup "$BENCH_WARMUP" \
        --runs "$BENCH_RUNS" \
        --command-name "Kujo bytecode compile: $workload" \
            "$KUJO_BIN check --quiet $SUITE_DIR/$workload.kujo" \
        --export-json "$RESULTS_DIR/$workload-kujo-compile.json" \
        --export-markdown "$RESULTS_DIR/$workload-kujo-compile.md"

    hyperfine \
        --warmup "$BENCH_WARMUP" \
        --runs "$BENCH_RUNS" \
        --prepare "rm -f $compile_output" \
        --command-name "Go native compile: $workload" \
            "go build -trimpath -o $compile_output $SUITE_DIR/$workload.go" \
        --export-json "$RESULTS_DIR/$workload-go-compile.json" \
        --export-markdown "$RESULTS_DIR/$workload-go-compile.md"

    hyperfine \
        --warmup "$BENCH_WARMUP" \
        --runs "$BENCH_RUNS" \
        --command-name "Kujo run: $workload" \
            "$KUJO_BIN run $SUITE_DIR/$workload.kujo" \
        --command-name "Kujo JIT run: $workload" \
            "$KUJO_BIN run --jit $SUITE_DIR/$workload.kujo 2>/dev/null" \
        --command-name "Go run: $workload" \
            "$BIN_DIR/$workload-go" \
        --export-json "$RESULTS_DIR/$workload-run.json" \
        --export-markdown "$RESULTS_DIR/$workload-run.md"
done

KUJO_VERSION="$($KUJO_BIN --version)"
GO_VERSION="$(go version)"
HYPERFINE_VERSION="$(hyperfine --version)"
if command -v sw_vers >/dev/null 2>&1; then
    OS_VERSION="macOS $(sw_vers -productVersion)"
else
    OS_VERSION="$(uname -sr)"
fi
ARCH="$(uname -m)"
if command -v sysctl >/dev/null 2>&1; then
    CPU="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)"
elif command -v lscpu >/dev/null 2>&1; then
    CPU="$(lscpu | awk -F: '/Model name/ {sub(/^[[:space:]]+/, "", $2); print $2; exit}')"
else
    CPU=""
fi

jq -n \
    --arg run_id "$RUN_ID" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg kujo_commit "$KUJO_COMMIT" \
    --argjson kujo_dirty "$KUJO_DIRTY" \
    --arg kujo_version "$KUJO_VERSION" \
    --arg kujo_sha256 "$(sha256_file "$KUJO_BIN")" \
    --arg go_version "$GO_VERSION" \
    --arg hyperfine_version "$HYPERFINE_VERSION" \
    --arg os_version "$OS_VERSION" \
    --arg arch "$ARCH" \
    --arg cpu "$CPU" \
    --argjson benchmark_warmups "$BENCH_WARMUP" \
    --argjson benchmark_runs "$BENCH_RUNS" \
    --argjson startup_warmups "$STARTUP_WARMUP" \
    --argjson startup_runs "$STARTUP_RUNS" \
    '{
        run_id: $run_id,
        recorded_at: $recorded_at,
        kujo: {
            commit: $kujo_commit,
            dirty: $kujo_dirty,
            version: $kujo_version,
            binary_sha256: $kujo_sha256,
            build: "cargo build --release --locked --bin kujo"
        },
        go: {
            version: $go_version,
            build: "go build -trimpath"
        },
        hyperfine: $hyperfine_version,
        host: {os_version: $os_version, arch: $arch, cpu: $cpu},
        sampling: {
            benchmark_warmups: $benchmark_warmups,
            benchmark_runs: $benchmark_runs,
            startup_warmups: $startup_warmups,
            startup_runs: $startup_runs
        }
    }' >"$RESULTS_DIR/metadata.json"

SUMMARY="$RESULTS_DIR/summary.md"
{
    echo "# Kujo and Go runtime benchmark"
    echo
    echo "Run: \`$RUN_ID\`"
    echo
    echo "All correctness checks passed. Times below are Hyperfine medians."
    echo
    echo "| Phase | Workload | Kujo VM | Kujo JIT | Go | Kujo JIT / Go |"
    echo "|---|---:|---:|---:|---:|---:|"

    startup_vm="$(jq '.results[0].median' "$RESULTS_DIR/startup.json")"
    startup_jit="$(jq '.results[1].median' "$RESULTS_DIR/startup.json")"
    startup_go="$(jq '.results[2].median' "$RESULTS_DIR/startup.json")"
    startup_ratio="$(jq -n --argjson jit "$startup_jit" --argjson go "$startup_go" '$jit / $go')"
    printf '| Startup | trivial | %.3f ms | %.3f ms | %.3f ms | %.2fx |\n' \
        "$(jq -n --argjson value "$startup_vm" '$value * 1000')" \
        "$(jq -n --argjson value "$startup_jit" '$value * 1000')" \
        "$(jq -n --argjson value "$startup_go" '$value * 1000')" \
        "$startup_ratio"

    for workload in prime_count integer_mix; do
        kujo_compile="$(jq '.results[0].median' "$RESULTS_DIR/$workload-kujo-compile.json")"
        go_compile="$(jq '.results[0].median' "$RESULTS_DIR/$workload-go-compile.json")"
        printf '| Compile | %s | %.3f ms | n/a | %.3f ms | n/a |\n' \
            "$workload" \
            "$(jq -n --argjson value "$kujo_compile" '$value * 1000')" \
            "$(jq -n --argjson value "$go_compile" '$value * 1000')"

        kujo_vm_run="$(jq '.results[0].median' "$RESULTS_DIR/$workload-run.json")"
        kujo_jit_run="$(jq '.results[1].median' "$RESULTS_DIR/$workload-run.json")"
        go_run="$(jq '.results[2].median' "$RESULTS_DIR/$workload-run.json")"
        run_ratio="$(jq -n --argjson jit "$kujo_jit_run" --argjson go "$go_run" '$jit / $go')"
        printf '| End-to-end run | %s | %.3f ms | %.3f ms | %.3f ms | %.2fx |\n' \
            "$workload" \
            "$(jq -n --argjson value "$kujo_vm_run" '$value * 1000')" \
            "$(jq -n --argjson value "$kujo_jit_run" '$value * 1000')" \
            "$(jq -n --argjson value "$go_run" '$value * 1000')" \
            "$run_ratio"
    done

    echo
    echo "Compile rows are not equivalent artifacts: Kujo checks and compiles to"
    echo "in-memory bytecode, while Go emits a reusable native binary. Kujo VM and"
    echo "JIT run rows still include startup and source compilation. See the raw Markdown"
    echo "and JSON files for means, standard deviations, ranges, and commands."
} >"$SUMMARY"

echo "$RESULTS_DIR"
