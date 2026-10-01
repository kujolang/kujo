#!/usr/bin/env bash
set -euo pipefail

SUITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KUJO_REPO="$(cd "$SUITE_DIR/../../.." && pwd)"
RESULTS_ROOT="$KUJO_REPO/benchmarks/cross-language/results"
RUN_ID="${RUN_ID:-kujo-four-language-$(date -u +%Y%m%dT%H%M%SZ)}"
RESULTS_DIR="${RESULTS_DIR:-$RESULTS_ROOT/$RUN_ID}"
BIN_DIR="$RESULTS_DIR/bin"

BENCH_WARMUP="${BENCH_WARMUP:-3}"
BENCH_RUNS="${BENCH_RUNS:-15}"
STARTUP_WARMUP="${STARTUP_WARMUP:-5}"
STARTUP_RUNS="${STARTUP_RUNS:-25}"

for command_name in cargo go hyperfine jq git python3 php; do
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "missing required command: $command_name" >&2
        exit 1
    fi
done

sha256_file() {
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        sha256sum "$1" | awk '{print $1}'
    fi
}

median_seconds() {
    jq ".results[$2].median" "$1"
}

milliseconds() {
    jq -n --argjson value "$1" '$value * 1000'
}

ratio() {
    jq -n --argjson numerator "$1" --argjson denominator "$2" '$numerator / $denominator'
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

(
    cd "$KUJO_REPO"
    /usr/bin/time -p cargo build --release --locked --no-default-features \
        --features runtime-jit --bin kujo-run
) 2>"$RESULTS_DIR/kujo-run-release-build.time"
cp "$KUJO_REPO/target/release/kujo-run" "$BIN_DIR/kujo-run"
KUJO_BIN="$BIN_DIR/kujo-run"

workloads=(startup prime_count integer_mix)
for workload in "${workloads[@]}"; do
    go build -trimpath -o "$BIN_DIR/$workload-go" "$SUITE_DIR/$workload.go"
done

CORRECTNESS_TSV="$RESULTS_DIR/correctness.tsv"
printf 'workload\tkujo_jit_output\tgo_output\tpython_output\tphp_output\texpected\tstatus\n' \
    >"$CORRECTNESS_TSV"
for workload in "${workloads[@]}"; do
    kujo_output="$($KUJO_BIN --jit "$SUITE_DIR/$workload.kujo")"
    go_output="$($BIN_DIR/$workload-go)"
    python_output="$(python3 -B "$SUITE_DIR/$workload.py")"
    php_output="$(php "$SUITE_DIR/$workload.php")"
    expected="$(<"$SUITE_DIR/expected/$workload.txt")"
    status="pass"
    if [[ "$kujo_output" != "$expected" || "$go_output" != "$expected" \
        || "$python_output" != "$expected" || "$php_output" != "$expected" ]]; then
        status="fail"
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$workload" "$kujo_output" "$go_output" "$python_output" "$php_output" \
        "$expected" "$status" >>"$CORRECTNESS_TSV"
    if [[ "$status" != "pass" ]]; then
        echo "correctness check failed for $workload" >&2
        exit 1
    fi
done

hyperfine \
    --shell=none \
    --warmup "$STARTUP_WARMUP" \
    --runs "$STARTUP_RUNS" \
    --command-name "Kujo lean JIT" "$KUJO_BIN --jit $SUITE_DIR/startup.kujo" \
    --command-name "Go" "$BIN_DIR/startup-go" \
    --command-name "Python" "python3 -B $SUITE_DIR/startup.py" \
    --command-name "PHP" "php $SUITE_DIR/startup.php" \
    --export-json "$RESULTS_DIR/startup.json" \
    --export-markdown "$RESULTS_DIR/startup.md"

for workload in prime_count integer_mix; do
    hyperfine \
        --shell=none \
        --warmup "$BENCH_WARMUP" \
        --runs "$BENCH_RUNS" \
        --command-name "Kujo lean JIT" "$KUJO_BIN --jit $SUITE_DIR/$workload.kujo" \
        --command-name "Go" "$BIN_DIR/$workload-go" \
        --command-name "Python" "python3 -B $SUITE_DIR/$workload.py" \
        --command-name "PHP" "php $SUITE_DIR/$workload.php" \
        --export-json "$RESULTS_DIR/$workload.json" \
        --export-markdown "$RESULTS_DIR/$workload.md"
done

jq -n \
    --arg run_id "$RUN_ID" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg kujo_commit "$KUJO_COMMIT" \
    --argjson kujo_dirty "$KUJO_DIRTY" \
    --arg kujo_version "$($KUJO_BIN --version)" \
    --arg kujo_sha256 "$(sha256_file "$KUJO_BIN")" \
    --arg go_version "$(go version)" \
    --arg python_version "$(python3 --version)" \
    --arg php_version "$(php --version | head -n 1)" \
    --arg hyperfine_version "$(hyperfine --version)" \
    --arg os_version "$(sw_vers -productVersion 2>/dev/null || uname -sr)" \
    --arg arch "$(uname -m)" \
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
            build: "cargo build --release --locked --no-default-features --features runtime-jit --bin kujo-run",
            launcher: "lean trusted VM/JIT launcher"
        },
        go: {version: $go_version, build: "go build -trimpath"},
        python: {version: $python_version, flags: "-B"},
        php: {version: $php_version},
        hyperfine: $hyperfine_version,
        host: {os_version: $os_version, arch: $arch},
        sampling: {
            shell: "none",
            benchmark_warmups: $benchmark_warmups,
            benchmark_runs: $benchmark_runs,
            startup_warmups: $startup_warmups,
            startup_runs: $startup_runs
        }
    }' >"$RESULTS_DIR/metadata.json"

SUMMARY="$RESULTS_DIR/summary.md"
{
    echo "# Kujo focused four-language runtime benchmark"
    echo
    echo "Run: \`$RUN_ID\`"
    echo
    echo "All exact-output checks passed. Times below are Hyperfine medians."
    echo
    echo "| Workload | Kujo lean JIT | Go | Python | PHP | Kujo vs Go |"
    echo "|---|---:|---:|---:|---:|---:|"
    for workload in startup prime_count integer_mix; do
        result_file="$RESULTS_DIR/$workload.json"
        kujo="$(median_seconds "$result_file" 0)"
        go="$(median_seconds "$result_file" 1)"
        python="$(median_seconds "$result_file" 2)"
        php="$(median_seconds "$result_file" 3)"
        printf '| %s | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.2fx |\n' \
            "$workload" "$(milliseconds "$kujo")" "$(milliseconds "$go")" \
            "$(milliseconds "$python")" "$(milliseconds "$php")" "$(ratio "$kujo" "$go")"
    done
    echo
    echo "Kujo uses the lean trusted launcher with the same parser, bytecode compiler, VM,"
    echo "regional JIT, builtins, imports, and scheduler as \`kujo run\`. It intentionally omits"
    echo "the general-purpose command router and optional DB/image/PDF/archive features."
} >"$SUMMARY"

echo "$RESULTS_DIR"
