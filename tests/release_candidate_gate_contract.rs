use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn release_candidate_gate_help_includes_modes() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/release_candidate_gate.sh", "--help"])
        .output()
        .expect("failed to run release candidate gate --help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help stdout should be utf-8");
    assert!(stdout.contains("--full"));
    assert!(stdout.contains("--roadmap-only"));
}

#[test]
fn release_candidate_gate_roadmap_only_mode_succeeds() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/release_candidate_gate.sh", "--roadmap-only"])
        .output()
        .expect("failed to run release candidate gate --roadmap-only");

    assert!(
        output.status.success(),
        "expected roadmap-only RC gate to succeed, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
