use crate::reserved_names;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const LOCKFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct KujoManifest {
    pub package: PackageMetadata,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct KujoLockfile {
    pub schema_version: u32,
    pub package: PackageMetadata,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PackageTrust {
    FirstParty,
    ThirdParty,
}

pub fn default_manifest(package_name: &str) -> String {
    let manifest = KujoManifest {
        package: PackageMetadata { name: package_name.to_string(), version: "0.1.0".to_string() },
        dependencies: BTreeMap::new(),
    };

    toml::to_string_pretty(&manifest).expect("manifest serialization should succeed")
}

pub fn parse_manifest(content: &str) -> Result<KujoManifest, String> {
    parse_manifest_with_trust(content, PackageTrust::ThirdParty)
}

pub fn parse_manifest_with_trust(
    content: &str,
    trust: PackageTrust,
) -> Result<KujoManifest, String> {
    let manifest = toml::from_str::<KujoManifest>(content)
        .map_err(|error| format!("Invalid kujo.toml: {}", error))?;
    validate_manifest(&manifest, trust)?;
    Ok(manifest)
}

pub fn parse_lockfile(content: &str) -> Result<KujoLockfile, String> {
    toml::from_str::<KujoLockfile>(content).map_err(|error| format!("Invalid kujo.lock: {}", error))
}

pub fn lockfile_from_manifest(manifest: &KujoManifest) -> KujoLockfile {
    KujoLockfile {
        schema_version: LOCKFILE_SCHEMA_VERSION,
        package: manifest.package.clone(),
        dependencies: manifest.dependencies.clone(),
    }
}

pub fn serialize_lockfile(lockfile: &KujoLockfile) -> Result<String, String> {
    toml::to_string_pretty(lockfile)
        .map_err(|error| format!("Failed to serialize kujo.lock: {}", error))
}

pub fn verify_lockfile_matches_manifest(
    manifest: &KujoManifest,
    lockfile: &KujoLockfile,
) -> Result<(), String> {
    if lockfile.schema_version != LOCKFILE_SCHEMA_VERSION {
        return Err(format!(
            "kujo.lock schema_version {} is unsupported; expected {}",
            lockfile.schema_version, LOCKFILE_SCHEMA_VERSION
        ));
    }

    if lockfile.package != manifest.package {
        return Err(format!(
            "kujo.lock package metadata is out of date (lockfile={} {}, manifest={} {})",
            lockfile.package.name,
            lockfile.package.version,
            manifest.package.name,
            manifest.package.version
        ));
    }

    if lockfile.dependencies != manifest.dependencies {
        return Err("kujo.lock dependencies are out of date; run `kujo package-install` to regenerate lockfile".to_string());
    }

    Ok(())
}

pub fn default_lockfile_path(manifest_path: &Path) -> PathBuf {
    manifest_path.parent().unwrap_or_else(|| Path::new(".")).join("kujo.lock")
}

pub fn add_dependency(content: &str, name: &str, version: &str) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Dependency name must not be empty".to_string());
    }
    if version.trim().is_empty() {
        return Err("Dependency version must not be empty".to_string());
    }

    let mut manifest = parse_manifest(content)?;
    manifest.dependencies.insert(name.to_string(), version.to_string());

    toml::to_string_pretty(&manifest)
        .map_err(|error| format!("Failed to serialize kujo.toml: {}", error))
}

fn validate_manifest(manifest: &KujoManifest, trust: PackageTrust) -> Result<(), String> {
    let package_name = manifest.package.name.trim();
    if package_name.is_empty() {
        return Err("Invalid kujo.toml: [package].name must not be empty".to_string());
    }
    if manifest.package.version.trim().is_empty() {
        return Err("Invalid kujo.toml: [package].version must not be empty".to_string());
    }

    if trust == PackageTrust::ThirdParty {
        if let Some(reservation) = reserved_names::reservation_for_package_name(package_name) {
            return Err(format!(
                "Package name '{}' is reserved by Kujo (category: {}). External packages cannot claim reserved package names.",
                package_name,
                reservation.category_label(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        add_dependency, default_lockfile_path, default_manifest, lockfile_from_manifest,
        parse_lockfile, parse_manifest, parse_manifest_with_trust, serialize_lockfile,
        verify_lockfile_matches_manifest, PackageTrust,
    };
    use std::path::Path;

    #[test]
    fn default_manifest_contains_package_section() {
        let content = default_manifest("demo");
        assert!(content.contains("[package]"));
        assert!(content.contains("name = \"demo\""));
        assert!(content.contains("version = \"0.1.0\""));
    }

    #[test]
    fn add_dependency_updates_manifest() {
        let content = default_manifest("demo");
        let updated =
            add_dependency(&content, "http", "1.2.3").expect("dependency update should succeed");

        let manifest = parse_manifest(&updated).expect("updated manifest should parse");
        assert_eq!(manifest.dependencies.get("http").map(|value| value.as_str()), Some("1.2.3"));
    }

    #[test]
    fn add_dependency_rejects_empty_values() {
        let content = default_manifest("demo");
        assert!(add_dependency(&content, "", "1.0.0").is_err());
        assert!(add_dependency(&content, "http", "").is_err());
    }

    #[test]
    fn lockfile_round_trip_is_deterministic() {
        let manifest = parse_manifest(&default_manifest("demo")).expect("manifest should parse");
        let lockfile = lockfile_from_manifest(&manifest);
        let serialized = serialize_lockfile(&lockfile).expect("lockfile should serialize");
        let reparsed = parse_lockfile(&serialized).expect("lockfile should parse");
        assert_eq!(lockfile, reparsed);
    }

    #[test]
    fn verify_lockfile_reports_manifest_drift() {
        let manifest_content = add_dependency(&default_manifest("demo"), "http", "1.2.3")
            .expect("dependency add should succeed");
        let manifest = parse_manifest(&manifest_content).expect("manifest should parse");
        let mut lockfile = lockfile_from_manifest(&manifest);
        lockfile.dependencies.insert("http".to_string(), "9.9.9".to_string());

        let error = verify_lockfile_matches_manifest(&manifest, &lockfile)
            .expect_err("lockfile mismatch should fail");
        assert!(error.contains("dependencies are out of date"));
    }

    #[test]
    fn default_lockfile_path_follows_manifest_directory() {
        let lockfile = default_lockfile_path(Path::new("/tmp/project/kujo.toml"));
        assert_eq!(lockfile, Path::new("/tmp/project/kujo.lock"));
    }

    #[test]
    fn parse_manifest_rejects_reserved_package_name_for_third_party() {
        let content = r#"
[package]
name = "kennel"
version = "0.1.0"
"#;
        let error = parse_manifest(content).expect_err("reserved package names should be rejected");
        assert!(error.contains("reserved by Kujo"));
    }

    #[test]
    fn parse_manifest_allows_reserved_package_name_for_first_party() {
        let content = r#"
[package]
name = "kennel"
version = "0.1.0"
"#;
        let parsed = parse_manifest_with_trust(content, PackageTrust::FirstParty)
            .expect("first-party manifests can use reserved package names");
        assert_eq!(parsed.package.name, "kennel");
    }
}
