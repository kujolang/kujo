use super::*;

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
            for (name, details) in declared_dependencies(&root) {
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
                let dependencies = declared_dependencies(&root);
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
    fn walk(root: &Path, current: &Path, depth: usize, matches: &mut Vec<String>) {
        if depth > 8 {
            return;
        }
        let Ok(entries) = fs::read_dir(current) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if matches!(
                    name.as_ref(),
                    ".git" | "kennel_packages" | ".kennel_tmp" | ".eval-results" | ".runledger"
                ) {
                    continue;
                }
                walk(root, &path, depth + 1, matches);
                continue;
            }
            if name == ".env.example"
                || path.metadata().map(|m| m.len() > 1_048_576).unwrap_or(true)
            {
                continue;
            }
            let Ok(source) = fs::read_to_string(&path) else {
                continue;
            };
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
                matches
                    .push(path.strip_prefix(root).unwrap_or(&path).to_string_lossy().into_owned());
            }
        }
    }
    let mut matches = Vec::new();
    walk(root, root, 0, &mut matches);
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
