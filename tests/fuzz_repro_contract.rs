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
    let pin = include_str!("../fuzz/RUST_TOOLCHAIN").trim();
    assert!(stdout.contains(&format!("[dry-run] cargo +{pin} fuzz run lexer")));
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
    let pin = include_str!("../fuzz/RUST_TOOLCHAIN").trim();
    assert!(stdout.contains(&format!("[dry-run] cargo +{pin} fuzz run parser")));
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

#[cfg(unix)]
#[test]
fn fuzz_environment_requires_locked_metadata_and_detects_lock_drift() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    std::fs::create_dir(root.join("fuzz")).unwrap();
    for name in ["RUST_TOOLCHAIN", "CARGO_FUZZ_VERSION"] {
        std::fs::copy(repo_root().join("fuzz").join(name), root.join("fuzz").join(name)).unwrap();
    }
    std::fs::write(root.join("fuzz/Cargo.lock"), "original lock").unwrap();
    let cargo = root.join("cargo");
    std::fs::write(&cargo, "#!/bin/sh\ntest \"$2\" = metadata && test \"$3\" = --locked\n")
        .unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = std::env::join_paths(
        std::iter::once(root.to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    for drift in [false, true] {
        let script = if drift {
            "repo_root=$1; source \"$2\"; fuzz_require_lock; echo changed >> \"$repo_root/fuzz/Cargo.lock\"; fuzz_check_lock"
        } else {
            "repo_root=$1; source \"$2\"; fuzz_require_lock; fuzz_check_lock"
        };
        let output = Command::new("bash")
            .args(["-e", "-c", script, "fuzz-contract"])
            .arg(root)
            .arg(repo_root().join("scripts/fuzz_environment.sh"))
            .env("PATH", &path)
            .output()
            .unwrap();
        assert_eq!(output.status.success(), !drift, "{:?}", output);
        assert!(String::from_utf8_lossy(&output.stdout).contains("lock_sha256="));
        if drift {
            assert!(String::from_utf8_lossy(&output.stderr).contains("lock changed"));
        }
    }
}
