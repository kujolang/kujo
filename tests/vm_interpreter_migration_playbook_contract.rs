use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn vm_interpreter_migration_playbook_includes_runtime_recipes() {
    let path = repo_root().join("docs").join("VM_INTERPRETER_MIGRATION_PLAYBOOK.md");
    let content = fs::read_to_string(&path)
        .expect("failed to read docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md");

    for marker in [
        "# VM-First Migration Playbook (From `--interpreter`-Pinned Workflows)",
        "## Quick Decision Table",
        "`kujo run <file>`",
        "`kujo test --runtime dual`",
        "`kujo test --runtime vm`",
        "`kujo run --interpreter <file>`",
        "## Recommended Verification Commands",
        "cargo run -- test --runtime vm",
        "cargo run -- test --runtime dual",
        "cargo test --test vm_interpreter_parity_surfaces",
        "cargo test --test package_module_workflow_integration",
    ] {
        assert!(content.contains(marker), "migration playbook should contain marker {marker:?}");
    }
}
