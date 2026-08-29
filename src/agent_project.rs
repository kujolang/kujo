use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

mod credentials;
mod doctor;
mod scaffold;

pub use doctor::doctor_report;
use scaffold::{scaffold, validate_name};

use crate::workflow_pack::types::{CheckResult, CheckSeverity, CheckStatus, DoctorReport};

const CONTRACT: &str = "kujo-agent-project/v1";
const INSPECT_CONTRACT: &str = "kujo-agent-inspect/v1";
const RUN_CONTRACT: &str = "kujo-agent-run/v1";
const EVAL_CONTRACT: &str = "kujo-agent-eval/v1";
const ERROR_CONTRACT: &str = "kujo-agent-error/v1";
const PROFILES: &[&str] =
    &["basic", "tools", "knowledge", "workflow", "hardened", "observable", "full"];

#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// Scaffold a deterministic Agent Project
    New(NewArgs),
    /// Run the project's Agents SDK entrypoint
    Run(RunArgs),
    /// Inspect all repository-owned agent boundaries
    Inspect(ProjectArgs),
    /// Run the project's deterministic evaluation
    Eval(ProjectArgs),
    /// Securely configure reusable provider and connector credentials
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Store a provider or connector credential without exposing it in shell history
    Set(AuthSetArgs),
    /// Report whether a credential is available without revealing it
    Status(AuthProjectArgs),
    /// Remove a stored provider or connector credential
    Remove(AuthProjectArgs),
}

#[derive(Args, Debug)]
pub struct AuthSetArgs {
    /// Built-in provider name (openai, openrouter, deepseek, or custom)
    #[arg(conflicts_with = "name")]
    pub provider: Option<String>,
    /// Explicit environment-variable contract for an API-key connector
    #[arg(long, conflicts_with = "provider")]
    pub name: Option<String>,
    /// Read the credential from stdin for non-interactive automation
    #[arg(long, conflicts_with = "from_env")]
    pub from_stdin: bool,
    /// Import the provider or connector's declared environment variable
    #[arg(long, conflicts_with = "from_stdin")]
    pub from_env: bool,
    /// Store the credential in this project's ignored .env.local instead of the OS credential store
    #[arg(long)]
    pub project: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct AuthProjectArgs {
    #[arg(conflicts_with = "name")]
    pub provider: Option<String>,
    #[arg(long, conflicts_with = "provider")]
    pub name: Option<String>,
    #[arg(long)]
    pub project: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct NewArgs {
    pub name: String,
    #[arg(long)]
    pub dir: Option<PathBuf>,
    #[arg(long, default_value = "basic")]
    pub profile: String,
    /// Provider id, or auto to reuse an already configured provider
    #[arg(long, default_value = "auto")]
    pub provider: String,
    /// Model id, or auto to use the provider's starter model
    #[arg(long, default_value = "auto")]
    pub model: String,
    #[arg(long)]
    pub no_git: bool,
    #[arg(long)]
    pub install: bool,
    /// Read a missing live-provider credential from stdin and save it in the OS credential store
    #[arg(long)]
    pub credential_stdin: bool,
    /// Scaffold a live-provider project without requiring a configured credential
    #[arg(long, conflicts_with = "credential_stdin")]
    pub no_credential: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    pub prompt: Option<String>,
    #[arg(long)]
    pub file: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
    /// Execute through the project's Workcell boundary
    #[arg(long)]
    pub workcell: bool,
}

#[derive(Args, Debug)]
pub struct ProjectArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy)]
struct ProviderSettings {
    mode: &'static str,
    base_url: &'static str,
    api_key_env: &'static str,
    env_example: &'static str,
}

fn provider_settings(provider: &str) -> Option<ProviderSettings> {
    match provider {
        "fixture" => Some(ProviderSettings {
            mode: "fixture",
            base_url: "",
            api_key_env: "",
            env_example: "# Fixture mode requires no credentials.\n",
        }),
        "openai" => Some(ProviderSettings {
            mode: "live",
            base_url: "https://api.openai.com/v1",
            api_key_env: "OPENAI_API_KEY",
            env_example: "OPENAI_API_KEY=\n",
        }),
        "openrouter" => Some(ProviderSettings {
            mode: "live",
            base_url: "https://openrouter.ai/api/v1",
            api_key_env: "OPENROUTER_API_KEY",
            env_example: "OPENROUTER_API_KEY=\n",
        }),
        "deepseek" => Some(ProviderSettings {
            mode: "live",
            base_url: "https://api.deepseek.com/v1",
            api_key_env: "DEEPSEEK_API_KEY",
            env_example: "DEEPSEEK_API_KEY=\n",
        }),
        "custom" => Some(ProviderSettings {
            mode: "live",
            base_url: "https://api.example.invalid/v1",
            api_key_env: "CUSTOM_API_KEY",
            env_example: "CUSTOM_API_KEY=\n",
        }),
        _ => None,
    }
}

pub struct AgentError {
    pub message: String,
    pub exit_code: i32,
}
fn usage(message: impl Into<String>) -> AgentError {
    AgentError { message: message.into(), exit_code: 2 }
}
fn ioerr(message: impl Into<String>) -> AgentError {
    AgentError { message: message.into(), exit_code: 5 }
}
fn fail(message: impl Into<String>) -> AgentError {
    AgentError { message: message.into(), exit_code: 1 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectManifest {
    contract: String,
    #[serde(default = "default_project_schema")]
    schema: String,
    name: String,
    profile: String,
    agent: AgentPaths,
    runtime: RuntimeConfig,
    integrations: BTreeMap<String, bool>,
    #[serde(default)]
    integration_configs: BTreeMap<String, String>,
    #[serde(default)]
    external_tools: BTreeMap<String, ExternalTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExternalTool {
    command: String,
    source: String,
    commit: String,
    required_for: Vec<String>,
}

fn default_project_schema() -> String {
    "schemas/agent-project.schema.json".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentPaths {
    definition: String,
    manifest: String,
    instructions: String,
    skills: String,
    tools: String,
    knowledge: String,
    policies: String,
    evals: String,
    workflows: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeConfig {
    entrypoint: String,
    provider_config: String,
    capabilities: Vec<String>,
    workcell: Option<String>,
    #[serde(skip)]
    fixture: bool,
}

pub fn execute(command: AgentCommands) -> Result<(), AgentError> {
    let json_output = match &command {
        AgentCommands::New(args) => args.json,
        AgentCommands::Run(args) => args.json,
        AgentCommands::Inspect(args) | AgentCommands::Eval(args) => args.json,
        AgentCommands::Auth { command } => match command {
            AuthCommands::Set(args) => args.json,
            AuthCommands::Status(args) | AuthCommands::Remove(args) => args.json,
        },
    };
    let result = match command {
        AgentCommands::New(a) => scaffold(a),
        AgentCommands::Run(a) => run(a),
        AgentCommands::Inspect(a) => inspect(a),
        AgentCommands::Eval(a) => eval(a),
        AgentCommands::Auth { command } => credentials::execute_auth(command),
    };
    result.map_err(|mut error| {
        if json_output {
            error.message = serde_json::to_string_pretty(&json!({
                "contract": ERROR_CONTRACT,
                "status": "error",
                "exit_code": error.exit_code,
                "message": error.message,
            }))
            .unwrap();
        }
        error
    })
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut p = start.to_path_buf();
    loop {
        if p.join(".git").exists() {
            return Some(p);
        }
        if !p.pop() {
            return None;
        }
    }
}
fn discover(start: &Path) -> Result<PathBuf, AgentError> {
    let mut p = fs::canonicalize(start).map_err(|e| ioerr(e.to_string()))?;
    let git = find_git_root(&p);
    loop {
        if p.join("agent.project.json").is_file() {
            return Ok(p);
        }
        if git.as_ref() == Some(&p) || !p.pop() {
            break;
        }
    }
    Err(usage("No Agent Project found; expected agent.project.json in this directory or a parent before the Git boundary."))
}
fn load(root: &Path) -> Result<ProjectManifest, AgentError> {
    let text =
        fs::read_to_string(root.join("agent.project.json")).map_err(|e| ioerr(e.to_string()))?;
    let mut m: ProjectManifest = serde_json::from_str(&text)
        .map_err(|e| usage(format!("Malformed Agent Project manifest: {e}")))?;
    if m.contract != CONTRACT {
        return Err(usage(format!("Unsupported Agent Project contract '{}'.", m.contract)));
    }
    validate_name(&m.name)?;
    if !PROFILES.contains(&m.profile.as_str()) {
        return Err(usage(format!("Unknown Agent Project profile '{}'.", m.profile)));
    }
    let root = root.canonicalize().map_err(|e| ioerr(e.to_string()))?;
    for rel in [
        &m.schema,
        &m.agent.definition,
        &m.agent.manifest,
        &m.agent.instructions,
        &m.agent.evals,
        &m.runtime.entrypoint,
        &m.runtime.provider_config,
    ] {
        validate_project_path(&root, rel, true)?;
    }
    for rel in
        [&m.agent.skills, &m.agent.tools, &m.agent.knowledge, &m.agent.policies, &m.agent.workflows]
    {
        validate_project_path(&root, rel, false)?;
    }
    for (integration, enabled) in &m.integrations {
        if *enabled {
            let config_key = match integration.as_str() {
                "mcp" => Some("mcp"),
                "retrieval" => Some("retrieval"),
                "dispatch" => Some("dispatch"),
                "relay" => Some("relay"),
                "workcell" => Some("workcell"),
                "watchdog" | "runledger" => Some("observability"),
                _ => {
                    return Err(usage(format!(
                        "Unknown Agent Project integration '{integration}'."
                    )))
                }
            };
            if let Some(key) = config_key {
                let rel = m.integration_configs.get(key).ok_or_else(|| {
                    usage(format!("Active integration '{integration}' has no configuration path."))
                })?;
                validate_project_path(&root, rel, true)?;
                parse_json_file(&root.join(rel), &format!("{integration} configuration"))?;
            }
        }
    }
    if let Some(workcell) = &m.runtime.workcell {
        validate_project_path(&root, workcell, true)?;
        validate_workcell(&root.join(workcell))?;
    }
    m.runtime.fixture = validate_provider_config(&root.join(&m.runtime.provider_config))?;
    validate_agent_package(&root, &m)?;
    Ok(m)
}

fn validate_project_path(root: &Path, rel: &str, file: bool) -> Result<PathBuf, AgentError> {
    let candidate = Path::new(rel);
    if candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
        })
    {
        return Err(usage(format!("Agent Project path escapes the project: {rel}")));
    }
    let canonical = root
        .join(candidate)
        .canonicalize()
        .map_err(|_| usage(format!("Referenced path is missing: {rel}")))?;
    if !canonical.starts_with(root) {
        return Err(usage(format!("Agent Project path escapes the project: {rel}")));
    }
    if file && !canonical.is_file() {
        return Err(usage(format!("Referenced path is not a file: {rel}")));
    }
    if !file && !canonical.is_dir() {
        return Err(usage(format!("Referenced path is not a directory: {rel}")));
    }
    Ok(canonical)
}

fn parse_json_file(path: &Path, label: &str) -> Result<Value, AgentError> {
    let source =
        fs::read_to_string(path).map_err(|e| usage(format!("Cannot read {label}: {e}")))?;
    serde_json::from_str(&source).map_err(|e| usage(format!("Malformed {label}: {e}")))
}

fn required_string<'a>(value: &'a Value, key: &str, label: &str) -> Result<&'a str, AgentError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| usage(format!("{label} requires a non-empty '{key}' string.")))
}

fn validate_provider_config(path: &Path) -> Result<bool, AgentError> {
    let value = parse_json_file(path, "provider configuration")?;
    let provider = required_string(&value, "provider", "Provider configuration")?;
    if provider_settings(provider).is_none() {
        return Err(usage(format!("Unsupported provider '{provider}'.")));
    }
    required_string(&value, "model", "Provider configuration")?;
    let mode = required_string(&value, "mode", "Provider configuration")?;
    if !matches!(mode, "fixture" | "live") {
        return Err(usage("Provider configuration mode must be 'fixture' or 'live'."));
    }
    if (provider == "fixture") != (mode == "fixture") {
        return Err(usage("Provider configuration uses an inconsistent provider and mode."));
    }
    if mode == "live" {
        let base_url = required_string(&value, "base_url", "Provider configuration")?;
        let api_key_env = required_string(&value, "api_key_env", "Provider configuration")?;
        if !api_key_env.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        }) {
            return Err(usage(
                "Provider configuration api_key_env must be an uppercase environment variable name.",
            ));
        }
        if base_url.contains('@') || base_url.contains('?') || base_url.contains('#') {
            return Err(usage(
                "Provider base_url may not contain credentials, query parameters, or fragments.",
            ));
        }
        let local =
            base_url.starts_with("http://127.0.0.1:") || base_url.starts_with("http://localhost:");
        let allow_local =
            value.get("allow_insecure_localhost").and_then(Value::as_bool).unwrap_or(false);
        if !base_url.starts_with("https://") && !(local && allow_local) {
            return Err(usage(
                "Provider base_url must use HTTPS; loopback HTTP requires allow_insecure_localhost.",
            ));
        }
    }
    Ok(mode == "fixture")
}

fn validate_agent_package(root: &Path, project: &ProjectManifest) -> Result<(), AgentError> {
    let manifest_path = root.join(&project.agent.manifest);
    let value = parse_json_file(&manifest_path, "Agent package manifest")?;
    if required_string(&value, "schema_version", "Agent package manifest")?
        != "kujo-agent-package/v1"
    {
        return Err(usage("Unsupported Agent package schema_version."));
    }
    let package_root =
        manifest_path.parent().ok_or_else(|| usage("Invalid Agent package path."))?;
    for key in ["instructions", "input_schema", "output_schema"] {
        let rel = required_string(&value, key, "Agent package manifest")?;
        let path = package_root.join(rel);
        let canonical = path
            .canonicalize()
            .map_err(|_| usage(format!("Agent package path is missing: {rel}")))?;
        if !canonical.starts_with(package_root) || !canonical.is_file() {
            return Err(usage(format!("Agent package path escapes its package: {rel}")));
        }
    }
    Ok(())
}

fn validate_workcell(path: &Path) -> Result<(), AgentError> {
    let value = parse_json_file(path, "Workcell definition")?;
    if value.get("version").and_then(Value::as_i64) != Some(1) {
        return Err(usage("Workcell definition requires version 1."));
    }
    if value.pointer("/filesystem/read_only_root").and_then(Value::as_bool) != Some(true) {
        return Err(usage("Workcell definition must keep filesystem.read_only_root enabled."));
    }
    if value.pointer("/network/mode").and_then(Value::as_str) != Some("none") {
        return Err(usage(
            "Generated hardened Agent Projects require Workcell network.mode 'none'.",
        ));
    }
    Ok(())
}
fn inspection(root: &Path, m: &ProjectManifest) -> Value {
    let model: Value = fs::read_to_string(root.join(&m.runtime.provider_config))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(json!({}));
    let skills = list_names(&root.join(&m.agent.skills));
    let tools = list_names(&root.join(&m.agent.tools));
    let dependencies = declared_dependencies(root);
    let mut unresolved: Vec<String> = dependencies
        .keys()
        .filter(|name| !root.join("kennel_packages").join(name).is_dir())
        .map(|name| format!("{name}: run `kennel install`"))
        .collect();
    let mut external_tools = BTreeMap::new();
    for (name, tool) in &m.external_tools {
        let available = external_tool_available(root, name, &tool.command);
        if !available {
            unresolved.push(format!("{name}: install with `bash install.sh --group agent`"));
        }
        external_tools.insert(
            name.clone(),
            json!({
                "command":tool.command,
                "source":tool.source,
                "commit":tool.commit,
                "required_for":tool.required_for,
                "available":available
            }),
        );
    }
    let credentials = credential_names(root);
    let credential_state =
        model.get("api_key_env").and_then(Value::as_str).filter(|name| !name.is_empty()).map(
            |name| {
                let resolved = credentials::resolve_for_project(root, name).ok().flatten();
                if resolved.is_none() {
                    unresolved.push(format!(
                        "{name}: run `kujo agent auth set {}`",
                        model.get("provider").and_then(Value::as_str).unwrap_or("custom")
                    ));
                }
                json!({
                    "name":name,
                    "configured":resolved.is_some(),
                    "source":resolved.map(|value| value.source.label())
                })
            },
        );
    unresolved.sort();
    let mcp_servers = m
        .integration_configs
        .get("mcp")
        .and_then(|path| parse_json_file(&root.join(path), "MCP configuration").ok())
        .and_then(|value| value.get("servers").and_then(Value::as_array).map(Vec::len))
        .unwrap_or(0);
    let observability = m
        .integration_configs
        .get("observability")
        .and_then(|path| parse_json_file(&root.join(path), "observability configuration").ok())
        .unwrap_or_else(|| json!({"watchdog":{"enabled":false},"runledger":{"enabled":false}}));
    json!({
        "contract":INSPECT_CONTRACT,
        "project":{
            "name":m.name,
            "profile":m.profile,
            "root":root,
            "manifest":"agent.project.json",
            "schema":m.schema
        },
        "agent":{
            "definition":m.agent.definition,
            "manifest":m.agent.manifest,
            "instructions":m.agent.instructions,
            "skills":{"path":m.agent.skills,"entries":skills},
            "tools":{"path":m.agent.tools,"entries":tools},
            "knowledge":{"path":m.agent.knowledge,"configured":m.integrations.get("retrieval").copied().unwrap_or(false)},
            "policies":m.agent.policies,
            "workflows":{"path":m.agent.workflows,"configured":m.integrations.get("dispatch").copied().unwrap_or(false)},
            "evals":{"suite":m.agent.evals,"engine":"eval"}
        },
        "runtime":{
            "entrypoint":m.runtime.entrypoint,
            "provider":model.get("provider"),
            "model":model.get("model"),
            "mode":model.get("mode"),
            "fixture":m.runtime.fixture,
            "capabilities":m.runtime.capabilities
        },
        "integrations":{
            "active":m.integrations,
            "configuration":m.integration_configs,
            "mcp_servers":mcp_servers,
            "observability":observability
        },
        "workcell":{"configured":m.runtime.workcell.is_some(),"definition":m.runtime.workcell},
        "dependencies":{"declared":dependencies,"external_tools":external_tools,"unresolved":unresolved},
        "external_state":{
            "credential_names":credentials,
            "credential":credential_state,
            "container_runtime":if m.runtime.workcell.is_some(){"Docker or Podman required for isolated runs"}else{"not required"},
            "watchdog":if observability.pointer("/watchdog/enabled").and_then(Value::as_bool)==Some(true){"configured"}else{"disabled"},
            "mcp_endpoints":mcp_servers
        }
    })
}

fn external_tool_available(root: &Path, name: &str, command: &str) -> bool {
    let local_entry = match name {
        "runledger" => Some(root.join("kennel_packages/runledger/runledger.kujo")),
        "watchdog" => Some(root.join("kennel_packages/watchdog/watchdog.kujo")),
        _ => None,
    };
    if local_entry.as_ref().is_some_and(|path| path.is_file()) {
        return true;
    }
    let candidate = Path::new(command);
    if candidate.components().count() > 1 {
        return candidate.is_file();
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .any(|directory| {
            let path = directory.join(command);
            if path.is_file() {
                return true;
            }
            #[cfg(windows)]
            {
                return directory.join(format!("{command}.exe")).is_file()
                    || directory.join(format!("{command}.cmd")).is_file();
            }
            #[cfg(not(windows))]
            false
        })
}

fn declared_dependencies(root: &Path) -> BTreeMap<String, Value> {
    let mut dependencies = BTreeMap::new();
    let source = fs::read_to_string(root.join("kennel.toml")).unwrap_or_default();
    let mut in_dependencies = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if !in_dependencies || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let field = |key: &str| {
            let marker = format!("{key} = \"");
            value.find(&marker).and_then(|start| {
                let rest = &value[start + marker.len()..];
                rest.find('"').map(|end| rest[..end].to_string())
            })
        };
        dependencies.insert(
            name.to_string(),
            json!({
                "source":field("source"),
                "commit":field("commit"),
                "installed":root.join("kennel_packages").join(name).is_dir()
            }),
        );
    }
    dependencies
}

fn credential_names(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(source) = fs::read_to_string(root.join(".env.example")) {
        for line in source.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((name, _)) = line.split_once('=') {
                if !name.trim().is_empty() {
                    names.push(name.trim().to_string());
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}
fn list_names(path: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for e in entries.flatten() {
            out.push(e.file_name().to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}
fn inspect(a: ProjectArgs) -> Result<(), AgentError> {
    let cwd = std::env::current_dir().map_err(|e| ioerr(e.to_string()))?;
    let root = discover(&cwd)?;
    let m = load(&root)?;
    let v = inspection(&root, &m);
    if a.json {
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else {
        let active: Vec<&str> = m
            .integrations
            .iter()
            .filter_map(|(name, enabled)| enabled.then_some(name.as_str()))
            .collect();
        let unresolved = v["dependencies"]["unresolved"].as_array().map(Vec::len).unwrap_or(0);
        let credentials: Vec<&str> = v["external_state"]["credential_names"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        println!(
            "Kujo Agent Project: {}\nContract: {}\nProfile: {}\nEntrypoint: {}\nProvider: {}\nModel: {}\nSkills: {}\nTools: {}\nKnowledge: {}\nWorkflow: {}\nEval: {} via Eval\nIntegrations: {}\nWorkcell: {}\nCapabilities: {}\nDependencies unresolved: {}\nExternal state:\n  Credentials: {}\n  Credential status: {}\n  Container runtime: {}\n  Watchdog: {}\n  MCP endpoints: {}",
            m.name,
            m.contract,
            m.profile,
            m.runtime.entrypoint,
            v["runtime"]["provider"].as_str().unwrap_or("unknown"),
            v["runtime"]["model"].as_str().unwrap_or("unknown"),
            v["agent"]["skills"]["entries"].as_array().map(Vec::len).unwrap_or(0),
            v["agent"]["tools"]["entries"].as_array().map(Vec::len).unwrap_or(0),
            m.agent.knowledge,
            m.agent.workflows,
            m.agent.evals,
            if active.is_empty() { "none".into() } else { active.join(", ") },
            m.runtime.workcell.as_deref().unwrap_or("disabled"),
            m.runtime.capabilities.join(", "),
            unresolved,
            if credentials.is_empty() { "none".into() } else { credentials.join(", ") },
            v["external_state"]["credential"]
                .get("configured")
                .and_then(Value::as_bool)
                .map(|configured| if configured {
                    format!(
                        "configured via {}",
                        v["external_state"]["credential"]["source"]
                            .as_str()
                            .unwrap_or("unknown source")
                    )
                } else {
                    "missing".into()
                })
                .unwrap_or_else(|| "not required".into()),
            v["external_state"]["container_runtime"].as_str().unwrap_or("unknown"),
            v["external_state"]["watchdog"].as_str().unwrap_or("unknown"),
            v["external_state"]["mcp_endpoints"].as_u64().unwrap_or(0)
        );
    }
    Ok(())
}
fn prompt_from(a: &RunArgs, root: &Path) -> Result<String, AgentError> {
    match (&a.prompt, &a.file) {
        (Some(_), Some(_)) => Err(usage("Use either a prompt argument or --file, not both.")),
        (Some(p), None) => Ok(p.clone()),
        (None, Some(f)) => {
            let p = if f.is_absolute() { f.clone() } else { root.join(f) };
            let c = p.canonicalize().map_err(|e| ioerr(e.to_string()))?;
            if !c.starts_with(root) {
                return Err(usage("Prompt file must remain inside the Agent Project."));
            }
            fs::read_to_string(c).map_err(|e| ioerr(e.to_string()))
        }
        (None, None) => Ok("Hello".into()),
    }
}

struct PreparedRun {
    prompt: String,
    evidence: BTreeMap<String, Value>,
}

fn prepare_integrations(
    root: &Path,
    project: &ProjectManifest,
    prompt: &str,
) -> Result<PreparedRun, AgentError> {
    let mut context = Vec::new();
    let mut evidence = BTreeMap::new();
    if project.integrations.get("mcp").copied().unwrap_or(false) {
        let mcp_root = installed_package(root, "mcp")?;
        run_kujo_script(
            root,
            &root.join("src/integrations/mcp_adapter.kujo"),
            &root.join("kennel_packages/agents-sdk"),
            &[],
            &[],
            true,
        )?;
        let output = run_kujo_script(
            root,
            &root.join("src/integrations/mcp_read.kujo"),
            &mcp_root,
            &[],
            &[],
            true,
        )?;
        let result = parse_last_json(&output, "MCP tool result")?;
        if result.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(fail("The project-local MCP read tool failed."));
        }
        let content = result.get("content").and_then(Value::as_str).unwrap_or("");
        context.push(format!("MCP read_project_docs result: {}", truncate_text(content, 1200)));
        evidence.insert(
            "mcp".into(),
            json!({"status":"pass","provider":"mcp","tool":"read_project_docs","transport":"local"}),
        );
    }
    if project.integrations.get("retrieval").copied().unwrap_or(false) {
        let rag_root = installed_package(root, "rag")?;
        let rag_entrypoint = rag_root.join("main.kujo");
        run_kujo_script(
            root,
            &rag_entrypoint,
            &rag_root,
            &[
                "ingest",
                "--path",
                "./agent/knowledge",
                "--recursive",
                "true",
                "--namespace",
                "owned-agent",
            ],
            &[],
            false,
        )?;
        let query_output = run_kujo_script(
            root,
            &rag_entrypoint,
            &rag_root,
            &["query", "--question", prompt, "--namespace", "owned-agent"],
            &[],
            false,
        )?;
        let rag_result = parse_last_json(&query_output, "RAG query result")?;
        if rag_result.get("answer").and_then(Value::as_str).is_none() {
            return Err(fail("RAG returned no answer."));
        }
        let adapter_output = run_kujo_script(
            root,
            &root.join("src/integrations/retrieval_adapter.kujo"),
            &root.join("kennel_packages/agents-sdk"),
            &[&query_output],
            &[],
            true,
        )?;
        let retrieval = parse_last_json(&adapter_output, "Agents SDK retrieval result")?;
        if retrieval.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(fail("Agents SDK rejected the RAG retrieval result."));
        }
        let answer = retrieval.pointer("/context/summary").and_then(Value::as_str).unwrap_or("");
        let citations = retrieval
            .pointer("/context/citations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        context.push(format!(
            "RAG retrieval context: {}\nCitations: {}",
            truncate_text(answer, 1800),
            serde_json::to_string(&citations).unwrap_or_else(|_| "[]".into())
        ));
        evidence.insert(
            "retrieval".into(),
            json!({"status":"pass","provider":"kujo-rag","adapter":"agents-sdk","citations":citations}),
        );
    }
    if project.integrations.get("dispatch").copied().unwrap_or(false) {
        let dispatch_root = installed_package(root, "dispatch")?;
        let ai_sdk_root = installed_package(root, "ai-sdk")?;
        let kujo_bin = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
        let dispatch_root_value = dispatch_root.to_string_lossy().into_owned();
        let ai_sdk_value = ai_sdk_root.to_string_lossy().into_owned();
        let kujo_value = kujo_bin.to_string_lossy().into_owned();
        run_kujo_script(
            root,
            &dispatch_root.join("dispatch.kujo"),
            &dispatch_root,
            &[
                "demo",
                prompt,
                "--workflow-file",
                "workflows/default.json",
                "--yes",
                "--non-interactive",
                "--output-root",
                ".dispatch-runs",
            ],
            &[
                ("DISPATCH_OFFLINE_FIXTURE", "true"),
                ("DISPATCH_ROOT", &dispatch_root_value),
                ("AI_SDK_PATH", &ai_sdk_value),
                ("KUJO_BIN", &kujo_value),
            ],
            false,
        )?;
        context.push("Dispatch completed the repository-owned workflow and wrote resumable evidence under .dispatch-runs.".into());
        evidence.insert(
            "dispatch".into(),
            json!({"status":"pass","engine":"dispatch","artifacts":".dispatch-runs"}),
        );
    }
    if project.integrations.get("watchdog").copied().unwrap_or(false) {
        let output = run_kujo_script(
            root,
            &root.join("src/integrations/watchdog_trace.kujo"),
            &root.join("kennel_packages/agents-sdk"),
            &[],
            &[],
            true,
        )?;
        let trace = parse_last_json(&output, "Watchdog trace adapter result")?;
        if trace.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(fail("Agents SDK could not create the Watchdog trace record."));
        }
        evidence.insert(
            "watchdog".into(),
            json!({
                "status":"prepared",
                "adapter":"agents-sdk/watchdog",
                "fixture_delivery":"local-only",
                "record":trace.get("record")
            }),
        );
    }
    if project.integrations.get("relay").copied().unwrap_or(false) {
        let relay_root = installed_package(root, "relay")?;
        let ai_sdk_root = installed_package(root, "ai-sdk")?;
        let agents_sdk_root = installed_package(root, "agents-sdk")?;
        let kujo_bin = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
        let relay_value = relay_root.to_string_lossy().into_owned();
        let ai_sdk_value = ai_sdk_root.to_string_lossy().into_owned();
        let agents_sdk_value = agents_sdk_root.to_string_lossy().into_owned();
        let kujo_value = kujo_bin.to_string_lossy().into_owned();
        let state_value = root.join(".relay").to_string_lossy().into_owned();
        let output = run_kujo_script(
            root,
            &relay_root.join("main.kujo"),
            &relay_root,
            &["chat", prompt, "--fixture", "--json"],
            &[
                ("RELAY_ROOT", &relay_value),
                ("RELAY_STATE_ROOT", &state_value),
                ("RELAY_AI_SDK_PATH", &ai_sdk_value),
                ("RELAY_AGENTS_SDK_PATH", &agents_sdk_value),
                ("KUJO_BIN", &kujo_value),
            ],
            false,
        )?;
        let relay = parse_last_json(&output, "Relay fixture result")?;
        if relay.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(fail("Relay fixture execution failed."));
        }
        let relay_text = relay.get("output_text").and_then(Value::as_str).unwrap_or("");
        context.push(format!("Relay mission adapter result: {}", truncate_text(relay_text, 800)));
        evidence.insert(
            "relay".into(),
            json!({
                "status":"pass",
                "mode":"fixture",
                "provider":relay.get("provider"),
                "model":relay.get("model"),
                "usage":relay.get("usage")
            }),
        );
    }
    let effective = if context.is_empty() {
        prompt.to_string()
    } else {
        format!("{prompt}\n\nRepository integration context:\n{}", context.join("\n\n"))
    };
    Ok(PreparedRun { prompt: truncate_text(&effective, 6000), evidence })
}

fn installed_package(root: &Path, package: &str) -> Result<PathBuf, AgentError> {
    let path = root.join("kennel_packages").join(package);
    if path.is_dir() {
        Ok(path)
    } else {
        Err(fail(format!(
            "{package} is not installed. Run `kennel install` from the Agent Project root."
        )))
    }
}

fn run_kujo_script(
    cwd: &Path,
    script: &Path,
    module_path: &Path,
    args: &[&str],
    environment: &[(&str, &str)],
    untrusted_read_only: bool,
) -> Result<String, AgentError> {
    if !script.is_file() {
        return Err(fail(format!("Integration entrypoint is missing: {}", script.display())));
    }
    let exe = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
    let mut command = Command::new(exe);
    command.arg("run").arg("--interpreter");
    if untrusted_read_only {
        command.arg("--untrusted").arg("--allow-fs-read").arg("--allow-clock");
    }
    command.arg(script);
    for arg in args {
        command.arg(arg);
    }
    command.env("KUJO_MODULE_PATH", module_path).current_dir(cwd);
    for (name, value) in environment {
        command.env(name, value);
    }
    let output = command.output().map_err(|e| ioerr(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(fail(if !stderr.is_empty() { stderr } else { stdout }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_last_json(source: &str, label: &str) -> Result<Value, AgentError> {
    if let Ok(value) = serde_json::from_str(source.trim()) {
        return Ok(value);
    }
    for line in source.lines().rev() {
        if let Ok(value) = serde_json::from_str(line.trim()) {
            return Ok(value);
        }
    }
    Err(fail(format!("{label} was not valid JSON.")))
}

fn truncate_text(source: &str, max_chars: usize) -> String {
    if source.chars().count() <= max_chars {
        return source.to_string();
    }
    source.chars().take(max_chars).collect::<String>() + "…"
}

fn run(a: RunArgs) -> Result<(), AgentError> {
    let cwd = std::env::current_dir().map_err(|e| ioerr(e.to_string()))?;
    let root = discover(&cwd)?;
    let m = load(&root)?;
    let prompt = prompt_from(&a, &root)?;
    if a.workcell {
        return run_in_workcell(&root, &m, &prompt, a.json);
    }
    let agents_sdk = root.join("kennel_packages/agents-sdk");
    if !agents_sdk.join("src/agents/runner.kujo").is_file() {
        return Err(fail(
            "Agents SDK is not installed. Run `kennel install` from the Agent Project root.",
        ));
    }
    let ledger = if m.integrations.get("runledger").copied().unwrap_or(false) {
        Some(start_runledger(&root, &m)?)
    } else {
        None
    };
    let outcome = (|| {
        let prepared = prepare_integrations(&root, &m, &prompt)?;
        let live_response = if m.runtime.fixture {
            None
        } else {
            Some(invoke_live_model(&root, &m, &prepared.prompt)?)
        };
        let exe = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
        let mut command = Command::new(exe);
        command.env("KUJO_MODULE_PATH", &agents_sdk);
        command.arg("run").arg("--untrusted");
        apply_runtime_capabilities(&mut command, &m.runtime.capabilities, m.runtime.fixture)?;
        command.arg(&m.runtime.entrypoint).arg("--").arg(&prepared.prompt);
        if let Some(response) = &live_response {
            command.arg(response);
        }
        let out = command.current_dir(&root).output().map_err(|e| ioerr(e.to_string()))?;
        if !out.status.success() {
            return Err(fail(String::from_utf8_lossy(&out.stderr).trim().to_string()));
        }
        Ok((String::from_utf8_lossy(&out.stdout).trim().to_string(), prepared.evidence))
    })();
    if let Some(run_id) = ledger.as_deref() {
        let success = outcome.is_ok();
        finish_runledger(&root, run_id, success)?;
    }
    let (text, integration_evidence) = outcome?;
    if a.json {
        println!("{}",serde_json::to_string_pretty(&json!({"contract":RUN_CONTRACT,"status":"ok","project":m.name,"provider_mode":if m.runtime.fixture{"fixture"}else{"live"},"output":text,"integrations":integration_evidence,"runledger_id":ledger})).unwrap());
    } else {
        println!("{text}");
    }
    Ok(())
}

fn invoke_live_model(
    root: &Path,
    project: &ProjectManifest,
    prompt: &str,
) -> Result<String, AgentError> {
    let ai_sdk = installed_package(root, "ai-sdk")?;
    let config_source = fs::read_to_string(root.join(&project.runtime.provider_config))
        .map_err(|e| ioerr(e.to_string()))?;
    let config: Value = serde_json::from_str(&config_source)
        .map_err(|e| usage(format!("Malformed provider configuration: {e}")))?;
    let credential_name = required_string(&config, "api_key_env", "Provider configuration")?;
    let credential = credentials::resolve_for_project(root, credential_name)?.ok_or_else(|| {
        fail(format!(
            "{credential_name} is not configured. Run `kujo agent auth set {}` or `kujo agent auth set {} --project`.",
            config.get("provider").and_then(Value::as_str).unwrap_or("custom"),
            config.get("provider").and_then(Value::as_str).unwrap_or("custom")
        ))
    })?;
    let allow_local =
        config.get("allow_insecure_localhost").and_then(Value::as_bool).unwrap_or(false);
    let exe = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
    let mut command = Command::new(exe);
    command
        .arg("run")
        .arg("--interpreter")
        .arg("--untrusted")
        .arg("--allow-fs-read")
        .arg("--allow-env-read")
        .arg("--allow-net-client")
        .arg("--allow-ai")
        .arg("--allow-clock");
    if !allow_local {
        command.arg("--deny-private-net");
    } else {
        command.env("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1");
    }
    let output = command
        .arg(root.join("src/live_model.kujo"))
        .arg(&config_source)
        .arg(prompt)
        .env(credential_name, &credential.value)
        .env("KUJO_MODULE_PATH", &ai_sdk)
        .current_dir(root)
        .output()
        .map_err(|e| ioerr(format!("Failed to launch AI SDK live provider bridge: {e}")))?;
    if !output.status.success() {
        let redact = |source: &[u8]| {
            String::from_utf8_lossy(source)
                .replace(&credential.value, "[REDACTED]")
                .trim()
                .to_string()
        };
        let stdout = redact(&output.stdout);
        let stderr = redact(&output.stderr);
        return Err(fail(if !stdout.is_empty() { stdout } else { stderr }));
    }
    let result = parse_last_json(&String::from_utf8_lossy(&output.stdout), "AI SDK live response")?;
    if result.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(fail(
            result
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("AI SDK live provider request failed.")
                .to_string(),
        ));
    }
    let normalized = json!({
        "ok":true,
        "provider":result.get("provider"),
        "model":result.get("model"),
        "request_id":result.get("request_id"),
        "output_text":result.get("output_text"),
        "finish_reason":result.get("finish_reason"),
        "tool_calls":result.get("tool_calls"),
        "usage":result.get("usage"),
        "status_code":result.get("status_code"),
        "contract_version":result.get("contract_version")
    });
    serde_json::to_string(&normalized)
        .map(|value| value.replace(&credential.value, "[REDACTED]"))
        .map_err(|e| ioerr(e.to_string()))
}

fn run_in_workcell(
    root: &Path,
    project: &ProjectManifest,
    prompt: &str,
    json_output: bool,
) -> Result<(), AgentError> {
    let definition = project
        .runtime
        .workcell
        .as_ref()
        .ok_or_else(|| usage("This Agent Project profile does not configure Workcell."))?;
    let agents_sdk = installed_package(root, "agents-sdk")?;
    let head = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|e| ioerr(format!("Cannot inspect Agent Project Git state: {e}")))?;
    if !head.status.success() {
        return Err(usage(
            "Workcell requires an immutable Git commit. Commit the generated project before using --workcell.",
        ));
    }
    let dirty = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=normal"])
        .current_dir(root)
        .output()
        .map_err(|e| ioerr(format!("Cannot inspect Agent Project Git state: {e}")))?;
    if !dirty.status.success() || !dirty.stdout.is_empty() {
        return Err(usage(
            "Workcell runs the committed project state. Commit or remove repository changes before using --workcell.",
        ));
    }
    prepare_workcell_agents_sdk(root, &agents_sdk)?;
    let mut value = parse_json_file(&root.join(definition), "Workcell definition")?;
    let command = value
        .get_mut("command")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| usage("Workcell definition command must be an array."))?;
    let last =
        command.last_mut().ok_or_else(|| usage("Workcell definition command may not be empty."))?;
    *last = Value::String(prompt.to_string());
    let runtime_dir = root.join(".kujo-agent");
    fs::create_dir_all(&runtime_dir).map_err(|e| ioerr(e.to_string()))?;
    let runtime_definition = runtime_dir.join("workcell-run.json");
    fs::write(&runtime_definition, format!("{}\n", serde_json::to_string_pretty(&value).unwrap()))
        .map_err(|e| ioerr(e.to_string()))?;

    let local = root.join("kennel_packages/workcell/bin/workcell");
    let mut command = if local.is_file() { Command::new(local) } else { Command::new("workcell") };
    let kujo = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
    let workcell_temp = root.join(".workcell/tmp");
    fs::create_dir_all(&workcell_temp).map_err(|e| ioerr(e.to_string()))?;
    let output = command
        .args(["run", "--file"])
        .arg(&runtime_definition)
        .arg("--repo")
        .arg(root)
        .arg("--no-pull")
        .env("KUJO_BIN", kujo)
        .env("TMPDIR", &workcell_temp)
        .current_dir(root)
        .output()
        .map_err(|e| {
            fail(format!(
                "Workcell is required for isolated execution but is unavailable: {e}. Install it with `bash install.sh --group agent`."
            ))
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(fail(if !stderr.is_empty() { stderr } else { stdout }));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let receipt = latest_child(&root.join(".workcell/runs"))
        .and_then(|path| path.strip_prefix(root).ok().map(Path::to_path_buf));
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract":RUN_CONTRACT,
                "status":"ok",
                "project":project.name,
                "isolation":"workcell",
                "output":stdout,
                "receipt":receipt
            }))
            .unwrap()
        );
    } else {
        println!("{stdout}");
        if let Some(receipt) = receipt {
            println!("Workcell receipt: {}", receipt.display());
        }
    }
    Ok(())
}

fn prepare_workcell_agents_sdk(root: &Path, source: &Path) -> Result<(), AgentError> {
    let image_root = root.join("workcell-image");
    let target = image_root.join("agents-sdk");
    let stage = image_root.join(format!(".agents-sdk-stage-{}", std::process::id()));
    if stage.exists() {
        return Err(ioerr("Workcell Agents SDK staging path already exists."));
    }
    copy_tree_bounded(source, &stage, 0)?;
    fs::write(stage.join(".kujo-agent-managed"), "agents-sdk\n")
        .map_err(|e| ioerr(e.to_string()))?;
    if target.exists() {
        if !target.join(".kujo-agent-managed").is_file() {
            let _ = fs::remove_dir_all(&stage);
            return Err(usage("Refusing to replace unmanaged workcell-image/agents-sdk content."));
        }
        fs::remove_dir_all(&target).map_err(|e| ioerr(e.to_string()))?;
    }
    fs::rename(&stage, &target).map_err(|e| ioerr(e.to_string()))
}

fn copy_tree_bounded(source: &Path, target: &Path, depth: usize) -> Result<(), AgentError> {
    if depth > 24 {
        return Err(usage("Dependency tree exceeds the Workcell copy depth limit."));
    }
    fs::create_dir_all(target).map_err(|e| ioerr(e.to_string()))?;
    for entry in fs::read_dir(source).map_err(|e| ioerr(e.to_string()))? {
        let entry = entry.map_err(|e| ioerr(e.to_string()))?;
        let name = entry.file_name();
        if matches!(name.to_str(), Some(".git" | "target" | ".kennel_tmp")) {
            continue;
        }
        let kind = entry.file_type().map_err(|e| ioerr(e.to_string()))?;
        let destination = target.join(&name);
        if kind.is_symlink() {
            return Err(usage(format!(
                "Workcell dependency contains a symlink and cannot be copied safely: {}",
                entry.path().display()
            )));
        }
        if kind.is_dir() {
            copy_tree_bounded(&entry.path(), &destination, depth + 1)?;
        } else if kind.is_file() {
            let size = entry.metadata().map_err(|e| ioerr(e.to_string()))?.len();
            if size > 16 * 1024 * 1024 {
                return Err(usage(format!(
                    "Workcell dependency file exceeds 16 MiB: {}",
                    entry.path().display()
                )));
            }
            fs::copy(entry.path(), destination).map_err(|e| ioerr(e.to_string()))?;
        }
    }
    Ok(())
}

fn latest_child(path: &Path) -> Option<PathBuf> {
    let mut entries: Vec<_> = fs::read_dir(path).ok()?.flatten().collect();
    entries.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
    entries.last().map(|entry| entry.path())
}

fn start_runledger(root: &Path, project: &ProjectManifest) -> Result<String, AgentError> {
    let model =
        parse_json_file(&root.join(&project.runtime.provider_config), "provider configuration")?;
    let provider = required_string(&model, "provider", "Provider configuration")?;
    let model_name = required_string(&model, "model", "Provider configuration")?;
    let task = format!("agent-{}", project.name);
    let repo_path = root.to_string_lossy().into_owned();
    let ledger_path = root.join(".runledger").to_string_lossy().into_owned();
    let output = invoke_runledger(
        root,
        &[
            "start",
            "--provider",
            provider,
            "--model",
            model_name,
            "--task",
            &task,
            "--repo",
            &repo_path,
            "--ledger",
            &ledger_path,
        ],
    )?;
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix("Started run: "))
        .map(str::to_string)
        .ok_or_else(|| fail("RunLedger did not return a run ID."))
}

fn finish_runledger(root: &Path, run_id: &str, success: bool) -> Result<(), AgentError> {
    let repo_path = root.to_string_lossy().into_owned();
    let ledger_path = root.join(".runledger").to_string_lossy().into_owned();
    invoke_runledger(
        root,
        &[
            "finish",
            run_id,
            "--status",
            if success { "pass" } else { "fail" },
            "--verdict",
            if success { "Agent Project run completed" } else { "Agent Project run failed" },
            "--repo",
            &repo_path,
            "--ledger",
            &ledger_path,
        ],
    )?;
    Ok(())
}

fn invoke_runledger(root: &Path, args: &[&str]) -> Result<String, AgentError> {
    let local_root = root.join("kennel_packages/runledger");
    let local_entry = local_root.join("runledger.kujo");
    let output = if local_entry.is_file() {
        run_kujo_script(&local_root, &local_entry, &local_root, args, &[], false)?
    } else {
        let mut command = Command::new("runledger");
        let output = command
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| {
                fail(format!(
                    "RunLedger is required by this profile but is unavailable: {e}. Install it with `bash install.sh --group agent`."
                ))
            })?;
        if !output.status.success() {
            return Err(fail(String::from_utf8_lossy(&output.stderr).trim().to_string()));
        }
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    Ok(output)
}

fn apply_runtime_capabilities(
    command: &mut Command,
    capabilities: &[String],
    fixture: bool,
) -> Result<(), AgentError> {
    for capability in capabilities {
        match capability.as_str() {
            "fs-read:project" => {
                command.arg("--allow-fs-read");
            }
            "clock" => {
                command.arg("--allow-clock");
            }
            "ai:configured-endpoints" => {
                if !fixture {
                    command
                        .arg("--allow-ai")
                        .arg("--allow-env-read")
                        .arg("--allow-net-client")
                        .arg("--deny-private-net");
                }
            }
            unsupported => {
                return Err(usage(format!(
                    "Unsupported Agent Project capability '{unsupported}'."
                )));
            }
        }
    }
    Ok(())
}

fn eval(a: ProjectArgs) -> Result<(), AgentError> {
    let cwd = std::env::current_dir().map_err(|e| ioerr(e.to_string()))?;
    let root = discover(&cwd)?;
    let m = load(&root)?;
    let eval_root = root.join("kennel_packages/eval");
    let eval_entrypoint = eval_root.join("main.kujo");
    if !eval_entrypoint.is_file() {
        return Err(fail("Eval is not installed. Run `kennel install` before evaluation."));
    }
    let exe = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
    let mut command = Command::new(exe);
    command.env("KUJO_MODULE_PATH", &eval_root);
    prepend_executable_dir_to_path(&mut command)?;
    let out = command
        .arg("run")
        .arg(&eval_entrypoint)
        .arg("run")
        .arg(root.join(&m.agent.evals))
        .arg("--output-dir")
        .arg(root.join(".eval-results"))
        .arg("--json")
        .current_dir(&root)
        .output()
        .map_err(|e| ioerr(e.to_string()))?;
    let passed = out.status.success();
    if a.json {
        let summary = fs::read_to_string(root.join(".eval-results/summary.json"))
            .ok()
            .and_then(|value| serde_json::from_str::<Value>(&value).ok());
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract": EVAL_CONTRACT,
                "status": if passed { "pass" } else { "fail" },
                "suite": m.agent.evals,
                "engine": "eval",
                "summary": summary,
                "artifacts": ".eval-results"
            }))
            .unwrap()
        );
    } else {
        println!("Agent eval: {} (Eval)", if passed { "PASS" } else { "FAIL" });
    }
    if passed {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        Err(fail(format!("Agent evaluation failed through Eval: {detail}")))
    }
}

fn prepend_executable_dir_to_path(command: &mut Command) -> Result<(), AgentError> {
    let exe = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
    let dir = exe.parent().ok_or_else(|| ioerr("Cannot resolve the Kujo executable directory."))?;
    let mut paths = vec![dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let joined = std::env::join_paths(paths)
        .map_err(|e| ioerr(format!("Cannot construct Eval PATH: {e}")))?;
    command.env("PATH", joined);
    Ok(())
}
