use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_markdown_files_with_unchecked_boxes(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("failed to read notes directory") {
        let entry = entry.expect("failed to read notes entry");
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files_with_unchecked_boxes(&path, files);
            continue;
        }

        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }

        let content = fs::read_to_string(&path).expect("failed to read notes markdown file");
        if content.lines().any(|line| line.starts_with("- [ ]")) {
            files.push(path);
        }
    }
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

    let mut files = Vec::new();
    collect_markdown_files_with_unchecked_boxes(&root.join("notes"), &mut files);
    assert_eq!(
        files.len(),
        194,
        "triage index inventory count should stay in sync with notes unchecked checkbox files"
    );
}
