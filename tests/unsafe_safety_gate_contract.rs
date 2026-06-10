use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn unsafe_safety_gate_help_lists_modes() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/unsafe_safety_gate.sh", "--help"])
        .output()
        .expect("failed to run unsafe safety gate help");

    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("help stdout should be utf-8");
    for expected in ["--dry-run", "--with-miri", "Failure modes"] {
        assert!(stdout.contains(expected), "expected help output to include {:?}", expected);
    }
}

#[test]
fn unsafe_safety_gate_dry_run_emits_expected_commands() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/unsafe_safety_gate.sh", "--dry-run", "--with-miri"])
        .output()
        .expect("failed to run unsafe safety gate dry-run");

    assert!(
        output.status.success(),
        "expected dry-run success, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    for expected in [
        "[dry-run] bash scripts/generate_unsafe_inventory.sh",
        "[dry-run] cargo test --test unsafe_inventory_contract",
        "[dry-run] cargo test --test vm_interpreter_parity_surfaces",
        "[dry-run] cargo +nightly miri test --test vm_interpreter_parity_surfaces vm_and_interpreter_resolve_defined_identifiers",
    ] {
        assert!(stdout.contains(expected), "expected dry-run output to include {:?}", expected);
    }
}

#[test]
fn unsafe_safety_gate_rejects_unknown_argument() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/unsafe_safety_gate.sh", "--nope"])
        .output()
        .expect("failed to run unsafe safety gate unknown-arg check");

    assert!(!output.status.success(), "unknown argument should fail");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("unsupported argument: --nope"));
}
