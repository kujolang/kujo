use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use crate::workflow_pack::types::{CheckResult, CheckSeverity, CheckStatus, DoctorReport};

const CONTRACT: &str = "kujo-agent-project/v1";
const INSPECT_CONTRACT: &str = "kujo-agent-inspect/v1";
const RUN_CONTRACT: &str = "kujo-agent-run/v1";
const EVAL_CONTRACT: &str = "kujo-agent-eval/v1";
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
}

#[derive(Args, Debug)]
pub struct NewArgs {
    pub name: String,
    #[arg(long)]
    pub dir: Option<PathBuf>,
    #[arg(long, default_value = "basic")]
    pub profile: String,
    #[arg(long, default_value = "fixture")]
    pub provider: String,
    #[arg(long, default_value = "fixture-owned-agent-v1")]
    pub model: String,
    #[arg(long)]
    pub no_git: bool,
    #[arg(long)]
    pub install: bool,
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
    match command {
        AgentCommands::New(a) => scaffold(a),
        AgentCommands::Run(a) => run(a),
        AgentCommands::Inspect(a) => inspect(a),
        AgentCommands::Eval(a) => eval(a),
    }
}

fn validate_name(name: &str) -> Result<(), AgentError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(usage("Agent name must contain only letters, numbers, '-' or '_'."));
    }
    Ok(())
}

fn scaffold(a: NewArgs) -> Result<(), AgentError> {
    validate_name(&a.name)?;
    if !PROFILES.contains(&a.profile.as_str()) {
        return Err(usage(format!(
            "Unknown agent profile '{}'. Expected one of: {}.",
            a.profile,
            PROFILES.join(", ")
        )));
    }
    provider_settings(&a.provider).ok_or_else(|| {
        usage(format!(
            "Unknown provider '{}'. Expected one of: fixture, openai, openrouter, deepseek, custom.",
            a.provider
        ))
    })?;
    let base = a.dir.clone().unwrap_or(std::env::current_dir().map_err(|e| ioerr(e.to_string()))?);
    if base.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(usage("Agent destination may not contain '..'."));
    }
    if base.parent().is_none() {
        return Err(usage("Agent destination may not be a filesystem root."));
    }
    let target = if base.file_name().and_then(|v| v.to_str()) == Some(a.name.as_str()) {
        base
    } else {
        base.join(&a.name)
    };
    if target.exists() && fs::read_dir(&target).map_err(|e| ioerr(e.to_string()))?.next().is_some()
    {
        return Err(usage(format!("Destination '{}' is not empty.", target.display())));
    }
    let parent = target.parent().ok_or_else(|| usage("Unsafe agent destination."))?;
    fs::create_dir_all(parent).map_err(|e| ioerr(e.to_string()))?;
    let parent_real = fs::canonicalize(parent).map_err(|e| ioerr(e.to_string()))?;
    if target.exists()
        && fs::symlink_metadata(&target).map_err(|e| ioerr(e.to_string()))?.file_type().is_symlink()
    {
        return Err(usage("Agent destination may not be a symlink."));
    }
    let stage = parent_real.join(format!(".{}.kujo-agent-stage-{}", a.name, std::process::id()));
    if stage.exists() {
        return Err(ioerr("Scaffold staging directory already exists."));
    }
    fs::create_dir(&stage).map_err(|e| ioerr(e.to_string()))?;
    let result = write_project(&stage, &a);
    if let Err(err) = result {
        let _ = fs::remove_dir_all(&stage);
        return Err(err);
    }
    if target.exists() {
        fs::remove_dir(&target).map_err(|e| ioerr(e.to_string()))?;
    }
    fs::rename(&stage, &target).map_err(|e| {
        let _ = fs::remove_dir_all(&stage);
        ioerr(format!("Failed to promote scaffold: {e}"))
    })?;
    if !a.no_git && find_git_root(target.parent().unwrap_or(&target)).is_none() {
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&target)
            .status()
            .map_err(|e| ioerr(format!("Failed to initialize Git: {e}")))?;
        if !status.success() {
            return Err(fail("Git initialization failed."));
        }
    }
    if a.install {
        let status = Command::new("kennel")
            .arg("install")
            .current_dir(&target)
            .status()
            .map_err(|e| ioerr(format!("Failed to run `kennel install`: {e}")))?;
        if !status.success() {
            return Err(fail("Kennel dependency installation failed."));
        }
    }
    let payload = json!({"contract":"kujo-agent-new/v1","status":"created","project":a.name,"profile":a.profile,"path":target,"git":!a.no_git});
    if a.json {
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else {
        println!("Created Kujo Agent Project '{}' ({}) at {}", a.name, a.profile, target.display());
        println!(
            "Next: cd {} && kennel install && kujo doctor agent && kujo agent inspect",
            target.display()
        );
    }
    Ok(())
}

fn write(path: &Path, rel: &str, body: &str) -> Result<(), AgentError> {
    let p = path.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| ioerr(e.to_string()))?;
    }
    fs::write(p, body).map_err(|e| ioerr(e.to_string()))
}
fn write_project(root: &Path, a: &NewArgs) -> Result<(), AgentError> {
    let provider = provider_settings(&a.provider)
        .ok_or_else(|| usage(format!("Unsupported provider '{}'.", a.provider)))?;
    let has = |p: &str| a.profile == p || a.profile == "full";
    for directory in [
        "agent/skills",
        "agent/tools",
        "agent/knowledge",
        "agent/policies",
        "evals",
        "workflows",
        "config",
        "schemas",
        "src",
    ] {
        fs::create_dir_all(root.join(directory)).map_err(|e| ioerr(e.to_string()))?;
    }
    let mut integrations = BTreeMap::new();
    let mut integration_configs = BTreeMap::new();
    let mut external_tools = BTreeMap::new();
    for key in ["mcp", "retrieval", "dispatch", "relay", "workcell", "watchdog", "runledger"] {
        integrations.insert(key.into(), false);
    }
    if has("tools") {
        integrations.insert("mcp".into(), true);
        integration_configs.insert("mcp".into(), "config/mcp.json".into());
    }
    if has("knowledge") {
        integrations.insert("retrieval".into(), true);
        integration_configs.insert("retrieval".into(), "config/retrieval.json".into());
    }
    if has("workflow") {
        integrations.insert("dispatch".into(), true);
        integration_configs.insert("dispatch".into(), "workflows/default.json".into());
    }
    if has("hardened") {
        integrations.insert("workcell".into(), true);
        integration_configs.insert("workcell".into(), "workcell.json".into());
    }
    if has("observable") {
        integrations.insert("watchdog".into(), true);
        integrations.insert("runledger".into(), true);
        integration_configs.insert("observability".into(), "config/observability.json".into());
        external_tools.insert(
            "watchdog".into(),
            ExternalTool {
                command: "watchdog".into(),
                source: "github:kujolang/watchdog".into(),
                commit: "1af292b3e03217760649dcb4f903e443f48c563c".into(),
                required_for: vec!["live provider telemetry".into()],
            },
        );
        external_tools.insert(
            "runledger".into(),
            ExternalTool {
                command: "runledger".into(),
                source: "github:kujolang/runledger".into(),
                commit: "12bbf2b3723325913eb75ececaba0ce3fdc68b87".into(),
                required_for: vec!["run evidence".into()],
            },
        );
    }
    if a.profile == "full" {
        integrations.insert("relay".into(), true);
        integration_configs.insert("relay".into(), "config/relay.json".into());
    }
    let capabilities =
        vec!["fs-read:project".into(), "clock".into(), "ai:configured-endpoints".into()];
    let manifest = ProjectManifest {
        contract: CONTRACT.into(),
        schema: "schemas/agent-project.schema.json".into(),
        name: a.name.clone(),
        profile: a.profile.clone(),
        agent: AgentPaths {
            definition: "agent/AGENT.md".into(),
            manifest: "agent/manifest.json".into(),
            instructions: "agent/instructions.md".into(),
            skills: "agent/skills".into(),
            tools: "agent/tools".into(),
            knowledge: "agent/knowledge".into(),
            policies: "agent/policies".into(),
            evals: "evals/eval.json".into(),
            workflows: "workflows".into(),
        },
        runtime: RuntimeConfig {
            entrypoint: "src/main.kujo".into(),
            provider_config: "config/model.json".into(),
            capabilities,
            workcell: if has("hardened") { Some("workcell.json".into()) } else { None },
            fixture: a.provider == "fixture",
        },
        integrations,
        integration_configs,
        external_tools,
    };
    write(
        root,
        "agent.project.json",
        &format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
    )?;
    write(
        root,
        "schemas/agent-project.schema.json",
        include_str!("../schemas/agent-project.schema.json"),
    )?;
    write(
        root,
        "config/model.json",
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "contract":"kujo-ai-sdk/model-preference/v1",
                "provider":a.provider,
                "model":a.model,
                "mode":provider.mode,
                "base_url":provider.base_url,
                "api_key_env":provider.api_key_env,
                "allow_insecure_localhost":false
            }))
            .unwrap()
        ),
    )?;
    write(root, "agent/manifest.json", &format!("{{\n  \"schema_version\": \"kujo-agent-package/v1\",\n  \"name\": \"{}\",\n  \"instructions\": \"instructions.md\",\n  \"input_schema\": \"input.schema.json\",\n  \"output_schema\": \"output.schema.json\"\n}}\n", a.name))?;
    write(
        root,
        "agent/AGENT.md",
        &format!(
            "# {}\n\nA repository-owned Kujo agent using AI SDK and Agents SDK boundaries.\n",
            a.name
        ),
    )?;
    write(root, "agent/instructions.md", "# Instructions\n\nAnswer clearly, cite repository knowledge when used, and stay within declared policies.\n")?;
    write(root, "agent/input.schema.json", "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\",\"required\":[\"prompt\"],\"properties\":{\"prompt\":{\"type\":\"string\"}}}\n")?;
    write(root, "agent/output.schema.json", "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\",\"required\":[\"output\"],\"properties\":{\"output\":{\"type\":\"string\"}}}\n")?;
    write(root, "agent/skills/owned-agent/SKILL.md", "---\nname: owned-agent\ndescription: Project-local behavior for this agent.\n---\n\nUse the repository contract and policies as authority.\n")?;
    write(
        root,
        "agent/policies/capabilities.json",
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({"capabilities":manifest.runtime.capabilities}))
                .unwrap()
        ),
    )?;
    write(
        root,
        "src/main.kujo",
        r#"from src.agents.testing.no_network import create_no_network_harness
from src.agents.runner import create_agent_runner, run_agent
from src.agents.core_types import create_agent, create_agent_run_request

let argv := args()
mut prompt := "Hello"
if len(argv) > 0 { prompt = argv[0] }

mut harness := create_no_network_harness({"model": {"output_text": "Owned agent fixture: " + prompt}})
if len(argv) > 1 {
    let normalized_response := parse_json(argv[1])
    harness = create_no_network_harness({"model": {
        "provider": normalized_response["provider"],
        "model": normalized_response["model"],
        "output_text": normalized_response["output_text"]
    }})
}
if harness["ok"] == false {
    print(to_json(harness))
} else {
    let runner := create_agent_runner({"ai_adapter": harness["model_adapter"]})
    let agent := create_agent({
        "id": "owned-agent",
        "name": "Owned Agent",
        "instructions": "Follow agent/instructions.md and the repository policy."
    })
    let request := create_agent_run_request(prompt, {
        "run_id": "run-owned-agent",
        "session_id": "session-owned-agent"
    })
    let result := run_agent(runner, agent, request, {"tool_registry": harness["tool_registry"]})
    print(result["output"]["text"])
}
"#,
    )?;
    write(
        root,
        "src/live_model.kujo",
        r#"from src.providers import openai_provider, openrouter_provider, deepseek_provider, custom_openai_compatible_provider_with_options
from src.ai_sdk import create_client, create_message, chat_completion

let argv := args()
let config := parse_json(argv[0])
let prompt := argv[1]
let provider_id := config["provider"]
mut provider := openai_provider()
if provider_id == "openrouter" {
    provider = openrouter_provider()
}
if provider_id == "deepseek" {
    provider = deepseek_provider()
}
if provider_id == "custom" {
    provider = custom_openai_compatible_provider_with_options(
        config["base_url"],
        config["api_key_env"],
        config["model"],
        config["allow_insecure_localhost"]
    )
}
provider["default_model"] := config["model"]
provider["supported_models"] := [config["model"]]
let credential := env(config["api_key_env"])
if credential == null || trim(to_string(credential)) == "" {
    print(to_json({"ok": false, "code": "missing_credential", "credential_name": config["api_key_env"]}))
    exit(2)
}
let client := create_client(provider, credential)
let messages := [
    create_message("system", "Follow the repository-owned Agent instructions and policies."),
    create_message("user", prompt)
]
let result := chat_completion(client, messages, {
    "model": config["model"],
    "timeout": 60,
    "max_retries": 2
})
print(to_json(result))
if result["ok"] == false { exit(1) }
"#,
    )?;
    write(
        root,
        "evals/eval.json",
        r#"{
  "name": "owned-agent-fixture",
  "description": "Deterministic Agents SDK fixture evaluation",
  "version": "1.0.0",
  "output_dir": ".eval-results",
  "stop_on_failure": true,
  "tests": [
    {
      "name": "fixture run succeeds",
      "check": "command_succeeds",
      "params": {
        "command": "kujo agent run fixture-check",
        "timeout_ms": 120000
      }
    }
  ]
}
"#,
    )?;
    write(root, "kujo.toml", &format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\nentrypoint = \"src/main.kujo\"\n\n[dependencies]\n", a.name))?;
    write(root, "kujo.lock", "version = 1\npackages = []\n")?;
    write(root, "kennel.toml", &kennel_manifest(&a.profile))?;
    write(
        root,
        ".env.example",
        if provider.api_key_env.is_empty() {
            "# Fixture mode requires no credentials.\n"
        } else {
            provider.env_example
        },
    )?;
    write(
        root,
        ".gitignore",
        ".env\n.kennel_tmp/\nkennel_packages/\n.eval-results/\n.runledger/\n.workcell/\n.relay/\n.dispatch-runs/\n.kujo-agent/\ndata/\nresults/\nworkcell-image/agents-sdk/\n",
    )?;
    write(root, "AGENTS.md", "# Agent Project Guide\n\nTreat `agent.project.json` as the root contract. Never commit credentials. Run `kujo doctor agent`, `kujo agent inspect`, `kujo agent run`, and `kujo agent eval`.\n")?;
    write(root, "README.md", &format!("# {}\n\nThis Git repository owns the agent definition, instructions, model preference, skills, tools, knowledge, policies, workflows, evals, and execution boundaries.\n\n```bash\nkennel install\nkujo doctor agent\nkujo agent inspect\nkujo agent run \"Hello\"\nkujo agent eval\n```\n\nFixture mode uses the Agents SDK no-network harness and requires no provider credentials. Change `config/model.json` for a live AI SDK provider and provide credentials through the environment only. Kujo capabilities authorize effects; they are not a sandbox. Hardened projects use Workcell for container-backed isolation.\n", a.name))?;
    if has("tools") {
        write(root, "agent/tools/read-project.json", "{\"name\":\"read_project_docs\",\"description\":\"Read allowlisted project documentation\",\"risk\":\"read_only\",\"approval\":\"never\"}\n")?;
        write(root, "config/mcp.json", "{\n  \"contract\": \"kujo-mcp/project/v1\",\n  \"servers\": [\n    {\n      \"id\": \"project-docs\",\n      \"transport\": \"local\",\n      \"package\": \"mcp\",\n      \"tool\": \"read_project_docs\",\n      \"config\": \"mcp-server.json\"\n    }\n  ]\n}\n")?;
        write(root, "mcp-server.json", "{\n  \"server\": {\"name\": \"owned-agent-tools\", \"version\": \"1.0.0\", \"description\": \"Read-only project tools\"},\n  \"permissions\": {\n    \"allowed_directories\": [\".\"],\n    \"max_file_size\": 1048576,\n    \"read_only_patterns\": [\"*.md\", \"*.txt\", \"*.json\"]\n  },\n  \"tools\": {\"enabled\": true, \"default_timeout_ms\": 30000},\n  \"resources\": {\"enabled\": true},\n  \"logging\": {\"max_entries\": 100, \"log_file\": \".kujo-agent/mcp-calls.log\"},\n  \"auth\": {\"enabled\": false, \"type\": \"bearer\", \"token\": \"\"}\n}\n")?;
        write(root, "src/integrations/mcp_read.kujo", "from src.core.framework import safe_read\n\nlet result := safe_read(\"README.md\", \".\", [\"*.md\"])\nprint(to_json(result))\n")?;
        write(root, "src/integrations/mcp_adapter.kujo", "from src.agents.integrations.adapters import map_mcp_2026_tool_to_registry_entry\n\nlet mapped := map_mcp_2026_tool_to_registry_entry({\n  \"name\": \"read_project_docs\",\n  \"description\": \"Read allowlisted project documentation\",\n  \"inputSchema\": {\"type\": \"object\", \"properties\": {\"file_name\": {\"type\": \"string\"}}, \"required\": [\"file_name\"]},\n  \"permissions\": [\"fs-read:project\"],\n  \"risk_level\": \"low\",\n  \"timeout_ms\": 30000,\n  \"approval_required\": false\n}, {\"provider_name\": \"mcp\"})\nprint(to_json(mapped))\n")?;
    }
    if has("knowledge") {
        write(
            root,
            "agent/knowledge/example.md",
            "# Owned knowledge\n\nKujo Agent Projects keep intelligence configuration in Git.\n",
        )?;
        write(root, "config/retrieval.json", "{\"provider\":\"kujo-rag\",\"embedding\":\"offline-hash\",\"namespace\":\"owned-agent\"}\n")?;
        write(root, "src/integrations/retrieval_adapter.kujo", "from src.agents.retrieval.provider import create_retrieval_result\n\nlet argv := args()\nlet raw := parse_json(argv[0])\nmut documents := []\nmut citations := []\nmut index := 0\nfor citation in raw[\"citations\"] {\n  let document_id := citation[\"doc_id\"]\n  documents = push(documents, {\n    \"document_id\": document_id,\n    \"title\": citation[\"path\"],\n    \"content\": citation[\"snippet\"],\n    \"score\": citation[\"score\"],\n    \"source\": citation[\"path\"]\n  })\n  citations = push(citations, {\n    \"citation_id\": \"rag-\" + to_string(index + 1),\n    \"document_id\": document_id,\n    \"span\": citation[\"path\"] + \":\" + to_string(citation[\"line_start\"]) + \"-\" + to_string(citation[\"line_end\"]),\n    \"score\": citation[\"score\"]\n  })\n  index = index + 1\n}\nlet result := create_retrieval_result({\n  \"ok\": true,\n  \"query\": {\"query_text\": raw[\"query\"], \"top_k\": 5},\n  \"context\": {\n    \"context_id\": \"rag-owned-agent\",\n    \"documents\": documents,\n    \"summary\": raw[\"answer\"],\n    \"citations\": citations,\n    \"metadata\": {\"provider\": \"kujo-rag\", \"namespace\": raw[\"namespace\"]}\n  },\n  \"count\": len(documents),\n  \"metadata\": {\"adapter\": \"rag-agents-sdk\"}\n})\nprint(to_json(result))\n")?;
    }
    if has("workflow") {
        write(
            root,
            "workflows/default.json",
            "{\n  \"id\": \"owned-agent-workflow\",\n  \"name\": \"Owned Agent Workflow\",\n  \"description\": \"Deterministic project-owned Dispatch workflow.\",\n  \"input_schema\": {\"type\": \"dict\", \"required\": [\"topic\"]},\n  \"agents\": {\n    \"planner\": {\n      \"id\": \"planner\",\n      \"name\": \"Planner\",\n      \"role\": \"planner\",\n      \"purpose\": \"Plan the requested work.\",\n      \"instructions\": \"Create a concise plan.\",\n      \"handler_id\": \"planner\",\n      \"execution_contract\": {\"id\": \"owned-agent-planner\", \"version\": \"1\"},\n      \"capabilities\": {\"plan_research\": true},\n      \"model\": {\"provider\": \"fixture\", \"model\": \"dispatch-starter\"},\n      \"model_candidates\": [],\n      \"tools\": [],\n      \"handoff_targets\": [],\n      \"output_schema\": {\"type\": \"dict\"},\n      \"guardrails\": [],\n      \"uses_model\": true\n    }\n  },\n  \"tools\": [],\n  \"steps\": [\n    {\"id\": \"plan\", \"name\": \"Plan\", \"type\": \"agent\", \"agent_id\": \"planner\", \"input_from\": [\"input\"], \"output_key\": \"plan\"},\n    {\"id\": \"finalize\", \"name\": \"Finalize\", \"type\": \"report\", \"input_from\": [\"plan\"], \"output_key\": \"final_output\"}\n  ],\n  \"memory_config\": {},\n  \"default_retry_policy\": {\"max_attempts\": 1, \"base_delay_ms\": 0, \"max_delay_ms\": 0, \"strategy\": \"none\", \"retry_on_codes\": []},\n  \"approval_config\": {\"auto_approve\": true},\n  \"output_schema\": {\"type\": \"dict\"}\n}\n",
        )?;
    }
    if has("hardened") {
        write(root, "workcell.json", "{\n  \"version\": 1,\n  \"name\": \"owned-agent-hardened\",\n  \"runtime\": {\n    \"backend\": \"docker\",\n    \"image\": \"kujo-owned-agent-workcell:local\",\n    \"build_context\": \"workcell-image\"\n  },\n  \"workspace\": {\n    \"strategy\": \"git-worktree\",\n    \"mount_path\": \"/workspace\",\n    \"run_as\": \"host\"\n  },\n  \"command\": [\"/usr/bin/env\", \"KUJO_MODULE_PATH=/opt/agents-sdk\", \"kujo\", \"run\", \"--untrusted\", \"--allow-fs-read\", \"--allow-clock\", \"src/main.kujo\", \"--\", \"workcell-fixture\"],\n  \"environment\": {\"allow\": [], \"set\": {}},\n  \"secrets\": [],\n  \"resources\": {\n    \"cpus\": 1,\n    \"memory\": \"512m\",\n    \"pids\": 64,\n    \"timeout_ms\": 60000,\n    \"max_output_bytes\": 1000000\n  },\n  \"network\": {\n    \"mode\": \"none\",\n    \"egress\": {\n      \"policy\": \"deny-by-default\",\n      \"dns\": \"blocked\",\n      \"proxy\": \"none\",\n      \"enforcement_profile\": \"none\"\n    }\n  },\n  \"filesystem\": {\"read_only_root\": true, \"tmpfs\": [\"/tmp\"]},\n  \"artifacts\": {\"export\": []},\n  \"cleanup\": {\"keep_failed\": false},\n  \"trust_profile\": \"contained-standard\",\n  \"receipt\": {\"path\": \".workcell/runs\"}\n}\n")?;
        write(root, "workcell-image/Dockerfile", "FROM kujolang/workcell-kujo:local\nUSER root\nCOPY agents-sdk /opt/agents-sdk\nRUN chown -R 65532:65532 /opt/agents-sdk\nWORKDIR /workspace\nUSER 65532:65532\nCMD [\"kujo\", \"--version\"]\n")?;
    }
    if has("observable") {
        write(root, "config/observability.json", "{\n  \"watchdog\": {\n    \"enabled\": true,\n    \"endpoint\": \"http://127.0.0.1:8789\",\n    \"required_for_fixture\": false,\n    \"route_live_provider_calls\": true\n  },\n  \"runledger\": {\n    \"enabled\": true,\n    \"path\": \".runledger\"\n  }\n}\n")?;
        write(root, "src/integrations/watchdog_trace.kujo", "from src.agents.integrations.adapters import create_watchdog_trace_adapter, watchdog_transform_trace_event\n\nlet adapter := create_watchdog_trace_adapter({\"metadata\": {\"target\": \"watchdog\"}})\nlet result := watchdog_transform_trace_event(adapter, {\n  \"trace_id\": \"trace-owned-agent\",\n  \"run_id\": \"run-owned-agent\",\n  \"agent_id\": \"owned-agent\",\n  \"event_kind\": \"fixture_run_prepared\",\n  \"timestamp\": \"\",\n  \"sequence\": 1,\n  \"payload\": {\"fixture\": true}\n}, {})\nprint(to_json(result))\n")?;
    }
    if a.profile == "full" {
        write(root, "config/relay.json", "{\"enabled\":true,\"adapter\":\"agent-project\"}\n")?;
    }
    Ok(())
}

fn kennel_manifest(profile: &str) -> String {
    let has = |name: &str| profile == name || profile == "full";
    let mut dependencies = vec![
        ("ai-sdk", "github:kujolang/ai-sdk", "be9617a32344728919b1394b80f72f46559d69a7"),
        ("agents-sdk", "github:kujolang/agents-sdk", "d3904d348754b492bda298b6c30f49c1eb24b7ea"),
        ("eval", "github:kujolang/eval", "955713f487c094b20b7b8c44414ae17395194cc9"),
    ];
    if has("tools") {
        dependencies.push((
            "mcp",
            "github:kujolang/mcp",
            "2ab8111f2c5174841204f5c762d8ce8d281e57b6",
        ));
    }
    if has("knowledge") {
        dependencies.push((
            "rag",
            "github:kujolang/rag",
            "28690e3aa1b7a5947616843574cacf03b32905c9",
        ));
    }
    if has("workflow") {
        dependencies.push((
            "dispatch",
            "github:kujolang/dispatch",
            "662417c264bd55f8d802eef3fc21f9f372590753",
        ));
    }
    if has("hardened") {
        dependencies.push((
            "workcell",
            "github:kujolang/workcell",
            "7bcdb7f29ddf74843aec6b70eafbf33cc7944c6f",
        ));
    }
    if has("observable") {}
    if profile == "full" {
        dependencies.push((
            "relay",
            "github:kujolang/relay",
            "0480733735a69f3b01d5452e6c86b4df3343c9d6",
        ));
    }
    let mut out = String::from("[package]\nname = \"owned-agent-project\"\nversion = \"0.1.0\"\n\n[kujo]\nminimum_version = \"1.0.2\"\nentry = \"src/main.kujo\"\nsources = [\".\"]\nexcludes = [\".git\", \"kennel_packages\", \".kennel_tmp\"]\n\n[dependencies]\n");
    for (name, source, commit) in dependencies {
        out.push_str(&format!("{name} = {{ source = \"{source}\", commit = \"{commit}\" }}\n"));
    }
    out
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
    unresolved.sort();
    let credentials = credential_names(root);
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
            "Kujo Agent Project: {}\nContract: {}\nProfile: {}\nEntrypoint: {}\nProvider: {}\nModel: {}\nSkills: {}\nTools: {}\nKnowledge: {}\nWorkflow: {}\nEval: {} via Eval\nIntegrations: {}\nWorkcell: {}\nCapabilities: {}\nDependencies unresolved: {}\nExternal state:\n  Credentials: {}\n  Container runtime: {}\n  Watchdog: {}\n  MCP endpoints: {}",
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
        .env("KUJO_MODULE_PATH", &ai_sdk)
        .current_dir(root)
        .output()
        .map_err(|e| ioerr(format!("Failed to launch AI SDK live provider bridge: {e}")))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
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
    serde_json::to_string(&normalized).map_err(|e| ioerr(e.to_string()))
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
