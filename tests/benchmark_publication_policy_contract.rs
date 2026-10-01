use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

#[test]
fn benchmark_publication_policy_defines_launch_claim_boundary() {
    let policy = read("docs/BENCHMARK_PUBLICATION_POLICY.md");
    let performance = read("docs/PERFORMANCE.md");
    let criterion_readme = read("benches/README.md");

    for marker in [
        "Only the following benchmark evidence is launch-safe for v1.0",
        "docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md",
        "docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md",
        "Internal Regression Signals",
        "Curated Cross-Language Inputs",
        "former ad hoc runners, unrelated workloads",
        "Publication Requirements",
        "Forbidden Without Fresh Evidence",
        "fixed VM/JIT speedup ranges for arbitrary programs",
        "cloud-provider pricing or instance recommendations without revalidating",
    ] {
        assert!(policy.contains(marker), "benchmark policy should contain marker {marker:?}");
    }

    assert!(
        performance.contains("docs/BENCHMARK_PUBLICATION_POLICY.md")
            && performance.contains("launch-safe claims versus internal"),
        "performance guide should link the benchmark publication policy"
    );

    assert!(
        criterion_readme.contains("cargo bench --bench v1_perf_benchmarks --no-run")
            && criterion_readme.contains("internal regression signals"),
        "Criterion bench directory should explain its purpose and publication boundary"
    );
}

#[test]
fn retired_ad_hoc_benchmark_and_checkpoint_artifacts_stay_removed() {
    for path in [
        "benchmarks/cross-language/run_benchmarks.sh",
        "benchmarks/cross-language/quick_bench.sh",
        "benchmarks/cross-language/current_results.txt",
        "benchmarks/cross-language/benchmark_results_new.txt",
        "checkpoints/bench_ssg_small.kujo",
    ] {
        assert!(
            !repo_root().join(path).exists(),
            "retired launch-unsafe artifact should stay removed: {path}"
        );
    }
}

#[test]
fn built_in_cross_language_benchmark_inputs_are_documented_and_present() {
    let readme = read("benchmarks/cross-language/README.md");
    for marker in [
        "cargo run --release -- bench-cross",
        "cargo run --release -- bench-ssg",
        "Python 3",
        "docs/BENCHMARK_PUBLICATION_POLICY.md",
    ] {
        assert!(readme.contains(marker), "cross-language README should contain {marker:?}");
    }

    for path in [
        "benchmarks/cross-language/bench_parallel_map.kujo",
        "benchmarks/cross-language/bench_process_pool.py",
        "benchmarks/cross-language/bench_ssg.kujo",
        "benchmarks/cross-language/bench_ssg.py",
    ] {
        assert!(repo_root().join(path).is_file(), "built-in benchmark input is missing: {path}");
    }
}

#[test]
fn kujo_runtime_matrix_preserves_fairness_and_evidence_contracts() {
    let suite = "benchmarks/cross-language/kujo-go-runtime";
    let readme = read(&format!("{suite}/README.md"));
    let runner = read(&format!("{suite}/run.sh"));
    let focused_runner = read(&format!("{suite}/run-four-language.sh"));

    for marker in [
        "function-local workloads",
        "checks exact output before accepting timings",
        "Go, optimized Rust, Python, and PHP",
        "Startup measures",
        "Compile/check measures",
        "Run measures",
        "Kujo internal phases",
        "BENCHMARK_PUBLICATION_POLICY.md",
    ] {
        assert!(readme.contains(marker), "runtime suite README should contain {marker:?}");
    }

    for workload in ["startup", "prime_count", "integer_mix"] {
        for extension in ["kujo", "go", "rs", "py", "php"] {
            let path = format!("{suite}/{workload}.{extension}");
            assert!(repo_root().join(&path).is_file(), "runtime suite input is missing: {path}");
        }
        let expected = format!("{suite}/expected/{workload}.txt");
        assert!(
            repo_root().join(&expected).is_file(),
            "runtime suite expected output is missing: {expected}"
        );
    }

    for marker in [
        "cargo build --release --locked --bin kujo",
        "rustc --edition=2021 -C opt-level=3",
        "python3 -B",
        "php -l",
        "correctness.tsv",
        "kujo-phases.json",
        "source_parse_wall_ns",
        "--warmup",
        "--runs",
        "--export-json",
        "--export-markdown",
        "metadata.json",
        "summary.md",
    ] {
        assert!(runner.contains(marker), "runtime suite runner should contain {marker:?}");
    }

    for marker in [
        "cargo build --release --locked --no-default-features",
        "--features runtime-jit --bin kujo-run",
        "go build -trimpath",
        "python3 -B",
        "php",
        "correctness.tsv",
        "--shell=none",
        "--warmup",
        "--runs",
        "--export-json",
        "--export-markdown",
        "metadata.json",
        "summary.md",
    ] {
        assert!(
            focused_runner.contains(marker),
            "focused runtime runner should contain {marker:?}"
        );
    }

    assert!(
        readme.contains("run-four-language.sh") && readme.contains("kujo-run"),
        "runtime suite README should document the focused four-language runner"
    );
}

#[test]
fn future_ssg_and_host_benchmark_docs_are_not_launch_evidence() {
    let ssg = read("docs/SSG_BENCHMARK_NEXT_STEPS.md");
    let hetzner = read("docs/HETZNER_BENCHMARK_SETUP_AND_PRICING.md");

    assert!(
        ssg.contains("Status: future benchmark campaign plan; not v1.0 launch evidence.")
            && ssg.contains("docs/BENCHMARK_PUBLICATION_POLICY.md")
            && ssg.contains("Use this section only after satisfying"),
        "SSG benchmark next-steps doc should be future campaign planning"
    );

    assert!(
        hetzner.contains("Status: future benchmark host planning; not v1.0 launch evidence.")
            && hetzner.contains("Historical Hetzner Cloud Plan Pricing Snapshot")
            && hetzner.contains("must be checked against Hetzner's live pricing"),
        "Hetzner benchmark doc should mark host/pricing guidance as historical planning"
    );
}

#[test]
fn example_benchmark_docs_are_local_signals_not_public_claims() {
    let compare_languages = read("examples/benchmarks/compare_languages.sh");

    for forbidden in [
        "Expected Speedup",
        "10-50x faster than interpreter",
        "100-500x faster than interpreter",
        "2-10x faster than Python",
        "Kujo should be 2-10x faster than Python",
        "Kujo with JIT should be",
    ] {
        assert!(
            !compare_languages.contains(forbidden),
            "example benchmark helper should not contain stale public claim {forbidden:?}"
        );
    }

    assert!(
        compare_languages.contains("local exploration, not v1.0 publication claims")
            && compare_languages.contains("docs/BENCHMARK_PUBLICATION_POLICY.md"),
        "cross-language helper should route users to publication policy"
    );
}

#[test]
fn tracked_jit_examples_do_not_emit_speed_promises() {
    for path in [
        "examples/benchmarks/jit/arithmetic_intensive.kujo",
        "examples/benchmarks/jit/loop_nested.kujo",
        "examples/benchmarks/jit/comparison_specialized.kujo",
        "examples/benchmarks/jit/run_all.kujo",
    ] {
        let content = read(path);
        for forbidden in [
            "Expected: Significant speedup",
            "Expected: Fastest performance",
            "provides significant speedup",
            "speedup gains",
        ] {
            assert!(
                !content.contains(forbidden),
                "{path} should not emit stale speed promise {forbidden:?}"
            );
        }
        assert!(
            content.contains("Local signal") || content.contains("must be measured"),
            "{path} should frame JIT benchmark output as a local signal"
        );
    }
}
