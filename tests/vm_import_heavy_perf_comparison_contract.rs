use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn parse_percent_delta(content: &str) -> f64 {
    let line = content
        .lines()
        .find(|line| line.starts_with("- Computed median delta: "))
        .expect("perf comparison artifact should contain computed delta");
    let percent_text = line
        .split('`')
        .nth(1)
        .expect("computed delta should be wrapped in backticks")
        .trim_end_matches('%');
    percent_text.parse::<f64>().expect("computed delta should parse as a float")
}

#[test]
fn import_heavy_perf_comparison_artifact_contains_required_markers() {
    let path =
        repo_root().join("docs").join("generated").join("VM_IMPORT_HEAVY_PERF_COMPARISON.md");
    let content = fs::read_to_string(&path)
        .expect("failed to read docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md");

    for marker in [
        "# VM Import-Heavy Nested Startup Perf Comparison",
        "Benchmark target: `module_resolution/import_heavy_nested_dotted_startup_cold_loader`",
        "## Tolerance Policy",
        "Unacceptable regression threshold:",
        "## Result",
        "Interpretation: `PASS`",
    ] {
        assert!(
            content.contains(marker),
            "perf comparison artifact should contain marker {:?}",
            marker
        );
    }

    let delta = parse_percent_delta(&content);
    assert!(
        delta <= 20.0,
        "perf comparison should remain under the regression threshold, got {}%",
        delta
    );
}
