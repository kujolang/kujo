// File: src/workflow_pack/mod.rs
//
// Kujo Workflow Pack System
//
// A modular system for discovering, validating, and executing project-specific
// command namespaces (workflow packs). Workflow packs let teams and third parties
// add custom CLI commands without modifying Kujo core.
//
// Architecture:
//   manifest       - YAML manifest parsing and validation (kujo-pack.yaml)
//   discovery      - Multi-source pack discovery (builtin, project, user, env)
//   registry       - Namespace/command registration and routing
//   types          - Check results, status enums, command context
//   renderer       - Human-readable and JSON output rendering
//   process_runner - Safe external process execution
//   builtins       - Built-in workflow pack infrastructure (currently empty)

pub mod builtins;
pub mod discovery;
pub mod manifest;
pub mod process_runner;
pub mod registry;
pub mod renderer;
pub mod types;

use crate::workflow_pack::discovery::discover_all_packs;
use crate::workflow_pack::discovery::PackSource;
use crate::workflow_pack::registry::WorkflowRegistry;
use crate::workflow_pack::types::CommandContext;
use std::path::{Path, PathBuf};

/// Initialize the workflow pack system: discover packs from all sources,
/// register them into the registry, and return a ready-to-use registry
/// along with any non-fatal discovery warnings.
pub fn initialize_registry(current_dir: &Path) -> (WorkflowRegistry, Vec<String>) {
    let mut registry = WorkflowRegistry::new();
    let mut warnings: Vec<String> = Vec::new();

    // 1. Register built-in packs first (highest priority).
    if let Err(e) = builtins::register_all(&mut registry) {
        warnings.push(format!("Built-in pack registration error: {}", e));
    }

    // 2. Discover external packs from all sources.
    let builtin_list = builtins::builtin_packs_list();
    let (discovered_packs, discovery_errors) = discover_all_packs(builtin_list, current_dir);

    for err in &discovery_errors {
        warnings.push(format!("Pack discovery: {}", err));
    }

    // 3. Register each discovered external pack into the registry.
    for pack in &discovered_packs {
        if pack.source == PackSource::Builtin {
            continue;
        }
        match registry.register_discovered_pack(pack) {
            Ok(()) => {}
            Err(e) => warnings.push(format!(
                "Failed to register pack '{}' (namespace '{}'): {}",
                pack.manifest.id, pack.manifest.namespace, e
            )),
        }
    }

    (registry, warnings)
}

/// Handle a workflow command from CLI external subcommand arguments.
///
/// Takes the external subcommand args (e.g., `["acme", "doctor", "--json"]`)
/// and routes to the appropriate workflow command.
pub fn handle_workflow_command(args: &[String], current_dir: &Path) -> Result<(), String> {
    if args.is_empty() {
        return Err(
            "No workflow namespace specified. Use 'kujo <namespace> <command>'.".to_string()
        );
    }

    let namespace = &args[0];

    // Parse remaining args for command name and flags.
    let mut command_parts: Vec<String> = Vec::new();
    let mut json_output = false;
    let mut save_output = false;
    let mut cmd_args: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json_output = true,
            "--save" => save_output = true,
            "--help" | "-h" => {
                return Err(format!(
					"Usage: kujo {} <command> [--json] [--save]\n\nSee 'kujo workflow list' for available commands.",
					namespace
				));
            }
            arg if !arg.starts_with('-') && command_parts.is_empty() => {
                command_parts.push(arg.to_string());
            }
            arg => {
                cmd_args.push(arg.to_string());
            }
        }
        i += 1;
    }

    let command_name = if command_parts.is_empty() {
        return Err(format!(
            "No command specified for namespace '{}'. Usage: kujo {} <command> [--json]",
            namespace, namespace
        ));
    } else {
        command_parts.join(" ")
    };

    // Initialize registry with all discovered packs.
    let (registry, warnings) = initialize_registry(current_dir);

    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }

    // Build command context.
    let ctx = CommandContext {
        cwd: current_dir.to_path_buf(),
        json_output,
        args: cmd_args,
        env_vars: std::collections::HashMap::new(),
    };

    // Resolve the command (needed for pack_dir when saving).
    let pack_dir = registry.resolve(namespace, &command_name).map(|cmd| cmd.pack_dir.clone());

    // Execute.
    match registry.execute(namespace, &command_name, &ctx) {
        Ok(mut report) => {
            // Inject cwd from the Rust side — the script doesn't need to shell out for it.
            report.cwd = Some(current_dir.to_string_lossy().to_string());

            if json_output {
                renderer::render_json(&report);
            } else {
                renderer::render_human(&report);
            }

            // Save to files if --save flag was set.
            if save_output {
                if let Some(ref dir) = pack_dir {
                    let project_name =
                        current_dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                    match save_report_to_pack(&report, dir, project_name) {
                        Ok(saved_path) => {
                            eprintln!("Report saved to: {}", saved_path.display());
                        }
                        Err(e) => {
                            eprintln!("Warning: failed to save report: {}", e);
                        }
                    }
                } else {
                    eprintln!("Warning: --save used but pack directory is unknown.");
                }
            }

            if report.status == "fail" {
                std::process::exit(1);
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Save a DoctorReport to a timestamped directory inside the pack's .reports folder.
///
/// Creates: `<pack_dir>/.reports/<project>-<YYYY-MM-DD>-<HH-MM-SS>/report.json`
/// and:     `<pack_dir>/.reports/<project>-<YYYY-MM-DD>-<HH-MM-SS>/report.md`
///
/// Returns the path to the report directory.
fn save_report_to_pack(
    report: &crate::workflow_pack::types::DoctorReport,
    pack_dir: &Path,
    project_name: &str,
) -> Result<PathBuf, String> {
    let now = chrono::Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H%M%S").to_string();
    let folder_name = format!("{}-{}-{}", project_name, date_str, time_str);

    let reports_root = pack_dir.join(".reports");
    let run_dir = reports_root.join(&folder_name);

    std::fs::create_dir_all(&run_dir)
        .map_err(|e| format!("Failed to create report directory '{}': {}", run_dir.display(), e))?;

    // Write JSON.
    let json_path = run_dir.join("report.json");
    let json_content = serde_json::to_string_pretty(report)
        .map_err(|e| format!("Failed to serialize report to JSON: {}", e))?;
    std::fs::write(&json_path, json_content)
        .map_err(|e| format!("Failed to write '{}': {}", json_path.display(), e))?;

    // Write Markdown.
    let md_path = run_dir.join("report.md");
    let md_content = renderer::render_markdown(report);
    std::fs::write(&md_path, md_content)
        .map_err(|e| format!("Failed to write '{}': {}", md_path.display(), e))?;

    Ok(run_dir)
}
