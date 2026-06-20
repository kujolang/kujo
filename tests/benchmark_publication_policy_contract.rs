use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

fn squashed(content: &str) -> String {
    content.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn benchmark_publication_policy_defines_launch_claim_boundary() {
    let policy = read("docs/BENCHMARK_PUBLICATION_POLICY.md");
    let performance = read("docs/PERFORMANCE.md");

    for marker in [
        "Only the following benchmark evidence is launch-safe for v1.0",
        "docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md",
        "docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md",
        "Internal Regression Signals",
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
    let benchmark_readme = read("examples/benchmarks/README.md");
    let real_world = read("examples/benchmarks/README_REAL_WORLD.md");
    let jit_readme = read("examples/benchmarks/jit/README.md");
    let compare_languages = read("examples/benchmarks/compare_languages.sh");

    for (name, content) in [
        ("examples/benchmarks/README.md", &benchmark_readme),
        ("examples/benchmarks/README_REAL_WORLD.md", &real_world),
        ("examples/benchmarks/jit/README.md", &jit_readme),
    ] {
        assert!(
            squashed(content).contains("not v1.0 launch benchmark claims"),
            "{name} should mark examples as non-launch evidence"
        );
    }

    for forbidden in [
        "Expected Speedup",
        "10-50x faster than interpreter",
        "100-500x faster than interpreter",
        "2-10x faster than Python",
        "Kujo should be 2-10x faster than Python",
        "Kujo with JIT should be",
    ] {
        assert!(
            !benchmark_readme.contains(forbidden)
                && !real_world.contains(forbidden)
                && !jit_readme.contains(forbidden)
                && !compare_languages.contains(forbidden),
            "example benchmark docs/scripts should not contain stale public claim {forbidden:?}"
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
