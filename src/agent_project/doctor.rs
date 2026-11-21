use super::*;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, OpenOptions};
use std::io::Read;

const SECRET_SCAN_FILE_BYTES: u64 = 1_048_576;

pub fn doctor_report(cwd: &Path, deep: bool) -> DoctorReport {
    let mut checks = Vec::new();
    match discover(cwd).and_then(|r| load(&r).map(|m| (r, m))) {
        Ok((root, m)) => {
            checks.push(check(
                "agent.project",
                "Agent Project contract",
                CheckStatus::Pass,
                CheckSeverity::Info,
                Some(m.contract),
                None,
            ));
            for (rel, id) in [
                (&m.agent.manifest, "agent.manifest"),
                (&m.runtime.entrypoint, "agent.entrypoint"),
                (&m.runtime.provider_config, "agent.provider"),
                (&m.agent.evals, "agent.evals"),
                (&m.schema, "agent.schema"),
            ] {
                let ok = root.join(rel).is_file();
                checks.push(check(
                    id,
                    rel,
                    if ok { CheckStatus::Pass } else { CheckStatus::Fail },
                    if ok { CheckSeverity::Info } else { CheckSeverity::High },
                    Some(if ok { "present".into() } else { "missing".into() }),
                    None,
                ));
            }
            let dependencies = declared_dependencies(&root);
            for (name, details) in &dependencies {
                let installed = details.get("installed").and_then(Value::as_bool).unwrap_or(false);
                checks.push(check(
                    &format!("agent.dependencies.{name}"),
                    &format!("{name} installed"),
                    if installed { CheckStatus::Pass } else { CheckStatus::Fail },
                    if installed { CheckSeverity::Info } else { CheckSeverity::High },
                    Some(if installed { "installed".into() } else { "missing".into() }),
                    if installed {
                        None
                    } else {
                        Some("Run `kennel install` from the Agent Project root.".into())
                    },
                ));
            }
            for (name, tool) in &m.external_tools {
                let available = external_tool_available(&root, name, &tool.command);
                checks.push(check(
                    &format!("agent.external.{name}"),
                    &format!("{name} command available"),
                    if available { CheckStatus::Pass } else { CheckStatus::Fail },
                    if available { CheckSeverity::Info } else { CheckSeverity::High },
                    Some(if available { tool.command.clone() } else { "missing".into() }),
                    if available {
                        None
                    } else {
                        Some(
                            "Install the focused ecosystem with `bash install.sh --group agent`."
                                .into(),
                        )
                    },
                ));
            }
            for (name, enabled) in &m.integrations {
                if *enabled {
                    checks.push(check(
                        &format!("agent.integration.{name}"),
                        &format!("{name} integration configuration"),
                        CheckStatus::Pass,
                        CheckSeverity::Info,
                        Some("active and structurally valid".into()),
                        None,
                    ));
                }
            }
            let provider =
                parse_json_file(&root.join(&m.runtime.provider_config), "provider configuration")
                    .unwrap_or_else(|_| json!({}));
            if let Some(name) =
                provider.get("api_key_env").and_then(Value::as_str).filter(|name| !name.is_empty())
            {
                let credential = credentials::resolve_for_project(&root, name);
                let (status, observed, message) = match credential {
                    Ok(Some(value)) => (
                        CheckStatus::Pass,
                        format!("configured via {}", value.source.label()),
                        None,
                    ),
                    Ok(None) => (
                        CheckStatus::Fail,
                        "missing".into(),
                        Some(format!(
                            "Run `kujo agent auth set {}` to save it securely.",
                            provider.get("provider").and_then(Value::as_str).unwrap_or("custom")
                        )),
                    ),
                    Err(error) => (CheckStatus::Fail, "unavailable".into(), Some(error.message)),
                };
                checks.push(check(
                    "agent.provider.credential",
                    &format!("{name} credential available"),
                    status,
                    if status == CheckStatus::Pass {
                        CheckSeverity::Info
                    } else {
                        CheckSeverity::High
                    },
                    Some(observed),
                    message,
                ));
            }
            let secret_files = secret_like_files(&root);
            checks.push(check(
                "agent.security.committed-secrets",
                "Repository configuration contains no secret-like values",
                if secret_files.is_empty() {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                },
                if secret_files.is_empty() {
                    CheckSeverity::Info
                } else {
                    CheckSeverity::High
                },
                Some(if secret_files.is_empty() {
                    "no secret-like values detected".into()
                } else {
                    format!("review files: {}", secret_files.join(", "))
                }),
                if secret_files.is_empty() {
                    None
                } else {
                    Some("Remove credentials from repository-owned configuration and rotate exposed values.".into())
                },
            ));
            let broad = m
                .runtime
                .capabilities
                .iter()
                .any(|capability| matches!(capability.as_str(), "all" | "allow-all"));
            checks.push(check(
                "agent.security.capabilities",
                "Runtime capabilities are explicit and bounded",
                if broad { CheckStatus::Warn } else { CheckStatus::Pass },
                if broad { CheckSeverity::High } else { CheckSeverity::Info },
                Some(m.runtime.capabilities.join(", ")),
                broad.then(|| {
                    "Replace broad capability grants with the minimum required set.".into()
                }),
            ));
            if deep {
                let pins_valid = !dependencies.is_empty()
                    && dependencies.values().all(|details| {
                        details
                            .get("commit")
                            .and_then(Value::as_str)
                            .map(|commit| {
                                commit.len() == 40
                                    && commit.chars().all(|character| character.is_ascii_hexdigit())
                            })
                            .unwrap_or(false)
                    });
                checks.push(check(
                    "agent.dependencies",
                    "Pinned ecosystem dependencies",
                    if pins_valid { CheckStatus::Pass } else { CheckStatus::Fail },
                    CheckSeverity::Medium,
                    Some(if pins_valid {
                        "all dependencies use immutable 40-character commits".into()
                    } else {
                        "one or more dependency pins are missing or mutable".into()
                    }),
                    if pins_valid {
                        None
                    } else {
                        Some("Use immutable git revisions in kennel.toml.".into())
                    },
                ));
            }
        }
        Err(e) => checks.push(check(
            "agent.project",
            "Agent Project contract",
            CheckStatus::Fail,
            CheckSeverity::High,
            None,
            Some(e.message),
        )),
    }
    let mut r = DoctorReport::new("kujo-doctor", "doctor", "doctor", checks);
    r.tool = Some("kujo-doctor".into());
    r.profile = Some("agent".into());
    r.schema_version = Some("0.1.0".into());
    r.cwd = Some(cwd.display().to_string());
    r
}

fn secret_like_files(root: &Path) -> Vec<String> {
    fn walk(directory: &Dir, relative: &Path, depth: usize, matches: &mut Vec<String>) {
        if depth > 8 {
            return;
        }
        let Ok(entries) = directory.entries() else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name();
            let path = relative.join(&name);
            if file_type.is_dir() {
                if matches!(
                    name.to_string_lossy().as_ref(),
                    ".git" | "kennel_packages" | ".kennel_tmp" | ".eval-results" | ".runledger"
                ) {
                    continue;
                }
                if let Ok(child) = directory.open_dir_nofollow(&name) {
                    walk(&child, &path, depth + 1, matches);
                }
                continue;
            }
            if !file_type.is_file()
                || name == ".env.example"
                || name == ".env.local"
                || name.to_string_lossy().starts_with(credentials::CREDENTIAL_TEMP_PREFIX)
            {
                continue;
            }
            let mut options = OpenOptions::new();
            options.read(true).follow(FollowSymlinks::No).nonblock(true);
            let Ok(file) = directory.open_with(&name, &options) else {
                continue;
            };
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            if !metadata.is_file() || metadata.len() > SECRET_SCAN_FILE_BYTES {
                continue;
            }
            let mut source = String::new();
            if file.take(SECRET_SCAN_FILE_BYTES + 1).read_to_string(&mut source).is_err()
                || source.len() as u64 > SECRET_SCAN_FILE_BYTES
            {
                continue;
            }
            let has_secret_prefix = source.lines().any(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    return false;
                }
                let assignment_secret = [
                    "OPENAI_API_KEY=",
                    "ANTHROPIC_API_KEY=",
                    "WATCHDOG_TOKEN=",
                    "MCP_TOKEN=",
                    "BEARER_TOKEN=",
                ]
                .iter()
                .any(|prefix| {
                    trimmed
                        .strip_prefix(prefix)
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
                });
                assignment_secret
                    || trimmed.contains("\"api_key\": \"sk-")
                    || trimmed.contains("\"token\": \"sk-")
            });
            if has_secret_prefix {
                matches.push(path.to_string_lossy().into_owned());
            }
        }
    }
    let mut matches = Vec::new();
    if let Ok(directory) = Dir::open_ambient_dir(root, cap_std::ambient_authority()) {
        walk(&directory, Path::new(""), 0, &mut matches);
    }
    matches.sort();
    matches.dedup();
    matches
}
fn check(
    id: &str,
    label: &str,
    status: CheckStatus,
    severity: CheckSeverity,
    observed: Option<String>,
    message: Option<String>,
) -> CheckResult {
    CheckResult {
        id: id.into(),
        label: label.into(),
        status,
        severity,
        observed,
        expected: None,
        message,
        suggested_fix: None,
        reason: None,
        category: Some("agent".into()),
        observed_major: None,
        minimum_major: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_scan_reports_sorted_small_project_files() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("config")).unwrap();
        fs::write(root.path().join("config/model.env"), "OPENAI_API_KEY=placeholder").unwrap();
        fs::write(root.path().join(".env"), "BEARER_TOKEN=placeholder").unwrap();
        fs::write(root.path().join(".env.local"), "BEARER_TOKEN=placeholder").unwrap();
        fs::write(root.path().join(".env.example"), "BEARER_TOKEN=placeholder").unwrap();
        let mut large = "MCP_TOKEN=placeholder\n".to_string();
        large.push_str(&" ".repeat(SECRET_SCAN_FILE_BYTES as usize));
        fs::write(root.path().join("large.env"), large).unwrap();

        assert_eq!(
            secret_like_files(root.path()),
            [
                ".env".to_string(),
                Path::new("config").join("model.env").to_string_lossy().into_owned()
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn secret_scan_does_not_follow_linked_files_or_directories() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let config = other.path().join("settings.env");
        fs::write(&config, "MCP_TOKEN=placeholder").unwrap();
        std::os::unix::fs::symlink(config, root.path().join("linked.env")).unwrap();
        std::os::unix::fs::symlink(other.path(), root.path().join("linked")).unwrap();

        assert!(secret_like_files(root.path()).is_empty());
    }
}
