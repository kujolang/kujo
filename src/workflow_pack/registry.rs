// File: src/workflow_pack/registry.rs
//
// Workflow pack registry: namespace resolution, command routing, and collision detection.
#![allow(dead_code)]

use crate::reserved_names;
use crate::workflow_pack::discovery::{DiscoveredPack, PackSource};
use crate::workflow_pack::process_runner;
use crate::workflow_pack::types::{
    CheckResult, CheckSeverity, CheckStatus, CommandContext, DoctorReport,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

/// A function that implements a built-in workflow command.
pub type CommandHandler = fn(&CommandContext) -> DoctorReport;

/// A registered workflow command entry.
#[derive(Clone)]
pub struct RegisteredCommand {
    pub namespace: String,
    pub command_name: String,
    pub pack_id: String,
    pub pack_source: PackSource,
    pub summary: String,
    pub is_builtin: bool,
    /// Path to the pack directory (for resolving external command entries).
    pub pack_dir: PathBuf,
    /// The entry point from the manifest (e.g., "builtin" or "commands/doctor.kujo").
    pub entry: String,
    /// Handler function for built-in commands (None for external).
    pub handler: Option<CommandHandler>,
}

/// The central workflow pack registry.
pub struct WorkflowRegistry {
    /// Commands keyed by (namespace, command_name) tuple.
    commands: BTreeMap<(String, String), RegisteredCommand>,
    /// Known namespaces for help/listing.
    namespaces: BTreeMap<String, NamespaceInfo>,
    /// Doctor profile contributions keyed by profile name.
    doctor_profiles: BTreeMap<String, RegisteredDoctorProfile>,
}

#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    pub pack_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub source: PackSource,
}

#[derive(Debug, Clone)]
pub struct RegisteredDoctorProfile {
    pub profile_name: String,
    pub pack_id: String,
    pub pack_source: PackSource,
    pub entry: String,
    pub summary: String,
    pub pack_dir: PathBuf,
}

/// External command execution timeout.
const EXTERNAL_CMD_TIMEOUT: Duration = Duration::from_secs(30);

impl WorkflowRegistry {
    pub fn new() -> Self {
        Self {
            commands: BTreeMap::new(),
            namespaces: BTreeMap::new(),
            doctor_profiles: BTreeMap::new(),
        }
    }

    /// Register a built-in pack with its command handlers.
    pub fn register_builtin_pack(
        &mut self,
        pack: &DiscoveredPack,
        handlers: BTreeMap<String, CommandHandler>,
    ) -> Result<(), String> {
        self.register_pack(pack, Some(handlers))
    }

    /// Register a discovered (external) pack.
    pub fn register_discovered_pack(&mut self, pack: &DiscoveredPack) -> Result<(), String> {
        let namespace = pack.manifest.namespace.clone();
        let pack_id = pack.manifest.id.clone();

        if pack.source != PackSource::Builtin
            && reserved_names::is_trusted_first_party_pack_id(&pack_id)
        {
            return Err(format!(
				"Pack '{}' uses a first-party reserved pack id. External packs cannot self-identify as first-party.",
				pack_id
			));
        }

        if pack.source != PackSource::Builtin {
            if let Some(reservation) = reserved_names::reservation_for_namespace(&namespace) {
                return Err(reserved_names::external_reserved_namespace_error(
                    &namespace,
                    reservation,
                ));
            }
        }

        if let Some(existing) = self.namespaces.get(&namespace) {
            if existing.source == PackSource::Builtin {
                return Err(format!(
					"Pack '{}' claims namespace '{}' which is already reserved by built-in pack '{}'. \
					 External packs cannot override built-in namespaces.",
					pack_id, namespace, existing.pack_id
				));
            }
        }

        self.register_pack(pack, None)
    }

    fn register_pack(
        &mut self,
        pack: &DiscoveredPack,
        handlers: Option<BTreeMap<String, CommandHandler>>,
    ) -> Result<(), String> {
        let manifest = &pack.manifest;
        let namespace = &manifest.namespace;
        let pack_id = &manifest.id;
        let is_builtin = handlers.is_some();
        let source = if is_builtin { PackSource::Builtin } else { pack.source.clone() };

        // Stage entries so any rejected command or profile leaves the registry intact.
        let mut commands = BTreeMap::new();
        for definition in &manifest.commands {
            let handler = handlers
                .as_ref()
                .map(|handlers| {
                    handlers.get(&definition.name).copied().ok_or_else(|| {
                        format!(
                            "Built-in pack '{}' declares command '{}' but no handler was provided.",
                            pack_id, definition.name
                        )
                    })
                })
                .transpose()?;
            let key = (namespace.clone(), definition.name.clone());
            if let Some(existing) = self.commands.get(&key).or_else(|| commands.get(&key)) {
                if existing.is_builtin && !is_builtin {
                    return Err(format!(
                        "Command '{}.{}' is already registered by built-in pack '{}'. External packs cannot override built-in commands.",
                        namespace, definition.name, existing.pack_id
                    ));
                }
                return Err(format!(
                    "Command '{}.{}' is already registered by pack '{}'.",
                    namespace, definition.name, existing.pack_id
                ));
            }
            commands.insert(
                key,
                RegisteredCommand {
                    namespace: namespace.clone(),
                    command_name: definition.name.clone(),
                    pack_id: pack_id.clone(),
                    pack_source: source.clone(),
                    summary: definition.summary.clone(),
                    is_builtin,
                    pack_dir: pack.pack_dir.clone(),
                    entry: definition.entry.clone(),
                    handler,
                },
            );
        }

        let mut profiles = BTreeMap::new();
        for profile in &manifest.contributes.doctor_profiles {
            if let Some(existing) =
                self.doctor_profiles.get(&profile.name).or_else(|| profiles.get(&profile.name))
            {
                if existing.pack_source == PackSource::Builtin && source != PackSource::Builtin {
                    return Err(format!(
                        "Doctor profile '{}' is already registered by built-in pack '{}'. External packs cannot override built-in profiles.",
                        profile.name, existing.pack_id
                    ));
                }
                return Err(format!(
                    "Doctor profile '{}' is already registered by pack '{}'.",
                    profile.name, existing.pack_id
                ));
            }
            profiles.insert(
                profile.name.clone(),
                RegisteredDoctorProfile {
                    profile_name: profile.name.clone(),
                    pack_id: pack_id.clone(),
                    pack_source: source.clone(),
                    entry: profile.entry.clone(),
                    summary: profile.summary.clone(),
                    pack_dir: pack.pack_dir.clone(),
                },
            );
        }

        self.namespaces.insert(
            namespace.clone(),
            NamespaceInfo {
                pack_id: pack_id.clone(),
                name: manifest.name.clone(),
                description: manifest.description.clone(),
                version: manifest.version.clone(),
                source,
            },
        );
        self.commands.extend(commands);
        self.doctor_profiles.extend(profiles);
        Ok(())
    }

    pub fn resolve(&self, namespace: &str, command_name: &str) -> Option<&RegisteredCommand> {
        self.commands.get(&(namespace.to_string(), command_name.to_string()))
    }

    pub fn list_namespaces(&self) -> Vec<&NamespaceInfo> {
        self.namespaces.values().collect()
    }

    pub fn get_namespace(&self, namespace: &str) -> Option<&NamespaceInfo> {
        self.namespaces.get(namespace)
    }

    pub fn resolve_doctor_profile(&self, profile_name: &str) -> Option<&RegisteredDoctorProfile> {
        self.doctor_profiles.get(profile_name)
    }

    pub fn list_doctor_profiles(&self) -> Vec<&RegisteredDoctorProfile> {
        self.doctor_profiles.values().collect()
    }

    pub fn execute_doctor_profile(
        &self,
        profile_name: &str,
        ctx: &CommandContext,
    ) -> Result<DoctorReport, String> {
        let profile = self.resolve_doctor_profile(profile_name).ok_or_else(|| {
            format!(
                "Unknown doctor profile '{}'. Use `kujo doctor --list-profiles` to inspect available profiles.",
                profile_name
            )
        })?;

        if profile.entry == "builtin" {
            return Err(format!(
                "Doctor profile '{}' is builtin-managed and cannot be executed as an external profile entry.",
                profile_name
            ));
        }

        let synthetic = RegisteredCommand {
            namespace: "doctor".to_string(),
            command_name: profile_name.to_string(),
            pack_id: profile.pack_id.clone(),
            pack_source: profile.pack_source.clone(),
            summary: if profile.summary.trim().is_empty() {
                format!("Doctor profile '{}'", profile_name)
            } else {
                profile.summary.clone()
            },
            is_builtin: profile.pack_source == PackSource::Builtin,
            pack_dir: profile.pack_dir.clone(),
            entry: profile.entry.clone(),
            handler: None,
        };

        self.execute_external(&synthetic, ctx)
    }

    pub fn list_commands(&self, namespace: &str) -> Vec<&RegisteredCommand> {
        self.commands.iter().filter(|((ns, _), _)| ns == namespace).map(|(_, cmd)| cmd).collect()
    }

    /// Execute a command by namespace and command name.
    pub fn execute(
        &self,
        namespace: &str,
        command_name: &str,
        ctx: &CommandContext,
    ) -> Result<DoctorReport, String> {
        let cmd = match self.resolve(namespace, command_name) {
            Some(cmd) => cmd,
            None => {
                if self.get_namespace(namespace).is_some() {
                    let available: Vec<String> = self
                        .list_commands(namespace)
                        .iter()
                        .map(|c| c.command_name.clone())
                        .collect();
                    return Err(format!(
                        "Unknown command '{}' in namespace '{}'. Available commands: {}",
                        command_name,
                        namespace,
                        if available.is_empty() {
                            "(none)".to_string()
                        } else {
                            available.join(", ")
                        }
                    ));
                } else {
                    return Err(format!(
						"Unknown workflow namespace '{}'. Use 'kujo workflow list' to see available namespaces.",
						namespace
					));
                }
            }
        };

        // Built-in: call Rust handler directly.
        if let Some(handler) = cmd.handler {
            return Ok(handler(ctx));
        }

        // External: execute via process runner.
        self.execute_external(cmd, ctx)
    }

    /// Execute an external pack command by running its entry script.
    fn execute_external(
        &self,
        cmd: &RegisteredCommand,
        ctx: &CommandContext,
    ) -> Result<DoctorReport, String> {
        if cmd.entry == "builtin" {
            return Err(format!(
                "Command '{}.{}' is marked as builtin but has no handler.",
                cmd.namespace, cmd.command_name
            ));
        }

        let entry_path = cmd.pack_dir.join(&cmd.entry);

        let result = if entry_path.extension().map(|e| e == "kujo").unwrap_or(false) {
            let kujo_binary = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "kujo".to_string());

            let entry = entry_path.to_string_lossy().to_string();
            let mut owned_args =
                vec!["run".to_string(), "--allow-all".to_string(), entry, "--".to_string()];
            owned_args.extend(ctx.args.iter().cloned());
            let args: Vec<&str> = owned_args.iter().map(String::as_str).collect();
            process_runner::run_command_with_limits(
                &kujo_binary,
                &args,
                Some(EXTERNAL_CMD_TIMEOUT),
                process_runner::MAX_STRUCTURED_OUTPUT_BYTES,
                process_runner::MAX_STRUCTURED_OUTPUT_LINES,
            )
        } else {
            let args: Vec<&str> = ctx.args.iter().map(String::as_str).collect();
            process_runner::run_command_with_limits(
                &entry_path.to_string_lossy(),
                &args,
                Some(EXTERNAL_CMD_TIMEOUT),
                process_runner::MAX_STRUCTURED_OUTPUT_BYTES,
                process_runner::MAX_STRUCTURED_OUTPUT_LINES,
            )
        };

        match result {
            Ok(proc_result) => {
                let stdout = proc_result.stdout.trim().to_string();
                if stdout.is_empty() && !proc_result.stderr.trim().is_empty() {
                    return Err(format!(
                        "Command '{}.{}' produced no stdout output. Stderr: {}",
                        cmd.namespace,
                        cmd.command_name,
                        proc_result.stderr.trim()
                    ));
                }

                if proc_result.stdout_truncated {
                    return Err(format!(
                        "Command '{}.{}' exceeded the structured output limit ({} bytes or {} lines).",
                        cmd.namespace,
                        cmd.command_name,
                        process_runner::MAX_STRUCTURED_OUTPUT_BYTES,
                        process_runner::MAX_STRUCTURED_OUTPUT_LINES
                    ));
                }

                match serde_json::from_str::<DoctorReport>(&stdout) {
                    Ok(report) => Ok(report),
                    Err(_) => {
                        let (status, severity) = if proc_result.success {
                            (CheckStatus::Info, CheckSeverity::Info)
                        } else {
                            (CheckStatus::Fail, CheckSeverity::High)
                        };

                        let check = CheckResult {
                            id: format!("{}.{}", cmd.namespace, cmd.command_name),
                            label: cmd.summary.clone(),
                            status,
                            severity,
                            observed: Some(truncate_str(&stdout, 500)),
                            expected: None,
                            message: if proc_result.stderr.trim().is_empty() {
                                None
                            } else {
                                Some(proc_result.stderr.trim().to_string())
                            },
                            suggested_fix: None,
                            reason: None,
                            category: None,
                            observed_major: None,
                            minimum_major: None,
                        };

                        Ok(DoctorReport::new(
                            &cmd.pack_id,
                            &cmd.namespace,
                            &cmd.command_name,
                            vec![check],
                        ))
                    }
                }
            }
            Err(e) => Err(format!(
                "Failed to execute command '{}.{}': {}",
                cmd.namespace, cmd.command_name, e
            )),
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut boundary = max_len;
    while !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}...", &s[..boundary])
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_pack::manifest::{
        CommandDef, Contributions, DoctorProfileDef, PackManifest,
    };

    fn make_pack(id: &str, ns: &str, cmds: Vec<CommandDef>, source: PackSource) -> DiscoveredPack {
        DiscoveredPack {
            manifest: PackManifest {
                id: id.to_string(),
                namespace: ns.to_string(),
                name: format!("Test {}", id),
                version: "0.1.0".to_string(),
                description: String::new(),
                commands: cmds,
                contributes: Contributions::default(),
            },
            pack_dir: PathBuf::from("/test"),
            source,
        }
    }

    fn dummy_handler(_ctx: &CommandContext) -> DoctorReport {
        DoctorReport {
            tool: None,
            pack: "test".to_string(),
            namespace: "test".to_string(),
            command: "test".to_string(),
            profile: None,
            schema_version: None,
            cwd: None,
            status: "pass".to_string(),
            summary: Default::default(),
            checks: vec![],
            recommended_next_actions: None,
            readiness: None,
            production_send_enabled: None,
            profile_data: None,
        }
    }

    fn test_ctx() -> CommandContext {
        CommandContext {
            cwd: PathBuf::from("."),
            json_output: false,
            args: vec![],
            env_vars: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn register_and_resolve_builtin() {
        let mut r = WorkflowRegistry::new();
        let pack = make_pack(
            "p",
            "ns",
            vec![CommandDef {
                name: "doctor".to_string(),
                summary: "s".to_string(),
                entry: "builtin".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: true,
                requires_network: false,
            }],
            PackSource::Builtin,
        );
        let mut h = BTreeMap::new();
        h.insert("doctor".to_string(), dummy_handler as CommandHandler);
        r.register_builtin_pack(&pack, h).unwrap();
        let cmd = r.resolve("ns", "doctor").unwrap();
        assert!(cmd.is_builtin);
    }

    #[test]
    fn unknown_namespace_errors() {
        let r = WorkflowRegistry::new();
        assert!(r.execute("nope", "doctor", &test_ctx()).is_err());
    }

    #[test]
    fn unknown_command_in_known_ns_errors() {
        let mut r = WorkflowRegistry::new();
        let pack = make_pack(
            "p",
            "ns",
            vec![CommandDef {
                name: "doctor".to_string(),
                summary: "s".to_string(),
                entry: "builtin".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: true,
                requires_network: false,
            }],
            PackSource::Builtin,
        );
        let mut h = BTreeMap::new();
        h.insert("doctor".to_string(), dummy_handler as CommandHandler);
        r.register_builtin_pack(&pack, h).unwrap();
        assert!(r.execute("ns", "nope", &test_ctx()).is_err());
    }

    #[test]
    fn builtin_collision_rejected() {
        let mut r = WorkflowRegistry::new();
        let builtin = make_pack(
            "b",
            "ns",
            vec![CommandDef {
                name: "cmd".to_string(),
                summary: "s".to_string(),
                entry: "builtin".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: false,
                requires_network: false,
            }],
            PackSource::Builtin,
        );
        let mut h = BTreeMap::new();
        h.insert("cmd".to_string(), dummy_handler as CommandHandler);
        r.register_builtin_pack(&builtin, h).unwrap();

        let ext = make_pack(
            "e",
            "ns",
            vec![CommandDef {
                name: "other".to_string(),
                summary: "s".to_string(),
                entry: "cmd.kujo".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: false,
                requires_network: false,
            }],
            PackSource::ProjectLocal,
        );
        assert!(r.register_discovered_pack(&ext).is_err());
    }

    #[test]
    fn external_pack_registers_without_handler() {
        let mut r = WorkflowRegistry::new();
        let ext = make_pack(
            "e",
            "ext",
            vec![CommandDef {
                name: "doctor".to_string(),
                summary: "s".to_string(),
                entry: "commands/doctor.kujo".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: true,
                requires_network: false,
            }],
            PackSource::ProjectLocal,
        );
        r.register_discovered_pack(&ext).unwrap();
        let cmd = r.resolve("ext", "doctor").unwrap();
        assert!(!cmd.is_builtin);
        assert!(cmd.handler.is_none());
        assert_eq!(cmd.entry, "commands/doctor.kujo");
    }

    #[test]
    fn external_pack_rejects_reserved_namespace() {
        let mut r = WorkflowRegistry::new();
        let ext = make_pack(
            "acme",
            "kujo",
            vec![CommandDef {
                name: "status".to_string(),
                summary: "s".to_string(),
                entry: "commands/status.kujo".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: true,
                requires_network: false,
            }],
            PackSource::ProjectLocal,
        );
        let err = r.register_discovered_pack(&ext).expect_err("reserved namespaces should fail");
        assert!(err.contains("reserved"));
    }

    #[test]
    fn external_pack_cannot_spoof_first_party_pack_id() {
        let mut r = WorkflowRegistry::new();
        let ext = make_pack(
            "kujo-doctor",
            "acme",
            vec![CommandDef {
                name: "status".to_string(),
                summary: "s".to_string(),
                entry: "commands/status.kujo".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: true,
                requires_network: false,
            }],
            PackSource::ProjectLocal,
        );
        let err =
            r.register_discovered_pack(&ext).expect_err("first-party spoofing should be rejected");
        assert!(err.contains("first-party reserved pack id"));
    }

    #[test]
    fn external_pack_registers_doctor_profile_contribution() {
        let mut r = WorkflowRegistry::new();
        let mut ext = make_pack(
            "acme",
            "acme",
            vec![CommandDef {
                name: "status".to_string(),
                summary: "s".to_string(),
                entry: "commands/status.kujo".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: true,
                requires_network: false,
            }],
            PackSource::ProjectLocal,
        );
        ext.manifest.contributes.doctor_profiles.push(DoctorProfileDef {
            name: "wordpress".to_string(),
            entry: "commands/doctor-wordpress.kujo".to_string(),
            summary: "WordPress doctor".to_string(),
        });

        r.register_discovered_pack(&ext).expect("doctor profile contribution should register");
        let profile =
            r.resolve_doctor_profile("wordpress").expect("doctor profile should be discoverable");
        assert_eq!(profile.pack_id, "acme");
        assert_eq!(profile.entry, "commands/doctor-wordpress.kujo");
    }

    #[test]
    fn conflicting_profile_rejects_the_whole_pack() {
        let mut registry = WorkflowRegistry::new();
        let mut first = make_pack("first", "team", vec![], PackSource::ProjectLocal);
        first.manifest.contributes.doctor_profiles.push(DoctorProfileDef {
            name: "local-check".to_string(),
            entry: "check.kujo".to_string(),
            summary: "Local check".to_string(),
        });
        registry.register_discovered_pack(&first).unwrap();

        let mut rejected = first.clone();
        rejected.manifest.id = "second".to_string();
        rejected.manifest.commands.push(CommandDef {
            name: "status".to_string(),
            summary: "Status".to_string(),
            entry: "status.kujo".to_string(),
            safe: true,
            writes_files: false,
            runs_processes: false,
            requires_network: false,
        });
        assert!(registry.register_discovered_pack(&rejected).is_err());
        assert_eq!(registry.get_namespace("team").unwrap().pack_id, "first");
        assert!(registry.resolve("team", "status").is_none());
        assert_eq!(registry.resolve_doctor_profile("local-check").unwrap().pack_id, "first");
    }

    #[test]
    fn fallback_preview_does_not_split_non_ascii_output() {
        let output = format!("{}é tail", "a".repeat(499));
        assert_eq!(truncate_str(&output, 500), format!("{}...", "a".repeat(499)));
    }

    #[test]
    fn builtin_pack_can_register_reserved_doctor_profile() {
        let mut r = WorkflowRegistry::new();
        let mut builtin = make_pack(
            "kujo-doctor",
            "doctor",
            vec![CommandDef {
                name: "status".to_string(),
                summary: "s".to_string(),
                entry: "builtin".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: false,
                requires_network: false,
            }],
            PackSource::Builtin,
        );
        builtin.manifest.contributes.doctor_profiles.push(DoctorProfileDef {
            name: "default".to_string(),
            entry: "commands/default.kujo".to_string(),
            summary: "Default profile".to_string(),
        });
        let mut handlers = BTreeMap::new();
        handlers.insert("status".to_string(), dummy_handler as CommandHandler);

        r.register_builtin_pack(&builtin, handlers)
            .expect("builtin packs should register reserved doctor profile names");
        assert!(r.resolve_doctor_profile("default").is_some());
    }

    #[test]
    fn execute_doctor_profile_errors_for_unknown_profile() {
        let r = WorkflowRegistry::new();
        let err = r
            .execute_doctor_profile("missing", &test_ctx())
            .expect_err("unknown profiles should return clear errors");
        assert!(err.contains("Unknown doctor profile"));
    }

    #[test]
    fn execute_doctor_profile_rejects_builtin_entry() {
        let mut r = WorkflowRegistry::new();
        let mut builtin = make_pack(
            "kujo-doctor",
            "doctor",
            vec![CommandDef {
                name: "status".to_string(),
                summary: "s".to_string(),
                entry: "builtin".to_string(),
                safe: true,
                writes_files: false,
                runs_processes: false,
                requires_network: false,
            }],
            PackSource::Builtin,
        );
        builtin.manifest.contributes.doctor_profiles.push(DoctorProfileDef {
            name: "generic".to_string(),
            entry: "builtin".to_string(),
            summary: "Generic profile".to_string(),
        });
        let mut handlers = BTreeMap::new();
        handlers.insert("status".to_string(), dummy_handler as CommandHandler);
        r.register_builtin_pack(&builtin, handlers).expect("builtin profile should register");

        let err = r
            .execute_doctor_profile("generic", &test_ctx())
            .expect_err("builtin profile entries should not execute as external scripts");
        assert!(err.contains("builtin-managed"));
    }
}
