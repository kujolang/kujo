use std::fs;
use std::path::PathBuf;

use kujo::interpreter::Interpreter;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn stdlib_reference_mentions_every_runtime_builtin() {
    let path = repo_root().join("docs").join("STANDARD_LIBRARY_REFERENCE.md");
    let content =
        fs::read_to_string(path).expect("failed to read docs/STANDARD_LIBRARY_REFERENCE.md");

    for builtin in Interpreter::get_builtin_names() {
        let marker = format!("`{builtin}`");
        assert!(
            content.contains(&marker),
            "standard library reference should mention runtime builtin '{builtin}'"
        );
    }
}

#[test]
fn stdlib_reference_defines_v1_tier_guarantee_policy() {
    let path = repo_root().join("docs").join("STANDARD_LIBRARY_REFERENCE.md");
    let content =
        fs::read_to_string(path).expect("failed to read docs/STANDARD_LIBRARY_REFERENCE.md");

    for marker in [
        "v1 contract policy for tiers:",
        "`stable`: in-scope for v1 compatibility guarantees.",
        "`preview`: in-scope for v1 usage, but not frozen; behavior may tighten during v1 hardening and must be treated as non-guaranteed until promoted.",
        "`experimental`: explicitly non-guaranteed for v1 compatibility commitments; available for advanced workflows only and may change or be restricted without stability guarantees.",
        "Release boundary: Kujo `v1.0.0` is stable, while `preview` and `experimental` tiers retain the narrower guarantees defined above.",
        "Deferred/non-goal policy source: `docs/V1_SCOPE.md`.",
    ] {
        assert!(
            content.contains(marker),
            "standard library reference should contain marker {marker:?}"
        );
    }
}
