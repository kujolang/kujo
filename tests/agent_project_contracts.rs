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
        install_agents_sdk_fixture(&project);
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
}

#[test]
fn project_is_portable_without_sibling_repositories() {
    let root = temp();
    assert!(run(&["agent", "new", "portable", "--dir", root.to_str().unwrap(), "--no-git"], &root)
        .status
        .success());
    install_agents_sdk_fixture(&root.join("portable"));
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

fn install_agents_sdk_fixture(project: &Path) {
    let ecosystem = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let source = ecosystem.join("agents-sdk/src");
    assert!(source.is_dir(), "Agents SDK checkout is required for the integration fixture");
    copy_dir(&source, &project.join("kennel_packages/agents-sdk/src"));
}
