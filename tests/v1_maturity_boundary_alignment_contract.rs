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
        "Release boundary: Kujo `v1.0.2` is the current stable release; explicit deferrals and compatibility guarantees are governed by `docs/V1_SCOPE.md`.";

    let docs = ["README.md", "docs/V1_SCOPE.md", "docs/LANGUAGE_SPEC.md"];

    for doc in docs {
        let content = read(doc);
        assert!(
            content.contains(canonical),
            "expected canonical stable-release boundary wording in {doc}"
        );
    }
}
