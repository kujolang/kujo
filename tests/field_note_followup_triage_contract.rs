use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn field_note_followup_triage_index_routes_old_checkboxes() {
    let root = repo_root();
    let triage_path = root.join("notes").join("FIELD_NOTE_FOLLOWUP_TRIAGE.md");
    let triage = fs::read_to_string(&triage_path).expect("failed to read field note triage index");

    for marker in ["`active`", "`post-v1`", "`archive`"] {
        assert!(
            triage.contains(marker),
            "triage index should define classification marker {marker}"
        );
    }

    for destination in [
        "docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md",
        "V1RR-P0-002",
        "V1RR-P2-002",
        "V1RR-P2-003",
        "V1RR-P2-004",
        "docs/OPTIONAL_TYPING_DESIGN.md",
        "docs/V1_SCOPE.md",
        "docs/PERFORMANCE.md",
        "docs/SSG_BENCHMARK_NEXT_STEPS.md",
        "docs/EDITOR_ADAPTER_BASELINES.md",
        "docs/INSTALLATION_LSP_EDITORS.md",
        "docs/LSP_RELIABILITY.md",
        "docs/TREE_SITTER_KUJO.md",
        "docs/V1_0_UNIVERSAL_USEFULNESS_EXPANSION_CHECKLIST.md",
        "docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md",
    ] {
        assert!(
            triage.contains(destination),
            "triage index should route active/post-v1 work to maintained destination {destination}"
        );
    }

    assert!(
        triage.contains("194") && triage.contains("rg -l"),
        "triage index should record the audited unchecked-note inventory count and command"
    );

    let output = Command::new("rg")
        .current_dir(&root)
        .args(["-l", "^- \\[ \\]", "notes", "-g", "*.md"])
        .output()
        .expect("failed to run rg inventory command");
    assert!(
        output.status.success(),
        "unchecked field-note inventory command should succeed, status={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let count = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert_eq!(
        count, 194,
        "triage index inventory count should stay in sync with notes unchecked checkbox files"
    );
}
