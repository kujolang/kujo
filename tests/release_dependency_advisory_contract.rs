use std::fs;
use std::path::PathBuf;

#[test]
fn lockfile_no_longer_contains_resolved_advisory_warning_crates() {
    let lockfile = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"))
        .expect("failed to read Cargo.lock");

    for removed in ["name = \"core2\"", "name = \"proc-macro-error2\"", "name = \"rsa\""] {
        assert!(
            !lockfile.contains(removed),
            "Cargo.lock should not reintroduce resolved advisory warning crate {removed:?}"
        );
    }
}

#[test]
fn release_gate_does_not_suppress_the_resolved_rsa_advisory() {
    let release_gate = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/release_gate.sh"),
    )
    .expect("failed to read release gate");

    assert!(release_gate.contains("cargo audit"));
    assert!(!release_gate.contains("RUSTSEC-2023-0071"));
}
