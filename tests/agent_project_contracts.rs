use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp() -> PathBuf {
    let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let p = std::env::temp_dir().join(format!("kujo-agent-{n}"));
    fs::create_dir_all(&p).unwrap();
    p
}
fn run(args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kujo")).args(args).current_dir(cwd).output().unwrap()
}

fn run_with_stdin(
    args: &[&str],
    cwd: &Path,
    stdin: &str,
    environment: &[(&str, &Path)],
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in environment {
        command.env(name, value);
    }
    let mut child = command.spawn().unwrap();
    child.stdin.as_mut().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn all_profiles_scaffold_deterministically_and_validate() {
    for profile in ["basic", "tools", "knowledge", "workflow", "hardened", "observable", "full"] {
        let root = temp();
        let o = run(
            &[
                "agent",
                "new",
                "owned",
                "--profile",
                profile,
                "--dir",
                root.to_str().unwrap(),
                "--no-git",
                "--json",
            ],
            &root,
        );
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let project = root.join("owned");
        let m: Value =
            serde_json::from_str(&fs::read_to_string(project.join("agent.project.json")).unwrap())
                .unwrap();
        assert_eq!(m["contract"], "kujo-agent-project/v1");
        assert_eq!(m["profile"], profile);
        assert_generated_shape(&project, profile);
        install_project_fixtures(&project);
        let commands: &[&[&str]] = &[
            &["agent", "inspect", "--json"],
            &["doctor", "agent", "--json"],
            &["agent", "run", "Hello", "--json"],
            &["agent", "eval", "--json"],
        ];
        for args in commands {
            let out = run(args, &project);
            assert!(out.status.success(), "{profile}: {}", String::from_utf8_lossy(&out.stderr));
            serde_json::from_slice::<Value>(&out.stdout).unwrap();
        }
    }
}

fn assert_generated_shape(project: &Path, profile: &str) {
    for required in [
        "agent.project.json",
        "schemas/agent-project.schema.json",
        "agent/manifest.json",
        "agent/instructions.md",
        "agent/input.schema.json",
        "agent/output.schema.json",
        "agent/skills/owned-agent/SKILL.md",
        "agent/policies/capabilities.json",
        "config/model.json",
        "src/main.kujo",
        "src/live_model.kujo",
        "evals/eval.json",
        "kennel.toml",
        "kujo.toml",
        "README.md",
        "AGENTS.md",
        ".env.example",
        ".gitignore",
    ] {
        assert!(project.join(required).is_file(), "{profile}: missing {required}");
    }
    let feature = |name: &str| profile == name || profile == "full";
    for (path, expected) in [
        ("config/mcp.json", feature("tools")),
        ("mcp-server.json", feature("tools")),
        ("config/retrieval.json", feature("knowledge")),
        ("agent/knowledge/example.md", feature("knowledge")),
        ("workflows/default.json", feature("workflow")),
        ("workcell.json", feature("hardened")),
        ("config/observability.json", feature("observable")),
        ("config/relay.json", profile == "full"),
    ] {
        assert_eq!(
            project.join(path).is_file(),
            expected,
            "{profile}: unexpected generated shape for {path}"
        );
    }
    for json_path in [
        "agent.project.json",
        "schemas/agent-project.schema.json",
        "agent/manifest.json",
        "agent/input.schema.json",
        "agent/output.schema.json",
        "agent/policies/capabilities.json",
        "config/model.json",
        "evals/eval.json",
    ] {
        serde_json::from_str::<Value>(&fs::read_to_string(project.join(json_path)).unwrap())
            .unwrap_or_else(|error| panic!("{profile}: invalid {json_path}: {error}"));
    }
}

#[test]
fn rejects_unsafe_and_conflicting_scaffolds() {
    let root = temp();
    fs::create_dir(root.join("busy")).unwrap();
    fs::write(root.join("busy/file"), "x").unwrap();
    assert_eq!(
        run(&["agent", "new", "busy", "--dir", root.to_str().unwrap()], &root).status.code(),
        Some(2)
    );
    assert_eq!(
        run(&["agent", "new", "x", "--profile", "unknown", "--dir", root.to_str().unwrap()], &root)
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        run(&["agent", "new", "../escape", "--dir", root.to_str().unwrap()], &root).status.code(),
        Some(2)
    );
    assert_eq!(run(&["agent", "doctor"], &root).status.code(), Some(2));
    let json_error = run(
        &["agent", "new", "x", "--profile", "unknown", "--dir", root.to_str().unwrap(), "--json"],
        &root,
    );
    assert_eq!(json_error.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&json_error.stderr).unwrap();
    assert_eq!(error["contract"], "kujo-agent-error/v1");
    assert_eq!(error["status"], "error");
    assert_eq!(
        run(&["agent", "new", "root-target", "--dir", "/", "--no-git"], &root).status.code(),
        Some(2)
    );
}

#[test]
fn reusable_credentials_are_stored_without_shell_arguments_or_output_leaks() {
    let root = temp();
    let store = root.join("credentials.json");
    let output = run_with_stdin(
        &["agent", "auth", "set", "openai", "--from-stdin", "--json"],
        &root,
        "sk-test-reusable\n",
        &[("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)],
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("sk-test-reusable"));

    let status = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["agent", "auth", "status", "openai", "--json"])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(status.status.success());
    let payload: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(payload["status"], "configured");
    assert_eq!(payload["source"], "OS credential store");
    assert!(!String::from_utf8_lossy(&status.stdout).contains("sk-test-reusable"));

    let scaffold = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args([
            "agent",
            "new",
            "live",
            "--provider",
            "openai",
            "--dir",
            root.to_str().unwrap(),
            "--no-git",
            "--json",
        ])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(scaffold.status.success(), "{}", String::from_utf8_lossy(&scaffold.stderr));
    let created: Value = serde_json::from_slice(&scaffold.stdout).unwrap();
    assert_eq!(created["credential_ready"], true);
    assert_eq!(created["provider"], "openai");
    assert!(!String::from_utf8_lossy(&scaffold.stdout).contains("sk-test-reusable"));
    let inspected = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["agent", "inspect", "--json"])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .current_dir(root.join("live"))
        .output()
        .unwrap();
    assert!(inspected.status.success());
    let inspection: Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(inspection["external_state"]["credential"]["configured"], true);
    assert_eq!(inspection["external_state"]["credential"]["source"], "OS credential store");
    assert!(!String::from_utf8_lossy(&inspected.stdout).contains("sk-test-reusable"));

    let automatic = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["agent", "new", "auto-live", "--dir", root.to_str().unwrap(), "--no-git"])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(automatic.status.success());
    let automatic_model: Value = serde_json::from_str(
        &fs::read_to_string(root.join("auto-live/config/model.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(automatic_model["provider"], "openai");
    assert_eq!(automatic_model["model"], "gpt-5-mini");

    let removed = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["agent", "auth", "remove", "openai", "--json"])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(removed.status.success());
    assert_eq!(serde_json::from_slice::<Value>(&removed.stdout).unwrap()["status"], "removed");
}

#[test]
fn connector_api_keys_use_the_same_redacted_credential_contract() {
    let root = temp();
    let store = root.join("connector-credentials.json");
    let saved = run_with_stdin(
        &["agent", "auth", "set", "--name", "LINEAR_API_TOKEN", "--from-stdin", "--json"],
        &root,
        "linear-secret-value\n",
        &[("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)],
    );
    assert!(saved.status.success());
    assert!(!String::from_utf8_lossy(&saved.stdout).contains("linear-secret-value"));
    let status = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["agent", "auth", "status", "--name", "LINEAR_API_TOKEN", "--json"])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .current_dir(&root)
        .output()
        .unwrap();
    let payload: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(payload["status"], "configured");
    assert_eq!(payload["credential_name"], "LINEAR_API_TOKEN");
    assert!(!String::from_utf8_lossy(&status.stdout).contains("linear-secret-value"));
}

#[test]
fn project_credentials_are_private_ignored_and_discoverable_from_subdirectories() {
    let project = scaffold("basic");
    let output = run_with_stdin(
        &["agent", "auth", "set", "openai", "--project", "--from-stdin"],
        &project,
        "sk-project-only\n",
        &[],
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let local = project.join(".env.local");
    assert!(local.is_file());
    assert!(fs::read_to_string(&local).unwrap().contains("OPENAI_API_KEY=sk-project-only"));
    assert!(fs::read_to_string(project.join(".gitignore"))
        .unwrap()
        .lines()
        .any(|line| line == ".env.local"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&local).unwrap().permissions().mode() & 0o777, 0o600);
    }
    let nested = project.join("agent/skills");
    let status = run(&["agent", "auth", "status", "openai", "--project", "--json"], &nested);
    assert!(status.status.success());
    let payload: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(payload["source"], "project .env.local");
    assert!(!String::from_utf8_lossy(&status.stdout).contains("sk-project-only"));
}

#[cfg(unix)]
#[test]
fn project_credentials_fail_closed_on_unsafe_permissions_or_symlinks() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let project = scaffold("basic");
    let local = project.join(".env.local");
    fs::write(&local, "OPENAI_API_KEY=never-print-permission-secret\n").unwrap();
    fs::set_permissions(&local, fs::Permissions::from_mode(0o644)).unwrap();
    let unsafe_permissions =
        run(&["agent", "auth", "status", "openai", "--project", "--json"], &project);
    assert_eq!(unsafe_permissions.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&unsafe_permissions.stderr)
        .contains("never-print-permission-secret"));

    fs::remove_file(&local).unwrap();
    let outside = project.parent().unwrap().join("outside-credential");
    fs::write(&outside, "OPENAI_API_KEY=never-print-symlink-secret\n").unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).unwrap();
    symlink(&outside, &local).unwrap();
    let symlinked = run(&["agent", "auth", "status", "openai", "--project", "--json"], &project);
    assert_eq!(symlinked.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&symlinked.stderr).contains("never-print-symlink-secret"));
}

#[test]
fn live_scaffold_requires_an_explicit_credential_decision() {
    let root = temp();
    let store = root.join("empty-credentials.json");
    let missing = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args([
            "agent",
            "new",
            "live",
            "--provider",
            "openai",
            "--dir",
            root.to_str().unwrap(),
            "--no-git",
            "--json",
        ])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .env_remove("OPENAI_API_KEY")
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&missing.stderr).unwrap();
    assert!(error["message"].as_str().unwrap().contains("kujo agent auth set openai"));
    assert!(!root.join("live").exists());

    let config_only = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args([
            "agent",
            "new",
            "live",
            "--provider",
            "openai",
            "--dir",
            root.to_str().unwrap(),
            "--no-git",
            "--no-credential",
            "--json",
        ])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .env_remove("OPENAI_API_KEY")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(config_only.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&config_only.stdout).unwrap()["credential_ready"],
        false
    );
}

#[test]
fn rejects_invalid_contracts_paths_configs_and_secrets_without_leaking() {
    let malformed = scaffold("basic");
    fs::write(malformed.join("agent.project.json"), "{").unwrap();
    assert_eq!(run(&["agent", "inspect"], &malformed).status.code(), Some(2));

    let unsupported = scaffold("basic");
    mutate_manifest(&unsupported, |manifest| {
        manifest["contract"] = Value::String("kujo-agent-project/v999".into())
    });
    assert_eq!(run(&["agent", "inspect"], &unsupported).status.code(), Some(2));

    let missing_package = scaffold("basic");
    fs::remove_file(missing_package.join("agent/manifest.json")).unwrap();
    assert_eq!(run(&["agent", "inspect"], &missing_package).status.code(), Some(2));

    let missing_entrypoint = scaffold("basic");
    fs::remove_file(missing_entrypoint.join("src/main.kujo")).unwrap();
    assert_eq!(run(&["agent", "inspect"], &missing_entrypoint).status.code(), Some(2));

    let malformed_provider = scaffold("basic");
    fs::write(malformed_provider.join("config/model.json"), "{").unwrap();
    assert_eq!(run(&["agent", "inspect"], &malformed_provider).status.code(), Some(2));

    for field in ["tools", "skills", "knowledge"] {
        let escaped = scaffold("basic");
        fs::create_dir_all(escaped.parent().unwrap().join("outside")).unwrap();
        mutate_manifest(&escaped, |manifest| {
            manifest["agent"][field] = Value::String("../outside".into())
        });
        assert_eq!(run(&["agent", "inspect"], &escaped).status.code(), Some(2));
    }

    let invalid_workcell = scaffold("hardened");
    fs::write(invalid_workcell.join("workcell.json"), "{\"version\":1}").unwrap();
    assert_eq!(run(&["agent", "inspect"], &invalid_workcell).status.code(), Some(2));

    let invalid_mcp = scaffold("tools");
    fs::write(invalid_mcp.join("config/mcp.json"), "{").unwrap();
    assert_eq!(run(&["agent", "inspect"], &invalid_mcp).status.code(), Some(2));

    let conflict = scaffold("basic");
    fs::write(conflict.join("prompt.txt"), "prompt").unwrap();
    assert_eq!(
        run(&["agent", "run", "prompt", "--file", "prompt.txt"], &conflict).status.code(),
        Some(2)
    );
    assert_eq!(run(&["agent", "run"], &conflict).status.code(), Some(1));
    assert_eq!(run(&["agent", "unknown"], &conflict).status.code(), Some(2));

    let secret = scaffold("basic");
    install_project_fixtures(&secret);
    let secret_value = "sk-never-print-this-value";
    fs::write(secret.join(".env"), format!("OPENAI_API_KEY={secret_value}\n")).unwrap();
    let inspected = run(&["agent", "inspect", "--json"], &secret);
    assert!(inspected.status.success());
    assert!(!String::from_utf8_lossy(&inspected.stdout).contains(secret_value));
    let doctor = run(&["doctor", "agent", "--json"], &secret);
    assert!(!doctor.status.success());
    let doctor_output = format!(
        "{}{}",
        String::from_utf8_lossy(&doctor.stdout),
        String::from_utf8_lossy(&doctor.stderr)
    );
    assert!(!doctor_output.contains(secret_value));
    assert!(doctor_output.contains(".env"));
}

#[cfg(unix)]
#[test]
fn rejects_symlink_destination() {
    use std::os::unix::fs::symlink;
    let root = temp();
    let outside = temp();
    symlink(&outside, root.join("linked")).unwrap();
    assert_eq!(
        run(&["agent", "new", "linked", "--dir", root.to_str().unwrap()], &root).status.code(),
        Some(2)
    );
}

#[test]
fn project_is_portable_without_sibling_repositories() {
    let root = temp();
    assert!(run(&["agent", "new", "portable", "--dir", root.to_str().unwrap(), "--no-git"], &root)
        .status
        .success());
    install_project_fixtures(&root.join("portable"));
    let copy = temp().join("restored");
    fs::create_dir_all(&copy).unwrap();
    for entry in fs::read_dir(root.join("portable")).unwrap() {
        let e = entry.unwrap();
        let dest = copy.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            fs::copy(e.path(), dest).unwrap();
        }
    }
    let commands: &[&[&str]] = &[
        &["doctor", "agent"],
        &["agent", "inspect"],
        &["agent", "run", "portable"],
        &["agent", "eval"],
    ];
    for args in commands {
        assert!(run(args, &copy).status.success());
    }
}

#[test]
fn live_custom_provider_flows_through_ai_sdk_and_agents_sdk() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 16_384];
        let _ = stream.read(&mut request).unwrap();
        let body = r#"{"id":"live-proof","model":"mock-model","choices":[{"message":{"content":"live bridge verified"},"finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });

    let project = scaffold("basic");
    install_project_fixtures(&project);
    let config_path = project.join("config/model.json");
    let mut config: Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    config["provider"] = Value::String("custom".into());
    config["mode"] = Value::String("live".into());
    config["model"] = Value::String("mock-model".into());
    config["api_key_env"] = Value::String("CUSTOM_API_KEY".into());
    config["base_url"] = Value::String(format!("http://127.0.0.1:{port}/v1"));
    config["allow_insecure_localhost"] = Value::Bool(true);
    fs::write(&config_path, format!("{}\n", serde_json::to_string_pretty(&config).unwrap()))
        .unwrap();
    let store = project.parent().unwrap().join("credential-test-store.json");
    let saved = run_with_stdin(
        &["agent", "auth", "set", "custom", "--from-stdin", "--json"],
        &project,
        "not-a-real-secret\n",
        &[("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)],
    );
    assert!(saved.status.success());
    let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["agent", "run", "live", "--json"])
        .env("KUJO_AGENT_TEST_CREDENTIAL_STORE", &store)
        .env_remove("CUSTOM_API_KEY")
        .current_dir(&project)
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["provider_mode"], "live");
    assert_eq!(payload["output"], "live bridge verified");
}
fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let d = dst.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &d);
        } else {
            fs::copy(e.path(), d).unwrap();
        }
    }
}

fn scaffold(profile: &str) -> PathBuf {
    let root = temp();
    let output = run(
        &[
            "agent",
            "new",
            "owned",
            "--profile",
            profile,
            "--dir",
            root.to_str().unwrap(),
            "--no-git",
        ],
        &root,
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    root.join("owned")
}

fn mutate_manifest(project: &Path, mutate: impl FnOnce(&mut Value)) {
    let path = project.join("agent.project.json");
    let mut manifest: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    mutate(&mut manifest);
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap())).unwrap();
}

fn install_project_fixtures(project: &Path) {
    let ecosystem = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for package in [
        "ai-sdk",
        "agents-sdk",
        "eval",
        "mcp",
        "rag",
        "dispatch",
        "workcell",
        "watchdog",
        "runledger",
        "relay",
    ] {
        fs::create_dir_all(project.join("kennel_packages").join(package)).unwrap();
    }
    copy_package_paths(ecosystem, project, "ai-sdk", &["src"]);
    let source = ecosystem.join("agents-sdk/src");
    assert!(source.is_dir(), "Agents SDK checkout is required for the integration fixture");
    copy_dir(&source, &project.join("kennel_packages/agents-sdk/src"));
    let eval = ecosystem.join("eval");
    assert!(
        eval.join("main.kujo").is_file(),
        "Eval checkout is required for the integration fixture"
    );
    for rel in ["main.kujo", "src"] {
        let source = eval.join(rel);
        let target = project.join("kennel_packages/eval").join(rel);
        if source.is_dir() {
            copy_dir(&source, &target);
        } else {
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(source, target).unwrap();
        }
    }
    copy_package_paths(ecosystem, project, "mcp", &["src"]);
    copy_package_paths(ecosystem, project, "rag", &["main.kujo", "src"]);
    copy_package_paths(
        ecosystem,
        project,
        "dispatch",
        &["dispatch.kujo", "bridge_chat.kujo", "sdk_adapter.kujo", "src", "examples"],
    );
    copy_package_paths(ecosystem, project, "runledger", &["runledger.kujo", "cli.kujo", "src"]);
    copy_package_paths(ecosystem, project, "watchdog", &["watchdog.kujo"]);
    copy_package_paths(ecosystem, project, "relay", &["main.kujo", "src", "schemas"]);
}

fn copy_package_paths(ecosystem: &Path, project: &Path, package: &str, paths: &[&str]) {
    for rel in paths {
        let source = ecosystem.join(package).join(rel);
        let target = project.join("kennel_packages").join(package).join(rel);
        assert!(source.exists(), "missing fixture source: {}", source.display());
        if source.is_dir() {
            copy_dir(&source, &target);
        } else {
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(source, target).unwrap();
        }
    }
}
