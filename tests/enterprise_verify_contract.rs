use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn enterprise_verify_help_lists_modes_and_replay_contract() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/enterprise_verify.sh", "--help"])
        .output()
        .expect("failed to run enterprise verify help");

    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("help stdout should be utf-8");
    for expected in ["--minimal", "--full", "--dry-run", "KUJO_AI_REPLAY_MODE=strict", "AI replay"]
    {
        assert!(stdout.contains(expected), "expected help output to include {expected:?}");
    }
}

#[test]
fn enterprise_verify_minimal_dry_run_emits_expected_commands() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/enterprise_verify.sh", "--minimal", "--dry-run"])
        .output()
        .expect("failed to run enterprise verify dry-run");

    assert!(
        output.status.success(),
        "expected dry-run success, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    for expected in [
        "[dry-run] bash scripts/repo_hygiene_audit.sh",
        "[dry-run] cargo test --test readme_contracts",
        "[dry-run] cargo test --test docs_policy_consistency_contract",
        "[dry-run] cargo test --test enterprise_verify_contract",
        "[dry-run] cargo test --test ai_replay_hermeticity_contract",
        "[dry-run] cargo test --test docs_examples",
        "[dry-run] cargo run -- check examples/ai_enterprise_replay_showcase.kujo",
        "[dry-run] KUJO_AI_REPLAY=tests/fixtures/ai_cassettes KUJO_AI_REPLAY_MODE=strict cargo run -- run examples/ai_enterprise_replay_showcase.kujo",
    ] {
        assert!(stdout.contains(expected), "expected dry-run output to include {expected:?}");
    }
}

#[test]
fn enterprise_verify_full_dry_run_includes_release_matrix() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/enterprise_verify.sh", "--full", "--dry-run"])
        .output()
        .expect("failed to run enterprise verify full dry-run");

    assert!(output.status.success(), "full dry-run should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    for expected in [
        "[dry-run] cargo fmt --check",
        "[dry-run] cargo check",
        "[dry-run] cargo test",
        "[dry-run] cargo run -- test --runtime vm",
        "[dry-run] cargo run -- test --runtime dual",
        "[dry-run] bash scripts/release_gate.sh --full",
    ] {
        assert!(stdout.contains(expected), "expected full dry-run output to include {expected:?}");
    }
}

#[test]
fn enterprise_verify_rejects_unknown_argument() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/enterprise_verify.sh", "--nope"])
        .output()
        .expect("failed to run enterprise verify unknown-arg check");

    assert!(!output.status.success(), "unknown argument should fail");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("unsupported argument: --nope"));
}
