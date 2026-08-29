use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
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
        run(&["agent", "new", "unsafe-root", "--dir", "/", "--no-git"], &root).status.code(),
        Some(2)
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
    let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["agent", "run", "live", "--json"])
        .env("CUSTOM_API_KEY", "not-a-real-secret")
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
