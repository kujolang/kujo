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
fn release_artifacts_require_a_strict_dependency_audit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = fs::read_to_string(root.join(".github/workflows/release-binaries.yml")).unwrap();
    let audit = fs::read_to_string(root.join(".github/workflows/supply-chain-audit.yml")).unwrap();

    assert!(release.contains("uses: ./.github/workflows/supply-chain-audit.yml"));
    assert!(release.contains("release-gate:\n    needs: dependency-audit"));
    assert!(release.contains("build-release-artifacts:\n    needs: release-gate"));
    assert!(release.contains("pack-source-archive:\n    needs: release-gate"));
    assert!(audit.contains("on:\n  workflow_call:"));
    assert!(audit.contains("run: cargo audit --deny warnings"));
    assert!(!audit.contains("continue-on-error: true"));
}

#[test]
fn native_upgrade_archives_remain_single_file_and_release_attaches_notices() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = fs::read_to_string(root.join(".github/workflows/release-binaries.yml")).unwrap();
    assert!(workflow
        .lines()
        .any(|line| { line.trim() == r#"tar -czf "${archive_name}" -C dist "${binary_name}""# }));
    assert!(workflow.contains(
        "Compress-Archive -Path 'dist/${binary_name}' -DestinationPath '${archive_name}'"
    ));
    assert!(workflow.contains("release-artifacts/release-source/THIRD_PARTY_NOTICES.md"));
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

#[test]
fn release_platform_docs_cover_linux_arm64_and_vendored_windows_builds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let validation = fs::read_to_string(root.join("docs/RELEASE_ARTIFACT_VALIDATION.md")).unwrap();
    let setup = fs::read_to_string(root.join("docs/SETUP_KUJO_ACTION.md")).unwrap();
    let workflow = fs::read_to_string(root.join(".github/workflows/release-binaries.yml")).unwrap();
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();

    assert!(workflow.contains("os: ubuntu-24.04-arm\n            platform: linux-arm64"));
    assert!(validation.contains("Linux arm64: `ubuntu-24.04-arm`"));
    assert!(manifest.lines().any(|line| {
        line.starts_with("openssl = ") && line.contains("features = [\"vendored\"]")
    }));
    assert!(setup.contains("The Windows release workflow compiles vendored OpenSSL through Cargo."));
}
