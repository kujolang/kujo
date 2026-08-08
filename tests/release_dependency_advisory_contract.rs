use std::fs;
use std::path::PathBuf;

#[test]
fn lockfile_no_longer_contains_resolved_advisory_warning_crates() {
    let lockfile = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"))
        .expect("failed to read Cargo.lock");

    for removed in ["name = \"core2\"", "name = \"proc-macro-error2\""] {
        assert!(
            !lockfile.contains(removed),
            "Cargo.lock should not reintroduce resolved advisory warning crate {removed:?}"
        );
    }
}
