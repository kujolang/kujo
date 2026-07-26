use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn measure_binary_size_help_lists_flags() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/measure_binary_size.sh", "--help"])
        .output()
        .expect("failed to run measure_binary_size help");

    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("help stdout should be utf-8");
    for expected in ["--dry-run", "--metadata-only", "target/release/kujo"] {
        assert!(stdout.contains(expected), "expected help output to include {expected:?}");
    }
}

#[test]
fn measure_binary_size_dry_run_emits_build_and_size_commands() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/measure_binary_size.sh", "--dry-run"])
        .output()
        .expect("failed to run measure_binary_size dry-run");

    assert!(
        output.status.success(),
        "dry-run should succeed, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    for expected in [
        "[dry-run] cargo build",
        "[dry-run] cargo build --release",
        "[dry-run] wc -c target/debug/kujo target/release/kujo",
    ] {
        assert!(stdout.contains(expected), "expected dry-run output to include {expected:?}");
    }
}

#[test]
fn measure_binary_size_rejects_unknown_argument() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/measure_binary_size.sh", "--nope"])
        .output()
        .expect("failed to run measure_binary_size unknown-arg check");

    assert!(!output.status.success(), "unknown argument should fail");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("unsupported argument: --nope"));
}
