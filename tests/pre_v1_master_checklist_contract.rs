use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn pre_v1_master_checklist_defines_closure_semantics_and_blocker_rules() {
    let checklist_path = repo_root().join("docs").join("PRE_V1_MASTER_UNFINISHED_CHECKLIST.md");
    let content = fs::read_to_string(&checklist_path)
        .expect("failed to read docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md");

    let required_markers = [
        "## Checklist Governance And Closure Semantics",
        "### Loop Selection Rule (Mandatory)",
        "### Closure Evidence Rule (Mandatory)",
        "### Blocker Semantics",
        "### Required Per-Loop Report Fields",
        "Choose the first unchecked item in top-to-bottom file order.",
        "Add a dated blocker note directly under the blocked item",
        "Complete exactly one unblocked checklist item per loop.",
        "Checklist row switched from `- [ ]` to `- [x]` with a dated evidence bullet.",
        "Commit message references the checklist ID",
    ];

    for marker in required_markers {
        assert!(
            content.contains(marker),
            "expected master checklist governance marker {:?}",
            marker
        );
    }
}
