use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("kujo_vm_runtime_diff_harness_{}_{}", prefix, nanos));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("generate_vm_runtime_diff_harness.sh")
}

#[test]
fn runtime_diff_harness_normalization_self_check_succeeds() {
    let output = Command::new("bash")
        .arg(script_path())
        .arg("--normalization-self-check-only")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to execute runtime diff harness self-check");

    assert!(
        output.status.success(),
        "normalization self-check should pass: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("normalization self-check: ok"),
        "expected explicit self-check success marker"
    );
}

#[test]
fn runtime_diff_harness_generates_expected_columns_and_summary() {
    let dir = unique_temp_dir("success");
    let output_md = dir.join("diff.md");
    let output_csv = dir.join("diff.csv");

    let output = Command::new("bash")
        .arg(script_path())
        .arg("--tests-dir")
        .arg("tests")
        .arg("--output-md")
        .arg(&output_md)
        .arg("--output-csv")
        .arg(&output_csv)
        .arg("--runner")
        .arg("target/debug/kujo")
        .arg("--max-fixtures")
        .arg("6")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to execute runtime diff harness");

    assert!(
        output.status.success(),
        "runtime diff harness should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let markdown = fs::read_to_string(&output_md).expect("diff markdown should exist");
    assert!(markdown.contains("# VM Runtime Diff Harness"));
    assert!(markdown.contains(
        "| Fixture | VM Exit | Interpreter Exit | Raw Equal | Normalized Equal | Diff Class |"
    ));
    assert!(markdown.contains("Summary: `6` fixtures compared"));

    let csv = fs::read_to_string(&output_csv).expect("diff csv should exist");
    assert!(csv.contains("fixture,vm_exit,interpreter_exit,raw_equal,normalized_equal,diff_class"));
    let has_valid_class = csv.lines().skip(1).filter(|line| !line.trim().is_empty()).any(|line| {
        line.ends_with(",raw_equal")
            || line.ends_with(",normalized_noise_only")
            || line.ends_with(",semantic_drift")
    });
    assert!(has_valid_class, "expected at least one classified diff row");
}
