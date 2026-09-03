use crate::workflow_pack::discovery::{DiscoveredPack, PackSource};
use crate::workflow_pack::manifest::parse_manifest_with_trust;
use crate::workflow_pack::registry::CommandHandler;
use crate::workflow_pack::types::{
    CheckResult, CheckSeverity, CheckStatus, CommandContext, DoctorReport,
};
use crate::{reserved_names::WorkflowPackTrust, workflow_pack::process_runner};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DOCTOR_PACK_ID: &str = "kujo-doctor";
pub const DOCTOR_NAMESPACE: &str = "doctor";
pub const DOCTOR_COMMAND: &str = "doctor";
pub const GENERIC_PROFILE_NAME: &str = "generic";
pub const DOCTOR_SCHEMA_VERSION: &str = "0.1.0";

const TOOL_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MIN_NODE_MAJOR: i64 = 18;
const MIN_PHP_MAJOR: i64 = 8;
const MIN_COMPOSER_MAJOR: i64 = 2;

#[derive(Debug, Clone)]
struct ProjectSignals {
    package_json: bool,
    node_modules: bool,
    package_lock: bool,
    composer_json: bool,
    vendor_dir: bool,
    composer_lock: bool,
    npm_scripts: Vec<String>,
    wordpress_signal: bool,
}

#[derive(Debug, Clone)]
struct CommandProbe {
    available: bool,
    observed_line: Option<String>,
    observed_major: Option<i64>,
    noisy: bool,
    noisy_excerpt: Option<String>,
}

pub fn discovered_pack() -> Result<DiscoveredPack, String> {
    let manifest_text = include_str!("../../../tools/kujo-doctor/kujo-pack.yaml");
    let manifest = parse_manifest_with_trust(manifest_text, WorkflowPackTrust::FirstParty)
        .map_err(|err| format!("Failed to parse bundled Kujo Doctor manifest: {}", err))?;
    Ok(DiscoveredPack {
        manifest,
        pack_dir: PathBuf::from("tools/kujo-doctor"),
        source: PackSource::Builtin,
    })
}

pub fn handlers() -> BTreeMap<String, CommandHandler> {
    let mut handlers = BTreeMap::new();
    handlers.insert(DOCTOR_COMMAND.to_string(), doctor_handler as CommandHandler);
    handlers
}

pub fn doctor_handler(ctx: &CommandContext) -> DoctorReport {
    let deep = ctx.args.iter().any(|arg| arg == "--deep");
    let mut report = run_generic_doctor_checks(&ctx.cwd, deep);
    decorate_report_metadata(&mut report, GENERIC_PROFILE_NAME);
    report
}

pub fn decorate_report_metadata(report: &mut DoctorReport, profile: &str) {
    report.tool = Some(DOCTOR_PACK_ID.to_string());
    report.pack = DOCTOR_PACK_ID.to_string();
    report.namespace = DOCTOR_NAMESPACE.to_string();
    report.command = DOCTOR_COMMAND.to_string();
    report.profile = Some(profile.to_string());
    report.schema_version = Some(DOCTOR_SCHEMA_VERSION.to_string());
}

pub fn merge_reports(
    base: &DoctorReport,
    profile_report: &DoctorReport,
    profile_name: &str,
) -> DoctorReport {
    let mut merged_checks = base.checks.clone();
    merged_checks.extend(profile_report.checks.clone());
    let mut merged =
        DoctorReport::new(DOCTOR_PACK_ID, DOCTOR_NAMESPACE, DOCTOR_COMMAND, merged_checks);
    merged.recommended_next_actions = Some(generate_recommended_actions(&merged.checks));
    merged.readiness = profile_report.readiness.clone();
    merged.production_send_enabled = profile_report.production_send_enabled;
    merged.profile_data = profile_report.profile_data.clone();
    decorate_report_metadata(&mut merged, profile_name);
    merged
}

fn run_generic_doctor_checks(cwd: &Path, deep: bool) -> DoctorReport {
    let mut checks: Vec<CheckResult> = Vec::new();
    let signals = detect_project_signals(cwd);

    let git_probe = probe_command("git", &["--version"]);
    checks.push(check_tool_presence(
        "env.git",
        "Git",
        &git_probe,
        "environment",
        true,
        "Install Git and ensure it is on PATH.",
    ));

    if git_probe.available {
        checks.extend(git_repository_checks());
    } else {
        checks.push(CheckResult {
            id: "repo.status".to_string(),
            label: "Git repository status".to_string(),
            status: CheckStatus::Skip,
            severity: CheckSeverity::Info,
            observed: None,
            expected: None,
            message: Some("Skipped because Git is not available.".to_string()),
            suggested_fix: None,
            reason: Some("not_applicable".to_string()),
            category: Some("repository".to_string()),
            observed_major: None,
            minimum_major: None,
        });
    }

    let node_probe = probe_command("node", &["--version"]);
    checks.push(check_tool_version(
        "env.node",
        "Node.js",
        &node_probe,
        MIN_NODE_MAJOR,
        signals.package_json,
        "environment",
        "Install Node.js 18 or later.",
        "Upgrade Node.js to version 18 or later.",
    ));

    let npm_probe = probe_command("npm", &["--version"]);
    checks.push(check_tool_presence(
        "env.npm",
        "npm",
        &npm_probe,
        "environment",
        signals.package_json,
        "Install npm (usually bundled with Node.js).",
    ));

    let php_probe = probe_command("php", &["--version"]);
    checks.push(check_tool_version(
        "env.php",
        "PHP",
        &php_probe,
        MIN_PHP_MAJOR,
        signals.composer_json,
        "environment",
        "Install PHP and ensure it is on PATH.",
        "Upgrade PHP to a supported major version.",
    ));

    let composer_probe = probe_command("composer", &["--version"]);
    checks.push(check_tool_version(
        "env.composer",
        "Composer",
        &composer_probe,
        MIN_COMPOSER_MAJOR,
        signals.composer_json,
        "environment",
        "Install Composer and ensure it is on PATH.",
        "Upgrade Composer to version 2 or later.",
    ));

    checks.push(check_wp_cli());

    checks.extend(project_dependency_checks(&signals, deep));
    checks.push(project_npm_scripts_check(&signals));
    checks.push(wordpress_signal_check(signals.wordpress_signal));

    let mut report = DoctorReport::new(DOCTOR_PACK_ID, DOCTOR_NAMESPACE, DOCTOR_COMMAND, checks);
    report.recommended_next_actions = Some(generate_recommended_actions(&report.checks));
    report
}

fn git_repository_checks() -> Vec<CheckResult> {
    let mut checks = Vec::new();

    let repo_probe = process_runner::run_command(
        "git",
        &["rev-parse", "--is-inside-work-tree"],
        Some(TOOL_PROBE_TIMEOUT),
    );
    match repo_probe {
        Ok(result) if result.success && result.stdout.trim() == "true" => {
            checks.push(CheckResult {
                id: "repo.detected".to_string(),
                label: "Git repository detected".to_string(),
                status: CheckStatus::Pass,
                severity: CheckSeverity::Info,
                observed: Some("inside repository".to_string()),
                expected: None,
                message: None,
                suggested_fix: None,
                reason: None,
                category: Some("repository".to_string()),
                observed_major: None,
                minimum_major: None,
            });
        }
        _ => {
            checks.push(CheckResult {
                id: "repo.detected".to_string(),
                label: "Git repository detected".to_string(),
                status: CheckStatus::Info,
                severity: CheckSeverity::Info,
                observed: Some("not a git repository".to_string()),
                expected: None,
                message: Some("Current directory is not inside a Git repository.".to_string()),
                suggested_fix: None,
                reason: Some("not_git_repo".to_string()),
                category: Some("repository".to_string()),
                observed_major: None,
                minimum_major: None,
            });
            return checks;
        }
    }

    let branch_probe =
        process_runner::run_command("git", &["branch", "--show-current"], Some(TOOL_PROBE_TIMEOUT));
    checks.push(match branch_probe {
        Ok(result) if result.success => CheckResult {
            id: "repo.branch".to_string(),
            label: "Current Git branch".to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some(if result.stdout.trim().is_empty() {
                "(detached HEAD)".to_string()
            } else {
                result.stdout.trim().to_string()
            }),
            expected: None,
            message: None,
            suggested_fix: None,
            reason: None,
            category: Some("repository".to_string()),
            observed_major: None,
            minimum_major: None,
        },
        _ => CheckResult {
            id: "repo.branch".to_string(),
            label: "Current Git branch".to_string(),
            status: CheckStatus::Warn,
            severity: CheckSeverity::Low,
            observed: None,
            expected: None,
            message: Some("Unable to determine the current Git branch.".to_string()),
            suggested_fix: Some(
                "Run `git branch --show-current` manually to inspect branch state.".to_string(),
            ),
            reason: Some("version_unparseable".to_string()),
            category: Some("repository".to_string()),
            observed_major: None,
            minimum_major: None,
        },
    });

    let dirty_probe =
        process_runner::run_command("git", &["status", "--porcelain"], Some(TOOL_PROBE_TIMEOUT));
    checks.push(match dirty_probe {
        Ok(result) if result.success => {
            let dirty_entries: Vec<&str> =
                result.stdout.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
            if dirty_entries.is_empty() {
                CheckResult {
                    id: "repo.working_tree".to_string(),
                    label: "Git working tree".to_string(),
                    status: CheckStatus::Pass,
                    severity: CheckSeverity::Info,
                    observed: Some("clean".to_string()),
                    expected: None,
                    message: None,
                    suggested_fix: None,
                    reason: None,
                    category: Some("repository".to_string()),
                    observed_major: None,
                    minimum_major: None,
                }
            } else {
                let preview = dirty_entries.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
                CheckResult {
                    id: "repo.working_tree".to_string(),
                    label: "Git working tree".to_string(),
                    status: CheckStatus::Warn,
                    severity: CheckSeverity::Medium,
                    observed: Some(format!("{} changed paths", dirty_entries.len())),
                    expected: Some("clean".to_string()),
                    message: Some(format!("Repository has uncommitted changes ({})", preview)),
                    suggested_fix: Some(
                        "Review/stage/commit changes before handoff to CI or automation."
                            .to_string(),
                    ),
                    reason: Some("dirty_worktree".to_string()),
                    category: Some("repository".to_string()),
                    observed_major: None,
                    minimum_major: None,
                }
            }
        }
        _ => CheckResult {
            id: "repo.working_tree".to_string(),
            label: "Git working tree".to_string(),
            status: CheckStatus::Warn,
            severity: CheckSeverity::Low,
            observed: None,
            expected: None,
            message: Some("Unable to determine Git working tree status.".to_string()),
            suggested_fix: Some(
                "Run `git status` manually to inspect repository state.".to_string(),
            ),
            reason: Some("version_unparseable".to_string()),
            category: Some("repository".to_string()),
            observed_major: None,
            minimum_major: None,
        },
    });

    checks
}

fn project_dependency_checks(signals: &ProjectSignals, deep: bool) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    checks.push(bool_presence_check(
        "project.package_json",
        "package.json",
        signals.package_json,
        "project",
        "config_missing",
    ));
    checks.push(bool_presence_check(
        "project.composer_json",
        "composer.json",
        signals.composer_json,
        "project",
        "config_missing",
    ));

    if signals.package_json {
        checks.push(if signals.node_modules {
            CheckResult {
                id: "project.node_modules".to_string(),
                label: "node_modules directory".to_string(),
                status: CheckStatus::Pass,
                severity: CheckSeverity::Info,
                observed: Some("present".to_string()),
                expected: None,
                message: None,
                suggested_fix: None,
                reason: None,
                category: Some("dependencies".to_string()),
                observed_major: None,
                minimum_major: None,
            }
        } else {
            CheckResult {
                id: "project.node_modules".to_string(),
                label: "node_modules directory".to_string(),
                status: CheckStatus::Warn,
                severity: CheckSeverity::Medium,
                observed: Some("missing".to_string()),
                expected: Some("present".to_string()),
                message: Some("package.json exists but node_modules is missing.".to_string()),
                suggested_fix: Some(
                    "Run `npm install` to install JavaScript dependencies.".to_string(),
                ),
                reason: Some("dependency_missing".to_string()),
                category: Some("dependencies".to_string()),
                observed_major: None,
                minimum_major: None,
            }
        });
    } else {
        checks.push(CheckResult {
            id: "project.node_modules".to_string(),
            label: "node_modules directory".to_string(),
            status: CheckStatus::Skip,
            severity: CheckSeverity::Info,
            observed: None,
            expected: None,
            message: Some("Skipped because package.json is not present.".to_string()),
            suggested_fix: None,
            reason: Some("not_applicable".to_string()),
            category: Some("dependencies".to_string()),
            observed_major: None,
            minimum_major: None,
        });
    }

    if signals.composer_json {
        checks.push(if signals.vendor_dir {
            CheckResult {
                id: "project.vendor".to_string(),
                label: "vendor directory".to_string(),
                status: CheckStatus::Pass,
                severity: CheckSeverity::Info,
                observed: Some("present".to_string()),
                expected: None,
                message: None,
                suggested_fix: None,
                reason: None,
                category: Some("dependencies".to_string()),
                observed_major: None,
                minimum_major: None,
            }
        } else {
            CheckResult {
                id: "project.vendor".to_string(),
                label: "vendor directory".to_string(),
                status: CheckStatus::Warn,
                severity: CheckSeverity::Medium,
                observed: Some("missing".to_string()),
                expected: Some("present".to_string()),
                message: Some("composer.json exists but vendor is missing.".to_string()),
                suggested_fix: Some(
                    "Run `composer install` to install PHP dependencies.".to_string(),
                ),
                reason: Some("dependency_missing".to_string()),
                category: Some("dependencies".to_string()),
                observed_major: None,
                minimum_major: None,
            }
        });
    } else {
        checks.push(CheckResult {
            id: "project.vendor".to_string(),
            label: "vendor directory".to_string(),
            status: CheckStatus::Skip,
            severity: CheckSeverity::Info,
            observed: None,
            expected: None,
            message: Some("Skipped because composer.json is not present.".to_string()),
            suggested_fix: None,
            reason: Some("not_applicable".to_string()),
            category: Some("dependencies".to_string()),
            observed_major: None,
            minimum_major: None,
        });
    }

    if deep {
        if signals.package_json && !signals.package_lock {
            checks.push(CheckResult {
                id: "project.node_lockfile".to_string(),
                label: "Node lockfile".to_string(),
                status: CheckStatus::Warn,
                severity: CheckSeverity::Low,
                observed: Some("missing".to_string()),
                expected: Some("package-lock.json, pnpm-lock.yaml, or yarn.lock".to_string()),
                message: Some("No Node lockfile detected.".to_string()),
                suggested_fix: Some(
                    "Generate and commit a lockfile for reproducible installs.".to_string(),
                ),
                reason: Some("config_missing".to_string()),
                category: Some("project".to_string()),
                observed_major: None,
                minimum_major: None,
            });
        }
        if signals.composer_json && !signals.composer_lock {
            checks.push(CheckResult {
                id: "project.composer_lock".to_string(),
                label: "Composer lockfile".to_string(),
                status: CheckStatus::Warn,
                severity: CheckSeverity::Low,
                observed: Some("missing".to_string()),
                expected: Some("composer.lock".to_string()),
                message: Some("composer.json is present but composer.lock is missing.".to_string()),
                suggested_fix: Some(
                    "Run `composer install` and commit composer.lock for deterministic installs."
                        .to_string(),
                ),
                reason: Some("config_missing".to_string()),
                category: Some("project".to_string()),
                observed_major: None,
                minimum_major: None,
            });
        }
    }

    checks
}

fn project_npm_scripts_check(signals: &ProjectSignals) -> CheckResult {
    if !signals.package_json {
        return CheckResult {
            id: "build.npm_scripts".to_string(),
            label: "npm script inventory".to_string(),
            status: CheckStatus::Skip,
            severity: CheckSeverity::Info,
            observed: None,
            expected: None,
            message: Some("Skipped because package.json is not present.".to_string()),
            suggested_fix: None,
            reason: Some("not_applicable".to_string()),
            category: Some("build".to_string()),
            observed_major: None,
            minimum_major: None,
        };
    }

    if signals.npm_scripts.is_empty() {
        CheckResult {
            id: "build.npm_scripts".to_string(),
            label: "npm script inventory".to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some("no scripts".to_string()),
            expected: None,
            message: Some("package.json has no scripts block.".to_string()),
            suggested_fix: None,
            reason: Some("not_applicable".to_string()),
            category: Some("build".to_string()),
            observed_major: None,
            minimum_major: None,
        }
    } else {
        CheckResult {
            id: "build.npm_scripts".to_string(),
            label: "npm script inventory".to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some(signals.npm_scripts.join(", ")),
            expected: None,
            message: None,
            suggested_fix: None,
            reason: None,
            category: Some("build".to_string()),
            observed_major: None,
            minimum_major: None,
        }
    }
}

fn wordpress_signal_check(wordpress_signal: bool) -> CheckResult {
    if wordpress_signal {
        CheckResult {
            id: "project.wordpress_signal".to_string(),
            label: "WordPress project signal".to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some("detected".to_string()),
            expected: None,
            message: Some(
                "Detected WordPress-like project structure (wp-config.php or wp-content)."
                    .to_string(),
            ),
            suggested_fix: None,
            reason: None,
            category: Some("project".to_string()),
            observed_major: None,
            minimum_major: None,
        }
    } else {
        CheckResult {
            id: "project.wordpress_signal".to_string(),
            label: "WordPress project signal".to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some("not detected".to_string()),
            expected: None,
            message: Some("No WordPress-specific project signals detected.".to_string()),
            suggested_fix: None,
            reason: Some("not_applicable".to_string()),
            category: Some("project".to_string()),
            observed_major: None,
            minimum_major: None,
        }
    }
}

fn check_wp_cli() -> CheckResult {
    let probe = probe_command("wp", &["--info"]);
    if !probe.available {
        return CheckResult {
            id: "project.wp_cli".to_string(),
            label: "WP-CLI".to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some("missing".to_string()),
            expected: None,
            message: Some("WP-CLI not found (optional).".to_string()),
            suggested_fix: None,
            reason: Some("missing".to_string()),
            category: Some("project".to_string()),
            observed_major: None,
            minimum_major: None,
        };
    }

    if probe.noisy {
        return CheckResult {
            id: "project.wp_cli".to_string(),
            label: "WP-CLI".to_string(),
            status: CheckStatus::Warn,
            severity: CheckSeverity::Medium,
            observed: probe.observed_line.clone(),
            expected: None,
            message: Some(format!(
                "WP-CLI emitted warning/deprecation/fatal output: {}",
                probe.noisy_excerpt.as_deref().unwrap_or("noisy output detected")
            )),
            suggested_fix: Some(
                "Update WP-CLI and verify PHP compatibility for local tooling.".to_string(),
            ),
            reason: Some("command_noisy".to_string()),
            category: Some("project".to_string()),
            observed_major: probe.observed_major,
            minimum_major: None,
        };
    }

    CheckResult {
        id: "project.wp_cli".to_string(),
        label: "WP-CLI".to_string(),
        status: CheckStatus::Pass,
        severity: CheckSeverity::Info,
        observed: probe.observed_line,
        expected: None,
        message: None,
        suggested_fix: None,
        reason: None,
        category: Some("project".to_string()),
        observed_major: probe.observed_major,
        minimum_major: None,
    }
}

fn check_tool_presence(
    id: &str,
    label: &str,
    probe: &CommandProbe,
    category: &str,
    required: bool,
    install_fix: &str,
) -> CheckResult {
    if probe.available {
        if probe.noisy {
            return CheckResult {
                id: id.to_string(),
                label: label.to_string(),
                status: CheckStatus::Warn,
                severity: CheckSeverity::Low,
                observed: probe.observed_line.clone(),
                expected: None,
                message: Some(format!(
                    "{} emitted noisy output: {}",
                    label,
                    probe.noisy_excerpt.as_deref().unwrap_or("unknown warning")
                )),
                suggested_fix: Some(format!(
                    "Investigate {} installation and clean warning output.",
                    label
                )),
                reason: Some("command_noisy".to_string()),
                category: Some(category.to_string()),
                observed_major: probe.observed_major,
                minimum_major: None,
            };
        }
        return CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Pass,
            severity: CheckSeverity::Info,
            observed: probe.observed_line.clone(),
            expected: None,
            message: None,
            suggested_fix: None,
            reason: None,
            category: Some(category.to_string()),
            observed_major: probe.observed_major,
            minimum_major: None,
        };
    }

    if required {
        CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Warn,
            severity: CheckSeverity::Medium,
            observed: Some("missing".to_string()),
            expected: Some("available on PATH".to_string()),
            message: Some(format!("{} is required for this repository context.", label)),
            suggested_fix: Some(install_fix.to_string()),
            reason: Some("missing".to_string()),
            category: Some(category.to_string()),
            observed_major: None,
            minimum_major: None,
        }
    } else {
        CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some("missing".to_string()),
            expected: None,
            message: Some(format!("{} is optional for this repository context.", label)),
            suggested_fix: None,
            reason: Some("not_applicable".to_string()),
            category: Some(category.to_string()),
            observed_major: None,
            minimum_major: None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_tool_version(
    id: &str,
    label: &str,
    probe: &CommandProbe,
    minimum_major: i64,
    required: bool,
    category: &str,
    install_fix: &str,
    upgrade_fix: &str,
) -> CheckResult {
    if !probe.available {
        return if required {
            CheckResult {
                id: id.to_string(),
                label: label.to_string(),
                status: CheckStatus::Warn,
                severity: CheckSeverity::Medium,
                observed: Some("missing".to_string()),
                expected: Some(format!("major >= {}", minimum_major)),
                message: Some(format!("{} is required but not installed.", label)),
                suggested_fix: Some(install_fix.to_string()),
                reason: Some("missing".to_string()),
                category: Some(category.to_string()),
                observed_major: None,
                minimum_major: Some(minimum_major),
            }
        } else {
            CheckResult {
                id: id.to_string(),
                label: label.to_string(),
                status: CheckStatus::Info,
                severity: CheckSeverity::Info,
                observed: Some("missing".to_string()),
                expected: None,
                message: Some(format!("{} is optional in the current project context.", label)),
                suggested_fix: None,
                reason: Some("not_applicable".to_string()),
                category: Some(category.to_string()),
                observed_major: None,
                minimum_major: Some(minimum_major),
            }
        };
    }

    if probe.noisy {
        return CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Warn,
            severity: CheckSeverity::Low,
            observed: probe.observed_line.clone(),
            expected: Some(format!("major >= {}", minimum_major)),
            message: Some(format!(
                "{} emitted warning/deprecation/fatal output: {}",
                label,
                probe.noisy_excerpt.as_deref().unwrap_or("noisy output detected")
            )),
            suggested_fix: Some(format!(
                "Update {} or adjust runtime compatibility to remove noisy output.",
                label
            )),
            reason: Some("command_noisy".to_string()),
            category: Some(category.to_string()),
            observed_major: probe.observed_major,
            minimum_major: Some(minimum_major),
        };
    }

    match probe.observed_major {
        Some(major) if major < minimum_major => CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Warn,
            severity: CheckSeverity::Medium,
            observed: probe.observed_line.clone(),
            expected: Some(format!("major >= {}", minimum_major)),
            message: Some(format!(
                "{} major version {} is below recommended minimum {}.",
                label, major, minimum_major
            )),
            suggested_fix: Some(upgrade_fix.to_string()),
            reason: Some("version_too_old".to_string()),
            category: Some(category.to_string()),
            observed_major: Some(major),
            minimum_major: Some(minimum_major),
        },
        Some(major) => CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Pass,
            severity: CheckSeverity::Info,
            observed: probe.observed_line.clone(),
            expected: Some(format!("major >= {}", minimum_major)),
            message: None,
            suggested_fix: None,
            reason: None,
            category: Some(category.to_string()),
            observed_major: Some(major),
            minimum_major: Some(minimum_major),
        },
        None => CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Warn,
            severity: CheckSeverity::Low,
            observed: probe.observed_line.clone(),
            expected: Some(format!("major >= {}", minimum_major)),
            message: Some(format!("Unable to parse {} version output.", label)),
            suggested_fix: Some(format!(
                "Inspect `{}` version output and verify installation.",
                label
            )),
            reason: Some("version_unparseable".to_string()),
            category: Some(category.to_string()),
            observed_major: None,
            minimum_major: Some(minimum_major),
        },
    }
}

fn bool_presence_check(
    id: &str,
    label: &str,
    present: bool,
    category: &str,
    missing_reason: &str,
) -> CheckResult {
    if present {
        CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Pass,
            severity: CheckSeverity::Info,
            observed: Some("present".to_string()),
            expected: None,
            message: None,
            suggested_fix: None,
            reason: None,
            category: Some(category.to_string()),
            observed_major: None,
            minimum_major: None,
        }
    } else {
        CheckResult {
            id: id.to_string(),
            label: label.to_string(),
            status: CheckStatus::Info,
            severity: CheckSeverity::Info,
            observed: Some("missing".to_string()),
            expected: None,
            message: Some(format!("{} was not found in the current directory.", label)),
            suggested_fix: None,
            reason: Some(missing_reason.to_string()),
            category: Some(category.to_string()),
            observed_major: None,
            minimum_major: None,
        }
    }
}

fn detect_project_signals(cwd: &Path) -> ProjectSignals {
    let package_json_path = cwd.join("package.json");
    let composer_json_path = cwd.join("composer.json");

    let package_json = package_json_path.is_file();
    let node_modules = cwd.join("node_modules").is_dir();
    let package_lock = cwd.join("package-lock.json").is_file()
        || cwd.join("pnpm-lock.yaml").is_file()
        || cwd.join("yarn.lock").is_file();
    let composer_json = composer_json_path.is_file();
    let vendor_dir = cwd.join("vendor").is_dir();
    let composer_lock = cwd.join("composer.lock").is_file();
    let wordpress_signal = cwd.join("wp-config.php").is_file() || cwd.join("wp-content").is_dir();
    let npm_scripts = read_npm_scripts(&package_json_path);

    ProjectSignals {
        package_json,
        node_modules,
        package_lock,
        composer_json,
        vendor_dir,
        composer_lock,
        npm_scripts,
        wordpress_signal,
    }
}

fn read_npm_scripts(package_json_path: &Path) -> Vec<String> {
    if !package_json_path.is_file() {
        return Vec::new();
    }
    let content = match std::fs::read_to_string(package_json_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    let parsed: Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let scripts = match parsed.get("scripts").and_then(Value::as_object) {
        Some(scripts) => scripts,
        None => return Vec::new(),
    };
    let mut names: Vec<String> = scripts.keys().cloned().collect();
    names.sort();
    names
}

fn probe_command(program: &str, args: &[&str]) -> CommandProbe {
    let result = process_runner::run_command(program, args, Some(TOOL_PROBE_TIMEOUT));
    let Ok(result) = result else {
        return CommandProbe {
            available: false,
            observed_line: None,
            observed_major: None,
            noisy: false,
            noisy_excerpt: None,
        };
    };

    let combined = format!("{}\n{}", result.stdout, result.stderr);
    let observed_line = extract_version_line(&result.stdout)
        .or_else(|| extract_version_line(&result.stderr))
        .or_else(|| non_empty_line(&result.stdout))
        .or_else(|| non_empty_line(&result.stderr));
    let observed_major = observed_line
        .as_deref()
        .and_then(parse_major_version)
        .or_else(|| parse_major_version(&combined));
    let noisy_excerpt = detect_noisy_output(&combined);

    CommandProbe {
        available: result.success || result.exit_code.is_some(),
        observed_line,
        observed_major,
        noisy: noisy_excerpt.is_some(),
        noisy_excerpt,
    }
}

fn extract_version_line(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.contains("version") || trimmed.starts_with('v') {
            Some(trimmed.to_string())
        } else {
            None
        }
    })
}

fn non_empty_line(output: &str) -> Option<String> {
    output.lines().map(str::trim).find(|line| !line.is_empty()).map(ToString::to_string)
}

fn parse_major_version(text: &str) -> Option<i64> {
    let mut current_digits = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            current_digits.push(ch);
            continue;
        }
        if !current_digits.is_empty() {
            break;
        }
    }
    if current_digits.is_empty() {
        None
    } else {
        current_digits.parse::<i64>().ok()
    }
}

fn detect_noisy_output(output: &str) -> Option<String> {
    let mut patterns = BTreeSet::new();
    patterns.insert("deprecated");
    patterns.insert("warning");
    patterns.insert("fatal");
    patterns.insert("error:");
    patterns.insert("notice:");
    patterns.insert("deprecation");

    output.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        let lowered = trimmed.to_ascii_lowercase();
        for pattern in &patterns {
            if lowered.contains(pattern) {
                return Some(trimmed.chars().take(180).collect());
            }
        }
        None
    })
}

fn generate_recommended_actions(checks: &[CheckResult]) -> Vec<String> {
    let mut actions = Vec::new();
    let mut seen = HashSet::new();

    let mut add_action = |action: String| {
        if seen.insert(action.clone()) {
            actions.push(action);
        }
    };

    for check in checks {
        if !matches!(check.status, CheckStatus::Warn | CheckStatus::Fail) {
            continue;
        }

        match (check.id.as_str(), check.reason.as_deref()) {
            ("project.node_modules", Some("dependency_missing")) => {
                add_action("Install JavaScript dependencies with `npm install`.".to_string());
            }
            ("project.vendor", Some("dependency_missing")) => {
                add_action("Install PHP dependencies with `composer install`.".to_string());
            }
            ("env.node", Some("missing")) => {
                add_action("Install Node.js (recommended major version 18 or later).".to_string());
            }
            ("env.node", Some("version_too_old")) => {
                add_action("Upgrade Node.js to major version 18 or later.".to_string());
            }
            (_, Some("version_unparseable")) => {
                add_action(
                    "Investigate tool version output and verify local installation health."
                        .to_string(),
                );
            }
            ("repo.working_tree", Some("dirty_worktree")) => {
                add_action(
                    "Review/stage/commit changes before handoff to CI or automation.".to_string(),
                );
            }
            ("project.wp_cli", Some("command_noisy")) => {
                add_action(
                    "Update WP-CLI and verify PHP compatibility to remove noisy output."
                        .to_string(),
                );
            }
            _ => {}
        }

        if let Some(fix) = &check.suggested_fix {
            add_action(fix.clone());
        }
    }

    actions
}
