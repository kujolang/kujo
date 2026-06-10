use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(path: &str) -> String {
    repo_root()
        .join("tests")
        .join("fixtures")
        .join("unsafe_safety_contracts")
        .join(path)
        .display()
        .to_string()
}

#[test]
fn checker_help_lists_schema_and_modes() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/check_jit_safety_contracts.sh", "--help"])
        .output()
        .expect("failed to run help");

    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    for expected in ["Canonical schema", "--allow-missing", "Exit codes"] {
        assert!(stdout.contains(expected), "help output should include {:?}", expected);
    }
}

#[test]
fn checker_passes_on_valid_fixture() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/check_jit_safety_contracts.sh", "--file", &fixture("valid_jit_like.rs")])
        .output()
        .expect("failed to run checker");

    assert!(
        output.status.success(),
        "expected checker success, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn checker_fails_on_missing_contract() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/check_jit_safety_contracts.sh", "--file", &fixture("missing_contract.rs")])
        .output()
        .expect("failed to run checker");

    assert!(!output.status.success(), "missing contract should fail");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("missing SAFETY contract"));
}

#[test]
fn checker_allow_missing_reports_but_succeeds() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args([
            "scripts/check_jit_safety_contracts.sh",
            "--allow-missing",
            "--file",
            &fixture("missing_contract.rs"),
        ])
        .output()
        .expect("failed to run checker");

    assert!(
        output.status.success(),
        "allow-missing should succeed, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("missing contracts:"));
}

#[test]
fn checker_fails_on_malformed_contract() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args([
            "scripts/check_jit_safety_contracts.sh",
            "--file",
            &fixture("malformed_contract.rs"),
        ])
        .output()
        .expect("failed to run checker");

    assert!(!output.status.success(), "malformed contract should fail");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("missing SAFETY contract"));
}

#[test]
fn checker_fails_on_wrong_heading_spelling() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/check_jit_safety_contracts.sh", "--file", &fixture("wrong_headings.rs")])
        .output()
        .expect("failed to run checker");

    assert!(!output.status.success(), "wrong headings should fail");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("missing SAFETY contract"));
}

#[test]
fn checker_rejects_unknown_argument() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/check_jit_safety_contracts.sh", "--nope"])
        .output()
        .expect("failed to run checker");

    assert!(!output.status.success(), "unknown argument should fail");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("unsupported argument: --nope"));
}

#[test]
fn checker_ignores_unsafe_extern_type_aliases() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/check_jit_safety_contracts.sh", "--file", &fixture("type_alias_only.rs")])
        .output()
        .expect("failed to run checker");

    assert!(
        output.status.success(),
        "type alias should not be treated as executable boundary, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("Checked 0 executable unsafe boundaries"));
}
