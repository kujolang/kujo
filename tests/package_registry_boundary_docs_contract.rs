use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect("failed to read doc")
}

#[test]
fn package_registry_boundary_distinguishes_history_from_current_ecosystem() {
    let readme = read("README.md");
    let release_process = read("docs/RELEASE_PROCESS.md");
    let workflow_packs = read("docs/WORKFLOW_PACKS.md");
    let kennel = read("docs/KENNEL_NAMESPACE_PLAN.md");
    let shipcheck = read("docs/SHIPCHECK_RELEASE_EXCEPTIONS.md");

    let required_boundary = "Kujo v1.0 package scope was local manifest and lockfile determinism";

    assert!(
        readme.contains(required_boundary),
        "README should name the local-only package launch boundary"
    );
    assert!(
        release_process.contains("### 2.6 Package registry and Kennel boundary")
            && release_process.contains(required_boundary)
            && release_process.contains("public Kennel registry that exists today")
            && release_process.contains("`kujo package-publish` is metadata preview only")
            && release_process.contains("--publish")
            && release_process.contains("must fail deterministically"),
        "release process should define the canonical package/Kennel v1 boundary"
    );
    assert!(
        workflow_packs.contains("## v1.0 Launch Boundary")
            && workflow_packs.contains("local workflow-pack execution only")
            && workflow_packs.contains("No public workflow-pack registry")
            && workflow_packs.contains("KUJO_PACK_PATH"),
        "workflow pack docs should limit v1 to local pack discovery and execution"
    );
    assert!(
        kennel.contains("## v1.0 Boundary")
            && kennel.contains("public Kennel registry is live")
            && kennel.contains("## Current boundary")
            && kennel.contains("not current release promises"),
        "Kennel namespace policy should separate v1 history from the current registry boundary"
    );
    assert!(
        shipcheck.contains("No kennel.toml found")
            && shipcheck.contains("local manifest/lockfile determinism")
            && shipcheck.contains("public Kennel registry is now live"),
        "ShipCheck exception should explain why missing kennel.toml remains intentional"
    );
}
