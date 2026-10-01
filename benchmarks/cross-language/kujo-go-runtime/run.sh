#!/usr/bin/env bash
set -euo pipefail

SUITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KUJO_REPO="$(cd "$SUITE_DIR/../../.." && pwd)"
RESULTS_ROOT="$KUJO_REPO/benchmarks/cross-language/results"
RUN_ID="${RUN_ID:-kujo-runtime-matrix-$(date -u +%Y%m%dT%H%M%SZ)}"
RESULTS_DIR="${RESULTS_DIR:-$RESULTS_ROOT/$RUN_ID}"
BIN_DIR="$RESULTS_DIR/bin"

BENCH_WARMUP="${BENCH_WARMUP:-1}"
BENCH_RUNS="${BENCH_RUNS:-5}"
STARTUP_WARMUP="${STARTUP_WARMUP:-3}"
STARTUP_RUNS="${STARTUP_RUNS:-15}"
PHASE_RUNS="${PHASE_RUNS:-$BENCH_RUNS}"

for command_name in cargo go hyperfine jq git python3 php rustc; do
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

median_seconds() {
    jq ".results[$2].median" "$1"
}

milliseconds() {
    jq -n --argjson value "$1" '$value * 1000'
}

ratio() {
    jq -n --argjson numerator "$1" --argjson denominator "$2" '$numerator / $denominator'
}

phase_median_ns() {
    jq \
        --arg workload "$2" \
        --arg mode "$3" \
        --arg counter "$4" \
        '[.[] | select(.workload == $workload and .mode == $mode) | .counters[$counter]] | sort as $values | ($values | length) as $count | if $count % 2 == 1 then $values[$count / 2 | floor] else (($values[$count / 2 - 1] + $values[$count / 2]) / 2) end' \
        "$1"
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
RUST_FLAGS=(--edition=2021 -C opt-level=3 -C debuginfo=0 -C strip=symbols -C codegen-units=1)
for workload in "${workloads[@]}"; do
    go build -trimpath -o "$BIN_DIR/$workload-go" "$SUITE_DIR/$workload.go"
    rustc "${RUST_FLAGS[@]}" -o "$BIN_DIR/$workload-rust" "$SUITE_DIR/$workload.rs"
done

CORRECTNESS_TSV="$RESULTS_DIR/correctness.tsv"
printf 'workload\tkujo_vm_output\tkujo_jit_output\tgo_output\trust_output\tpython_output\tphp_output\texpected\tstatus\n' >"$CORRECTNESS_TSV"

for workload in "${workloads[@]}"; do
    kujo_vm_output="$($KUJO_BIN run "$SUITE_DIR/$workload.kujo")"
    kujo_jit_output="$($KUJO_BIN run --jit "$SUITE_DIR/$workload.kujo" 2>/dev/null)"
    go_output="$($BIN_DIR/$workload-go)"
    rust_output="$($BIN_DIR/$workload-rust)"
    python_output="$(python3 -B "$SUITE_DIR/$workload.py")"
    php_output="$(php "$SUITE_DIR/$workload.php")"
    expected="$(<"$SUITE_DIR/expected/$workload.txt")"

    status="pass"
    if [[ "$kujo_vm_output" != "$expected" || "$kujo_jit_output" != "$expected" \
        || "$go_output" != "$expected" || "$rust_output" != "$expected" \
        || "$python_output" != "$expected" || "$php_output" != "$expected" ]]; then
        status="fail"
    fi

    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$workload" "$kujo_vm_output" "$kujo_jit_output" "$go_output" \
        "$rust_output" "$python_output" "$php_output" "$expected" "$status" \
        >>"$CORRECTNESS_TSV"

    if [[ "$status" != "pass" ]]; then
        echo "correctness check failed for $workload" >&2
        exit 1
    fi
done

hyperfine \
    --shell=none \
    --warmup "$STARTUP_WARMUP" \
    --runs "$STARTUP_RUNS" \
    --command-name "Kujo startup" "$KUJO_BIN run $SUITE_DIR/startup.kujo" \
    --command-name "Kujo JIT startup" "$KUJO_BIN run --jit $SUITE_DIR/startup.kujo" \
    --command-name "Go startup" "$BIN_DIR/startup-go" \
    --command-name "Rust startup" "$BIN_DIR/startup-rust" \
    --command-name "Python startup" "python3 -B $SUITE_DIR/startup.py" \
    --command-name "PHP startup" "php $SUITE_DIR/startup.php" \
    --export-json "$RESULTS_DIR/startup.json" \
    --export-markdown "$RESULTS_DIR/startup.md"

for workload in prime_count integer_mix; do
    go_compile_output="$BIN_DIR/$workload-compile-go"
    rust_compile_output="$BIN_DIR/$workload-compile-rust"
    python_compile_output="$BIN_DIR/$workload.pyc"

    hyperfine \
        --shell=none \
        --warmup "$BENCH_WARMUP" \
        --runs "$BENCH_RUNS" \
        --command-name "Kujo bytecode compile: $workload" \
            "$KUJO_BIN check --quiet $SUITE_DIR/$workload.kujo" \
        --export-json "$RESULTS_DIR/$workload-kujo-compile.json" \
        --export-markdown "$RESULTS_DIR/$workload-kujo-compile.md"

    hyperfine \
        --shell=none \
        --warmup "$BENCH_WARMUP" \
        --runs "$BENCH_RUNS" \
        --prepare "/bin/rm -f $go_compile_output $rust_compile_output $python_compile_output" \
        --command-name "Go native compile: $workload" \
            "go build -trimpath -o $go_compile_output $SUITE_DIR/$workload.go" \
        --command-name "Rust native compile: $workload" \
            "rustc --edition=2021 -C opt-level=3 -C debuginfo=0 -C strip=symbols -C codegen-units=1 -o $rust_compile_output $SUITE_DIR/$workload.rs" \
        --command-name "Python bytecode compile: $workload" \
            "python3 -B $SUITE_DIR/python_compile.py $SUITE_DIR/$workload.py $python_compile_output" \
        --command-name "PHP syntax check: $workload" \
            "php -l $SUITE_DIR/$workload.php" \
        --export-json "$RESULTS_DIR/$workload-other-compile.json" \
        --export-markdown "$RESULTS_DIR/$workload-other-compile.md"

    hyperfine \
        --shell=none \
        --warmup "$BENCH_WARMUP" \
        --runs "$BENCH_RUNS" \
        --command-name "Kujo run: $workload" \
            "$KUJO_BIN run $SUITE_DIR/$workload.kujo" \
        --command-name "Kujo JIT run: $workload" \
            "$KUJO_BIN run --jit $SUITE_DIR/$workload.kujo" \
        --command-name "Go run: $workload" \
            "$BIN_DIR/$workload-go" \
        --command-name "Rust run: $workload" \
            "$BIN_DIR/$workload-rust" \
        --command-name "Python run: $workload" \
            "python3 -B $SUITE_DIR/$workload.py" \
        --command-name "PHP run: $workload" \
            "php $SUITE_DIR/$workload.php" \
        --export-json "$RESULTS_DIR/$workload-run.json" \
        --export-markdown "$RESULTS_DIR/$workload-run.md"
done

PHASES_JSONL="$RESULTS_DIR/kujo-phases.jsonl"
PHASES_JSON="$RESULTS_DIR/kujo-phases.json"
: >"$PHASES_JSONL"
for workload in prime_count integer_mix; do
    for mode in vm jit; do
        for ((run_index = 1; run_index <= PHASE_RUNS; run_index++)); do
            phase_report="$RESULTS_DIR/.phase-$workload-$mode-$run_index.json"
            if [[ "$mode" == "jit" ]]; then
                "$KUJO_BIN" run --jit --measurements "$phase_report" \
                    "$SUITE_DIR/$workload.kujo" >/dev/null 2>/dev/null
            else
                "$KUJO_BIN" run --measurements "$phase_report" \
                    "$SUITE_DIR/$workload.kujo" >/dev/null 2>/dev/null
            fi
            jq \
                --arg workload "$workload" \
                --arg mode "$mode" \
                --argjson run "$run_index" \
                '. + {workload: $workload, mode: $mode, run: $run}' \
                "$phase_report" >>"$PHASES_JSONL"
            rm "$phase_report"
        done
    done
done
jq -s '.' "$PHASES_JSONL" >"$PHASES_JSON"
rm "$PHASES_JSONL"

KUJO_VERSION="$($KUJO_BIN --version)"
GO_VERSION="$(go version)"
RUST_VERSION="$(rustc --version)"
PYTHON_VERSION="$(python3 --version)"
PHP_VERSION="$(php --version | head -n 1)"
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
    --arg go_prime_sha256 "$(sha256_file "$BIN_DIR/prime_count-go")" \
    --arg go_integer_sha256 "$(sha256_file "$BIN_DIR/integer_mix-go")" \
    --arg rust_version "$RUST_VERSION" \
    --arg rust_prime_sha256 "$(sha256_file "$BIN_DIR/prime_count-rust")" \
    --arg rust_integer_sha256 "$(sha256_file "$BIN_DIR/integer_mix-rust")" \
    --arg python_version "$PYTHON_VERSION" \
    --arg python_executable "$(command -v python3)" \
    --arg php_version "$PHP_VERSION" \
    --arg php_executable "$(command -v php)" \
    --arg hyperfine_version "$HYPERFINE_VERSION" \
    --arg os_version "$OS_VERSION" \
    --arg arch "$ARCH" \
    --arg cpu "$CPU" \
    --argjson benchmark_warmups "$BENCH_WARMUP" \
    --argjson benchmark_runs "$BENCH_RUNS" \
    --argjson startup_warmups "$STARTUP_WARMUP" \
    --argjson startup_runs "$STARTUP_RUNS" \
    --argjson phase_runs "$PHASE_RUNS" \
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
            build: "go build -trimpath",
            binary_sha256: {prime_count: $go_prime_sha256, integer_mix: $go_integer_sha256}
        },
        rust: {
            version: $rust_version,
            build: "rustc --edition=2021 -C opt-level=3 -C debuginfo=0 -C strip=symbols -C codegen-units=1",
            binary_sha256: {prime_count: $rust_prime_sha256, integer_mix: $rust_integer_sha256}
        },
        python: {version: $python_version, executable: $python_executable, flags: "-B"},
        php: {version: $php_version, executable: $php_executable},
        hyperfine: $hyperfine_version,
        host: {os_version: $os_version, arch: $arch, cpu: $cpu},
        sampling: {
            shell: "none",
            benchmark_warmups: $benchmark_warmups,
            benchmark_runs: $benchmark_runs,
            startup_warmups: $startup_warmups,
            startup_runs: $startup_runs,
            phase_runs: $phase_runs
        }
    }' >"$RESULTS_DIR/metadata.json"

SUMMARY="$RESULTS_DIR/summary.md"
{
    echo "# Kujo cross-language runtime benchmark"
    echo
    echo "Run: \`$RUN_ID\`"
    echo
    echo "All correctness checks passed. Times below are Hyperfine medians."
    echo
    echo "| Phase | Workload | Kujo VM | Kujo JIT | Go | Rust | Python | PHP |"
    echo "|---|---|---:|---:|---:|---:|---:|---:|"

    startup_vm="$(median_seconds "$RESULTS_DIR/startup.json" 0)"
    startup_jit="$(median_seconds "$RESULTS_DIR/startup.json" 1)"
    startup_go="$(median_seconds "$RESULTS_DIR/startup.json" 2)"
    startup_rust="$(median_seconds "$RESULTS_DIR/startup.json" 3)"
    startup_python="$(median_seconds "$RESULTS_DIR/startup.json" 4)"
    startup_php="$(median_seconds "$RESULTS_DIR/startup.json" 5)"
    printf '| Startup | trivial | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.3f ms |\n' \
        "$(milliseconds "$startup_vm")" "$(milliseconds "$startup_jit")" \
        "$(milliseconds "$startup_go")" "$(milliseconds "$startup_rust")" \
        "$(milliseconds "$startup_python")" "$(milliseconds "$startup_php")"

    for workload in prime_count integer_mix; do
        kujo_compile="$(median_seconds "$RESULTS_DIR/$workload-kujo-compile.json" 0)"
        go_compile="$(median_seconds "$RESULTS_DIR/$workload-other-compile.json" 0)"
        rust_compile="$(median_seconds "$RESULTS_DIR/$workload-other-compile.json" 1)"
        python_compile="$(median_seconds "$RESULTS_DIR/$workload-other-compile.json" 2)"
        php_compile="$(median_seconds "$RESULTS_DIR/$workload-other-compile.json" 3)"
        printf '| Compile/check | %s | %.3f ms | n/a | %.3f ms | %.3f ms | %.3f ms | %.3f ms |\n' \
            "$workload" "$(milliseconds "$kujo_compile")" "$(milliseconds "$go_compile")" \
            "$(milliseconds "$rust_compile")" "$(milliseconds "$python_compile")" \
            "$(milliseconds "$php_compile")"

        kujo_vm_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 0)"
        kujo_jit_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 1)"
        go_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 2)"
        rust_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 3)"
        python_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 4)"
        php_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 5)"
        printf '| End-to-end run | %s | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.3f ms |\n' \
            "$workload" "$(milliseconds "$kujo_vm_run")" "$(milliseconds "$kujo_jit_run")" \
            "$(milliseconds "$go_run")" "$(milliseconds "$rust_run")" \
            "$(milliseconds "$python_run")" "$(milliseconds "$php_run")"
    done

    echo
    echo "## Kujo internal phase medians"
    echo
    echo "These instrumented phase measurements exclude process and CLI startup. JIT compilation is"
    echo "included within VM execution and shown separately as a subset."
    echo
    echo "| Workload | Mode | Source parse | Bytecode compile | VM setup | VM execution | JIT compile |"
    echo "|---|---|---:|---:|---:|---:|---:|"
    for workload in prime_count integer_mix; do
        for mode in vm jit; do
            printf '| %s | %s | %.3f ms | %.3f ms | %.3f ms | %.3f ms | %.3f ms |\n' \
                "$workload" "$mode" \
                "$(jq -n --argjson value "$(phase_median_ns "$PHASES_JSON" "$workload" "$mode" source_parse_wall_ns)" '$value / 1000000')" \
                "$(jq -n --argjson value "$(phase_median_ns "$PHASES_JSON" "$workload" "$mode" bytecode_compile_wall_ns)" '$value / 1000000')" \
                "$(jq -n --argjson value "$(phase_median_ns "$PHASES_JSON" "$workload" "$mode" vm_setup_wall_ns)" '$value / 1000000')" \
                "$(jq -n --argjson value "$(phase_median_ns "$PHASES_JSON" "$workload" "$mode" vm_inclusive_wall_ns)" '$value / 1000000')" \
                "$(jq -n --argjson value "$(phase_median_ns "$PHASES_JSON" "$workload" "$mode" jit_compile_inclusive_wall_ns)" '$value / 1000000')"
        done
    done

    echo
    echo "## Kujo JIT run ratios"
    echo
    echo "Values below 1.00x mean Kujo JIT was faster; values above 1.00x mean the comparison language was faster."
    echo
    echo "| Workload | vs Go | vs Rust | vs Python | vs PHP |"
    echo "|---|---:|---:|---:|---:|"
    printf '| Startup | %.2fx | %.2fx | %.2fx | %.2fx |\n' \
        "$(ratio "$startup_jit" "$startup_go")" "$(ratio "$startup_jit" "$startup_rust")" \
        "$(ratio "$startup_jit" "$startup_python")" "$(ratio "$startup_jit" "$startup_php")"
    for workload in prime_count integer_mix; do
        kujo_jit_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 1)"
        go_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 2)"
        rust_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 3)"
        python_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 4)"
        php_run="$(median_seconds "$RESULTS_DIR/$workload-run.json" 5)"
        printf '| %s | %.2fx | %.2fx | %.2fx | %.2fx |\n' \
            "$workload" "$(ratio "$kujo_jit_run" "$go_run")" \
            "$(ratio "$kujo_jit_run" "$rust_run")" "$(ratio "$kujo_jit_run" "$python_run")" \
            "$(ratio "$kujo_jit_run" "$php_run")"
    done

    echo
    echo "Compile/check rows are intentionally not treated as equivalent artifacts: Kujo checks and"
    echo "compiles to in-memory bytecode, Go and Rust emit optimized reusable native binaries, Python"
    echo "emits interpreter bytecode, and PHP performs a syntax check. Kujo VM and JIT run rows include"
    echo "startup and source compilation; Python and PHP likewise include interpreter startup and source"
    echo "loading. See the raw Markdown and JSON files for distributions and exact commands."
} >"$SUMMARY"

echo "$RESULTS_DIR"
