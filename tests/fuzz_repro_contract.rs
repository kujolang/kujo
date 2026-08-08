use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn fuzz_repro_help_lists_required_flags() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/fuzz_repro.sh", "--help"])
        .output()
        .expect("failed to run fuzz_repro help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    for expected in ["--artifact", "--target", "--dry-run", "--check-prereqs"] {
        assert!(stdout.contains(expected), "expected help output to include {expected:?}");
    }
}

#[test]
fn fuzz_repro_dry_run_succeeds_with_explicit_target() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args([
            "scripts/fuzz_repro.sh",
            "--target",
            "lexer",
            "--artifact",
            "tests/fixtures/fuzz/synthetic_crash_input.kujo",
            "--dry-run",
        ])
        .output()
        .expect("failed to run fuzz_repro dry-run");

    assert!(
        output.status.success(),
        "expected dry-run success, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("[dry-run] cargo +nightly fuzz run lexer"));
}

#[test]
fn fuzz_repro_dry_run_infers_target_from_artifacts_path() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args([
            "scripts/fuzz_repro.sh",
            "--artifact",
            "tests/fixtures/fuzz/artifacts/parser/crash-synthetic.kujo",
            "--dry-run",
        ])
        .output()
        .expect("failed to run fuzz_repro inferred-target dry-run");

    assert!(
        output.status.success(),
        "expected inferred-target dry-run success, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("inferred fuzz target 'parser'"));
    assert!(stdout.contains("[dry-run] cargo +nightly fuzz run parser"));
}

#[test]
fn fuzz_repro_requires_artifact_file() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args([
            "scripts/fuzz_repro.sh",
            "--target",
            "lexer",
            "--artifact",
            "tests/fixtures/fuzz/does-not-exist.kujo",
            "--dry-run",
        ])
        .output()
        .expect("failed to run fuzz_repro missing-artifact check");

    assert!(!output.status.success(), "expected missing artifact path to fail");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("artifact file not found"));
}
