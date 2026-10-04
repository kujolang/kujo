use std::{fs, path::Path};

#[test]
fn local_rc_builder_uses_the_current_cargo_version() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = fs::read_to_string(root.join("scripts/build_local_rc.sh")).unwrap();
    assert!(script.contains("Invalid Cargo package version"));
    assert!(!script.contains("[[ \"$version\" == 1.6.0 ]]"));
}

#[test]
fn tagged_release_packages_and_publishes_the_declared_source_archive() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = fs::read_to_string(root.join(".github/workflows/release-binaries.yml")).unwrap();
    assert!(workflow.contains("pack-source-archive:"));
    assert!(workflow.contains("kujo-${RELEASE_NAME}-source.tar.gz"));
    assert!(workflow.contains("git archive --format=tar"));
    assert!(workflow
        .contains("needs: [build-release-artifacts, pack-source-archive, pack-npm-runtime]"));
}

#[test]
fn published_source_archive_is_checksum_and_version_verified() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        fs::read_to_string(root.join(".github/workflows/release-published-artifact-smoke.yml"))
            .unwrap();
    assert!(workflow.contains("validate-published-source:"));
    assert!(workflow.contains("sha256sum -c \"${archive_name}.sha256\""));
    assert!(workflow
        .contains("KUJO_RELEASE_TAG=\"${tag_name}\" bash .github/scripts/check-tag-version.sh"));
}
