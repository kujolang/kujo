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
}

#[derive(Args, Debug)]
pub struct ProjectArgs {
    #[arg(long)]
    pub json: bool,
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
    name: String,
    profile: String,
    agent: AgentPaths,
    runtime: RuntimeConfig,
    integrations: BTreeMap<String, bool>,
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
    let base = a.dir.clone().unwrap_or(std::env::current_dir().map_err(|e| ioerr(e.to_string()))?);
    if base.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(usage("Agent destination may not contain '..'."));
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
    let has = |p: &str| a.profile == p || a.profile == "full";
    let mut integrations = BTreeMap::new();
    for key in ["mcp", "retrieval", "dispatch", "relay", "workcell", "watchdog", "runledger"] {
        integrations.insert(key.into(), false);
    }
    if has("tools") {
        integrations.insert("mcp".into(), true);
    }
    if has("knowledge") {
        integrations.insert("retrieval".into(), true);
    }
    if has("workflow") {
        integrations.insert("dispatch".into(), true);
    }
    if has("hardened") {
        integrations.insert("workcell".into(), true);
    }
    if has("observable") {
        integrations.insert("watchdog".into(), true);
        integrations.insert("runledger".into(), true);
    }
    if a.profile == "full" {
        integrations.insert("relay".into(), true);
    }
    let capabilities = if has("hardened") {
        vec!["fs-read:project".into(), "ai:configured-endpoints".into()]
    } else {
        vec!["fs-read:project".into()]
    };
    let manifest = ProjectManifest {
        contract: CONTRACT.into(),
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
    };
    write(
        root,
        "agent.project.json",
        &format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
    )?;
    write(root, "config/model.json", &format!("{{\n  \"contract\": \"kujo-ai-sdk/model-preference/v1\",\n  \"provider\": \"{}\",\n  \"model\": \"{}\",\n  \"mode\": \"{}\"\n}}\n", a.provider, a.model, if a.provider == "fixture" {"fixture"} else {"live"}))?;
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
    write(root, "src/main.kujo", "from src.agents.testing.no_network import create_no_network_harness\nfrom src.agents.runner import create_agent_runner, run_agent\nfrom src.agents.core_types import create_agent, create_agent_run_request\n\nlet argv := args()\nmut prompt := \"Hello\"\nif len(argv) > 0 { prompt = argv[0] }\nlet harness := create_no_network_harness({\"model\": {\"output_text\": \"Owned agent fixture: \" + prompt}})\nif harness[\"ok\"] == false {\n    print(to_json(harness))\n} else {\n    let runner := create_agent_runner({\"ai_adapter\": harness[\"model_adapter\"]})\n    let agent := create_agent({\"id\": \"owned-agent\", \"name\": \"Owned Agent\", \"instructions\": \"Follow agent/instructions.md and the repository policy.\"})\n    let request := create_agent_run_request(prompt, {\"run_id\": \"run-owned-fixture\", \"session_id\": \"session-owned-fixture\"})\n    let result := run_agent(runner, agent, request, {\"tool_registry\": harness[\"tool_registry\"]})\n    print(result[\"output\"][\"text\"])\n}\n")?;
    write(root, "evals/eval.json", "{\n  \"name\": \"owned-agent-fixture\",\n  \"description\": \"Deterministic Agents SDK fixture evaluation\",\n  \"version\": \"1.0.0\",\n  \"output_dir\": \".eval-results\",\n  \"stop_on_failure\": true,\n  \"tests\": [\n    {\n      \"name\": \"fixture response\",\n      \"check\": \"output_contains\",\n      \"params\": {\n        \"command\": \"kujo agent run fixture-check\",\n        \"expected\": \"Owned agent fixture: fixture-check\"\n      }\n    }\n  ]\n}\n")?;
    write(root, "kujo.toml", &format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\nentrypoint = \"src/main.kujo\"\n\n[dependencies]\n", a.name))?;
    write(root, "kujo.lock", "version = 1\npackages = []\n")?;
    write(root, "kennel.toml", &kennel_manifest(&a.profile))?;
    write(
        root,
        ".env.example",
        if a.provider == "fixture" {
            "# Fixture mode requires no credentials.\n"
        } else {
            "OPENAI_API_KEY=\n"
        },
    )?;
    write(
        root,
        ".gitignore",
        ".env\n.kennel_tmp/\nkennel_packages/\n.eval-results/\n.runledger/\n",
    )?;
    write(root, "AGENTS.md", "# Agent Project Guide\n\nTreat `agent.project.json` as the root contract. Never commit credentials. Run `kujo doctor agent`, `kujo agent inspect`, `kujo agent run`, and `kujo agent eval`.\n")?;
    write(root, "README.md", &format!("# {}\n\nThis Git repository owns the agent definition, instructions, model preference, skills, tools, knowledge, policies, workflows, evals, and execution boundaries.\n\n```bash\nkennel install\nkujo doctor agent\nkujo agent inspect\nkujo agent run \"Hello\"\nkujo agent eval\n```\n\nFixture mode uses the Agents SDK no-network harness and requires no provider credentials. Change `config/model.json` for a live AI SDK provider and provide credentials through the environment only. Kujo capabilities authorize effects; they are not a sandbox. Hardened projects use Workcell for container-backed isolation.\n", a.name))?;
    if has("tools") {
        write(root, "agent/tools/read-project.json", "{\"name\":\"read_project_docs\",\"description\":\"Read allowlisted project documentation\",\"risk\":\"read_only\",\"approval\":\"never\"}\n")?;
        write(root, "config/mcp.json", "{\"contract\":\"kujo-mcp/project/v1\",\"servers\":[]}\n")?;
    }
    if has("knowledge") {
        write(
            root,
            "agent/knowledge/example.md",
            "# Owned knowledge\n\nKujo Agent Projects keep intelligence configuration in Git.\n",
        )?;
        write(root, "config/retrieval.json", "{\"provider\":\"kujo-rag\",\"embedding\":\"offline-hash\",\"namespace\":\"owned-agent\"}\n")?;
    }
    if has("workflow") {
        write(
            root,
            "workflows/default.json",
            "{\"engine\":\"dispatch\",\"steps\":[\"plan\",\"act\",\"verify\"],\"receipts\":true}\n",
        )?;
    }
    if has("hardened") {
        write(root, "workcell.json", "{\"contract\":\"kujo-workcell/v1\",\"read_only_root\":true,\"memory_mb\":512,\"cpus\":1,\"pids\":64,\"timeout_seconds\":60,\"writable_paths\":[\"/artifacts\"],\"network\":\"deny\"}\n")?;
    }
    if has("observable") {
        write(root, "config/observability.json", "{\"watchdog\":{\"enabled\":false},\"runledger\":{\"enabled\":true,\"path\":\".runledger\"}}\n")?;
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
    if has("observable") {
        dependencies.push((
            "watchdog",
            "github:kujolang/watchdog",
            "1af292b3e03217760649dcb4f903e443f48c563c",
        ));
        dependencies.push((
            "runledger",
            "github:kujolang/runledger",
            "12bbf2b3723325913eb75ececaba0ce3fdc68b87",
        ));
    }
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
    let m: ProjectManifest = serde_json::from_str(&text)
        .map_err(|e| usage(format!("Malformed Agent Project manifest: {e}")))?;
    if m.contract != CONTRACT {
        return Err(usage(format!("Unsupported Agent Project contract '{}'.", m.contract)));
    }
    for rel in [
        &m.agent.definition,
        &m.agent.manifest,
        &m.agent.instructions,
        &m.runtime.entrypoint,
        &m.runtime.provider_config,
    ] {
        let p = root.join(rel);
        let c =
            p.canonicalize().map_err(|_| usage(format!("Referenced file is missing: {rel}")))?;
        if !c.starts_with(root) {
            return Err(usage(format!("Agent Project path escapes the project: {rel}")));
        }
    }
    Ok(m)
}
fn inspection(root: &Path, m: &ProjectManifest) -> Value {
    let model: Value = fs::read_to_string(root.join(&m.runtime.provider_config))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(json!({}));
    let skills = list_names(&root.join(&m.agent.skills));
    let tools = list_names(&root.join(&m.agent.tools));
    let unresolved = if root.join("kennel_packages/agents-sdk/src/agents/runner.kujo").is_file() {
        Vec::<String>::new()
    } else {
        vec!["agents-sdk: run `kennel install`".to_string()]
    };
    json!({"contract":INSPECT_CONTRACT,"project":{"name":m.name,"profile":m.profile,"root":root},"agent":{"definition":m.agent.definition,"instructions":m.agent.instructions,"skills":skills,"tools":tools},"runtime":{"entrypoint":m.runtime.entrypoint,"provider":model.get("provider"),"model":model.get("model"),"fixture":m.runtime.fixture},"integrations":m.integrations,"workcell":m.runtime.workcell,"capabilities":m.runtime.capabilities,"unresolved_dependencies":unresolved,"external_state":{"credential_names":if m.runtime.fixture {Vec::<String>::new()} else {vec!["OPENAI_API_KEY".to_string()]},"container_runtime":if m.runtime.workcell.is_some(){"Docker or Podman required"}else{"not required"}}})
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
        println!("Kujo Agent Project: {}\nContract: {}\nProfile: {}\nEntrypoint: {}\nProvider: {}\nModel: {}\nSkills: {}\nTools: {}\nWorkcell: {}\nCapabilities: {}",m.name,m.contract,m.profile,m.runtime.entrypoint,v["runtime"]["provider"].as_str().unwrap_or("unknown"),v["runtime"]["model"].as_str().unwrap_or("unknown"),v["agent"]["skills"].as_array().map(|a|a.len()).unwrap_or(0),v["agent"]["tools"].as_array().map(|a|a.len()).unwrap_or(0),m.runtime.workcell.as_deref().unwrap_or("disabled"),m.runtime.capabilities.join(", "));
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
fn run(a: RunArgs) -> Result<(), AgentError> {
    let cwd = std::env::current_dir().map_err(|e| ioerr(e.to_string()))?;
    let root = discover(&cwd)?;
    let m = load(&root)?;
    let prompt = prompt_from(&a, &root)?;
    let agents_sdk = root.join("kennel_packages/agents-sdk");
    if !agents_sdk.join("src/agents/runner.kujo").is_file() {
        return Err(fail(
            "Agents SDK is not installed. Run `kennel install` from the Agent Project root.",
        ));
    }
    let exe = std::env::current_exe().map_err(|e| ioerr(e.to_string()))?;
    let mut command = Command::new(exe);
    command.env("KUJO_MODULE_PATH", &agents_sdk);
    let out = command
        .arg("run")
        .arg(&m.runtime.entrypoint)
        .arg("--")
        .arg(&prompt)
        .current_dir(&root)
        .output()
        .map_err(|e| ioerr(e.to_string()))?;
    if !out.status.success() {
        return Err(fail(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if a.json {
        println!("{}",serde_json::to_string_pretty(&json!({"contract":RUN_CONTRACT,"status":"ok","project":m.name,"provider_mode":if m.runtime.fixture{"fixture"}else{"live"},"output":text})).unwrap());
    } else {
        println!("{text}");
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
            let sdk_installed =
                root.join("kennel_packages/agents-sdk/src/agents/runner.kujo").is_file();
            checks.push(check(
                "agent.dependencies.agents-sdk",
                "Agents SDK installed",
                if sdk_installed { CheckStatus::Pass } else { CheckStatus::Fail },
                if sdk_installed { CheckSeverity::Info } else { CheckSeverity::High },
                Some(if sdk_installed { "installed".into() } else { "missing".into() }),
                if sdk_installed {
                    None
                } else {
                    Some("Run `kennel install` from the Agent Project root.".into())
                },
            ));
            if deep {
                checks.push(check(
                    "agent.dependencies",
                    "Pinned ecosystem dependencies",
                    if fs::read_to_string(root.join("kennel.toml"))
                        .map(|s| s.contains("commit ="))
                        .unwrap_or(false)
                    {
                        CheckStatus::Pass
                    } else {
                        CheckStatus::Warn
                    },
                    CheckSeverity::Medium,
                    None,
                    Some("Use immutable git revisions in kennel.toml.".into()),
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
