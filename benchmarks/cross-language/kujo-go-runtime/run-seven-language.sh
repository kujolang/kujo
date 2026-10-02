#!/usr/bin/env bash
set -euo pipefail

SUITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KUJO_REPO="$(cd "$SUITE_DIR/../../.." && pwd)"
RESULTS_ROOT="$KUJO_REPO/benchmarks/cross-language/results"
RUN_ID="${RUN_ID:-kujo-seven-language-$(date -u +%Y%m%dT%H%M%SZ)}"
RESULTS_DIR="${RESULTS_DIR:-$RESULTS_ROOT/$RUN_ID}"
BIN_DIR="$RESULTS_DIR/bin"

BENCH_WARMUP="${BENCH_WARMUP:-3}"
BENCH_RUNS="${BENCH_RUNS:-15}"
STARTUP_WARMUP="${STARTUP_WARMUP:-5}"
STARTUP_RUNS="${STARTUP_RUNS:-25}"
ZERO_BIN="${ZERO_BIN:-${HOME}/.zero/bin/zero}"
BEND_BIN="${BEND_BIN:-${HOME}/.bend/bin/bend}"

for command_name in cargo go rustc hyperfine jq git python3 php; do
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "missing required command: $command_name" >&2
        exit 1
    fi
done
for executable in "$ZERO_BIN" "$BEND_BIN"; do
    if [[ ! -x "$executable" ]]; then
        echo "missing required executable: $executable" >&2
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

milliseconds() {
    jq -n --argjson value "$1" '$value * 1000'
}

ratio() {
    jq -n --argjson numerator "$1" --argjson denominator "$2" '$numerator / $denominator'
}

mkdir -p "$BIN_DIR"
export BEND_NO_TELEMETRY=1

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
    /usr/bin/time -p cargo build --release --locked --no-default-features --bin kujo-aot
) 2>"$RESULTS_DIR/kujo-aot-release-build.time"
cp "$KUJO_REPO/target/release/kujo-aot" "$BIN_DIR/kujo-aot"
KUJO_COMPILER="$BIN_DIR/kujo-aot"

workloads=(startup prime_count integer_mix)
RUST_FLAGS=(--edition=2021 -C opt-level=3 -C debuginfo=0 -C strip=symbols -C codegen-units=1)
for workload in "${workloads[@]}"; do
    /usr/bin/time -p "$KUJO_COMPILER" "$SUITE_DIR/$workload.kujo" \
        -o "$BIN_DIR/$workload-kujo" \
        >"$RESULTS_DIR/$workload-kujo-compile.stdout" \
        2>"$RESULTS_DIR/$workload-kujo-compile.time"
    go build -trimpath -ldflags='-s -w' -o "$BIN_DIR/$workload-go" "$SUITE_DIR/$workload.go"
    rustc "${RUST_FLAGS[@]}" -o "$BIN_DIR/$workload-rust" "$SUITE_DIR/$workload.rs"
    (
        cd "$SUITE_DIR/zero/$workload"
        "$ZERO_BIN" build --profile release-fast --target host --emit exe \
            --out "$BIN_DIR/$workload-zero"
    )
    "$BEND_BIN" "$SUITE_DIR/$workload.bend" -o "$BIN_DIR/$workload-bend"
done

CORRECTNESS_TSV="$RESULTS_DIR/correctness.tsv"
printf 'workload\tkujo_output\tgo_output\trust_output\tpython_output\tphp_output\tzero_output\tbend_output\texpected\tstatus\n' \
    >"$CORRECTNESS_TSV"
for workload in "${workloads[@]}"; do
    kujo_output="$($BIN_DIR/$workload-kujo)"
    go_output="$($BIN_DIR/$workload-go)"
    rust_output="$($BIN_DIR/$workload-rust)"
    python_output="$(python3 -B "$SUITE_DIR/$workload.py")"
    php_output="$(php "$SUITE_DIR/$workload.php")"
    zero_output="$($BIN_DIR/$workload-zero)"
    bend_output="$($BIN_DIR/$workload-bend)"
    expected="$(<"$SUITE_DIR/expected/$workload.txt")"
    status="pass"
    for output in "$kujo_output" "$go_output" "$rust_output" "$python_output" \
        "$php_output" "$zero_output" "$bend_output"; do
        if [[ "$output" != "$expected" ]]; then
            status="fail"
        fi
    done
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$workload" "$kujo_output" "$go_output" "$rust_output" "$python_output" \
        "$php_output" "$zero_output" "$bend_output" "$expected" "$status" \
        >>"$CORRECTNESS_TSV"
    if [[ "$status" != "pass" ]]; then
        echo "correctness check failed for $workload" >&2
        exit 1
    fi
done

for workload in "${workloads[@]}"; do
    warmups="$BENCH_WARMUP"
    runs="$BENCH_RUNS"
    if [[ "$workload" == "startup" ]]; then
        warmups="$STARTUP_WARMUP"
        runs="$STARTUP_RUNS"
    fi
    hyperfine \
        --shell=none \
        --warmup "$warmups" \
        --runs "$runs" \
        --command-name "Kujo" "$BIN_DIR/$workload-kujo" \
        --command-name "Go" "$BIN_DIR/$workload-go" \
        --command-name "Rust" "$BIN_DIR/$workload-rust" \
        --command-name "Python" "python3 -B $SUITE_DIR/$workload.py" \
        --command-name "PHP" "php $SUITE_DIR/$workload.php" \
        --command-name "Zero" "$BIN_DIR/$workload-zero" \
        --command-name "Bend" "$BIN_DIR/$workload-bend" \
        --export-json "$RESULTS_DIR/$workload.json" \
        --export-markdown "$RESULTS_DIR/$workload.md"
done

ZERO_VERSION="$($ZERO_BIN --version)"
BEND_VERSION="$($BEND_BIN version)"
jq -n \
    --arg run_id "$RUN_ID" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg kujo_commit "$KUJO_COMMIT" \
    --argjson kujo_dirty "$KUJO_DIRTY" \
    --arg kujo_version "$($KUJO_COMPILER --version)" \
    --arg go_version "$(go version)" \
    --arg rust_version "$(rustc --version)" \
    --arg python_version "$(python3 --version)" \
    --arg php_version "$(php --version | head -n 1)" \
    --arg zero_version "$ZERO_VERSION" \
    --arg bend_version "$BEND_VERSION" \
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
        kujo: {commit: $kujo_commit, dirty: $kujo_dirty, version: $kujo_version,
            build: "cargo build --release --locked --no-default-features --bin kujo-aot",
            execution: "precompiled freestanding native scalar-core executable"},
        go: {version: $go_version, build: "go build -trimpath -ldflags=-s\\ -w"},
        rust: {version: $rust_version,
            build: "rustc --edition=2021 -C opt-level=3 -C debuginfo=0 -C strip=symbols -C codegen-units=1"},
        python: {version: $python_version, flags: "-B"},
        php: {version: $php_version},
        zero: {version: $zero_version,
            build: "zero build --profile release-fast --target host --emit exe"},
        bend: {version: $bend_version, build: "bend SOURCE -o OUTPUT"},
        hyperfine: $hyperfine_version,
        host: {os_version: $os_version, arch: $arch},
        sampling: {shell: "none", benchmark_warmups: $benchmark_warmups,
            benchmark_runs: $benchmark_runs, startup_warmups: $startup_warmups,
            startup_runs: $startup_runs}
    }' >"$RESULTS_DIR/metadata.json"

SUMMARY="$RESULTS_DIR/summary.md"
{
    echo "# Kujo seven-language runtime benchmark"
    echo
    echo "Run: \`$RUN_ID\`"
    echo
    echo "All exact-output checks passed. Times are Hyperfine medians of separately launched processes."
    echo
    echo "| Workload | Kujo | Go | Rust | Python | PHP | Zero | Bend |"
    echo "|---|---:|---:|---:|---:|---:|---:|---:|"
    for workload in "${workloads[@]}"; do
        values=()
        for index in 0 1 2 3 4 5 6; do
            seconds="$(jq ".results[$index].median" "$RESULTS_DIR/$workload.json")"
            values+=("$(milliseconds "$seconds")")
        done
        printf '| %s | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.3f ms |\n' \
            "$workload" "${values[@]}"
    done

    echo
    echo "## Kujo versus each language"
    echo
    echo "Below 1.00x means Kujo was faster; above 1.00x means the other language was faster."
    echo
    echo "| Workload | Go | Rust | Python | PHP | Zero | Bend |"
    echo "|---|---:|---:|---:|---:|---:|---:|"
    for workload in "${workloads[@]}"; do
        result_file="$RESULTS_DIR/$workload.json"
        kujo="$(jq '.results[0].median' "$result_file")"
        ratios=()
        for index in 1 2 3 4 5 6; do
            other="$(jq ".results[$index].median" "$result_file")"
            ratios+=("$(ratio "$kujo" "$other")")
        done
        printf '| %s | %.2fx | %.2fx | %.2fx | %.2fx | %.2fx | %.2fx |\n' \
            "$workload" "${ratios[@]}"
    done

    echo
    echo "## Leaderboards"
    for workload in "${workloads[@]}"; do
        echo
        echo "### $workload"
        echo
        echo "| Rank | Language | Median | vs leader |"
        echo "|---:|---|---:|---:|"
        jq -r '
            [.results[] | {language: .command, median: .median}] | sort_by(.median) |
            .[0].median as $leader |
            to_entries[] |
            "| \(.key + 1) | \(.value.language) | \((.value.median * 1000 * 1000 | round) / 1000) ms | \((.value.median / $leader * 100 | round) / 100)x |"
        ' "$RESULTS_DIR/$workload.json"
    done

    echo
    echo "Kujo uses its allocation-free scalar-core AOT backend; unsupported language features are"
    echo "rejected rather than silently falling back. Go, Rust, Zero, and Bend run prebuilt native"
    echo "executables; Python and PHP include interpreter startup. Kujo compilation is recorded"
    echo "separately and excluded from execution timings. Bend lacks a 64-bit integer type,"
    echo "so its integer-mix version uses overflow-safe U32 modular multiplication to preserve the"
    echo "same recurrence and exact result; that workload is semantically equivalent but not operation-for-operation identical."
} >"$SUMMARY"

echo "$RESULTS_DIR"
