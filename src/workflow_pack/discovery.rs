// File: src/workflow_pack/discovery.rs
//
// Workflow pack discovery system.
// Discovers packs from multiple locations: built-in, project-local, user-local, env paths.
#![allow(dead_code)]

use crate::workflow_pack::manifest::{self, PackManifest};
use std::path::{Path, PathBuf};

/// Environment variable for additional pack search paths (colon-separated).
pub const KUJO_PACK_PATH_ENV: &str = "KUJO_PACK_PATH";

/// Subdirectory name for project-local workflow packs.
pub const PROJECT_PACKS_DIR: &str = ".kujo/packs";

/// Subdirectory name for user-local workflow packs.
pub const USER_PACKS_DIR: &str = ".kujo/packs";

/// A discovered workflow pack with its manifest and source location.
#[derive(Debug, Clone)]
pub struct DiscoveredPack {
    pub manifest: PackManifest,
    /// The directory containing the manifest file.
    pub pack_dir: PathBuf,
    /// Where this pack was found.
    pub source: PackSource,
}

/// Where a workflow pack was discovered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackSource {
    /// Compiled into the Kujo binary.
    Builtin,
    /// Found in the project's .kujo/packs directory.
    ProjectLocal,
    /// Found in the user's home ~/.kujo/packs directory.
    UserLocal,
    /// Found via KUJO_PACK_PATH environment variable.
    EnvPath,
}

impl PackSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            PackSource::Builtin => "builtin",
            PackSource::ProjectLocal => "project-local",
            PackSource::UserLocal => "user-local",
            PackSource::EnvPath => "env-path",
        }
    }
}

/// Errors that can occur during pack discovery.
#[derive(Debug, Clone)]
pub struct DiscoveryError {
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

/// Discover all workflow packs from all sources.
///
/// Returns a list of successfully discovered packs and a list of non-fatal
/// discovery errors (e.g., malformed manifests in a pack directory).
///
/// Discovery order:
/// 1. Built-in packs (registered via `register_builtin_packs`)
/// 2. Project-local packs (./.kujo/packs/*)
/// 3. User-local packs (~/.kujo/packs/*)
/// 4. Env-path packs (KUJO_PACK_PATH)
pub fn discover_all_packs(
    builtin_packs: Vec<DiscoveredPack>,
    current_dir: &Path,
) -> (Vec<DiscoveredPack>, Vec<DiscoveryError>) {
    let mut packs: Vec<DiscoveredPack> = Vec::new();
    let mut errors: Vec<DiscoveryError> = Vec::new();

    // 1. Built-in packs always come first and have highest priority.
    packs.extend(builtin_packs);

    // 2. Project-local packs.
    let project_packs_dir = current_dir.join(PROJECT_PACKS_DIR);
    if project_packs_dir.is_dir() {
        let (mut found, mut errs) =
            discover_in_directory(&project_packs_dir, PackSource::ProjectLocal);
        packs.append(&mut found);
        errors.append(&mut errs);
    }

    // 3. User-local packs.
    if let Some(home) = dirs_home() {
        let user_packs_dir = home.join(USER_PACKS_DIR);
        if user_packs_dir.is_dir() {
            let (mut found, mut errs) =
                discover_in_directory(&user_packs_dir, PackSource::UserLocal);
            packs.append(&mut found);
            errors.append(&mut errs);
        }
    }

    // 4. Env-path packs.
    if let Ok(env_paths) = std::env::var(KUJO_PACK_PATH_ENV) {
        for path_str in env_paths.split(':') {
            let path_str = path_str.trim();
            if path_str.is_empty() {
                continue;
            }
            let pack_path = PathBuf::from(path_str);
            if pack_path.is_dir() {
                match discover_single_pack(&pack_path, PackSource::EnvPath) {
                    Ok(pack) => packs.push(pack),
                    Err(err) => errors.push(err),
                }
            }
        }
    }

    (packs, errors)
}

/// Discover packs in a directory by scanning subdirectories for manifest files.
fn discover_in_directory(
    dir: &Path,
    source: PackSource,
) -> (Vec<DiscoveredPack>, Vec<DiscoveryError>) {
    let mut packs = Vec::new();
    let mut errors = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(DiscoveryError {
                path: dir.to_path_buf(),
                message: format!("Failed to read directory: {}", e),
            });
            return (packs, errors);
        }
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        match discover_single_pack(&entry_path, source.clone()) {
            Ok(pack) => packs.push(pack),
            Err(err) => errors.push(err),
        }
    }

    (packs, errors)
}

/// Discover a single pack in a directory by looking for its manifest file.
fn discover_single_pack(
    pack_dir: &Path,
    source: PackSource,
) -> Result<DiscoveredPack, DiscoveryError> {
    let manifest_path = pack_dir.join(manifest::MANIFEST_FILENAME);

    if !manifest_path.is_file() {
        return Err(DiscoveryError {
            path: manifest_path,
            message: "Manifest file not found; expected kujo-pack.yaml in pack directory."
                .to_string(),
        });
    }

    let parsed = manifest::parse_manifest_file_with_trust(
        &manifest_path,
        crate::reserved_names::WorkflowPackTrust::ThirdParty,
    )
    .map_err(|e| DiscoveryError { path: manifest_path.clone(), message: e.message })?;

    Ok(DiscoveredPack { manifest: parsed, pack_dir: pack_dir.to_path_buf(), source })
}

/// Get the user's home directory.
fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from).or_else(|| {
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERPROFILE").ok().map(PathBuf::from)
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_pack::manifest::Contributions;

    #[test]
    fn discover_builtin_packs_always_returned() {
        let builtin = vec![DiscoveredPack {
            manifest: PackManifest {
                id: "test".to_string(),
                namespace: "test".to_string(),
                name: "Test".to_string(),
                version: "0.1.0".to_string(),
                description: String::new(),
                commands: vec![],
                contributes: Contributions::default(),
            },
            pack_dir: PathBuf::from("/builtin/test"),
            source: PackSource::Builtin,
        }];

        let (packs, errors) = discover_all_packs(builtin, Path::new("/nonexistent"));
        assert!(errors.is_empty());
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].source, PackSource::Builtin);
    }

    #[test]
    fn empty_builtin_list_works() {
        let (packs, errors) = discover_all_packs(vec![], Path::new("/nonexistent"));
        assert!(errors.is_empty());
        assert!(packs.is_empty());
    }
}
