use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect("failed to read doc")
}

#[test]
fn readiness_boundary_wording_is_consistent_across_core_scope_docs() {
    let canonical =
        "Canonical readiness boundary: Kujo remains pre-1.0 until `ROADMAP.md` and `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` release gates are closed.";

    let docs = [
        "README.md",
        "docs/V1_SCOPE.md",
        "docs/LANGUAGE_SPEC.md",
        "docs/KUJO_FEATURE_INVENTORY.md",
        "docs/UNFINISHED_AND_MVP_AUDIT.md",
    ];

    for doc in docs {
        let content = read(doc);
        assert!(
            content.contains(canonical),
            "expected canonical readiness-boundary wording in {}",
            doc
        );
    }
}
