use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn stdlib_reference_defines_v1_tier_guarantee_policy() {
    let path = repo_root().join("docs").join("STANDARD_LIBRARY_REFERENCE.md");
    let content =
        fs::read_to_string(path).expect("failed to read docs/STANDARD_LIBRARY_REFERENCE.md");

    for marker in [
        "v1 contract policy for tiers:",
        "`stable`: in-scope for v1 compatibility guarantees.",
        "`preview`: in-scope for v1 usage, but not frozen; behavior may tighten during pre-v1 hardening and must be treated as non-guaranteed until promoted.",
        "`experimental`: explicitly non-guaranteed for v1 compatibility commitments; available for advanced workflows only and may change or be restricted without stability guarantees.",
        "Canonical readiness boundary: Kujo remains a pre-tag `1.0.0` release candidate until `ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`, and tag-time artifact evidence are closed.",
        "Deferred/non-goal policy source: `docs/V1_SCOPE.md`.",
    ] {
        assert!(
            content.contains(marker),
            "standard library reference should contain marker {:?}",
            marker
        );
    }
}
