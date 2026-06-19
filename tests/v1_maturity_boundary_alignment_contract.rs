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
        "Canonical readiness boundary: Kujo remains a pre-tag `1.0.0` release candidate until `ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`, and tag-time artifact evidence are closed.";

    let docs = ["README.md", "docs/V1_SCOPE.md", "docs/LANGUAGE_SPEC.md"];

    for doc in docs {
        let content = read(doc);
        assert!(
            content.contains(canonical),
            "expected canonical readiness-boundary wording in {}",
            doc
        );
    }
}
