use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn release_process_doc_covers_required_policy_sections() {
    let doc_path = repo_root().join("docs").join("RELEASE_PROCESS.md");
    let content = fs::read_to_string(&doc_path).expect("failed to read release process doc");

    let required_markers = [
        "## 1. Versioning Policy",
        "## 2. Compatibility Policy",
        "### 2.1 Language compatibility",
        "### 2.2 Standard library compatibility",
        "### 2.3 Diagnostics and machine-readable contract stability",
        "### 2.5 Dependency lockfile determinism",
        "### 2.6 Package registry and Kennel boundary",
        "`kujo package-publish` is metadata preview only",
        "## 4. Release Candidate (RC) Process",
        "## 5. Changelog Format Policy",
        "## 8. Tagging And Publication Order",
        "### 8.0 Explicit Release Directive",
        "UNBLOCK_V1_RELEASE",
        "must not",
        "cargo publish --dry-run",
        "cargo publish",
    ];

    for marker in required_markers {
        assert!(
            content.contains(marker),
            "expected release process doc to contain marker {marker:?}"
        );
    }
}

#[test]
fn release_gate_script_help_documents_modes_and_env_controls() {
    let output = Command::new("bash")
        .current_dir(repo_root())
        .args(["scripts/release_gate.sh", "--help"])
        .output()
        .expect("failed to run release gate help");

    assert!(
        output.status.success(),
        "expected release gate help to succeed, got status={:?}",
        output.status.code()
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    for expected in [
        "--full",
        "--minimal",
        "KUJO_ENABLE_SOCKET_TESTS",
        "KUJO_RELEASE_GATE_RUN_BENCH",
        "KUJO_RELEASE_GATE_MODE",
    ] {
        assert!(
            stdout.contains(expected),
            "expected release gate help output to include {expected:?}"
        );
    }
}
