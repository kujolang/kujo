// File: src/workflow_pack/manifest.rs
//
// Workflow pack manifest parsing and validation.
// Uses YAML format (kujo-pack.yaml) with serde_yaml.

use crate::reserved_names::{self, WorkflowPackTrust};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The canonical filename for a workflow pack manifest.
pub const MANIFEST_FILENAME: &str = "kujo-pack.yaml";

/// A single command definition in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandDef {
    pub name: String,
    pub summary: String,
    pub entry: String,
    #[serde(default)]
    pub safe: bool,
    #[serde(default)]
    pub writes_files: bool,
    #[serde(default)]
    pub runs_processes: bool,
    #[serde(default)]
    pub requires_network: bool,
}

/// A doctor profile contribution from a workflow pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorProfileDef {
    pub name: String,
    pub entry: String,
    #[serde(default)]
    pub summary: String,
}

/// Declared extension-point contributions for a pack.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contributions {
    #[serde(default)]
    pub doctor_profiles: Vec<DoctorProfileDef>,
}

/// The full workflow pack manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackManifest {
    pub id: String,
    pub namespace: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub commands: Vec<CommandDef>,
    #[serde(default)]
    pub contributes: Contributions,
}

/// Validation error with a human-readable message.
#[derive(Debug, Clone)]
pub struct ManifestError {
    pub message: String,
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<String> for ManifestError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

/// Parse a manifest from YAML string content.
#[allow(dead_code)]
pub fn parse_manifest(content: &str) -> Result<PackManifest, ManifestError> {
    parse_manifest_with_trust(content, WorkflowPackTrust::ThirdParty)
}

/// Parse a manifest from YAML string content with explicit trust policy.
pub fn parse_manifest_with_trust(
    content: &str,
    trust: WorkflowPackTrust,
) -> Result<PackManifest, ManifestError> {
    let manifest: PackManifest = serde_yaml::from_str(content)
        .map_err(|e| ManifestError { message: format!("Failed to parse manifest: {}", e) })?;

    validate_manifest_with_trust(&manifest, trust)?;
    Ok(manifest)
}

/// Parse a manifest from a file path.
#[allow(dead_code)]
pub fn parse_manifest_file(path: &Path) -> Result<PackManifest, ManifestError> {
    parse_manifest_file_with_trust(path, WorkflowPackTrust::ThirdParty)
}

/// Parse a manifest from a file path with explicit trust policy.
pub fn parse_manifest_file_with_trust(
    path: &Path,
    trust: WorkflowPackTrust,
) -> Result<PackManifest, ManifestError> {
    let content = std::fs::read_to_string(path).map_err(|e| ManifestError {
        message: format!("Failed to read manifest file '{}': {}", path.display(), e),
    })?;
    parse_manifest_with_trust(&content, trust)
}

/// Validate a parsed manifest.
#[allow(dead_code)]
pub fn validate_manifest(manifest: &PackManifest) -> Result<(), ManifestError> {
    validate_manifest_with_trust(manifest, WorkflowPackTrust::ThirdParty)
}

/// Validate a parsed manifest with explicit trust policy.
pub fn validate_manifest_with_trust(
    manifest: &PackManifest,
    trust: WorkflowPackTrust,
) -> Result<(), ManifestError> {
    if manifest.id.trim().is_empty() {
        return Err(ManifestError {
            message: "Manifest 'id' is required and must not be empty.".to_string(),
        });
    }

    if manifest.namespace.trim().is_empty() {
        return Err(ManifestError {
            message: "Manifest 'namespace' is required and must not be empty.".to_string(),
        });
    }

    if !is_cli_safe_name(&manifest.namespace) {
        return Err(ManifestError {
            message: format!(
                "Manifest 'namespace' '{}' is not CLI-safe. Namespace must match [a-z][a-z0-9-]*.",
                manifest.namespace
            ),
        });
    }

    if trust == WorkflowPackTrust::ThirdParty {
        if let Some(reservation) = reserved_names::reservation_for_namespace(&manifest.namespace) {
            return Err(ManifestError {
                message: reserved_names::external_reserved_namespace_error(
                    &manifest.namespace,
                    reservation,
                ),
            });
        }
    }

    if manifest.name.trim().is_empty() {
        return Err(ManifestError {
            message: "Manifest 'name' is required and must not be empty.".to_string(),
        });
    }

    if manifest.version.trim().is_empty() {
        return Err(ManifestError {
            message: "Manifest 'version' is required and must not be empty.".to_string(),
        });
    }

    if manifest.commands.is_empty() && manifest.contributes.doctor_profiles.is_empty() {
        return Err(ManifestError {
            message: "Manifest must declare at least one command or contribution.".to_string(),
        });
    }

    let mut seen_command_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for cmd in &manifest.commands {
        if cmd.name.trim().is_empty() {
            return Err(ManifestError {
                message: "Each command must have a non-empty 'name'.".to_string(),
            });
        }

        if !is_cli_safe_command_name(&cmd.name) {
            return Err(ManifestError {
				message: format!(
					"Command name '{}' is not CLI-safe. Use lowercase letters, digits, hyphens, and spaces for nested commands.",
					cmd.name
				),
			});
        }

        if cmd.entry.trim().is_empty() {
            return Err(ManifestError {
                message: format!("Command '{}' must have a non-empty 'entry'.", cmd.name),
            });
        }

        if !seen_command_names.insert(cmd.name.clone()) {
            return Err(ManifestError {
                message: format!("Duplicate command name '{}' in manifest.", cmd.name),
            });
        }
    }

    let mut seen_profile_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for profile in &manifest.contributes.doctor_profiles {
        if profile.name.trim().is_empty() {
            return Err(ManifestError {
                message: "Each doctor profile contribution must have a non-empty 'name'."
                    .to_string(),
            });
        }
        if !is_cli_safe_name(&profile.name) {
            return Err(ManifestError {
                message: format!(
					"Doctor profile name '{}' is not CLI-safe. Profile names must match [a-z][a-z0-9-]*.",
					profile.name
				),
            });
        }
        if trust == WorkflowPackTrust::ThirdParty {
            if let Some(reservation) = reserved_names::reservation_for_profile_name(&profile.name) {
                return Err(ManifestError {
                    message: format!(
                        "Doctor profile name '{}' is reserved by Kujo (category: {}).",
                        profile.name,
                        reservation.category_label(),
                    ),
                });
            }
        }
        if profile.entry.trim().is_empty() {
            return Err(ManifestError {
                message: format!(
                    "Doctor profile '{}' must have a non-empty 'entry'.",
                    profile.name
                ),
            });
        }
        if !seen_profile_names.insert(profile.name.clone()) {
            return Err(ManifestError {
                message: format!(
                    "Duplicate doctor profile name '{}' in manifest contributions.",
                    profile.name
                ),
            });
        }
    }

    Ok(())
}

/// Check if a name is safe for CLI usage (namespace).
/// Must match [a-z][a-z0-9-]* (lowercase start, lowercase, digits, hyphens only).
fn is_cli_safe_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let bytes = name.as_bytes();

    // First character must be lowercase letter.
    if !bytes[0].is_ascii_lowercase() {
        return false;
    }

    // Remaining characters must be lowercase, digit, or hyphen.
    for &b in &bytes[1..] {
        if !b.is_ascii_lowercase() && !b.is_ascii_digit() && b != b'-' {
            return false;
        }
    }

    true
}

/// Check if a command name is CLI-safe.
/// Allows lowercase, digits, hyphens, and spaces (for nested commands like "card check").
fn is_cli_safe_command_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let bytes = name.as_bytes();

    for &b in bytes {
        if !b.is_ascii_lowercase() && !b.is_ascii_digit() && b != b'-' && b != b' ' {
            return false;
        }
    }

    // No leading/trailing spaces.
    if bytes[0] == b' ' || bytes[bytes.len() - 1] == b' ' {
        return false;
    }

    // No double spaces.
    for window in bytes.windows(2) {
        if window[0] == b' ' && window[1] == b' ' {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest_yaml() -> String {
        r#"id: acme-tools
namespace: acme
name: Acme Tools
version: 0.1.0
description: Workflow commands for Acme teams.

commands:
  - name: doctor
    summary: Check whether the local development environment is ready.
    entry: builtin
    safe: true
    writes_files: false
    runs_processes: true
    requires_network: false
"#
        .to_string()
    }

    #[test]
    fn parse_valid_manifest() {
        let manifest = parse_manifest(&valid_manifest_yaml()).expect("valid manifest should parse");
        assert_eq!(manifest.id, "acme-tools");
        assert_eq!(manifest.namespace, "acme");
        assert_eq!(manifest.name, "Acme Tools");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.commands.len(), 1);
        assert_eq!(manifest.commands[0].name, "doctor");
    }

    #[test]
    fn reject_empty_id() {
        let yaml = valid_manifest_yaml().replace("id: acme-tools", "id: ''");
        let err = parse_manifest(&yaml).expect_err("empty id should fail");
        assert!(err.message.contains("id"));
    }

    #[test]
    fn reject_empty_namespace() {
        let yaml = valid_manifest_yaml().replace("namespace: acme", "namespace: ''");
        let err = parse_manifest(&yaml).expect_err("empty namespace should fail");
        assert!(err.message.contains("namespace"));
    }

    #[test]
    fn reject_invalid_namespace() {
        let yaml = valid_manifest_yaml().replace("namespace: acme", "namespace: AcmeTeam");
        let err = parse_manifest(&yaml).expect_err("camelCase namespace should fail");
        assert!(err.message.contains("CLI-safe"));
    }

    #[test]
    fn reject_namespace_with_underscore() {
        let yaml = valid_manifest_yaml().replace("namespace: acme", "namespace: team_updraft");
        let err = parse_manifest(&yaml).expect_err("namespace with underscore should fail");
        assert!(err.message.contains("CLI-safe"));
    }

    #[test]
    fn reject_namespace_starting_with_digit() {
        let yaml = valid_manifest_yaml().replace("namespace: acme", "namespace: 2team");
        let err = parse_manifest(&yaml).expect_err("namespace starting with digit should fail");
        assert!(err.message.contains("CLI-safe"));
    }

    #[test]
    fn accept_namespace_with_hyphens() {
        let yaml = valid_manifest_yaml().replace("namespace: acme", "namespace: my-team");
        let manifest = parse_manifest(&yaml).expect("hyphenated namespace should parse");
        assert_eq!(manifest.namespace, "my-team");
    }

    #[test]
    fn reject_empty_command_name() {
        let yaml = valid_manifest_yaml().replace("name: doctor", "name: ''");
        let err = parse_manifest(&yaml).expect_err("empty command name should fail");
        assert!(err.message.contains("non-empty"));
    }

    #[test]
    fn reject_missing_commands() {
        let yaml = r#"id: test
namespace: team
name: Test
version: 0.1.0
commands: []
"#;
        let err = parse_manifest(yaml).expect_err("empty commands should fail");
        assert!(err.message.contains("at least one command or contribution"));
    }

    #[test]
    fn reject_duplicate_command_names() {
        let yaml = r#"id: test
namespace: team
name: Test
version: 0.1.0
commands:
  - name: doctor
    summary: Check env.
    entry: builtin
  - name: doctor
    summary: Duplicate.
    entry: builtin
"#;
        let err = parse_manifest(yaml).expect_err("duplicate command names should fail");
        assert!(err.message.contains("Duplicate"));
    }

    #[test]
    fn accept_nested_command_name() {
        let yaml = r#"id: test
namespace: team
name: Test
version: 0.1.0
commands:
  - name: card check
    summary: Check cards.
    entry: builtin
"#;
        let manifest = parse_manifest(yaml).expect("nested command name should parse");
        assert_eq!(manifest.commands[0].name, "card check");
    }

    #[test]
    fn reject_command_name_with_leading_space() {
        let yaml = r#"id: test
namespace: team
name: Test
version: 0.1.0
commands:
  - name: " doctor"
    summary: Bad.
    entry: builtin
"#;
        let err = parse_manifest(yaml).expect_err("leading space should fail");
        assert!(err.message.contains("CLI-safe"));
    }

    #[test]
    fn reject_missing_entry() {
        let yaml = r#"id: test
namespace: team
name: Test
version: 0.1.0
commands:
  - name: doctor
    summary: Check env.
    entry: ''
"#;
        let err = parse_manifest(yaml).expect_err("empty entry should fail");
        assert!(err.message.contains("entry"));
    }

    #[test]
    fn reject_reserved_namespace_for_third_party_pack() {
        let yaml = valid_manifest_yaml().replace("namespace: acme", "namespace: kujo");
        let err =
            parse_manifest(&yaml).expect_err("reserved namespace should fail for third-party");
        assert!(err.message.contains("reserved"));
    }

    #[test]
    fn reject_core_and_first_party_namespaces_for_third_party_pack() {
        for namespace in ["run", "build", "doctor", "eval", "spec", "kujo", "std", "kennel"] {
            let yaml = valid_manifest_yaml()
                .replace("namespace: acme", &format!("namespace: {}", namespace));
            let err = parse_manifest(&yaml)
                .expect_err("reserved namespaces should fail for third-party packs");
            assert!(
                err.message.contains("reserved"),
                "expected reserved namespace message for '{}', got: {}",
                namespace,
                err.message
            );
        }
    }

    #[test]
    fn allow_reserved_namespace_for_first_party_pack() {
        let yaml = valid_manifest_yaml().replace("namespace: acme", "namespace: kujo");
        let manifest = parse_manifest_with_trust(&yaml, WorkflowPackTrust::FirstParty)
            .expect("first-party manifests can use reserved namespaces");
        assert_eq!(manifest.namespace, "kujo");
    }

    #[test]
    fn accept_doctor_profile_contribution() {
        let yaml = r#"id: acme-tools
namespace: acme
name: Acme
version: 0.1.0
commands:
  - name: status
    summary: Status check.
    entry: commands/status.kujo
contributes:
  doctor_profiles:
    - name: wordpress
      entry: commands/doctor-wordpress.kujo
      summary: WordPress project doctor profile.
"#;
        let manifest = parse_manifest(yaml).expect("doctor profile contributions should parse");
        assert_eq!(manifest.contributes.doctor_profiles.len(), 1);
        assert_eq!(manifest.contributes.doctor_profiles[0].name, "wordpress");
    }

    #[test]
    fn reject_reserved_doctor_profile_name() {
        let yaml = r#"id: acme-tools
namespace: acme
name: Acme
version: 0.1.0
commands:
  - name: status
    summary: Status check.
    entry: commands/status.kujo
contributes:
  doctor_profiles:
    - name: default
      entry: commands/doctor-default.kujo
"#;
        let err = parse_manifest(yaml).expect_err("reserved doctor profile names should fail");
        assert!(err.message.contains("reserved"));
    }

    #[test]
    fn first_party_can_use_reserved_doctor_profile_name() {
        let yaml = r#"id: kujo-doctor
namespace: doctor
name: Kujo Doctor
version: 0.1.0
commands:
  - name: status
    summary: Status.
    entry: builtin
contributes:
  doctor_profiles:
    - name: default
      entry: commands/doctor-default.kujo
"#;
        let manifest = parse_manifest_with_trust(yaml, WorkflowPackTrust::FirstParty)
            .expect("first-party doctor profiles can use reserved profile names");
        assert_eq!(manifest.contributes.doctor_profiles[0].name, "default");
    }
}
