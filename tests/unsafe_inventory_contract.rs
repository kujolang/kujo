use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_unsafe_inventory_{prefix}_{nanos}"));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts").join("generate_unsafe_inventory.sh")
}

#[test]
fn unsafe_inventory_script_generates_expected_artifacts() {
    let dir = unique_temp_dir("success");
    let output_md = dir.join("unsafe_inventory.md");
    let output_csv = dir.join("unsafe_inventory.csv");

    let output = Command::new("bash")
        .arg(script_path())
        .arg("--output-md")
        .arg(&output_md)
        .arg("--output-csv")
        .arg(&output_csv)
        .arg("--strict")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to execute unsafe inventory script");

    assert!(
        output.status.success(),
        "unsafe inventory script should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let markdown = fs::read_to_string(&output_md).expect("unsafe inventory markdown should exist");
    assert!(markdown.contains("# Unsafe Inventory"));
    assert!(markdown.contains("| Path | Line | Kind | Classification | Text |"));
    assert!(markdown.contains("src/jit.rs"));
    assert!(markdown.contains("jit_executable"));

    let csv = fs::read_to_string(&output_csv).expect("unsafe inventory csv should exist");
    assert!(csv.contains("path,line,kind,classification,text"));
    assert!(csv.contains("\"src/jit.rs\""));
    assert!(csv.contains("\"executable\""));
    assert!(csv.contains("\"non_executable\""));
}

#[test]
fn unsafe_inventory_script_is_deterministic_for_repo_scan() {
    let dir = unique_temp_dir("determinism");
    let output_md_a = dir.join("unsafe_inventory_a.md");
    let output_csv_a = dir.join("unsafe_inventory_a.csv");
    let output_md_b = dir.join("unsafe_inventory_b.md");
    let output_csv_b = dir.join("unsafe_inventory_b.csv");

    let run_once = |output_md: &PathBuf, output_csv: &PathBuf| {
        let output = Command::new("bash")
            .arg(script_path())
            .arg("--output-md")
            .arg(output_md)
            .arg("--output-csv")
            .arg(output_csv)
            .arg("--strict")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("failed to execute unsafe inventory script");

        assert!(
            output.status.success(),
            "unsafe inventory script should succeed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run_once(&output_md_a, &output_csv_a);
    run_once(&output_md_b, &output_csv_b);

    let markdown_a = fs::read_to_string(&output_md_a).expect("first markdown output should exist");
    let markdown_b = fs::read_to_string(&output_md_b).expect("second markdown output should exist");
    assert_eq!(markdown_a, markdown_b);

    let csv_a = fs::read_to_string(&output_csv_a).expect("first csv output should exist");
    let csv_b = fs::read_to_string(&output_csv_b).expect("second csv output should exist");
    assert_eq!(csv_a, csv_b);
}

#[test]
fn unsafe_inventory_enforces_current_executable_budget() {
    let dir = unique_temp_dir("budget");
    let output_md = dir.join("unsafe_inventory_budget.md");
    let output_csv = dir.join("unsafe_inventory_budget.csv");

    let output = Command::new("bash")
        .arg(script_path())
        .arg("--output-md")
        .arg(&output_md)
        .arg("--output-csv")
        .arg(&output_csv)
        .arg("--strict")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to execute unsafe inventory script");

    assert!(
        output.status.success(),
        "unsafe inventory script should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let markdown = fs::read_to_string(&output_md).expect("unsafe inventory markdown should exist");
    let executable_line = markdown
        .lines()
        .find(|line| line.starts_with("- Executable matches:"))
        .expect("summary should contain executable matches");
    let executable_count: usize = executable_line
        .split(':')
        .nth(1)
        .expect("executable summary should contain value")
        .trim()
        .parse()
        .expect("executable summary value should be numeric");

    // v1.2.3 adds two audited Windows ReOpenFile boundaries for read-only
    // process stdin handles and one Unix mkfifo test boundary. Keep the
    // budget exact so any additional executable unsafe requires review.
    assert_eq!(
        executable_count, 61,
        "executable unsafe budget changed: expected 61, got {executable_count}"
    );

    let csv = fs::read_to_string(&output_csv).expect("unsafe inventory csv should exist");
    let jit_executable_count = csv
        .lines()
        .skip(1)
        .filter(|line| line.contains("\"src/jit.rs\"") && line.contains("\"executable\""))
        .count();

    assert!(
        jit_executable_count <= 45,
        "jit executable unsafe budget regression: expected <= 45, got {jit_executable_count}"
    );
}
