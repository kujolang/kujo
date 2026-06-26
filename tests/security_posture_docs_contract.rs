use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kujo_binary() -> String {
    env!("CARGO_BIN_EXE_kujo").to_string()
}

fn expected_capability_flags() -> [&'static str; 15] {
    [
        "--allow-fs-read",
        "--allow-fs-write",
        "--allow-fs-delete",
        "--allow-process-exec",
        "--allow-shell-exec",
        "--allow-env-read",
        "--allow-env-write",
        "--allow-net-client",
        "--allow-net-server",
        "--allow-net",
        "--allow-ai",
        "--allow-database",
        "--allow-clock",
        "--allow-random",
        "--allow-all",
    ]
}

#[test]
fn native_security_posture_doc_covers_required_operator_sections_and_flags() {
    let doc_path = repo_root().join("docs").join("NATIVE_API_SECURITY_POSTURE.md");
    let content =
        fs::read_to_string(&doc_path).expect("failed to read native security posture doc");

    let required_sections = [
        "Kujo is not a sandbox",
        "## 1. Threat Model",
        "## 2. Trust Modes And Defaults",
        "## 5. Static Server (`kujo serve`) Security Defaults",
        "## 6. Safe vs Unsafe Configuration Patterns",
        "## 7. Recommended External Sandboxing Controls",
    ];

    for section in required_sections {
        assert!(
            content.contains(section),
            "expected security posture doc to include section marker {:?}",
            section
        );
    }

    for flag in expected_capability_flags() {
        assert!(
            content.contains(flag),
            "expected security posture doc to include capability flag {:?}",
            flag
        );
    }

    assert!(
        content.contains("--untrusted"),
        "expected security posture doc to include explicit --untrusted guidance"
    );
}

#[test]
fn cli_help_includes_documented_capability_flags() {
    let output = Command::new(kujo_binary())
        .args(["run", "--help"])
        .output()
        .expect("failed to execute kujo run --help");

    assert!(
        output.status.success(),
        "expected kujo run --help to succeed, got status={:?}",
        output.status.code()
    );

    let stdout = String::from_utf8(output.stdout).expect("run --help stdout should be utf-8");
    for flag in expected_capability_flags() {
        assert!(
            stdout.contains(flag),
            "expected run --help output to include capability flag {:?}",
            flag
        );
    }
    assert!(stdout.contains("--untrusted"), "expected run --help output to include --untrusted");
}
