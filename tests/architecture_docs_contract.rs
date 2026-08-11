use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn architecture_doc_matches_current_runtime_posture() {
    let path = repo_root().join("docs").join("ARCHITECTURE.md");
    let content = fs::read_to_string(&path).expect("failed to read docs/ARCHITECTURE.md");

    for marker in [
        "# Kujo Architecture",
        "Current stable release: `v1.0.1`",
        "VM (default `kujo run` path)",
        "Tree-walking interpreter (explicit fallback path)",
        "Package bootstrap and lockfile verification are tracked as separate tooling contracts",
        "### 3.5 Package workflow and lockfiles",
        "kujo package-install --frozen",
        "Supports `--runtime dual|vm|interpreter`.",
        "Top-level generator iteration (`func*` + `yield`) is intentionally divergent",
        "docs/VM_INTERPRETER_PARITY_MATRIX.md",
        "docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md",
    ] {
        assert!(content.contains(marker), "architecture doc should contain marker {marker:?}");
    }

    for stale in [
        "Tree-walking interpreter** (current primary execution path)",
        "Bytecode VM** (experimental, not yet default)",
        "Version**: v0.8.0",
        "v0.9.0 modularization in progress",
    ] {
        assert!(
            !content.contains(stale),
            "architecture doc should not contain stale marker {stale:?}"
        );
    }
}
