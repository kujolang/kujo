use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const EXIT_USAGE_ERROR: i32 = 2;
const EXIT_RUNTIME_ERROR: i32 = 4;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp(label: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir()
        .join(format!("kujo-mcp-{label}-{}-{nanos}-{sequence}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

/// Creates a minimal package that satisfies the first-party MCP package
/// identity (`kennel.toml [package] name = "mcp"`) and exposes the stable
/// `mcp.kujo make` entrypoint. The stub records its delegated argv so tests
/// can prove CLI plumbing without depending on the real MCP checkout.
fn stub_mcp_package(root: &Path) -> PathBuf {
    let package_root = root.join("mcp");
    fs::create_dir_all(&package_root).unwrap();
    fs::write(package_root.join("kennel.toml"), "[package]\nname = \"mcp\"\nversion = \"1.0.0\"\n")
        .unwrap();
    fs::write(
        package_root.join("mcp.kujo"),
        r#"# Contract-test stub for the canonical Kujo MCP `make` entrypoint.
log_path := env_or("KUJO_MCP_CONTRACT_LOG", "")
if log_path == "" {
    print("stub error: KUJO_MCP_CONTRACT_LOG is required")
    exit(1)
}
records := []
for a in args() {
    records := push(records, to_string(a))
}
write_file(log_path, to_json(records), true)
print("stub make ok")
"#,
    )
    .unwrap();
    package_root
}

fn temp_repo(path: &Path) {
    fs::create_dir_all(path.join("src")).unwrap();
    fs::write(path.join("README.md"), "# temp repo\n").unwrap();
}

fn make_command(args: &[&str], cwd: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(args).current_dir(cwd);
    // Tests capture the outer process; the delegated generator inherits these
    // handles so its output lands in the same captured pipes.
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command
}

fn run_isolated_mcp(args: &[&str], cwd: &Path, package_root: &Path, log: &Path) -> Output {
    let install_root = temp("isolated-install");
    let output = make_command(args, cwd)
        .env("KUJO_MCP_PATH", package_root)
        .env("KUJO_MCP_CONTRACT_LOG", log)
        .env("KUJO_INSTALL_ROOT", &install_root)
        .output()
        .expect("failed to execute kujo binary");
    fs::remove_dir_all(&install_root).ok();
    output
}

fn stub_records(log: &Path) -> Vec<String> {
    let text = fs::read_to_string(log).expect("stub should record delegated argv");
    serde_json::from_str(&text).expect("stub log should be a JSON array")
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn mcp_help_lists_make_as_first_party_command() {
    let output = make_command(&["mcp", "--help"], &temp("help-cwd")).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("make"), "mcp help should list make: {stdout}");
    assert!(
        stdout.contains("Generate a repo-specific MCP server"),
        "mcp help should describe make: {stdout}"
    );

    let top_level = make_command(&["--help"], &temp("help-cwd")).output().unwrap();
    assert_eq!(top_level.status.code(), Some(0));
    let top_stdout = String::from_utf8_lossy(&top_level.stdout);
    assert!(
        top_stdout.contains("mcp"),
        "top-level help should list the first-party mcp command: {top_stdout}"
    );
}

#[test]
fn mcp_make_help_documents_repository_and_options() {
    let output = make_command(&["mcp", "make", "--help"], &temp("make-help-cwd")).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "[REPO]",
        "current directory",
        "--out",
        "--artifacts",
        "--profile-only",
        "--artifacts-only",
        "--no-ai",
        "--validate",
        "--dry-run",
    ] {
        assert!(stdout.contains(expected), "mcp make help should include {expected}: {stdout}");
    }
}

#[test]
fn unrecognized_mcp_subcommand_is_rejected_by_the_parser() {
    let output =
        make_command(&["mcp", "not-a-real-mcp-command"], &temp("bad-sub-cwd")).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let text = output_text(&output);
    assert!(text.contains("Usage: kujo mcp"), "usage should print: {text}");
}

#[test]
fn make_without_repo_delegates_current_directory() {
    let root = temp("cwd-default");
    let repo = root.join("repo");
    temp_repo(&repo);
    let package_root = stub_mcp_package(&root);
    let log = root.join("records.json");

    let output = run_isolated_mcp(&["mcp", "make"], &repo, &package_root, &log);
    assert!(output.status.success(), "{}", output_text(&output));

    let records = stub_records(&log);
    assert_eq!(records.first().map(String::as_str), Some("make"));
    assert_eq!(
        records.get(1).map(|value| PathBuf::from(value).canonicalize().unwrap()),
        Some(repo.canonicalize().unwrap())
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn make_dot_delegates_current_directory() {
    let root = temp("cwd-dot");
    let package_root = stub_mcp_package(&root);
    let log = root.join("records.json");

    let output = run_isolated_mcp(&["mcp", "make", "."], &root, &package_root, &log);
    assert!(output.status.success(), "{}", output_text(&output));

    let records = stub_records(&log);
    assert_eq!(
        records.get(1).map(|value| PathBuf::from(value).canonicalize().unwrap()),
        Some(root.canonicalize().unwrap())
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn make_relative_repo_path_delegates_subdirectory() {
    let root = temp("cwd-relative");
    let repo = root.join("nested");
    temp_repo(&repo);
    let package_root = stub_mcp_package(&root);
    let log = root.join("records.json");

    let output = run_isolated_mcp(&["mcp", "make", "./nested"], &root, &package_root, &log);
    assert!(output.status.success(), "{}", output_text(&output));

    let records = stub_records(&log);
    assert_eq!(
        records.get(1).map(|value| PathBuf::from(value).canonicalize().unwrap()),
        Some(repo.canonicalize().unwrap())
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn make_absolute_repo_path_is_forwarded_verbatim() {
    let root = temp("cwd-absolute");
    let repo = root.join("outside-repo");
    temp_repo(&repo);
    let package_root = stub_mcp_package(&root);
    let log = root.join("records.json");

    let output =
        run_isolated_mcp(&["mcp", "make", &repo.to_string_lossy()], &root, &package_root, &log);
    assert!(output.status.success(), "{}", output_text(&output));

    let records = stub_records(&log);
    assert_eq!(records.first().map(String::as_str), Some("make"));
    assert_eq!(records.get(1).map(String::as_str), Some(repo.to_string_lossy().as_ref()));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn make_forwards_generator_options_to_canonical_entrypoint() {
    let root = temp("options");
    let repo = root.join("repo");
    temp_repo(&repo);
    let package_root = stub_mcp_package(&root);
    let log = root.join("records.json");

    let output = run_isolated_mcp(
        &[
            "mcp",
            "make",
            "./repo",
            "--out",
            "custom-server",
            "--artifacts",
            "custom-artifacts",
            "--profile-only",
            "--no-ai",
            "--validate",
        ],
        &root,
        &package_root,
        &log,
    );
    assert!(output.status.success(), "{}", output_text(&output));

    let records = stub_records(&log);
    assert!(records.contains(&"--out".to_string()));
    assert!(records.contains(&"custom-server".to_string()));
    assert!(records.contains(&"--artifacts".to_string()));
    assert!(records.contains(&"custom-artifacts".to_string()));
    assert!(records.contains(&"--profile-only".to_string()));
    assert!(records.contains(&"--no-ai".to_string()));
    assert!(records.contains(&"--validate".to_string()));

    // A second run with a disjoint flag set proves each option forwards only
    // when the user passes it (no hidden defaults leak into the generator).
    let log2 = root.join("records2.json");
    let output2 = run_isolated_mcp(
        &["mcp", "make", "./repo", "--artifacts-only", "--dry-run"],
        &root,
        &package_root,
        &log2,
    );
    assert!(output2.status.success(), "{}", output_text(&output2));
    let records2 = stub_records(&log2);
    assert!(records2.contains(&"--artifacts-only".to_string()));
    assert!(records2.contains(&"--dry-run".to_string()));
    assert!(!records2.contains(&"--profile-only".to_string()));
    assert!(!records2.contains(&"--no-ai".to_string()));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn profile_only_and_artifacts_only_conflict_at_parse_time() {
    let root = temp("conflict");
    let repo = root.join("repo");
    temp_repo(&repo);
    let output = make_command(&["mcp", "make", "--profile-only", "--artifacts-only"], &repo)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "clap should reject conflicting modes");
    let text = output_text(&output);
    assert!(text.contains("cannot be used with"), "{text}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_repo_path_fails_clearly_with_usage_error() {
    let root = temp("missing-path");
    let output = make_command(&["mcp", "make", "./does-not-exist"], &root).output().unwrap();
    assert_eq!(output.status.code(), Some(EXIT_USAGE_ERROR));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist"), "{stderr}");
    assert!(stderr.contains("./does-not-exist"), "{stderr}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn file_target_fails_clearly_with_usage_error() {
    let root = temp("file-target");
    let file_target = root.join("README.md");
    fs::write(&file_target, "# not a repository\n").unwrap();

    let output = make_command(&["mcp", "make", "README.md"], &root).output().unwrap();
    assert_eq!(output.status.code(), Some(EXIT_USAGE_ERROR));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("is not a directory"), "{stderr}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn unresolvable_package_fails_with_install_guidance() {
    let root = temp("unresolved-package");
    let repo = root.join("repo");
    temp_repo(&repo);
    let empty_install_root = root.join("empty-install");
    fs::create_dir_all(&empty_install_root).unwrap();

    let output = make_command(&["mcp", "make"], &repo)
        .env_remove("KUJO_MCP_PATH")
        .env("KUJO_INSTALL_ROOT", &empty_install_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(EXIT_RUNTIME_ERROR));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("canonical 'mcp' package was not found"), "{stderr}");
    assert!(stderr.contains("KUJO_MCP_PATH"), "{stderr}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn invalid_env_override_fails_instead_of_falling_back() {
    let root = temp("invalid-env");
    let repo = root.join("repo");
    temp_repo(&repo);
    let empty_install_root = root.join("empty-install");
    fs::create_dir_all(&empty_install_root).unwrap();

    let output = make_command(&["mcp", "make"], &repo)
        .env("KUJO_MCP_PATH", root.join("not-the-mcp-package"))
        .env("KUJO_INSTALL_ROOT", &empty_install_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(EXIT_RUNTIME_ERROR));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("KUJO_MCP_PATH"), "{stderr}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn make_resolves_kennel_locked_package_without_env_override() {
    let root = temp("kennel-locked");
    let project_root = root.join("consumer");
    temp_repo(&project_root);
    let _package_root = stub_mcp_package(&project_root.join("kennel_packages"));
    let log = root.join("records.json");

    fs::write(
        project_root.join("kennel.lock"),
        "version = 1\n\n[[package]]\nname = \"mcp\"\ninstall_path = \"kennel_packages/mcp\"\n",
    )
    .unwrap();

    let empty_install_root = root.join("empty-install");
    fs::create_dir_all(&empty_install_root).unwrap();

    let output = make_command(&["mcp", "make"], &project_root)
        .env_remove("KUJO_MCP_PATH")
        .env("KUJO_MCP_CONTRACT_LOG", &log)
        .env("KUJO_INSTALL_ROOT", &empty_install_root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));

    let records = stub_records(&log);
    assert_eq!(records.first().map(String::as_str), Some("make"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn make_resolves_dogfood_package_from_cwd_walk_up() {
    let root = temp("dogfood");
    let package_root = stub_mcp_package(&root);
    let repo = package_root.join("sample-repo");
    temp_repo(&repo);
    let log = root.join("records.json");

    let empty_install_root = root.join("empty-install");
    fs::create_dir_all(&empty_install_root).unwrap();

    let output = make_command(&["mcp", "make"], &repo)
        .env_remove("KUJO_MCP_PATH")
        .env("KUJO_MCP_CONTRACT_LOG", &log)
        .env("KUJO_INSTALL_ROOT", &empty_install_root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));

    let records = stub_records(&log);
    assert_eq!(records.first().map(String::as_str), Some("make"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn make_resolves_locked_kennel_package_in_analyzed_repository() {
    let root = temp("kennel-locked");
    let consumer = root.join("consumer");
    temp_repo(&consumer);
    let _package_root = stub_mcp_package(&consumer.join("kennel_packages"));
    let log = root.join("records.json");
    fs::write(
        consumer.join("kennel.lock"),
        "version = 1\n\n[[package]]\nname = \"mcp\"\ninstall_path = \"kennel_packages/mcp\"\n",
    )
    .unwrap();

    let empty_install_root = root.join("empty-install");
    fs::create_dir_all(&empty_install_root).unwrap();

    let output = make_command(&["mcp", "make"], &consumer)
        .env_remove("KUJO_MCP_PATH")
        .env("KUJO_MCP_CONTRACT_LOG", &log)
        .env("KUJO_INSTALL_ROOT", &empty_install_root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));

    let records = stub_records(&log);
    assert_eq!(records.first().map(String::as_str), Some("make"));
    assert_eq!(
        records.get(1).map(|value| PathBuf::from(value).canonicalize().unwrap()),
        Some(consumer.canonicalize().unwrap())
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn legacy_primitive_invocation_still_works() {
    let root = temp("legacy");
    let package_root = stub_mcp_package(&root);
    let log = root.join("records.json");

    let empty_install_root = root.join("empty-install");
    fs::create_dir_all(&empty_install_root).unwrap();

    let entry = package_root.join("mcp.kujo");
    let output = make_command(&["run", &entry.to_string_lossy(), "make", "."], &root)
        .env_remove("KUJO_MCP_PATH")
        .env("KUJO_MCP_CONTRACT_LOG", &log)
        .env("KUJO_INSTALL_ROOT", &empty_install_root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", output_text(&output));
    let records = stub_records(&log);
    assert_eq!(records.first().map(String::as_str), Some("make"));
    fs::remove_dir_all(&root).ok();
}
