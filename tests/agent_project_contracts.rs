use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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
