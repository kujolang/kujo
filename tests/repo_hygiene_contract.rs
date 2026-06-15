use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn stdout_text(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr_text(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

fn collect_root_files_from_fs(root: &Path) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(root)
        .expect("failed to read repository root")
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = entry.metadata().ok()?;
            if metadata.is_file() {
                path.file_name().and_then(|name| name.to_str()).map(ToOwned::to_owned)
            } else {
                None
            }
        })
        .filter(|name| !name.contains('/') && name != "blocked.txt" && name != ".DS_Store")
        .collect();
    files.sort();
    files
}

fn tracked_root_files(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files"])
        .output()
        .expect("failed to run git ls-files");
    if output.status.success() {
        let mut files: Vec<String> = stdout_text(&output)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.contains('/'))
            .map(ToOwned::to_owned)
            .collect();
        files.sort();
        if !files.is_empty() {
            return files;
        }
    }

    collect_root_files_from_fs(root)
}

fn run_hygiene_audit(root: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(repo_root().join("scripts").join("repo_hygiene_audit.sh"))
        .arg("--root")
        .arg(root)
        .current_dir(root)
        .output()
        .expect("failed to execute repo hygiene audit")
}

fn seed_hygiene_policy_repo(root: &Path) {
    let policy_dir = root.join("docs");
    fs::create_dir_all(&policy_dir).expect("failed to create docs dir");
    fs::write(
        policy_dir.join("REPO_HYGIENE_POLICY.md"),
        fs::read_to_string(repo_root().join("docs/REPO_HYGIENE_POLICY.md"))
            .expect("failed to read canonical hygiene policy"),
    )
    .expect("failed to write policy file");

    for entry in [
        ".editorconfig",
        ".gitignore",
        "AGENTS.md",
        "CHANGELOG.md",
        "CONTRIBUTING.md",
        "Cargo.lock",
        "Cargo.toml",
        "INSTALLATION.md",
        "LICENSE",
        "README.md",
        "ROADMAP.md",
        "rustfmt.toml",
    ] {
        fs::write(root.join(entry), format!("seeded {}\n", entry))
            .expect("failed to write allowlisted root file");
    }

    let output = Command::new("git")
        .current_dir(root)
        .args(["add", "."])
        .output()
        .expect("failed to stage temp hygiene repo");
    assert!(
        output.status.success(),
        "staging temp hygiene repo failed: stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn tracked_root_surface_matches_hygiene_allowlist() {
    let root = repo_root();
    let tracked = tracked_root_files(&root);
    let expected = vec![
        ".editorconfig".to_string(),
        ".gitignore".to_string(),
        "AGENTS.md".to_string(),
        "CHANGELOG.md".to_string(),
        "CONTRIBUTING.md".to_string(),
        "Cargo.lock".to_string(),
        "Cargo.toml".to_string(),
        "INSTALLATION.md".to_string(),
        "LICENSE".to_string(),
        "README.md".to_string(),
        "ROADMAP.md".to_string(),
        "rustfmt.toml".to_string(),
    ];

    assert_eq!(
        tracked, expected,
        "tracked root surface drifted; update policy+test together if intentional"
    );
}

#[test]
fn repo_hygiene_policy_lists_current_root_contract() {
    let policy_path = repo_root().join("docs/REPO_HYGIENE_POLICY.md");
    let content = fs::read_to_string(&policy_path).expect("failed to read REPO_HYGIENE_POLICY.md");

    for marker in [
        "# Repository Hygiene Policy",
        "## Root Surface Contract",
        "## Retention And Cleanup",
        "bash scripts/repo_hygiene_audit.sh",
        "cargo test --test repo_hygiene_contract",
        "*.db",
        "tmp/",
        "var/",
        "root clutter",
    ] {
        assert!(content.contains(marker), "missing policy marker: {}", marker);
    }
}

#[test]
fn repo_hygiene_audit_rejects_disallowed_root_clutter_patterns() {
    let root = std::env::temp_dir()
        .join(format!("kujo_repo_hygiene_rejects_clutter_{}", std::process::id()));
    fs::create_dir_all(&root).expect("failed to create temp repo");
    let _ = Command::new("git")
        .current_dir(&root)
        .arg("init")
        .output()
        .expect("failed to init temp repo");
    seed_hygiene_policy_repo(&root);

    fs::write(root.join("scratch.db"), "sqlite").expect("failed to write clutter file");
    fs::create_dir_all(root.join("scratch_bundle")).expect("failed to create clutter dir");
    fs::write(root.join("scratch_bundle").join("payload.txt"), "payload")
        .expect("failed to write clutter payload");

    let output = run_hygiene_audit(&root);
    assert!(
        !output.status.success(),
        "repo hygiene audit should reject disallowed clutter: stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("disallowed local root clutter"),
        "repo hygiene audit should explain clutter rejection, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn repo_hygiene_audit_allows_practical_tmp_and_var_workflows() {
    let root =
        std::env::temp_dir().join(format!("kujo_repo_hygiene_allows_tmp_{}", std::process::id()));
    fs::create_dir_all(&root).expect("failed to create temp repo");
    let _ = Command::new("git")
        .current_dir(&root)
        .arg("init")
        .output()
        .expect("failed to init temp repo");
    seed_hygiene_policy_repo(&root);

    fs::create_dir_all(root.join("tmp").join("nested")).expect("failed to create tmp dir");
    fs::write(root.join("tmp").join("nested").join("payload.txt"), "scratch")
        .expect("failed to write tmp payload");
    fs::create_dir_all(root.join("var").join("cache")).expect("failed to create var dir");
    fs::write(root.join("var").join("cache").join("payload.db"), "scratch")
        .expect("failed to write var payload");

    let output = run_hygiene_audit(&root);
    assert!(
        output.status.success(),
        "repo hygiene audit should allow practical tmp/var workflows: stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}
