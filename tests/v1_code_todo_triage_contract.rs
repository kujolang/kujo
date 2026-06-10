use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_v1_code_todo_triage_{}_{}", prefix, nanos));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("generate_v1_code_todo_triage.sh")
}

#[test]
fn v1_code_todo_triage_script_generates_expected_repo_artifacts() {
    let dir = unique_temp_dir("success");
    let output_md = dir.join("triage.md");
    let output_csv = dir.join("triage.csv");

    let output = Command::new("bash")
        .arg(script_path())
        .arg("--source-root")
        .arg("src")
        .arg("--output-md")
        .arg(&output_md)
        .arg("--output-csv")
        .arg(&output_csv)
        .arg("--strict")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to execute triage script");

    assert!(
        output.status.success(),
        "triage script should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let markdown = fs::read_to_string(&output_md).expect("triage markdown should exist");
    assert!(markdown.contains("# V1 Code TODO/FIXME/HACK Triage"));
    assert!(markdown.contains("| ID | File | Line | Marker | Summary | Severity | Owner | Target Release Bucket | Scope | Rationale |"));
    assert!(markdown.contains("`src/type_checker.rs`"));
    assert!(
        !markdown.contains("| high |"),
        "high-severity TODO markers should be resolved or explicitly deferred"
    );
    assert!(markdown.contains("Summary: `"));

    let csv = fs::read_to_string(&output_csv).expect("triage csv should exist");
    assert!(csv.contains(
        "id,file,line,marker,summary,severity,owner,target_release_bucket,scope,rationale"
    ));
    assert!(!csv.contains(",v1,"));
    assert!(csv.contains(",post-v1,"));
}

#[test]
fn v1_code_todo_triage_script_fails_strict_mode_for_unclassified_paths() {
    let dir = unique_temp_dir("strict_failure");
    let source_root = dir.join("scratch");
    fs::create_dir_all(&source_root).expect("failed to create source root");

    let source_file = source_root.join("mystery.rs");
    fs::write(&source_file, "fn main() { /* TODO: unresolved mystery debt */ }\n")
        .expect("failed to write scratch source");

    let output_md = dir.join("triage.md");
    let output_csv = dir.join("triage.csv");

    let output = Command::new("bash")
        .arg(script_path())
        .arg("--source-root")
        .arg(&source_root)
        .arg("--output-md")
        .arg(&output_md)
        .arg("--output-csv")
        .arg(&output_csv)
        .arg("--strict")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to execute triage script");

    assert!(!output.status.success(), "strict mode should fail for unclassified items");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unclassified TODO/FIXME/HACK"), "unexpected stderr: {stderr}");
}

#[test]
fn v1_code_todo_triage_script_is_deterministic_for_repo_scan() {
    let dir = unique_temp_dir("determinism");
    let output_md_a = dir.join("triage_a.md");
    let output_csv_a = dir.join("triage_a.csv");
    let output_md_b = dir.join("triage_b.md");
    let output_csv_b = dir.join("triage_b.csv");

    let run_once = |output_md: &PathBuf, output_csv: &PathBuf| {
        let output = Command::new("bash")
            .arg(script_path())
            .arg("--source-root")
            .arg("src")
            .arg("--output-md")
            .arg(output_md)
            .arg("--output-csv")
            .arg(output_csv)
            .arg("--strict")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("failed to execute triage script");

        assert!(
            output.status.success(),
            "triage script should succeed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run_once(&output_md_a, &output_csv_a);
    run_once(&output_md_b, &output_csv_b);

    let markdown_a = fs::read_to_string(output_md_a).expect("first markdown output should exist");
    let markdown_b = fs::read_to_string(output_md_b).expect("second markdown output should exist");
    assert_eq!(markdown_a, markdown_b, "markdown output should be deterministic");

    let csv_a = fs::read_to_string(output_csv_a).expect("first csv output should exist");
    let csv_b = fs::read_to_string(output_csv_b).expect("second csv output should exist");
    assert_eq!(csv_a, csv_b, "csv output should be deterministic");
}
