use std::{fs, path::Path, process::Command};

#[test]
fn candidate_identity_matches_cli_lock_packages_and_public_docs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(version, "1.6.0");
    let metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("release/kujo-1.6.0-rc.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["version"], version);
    assert_eq!(metadata["intended_tag"], format!("v{version}"));
    assert_eq!(metadata["publication_authorized"], false);
    assert_eq!(metadata["native_archive"], format!("kujo-v{version}-{{platform}}.tar.gz"));
    assert_eq!(metadata["source_archive"], format!("kujo-v{version}-source.tar.gz"));
    let lock = fs::read_to_string(root.join("Cargo.lock")).unwrap();
    assert!(lock.contains(&format!("name = \"kujolang\"\nversion = \"{version}\"")));
    for path in [
        "npm/package.json",
        "npm/runtime/package.json",
        "npm/platforms/darwin-x64/package.json",
        "npm/platforms/darwin-arm64/package.json",
        "npm/platforms/linux-x64/package.json",
        "npm/platforms/linux-arm64/package.json",
        "npm/platforms/win32-x64/package.json",
    ] {
        let package: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join(path)).unwrap()).unwrap();
        assert_eq!(package["version"], version, "{path}");
        if let Some(deps) = package["optionalDependencies"].as_object() {
            assert!(deps.values().all(|v| v == version));
        }
    }
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.contains(&format!("The source tree is currently at `{version}`")));
    assert!(
        readme.contains("1.6.0 is released for Linux x64/arm64, macOS x64/arm64 and Windows x64")
    );
    let output = Command::new(env!("CARGO_BIN_EXE_kujo")).arg("--version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), format!("kujo {version}"));
    assert!(output.stderr.is_empty());
}
