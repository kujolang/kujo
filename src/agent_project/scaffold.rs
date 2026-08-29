use super::*;

pub(super) fn validate_name(name: &str) -> Result<(), AgentError> {
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

pub(super) fn scaffold(mut a: NewArgs) -> Result<(), AgentError> {
    validate_name(&a.name)?;
    if !PROFILES.contains(&a.profile.as_str()) {
        return Err(usage(format!(
            "Unknown agent profile '{}'. Expected one of: {}.",
            a.profile,
            PROFILES.join(", ")
        )));
    }
    if a.provider == "auto" {
        a.provider = credentials::preferred_provider(a.json)?;
    }
    provider_settings(&a.provider).ok_or_else(|| {
        usage(format!(
            "Unknown provider '{}'. Expected one of: fixture, openai, openrouter, deepseek, custom.",
            a.provider
        ))
    })?;
    if a.model == "auto" {
        a.model = default_model(&a.provider).to_string();
    }
    credentials::configure_new_project_credential(&a)?;
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
    let provider = provider_settings(&a.provider).unwrap();
    let credential_ready = provider.mode == "fixture"
        || credentials::resolve_for_project(&target, provider.api_key_env)?.is_some();
    let payload = json!({"contract":"kujo-agent-new/v1","status":"created","project":a.name,"profile":a.profile,"path":target,"git":!a.no_git,"provider":a.provider,"credential_ready":credential_ready});
    if a.json {
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else {
        println!("Created Kujo Agent Project '{}' ({}) at {}", a.name, a.profile, target.display());
        println!(
            "Next: cd {} && kennel install && kujo doctor agent && kujo agent inspect",
            target.display()
        );
        if provider.mode == "live" && !credential_ready {
            println!(
                "Credential pending: run `kujo agent auth set {}` before the first live request.",
                a.provider
            );
        }
    }
    Ok(())
}

fn default_model(provider: &str) -> &'static str {
    match provider {
        "fixture" => "fixture-owned-agent-v1",
        "openai" => "gpt-5-mini",
        "openrouter" => "openai/gpt-5-mini",
        "deepseek" => "deepseek-chat",
        "custom" => "custom-model",
        _ => "fixture-owned-agent-v1",
    }
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
        include_str!("../../schemas/agent-project.schema.json"),
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
        ".env\n.env.local\n.kennel_tmp/\nkennel_packages/\n.eval-results/\n.runledger/\n.workcell/\n.relay/\n.dispatch-runs/\n.kujo-agent/\ndata/\nresults/\nworkcell-image/agents-sdk/\n",
    )?;
    write(root, "AGENTS.md", "# Agent Project Guide\n\nTreat `agent.project.json` as the root contract. Never commit credentials. Run `kujo doctor agent`, `kujo agent inspect`, `kujo agent run`, and `kujo agent eval`.\n")?;
    write(root, "README.md", &format!("# {}\n\nThis Git repository owns the agent definition, instructions, model preference, skills, tools, knowledge, policies, workflows, evals, and execution boundaries.\n\n```bash\nkennel install\nkujo doctor agent\nkujo agent inspect\nkujo agent run \"Hello\"\nkujo agent eval\n```\n\nFixture mode uses the Agents SDK no-network harness and requires no provider credentials. For live providers, store a reusable credential in the operating-system credential store with `kujo agent auth set <provider>`. Use `--project` only when the agent needs its own credential; Kujo writes it to the ignored, owner-only `.env.local`. Environment variables remain supported for CI. Kujo capabilities authorize effects; they are not a sandbox. Hardened projects use Workcell for container-backed isolation.\n", a.name))?;
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
    let mut out = String::from("[package]\nname = \"owned-agent-project\"\nversion = \"0.1.0\"\n\n[kujo]\nminimum_version = \"1.1.0\"\nentry = \"src/main.kujo\"\nsources = [\".\"]\nexcludes = [\".git\", \"kennel_packages\", \".kennel_tmp\"]\n\n[dependencies]\n");
    for (name, source, commit) in dependencies {
        out.push_str(&format!("{name} = {{ source = \"{source}\", commit = \"{commit}\" }}\n"));
    }
    out
}
