// File: src/mcp_make.rs
//
// First-class `kujo mcp make` command surfaces.
//
// The MCP generator itself stays canonical in the Kujo MCP package. This
// module only resolves that package and delegates to its stable `make`
// entrypoint through `kujo run`, so users never need to know about the
// interpreter or the underlying script path. The generated-server safety
// model (read-only defaults, allowlisted commands, blocked risky operations,
// and repository-bounded access) lives entirely in the MCP package.

use clap::Subcommand;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const MCP_ENTRY_FILE: &str = "mcp.kujo";
const MCP_PACKAGE_NAME: &str = "mcp";
const MCP_PACKAGE_PATH_ENV: &str = "KUJO_MCP_PATH";
const ECOSYSTEM_INSTALL_ROOT_ENV: &str = "KUJO_INSTALL_ROOT";
const DEFAULT_ECOSYSTEM_INSTALL_DIR: &str = ".kujo";

const EXIT_USAGE_ERROR: i32 = 2;
const EXIT_RUNTIME_ERROR: i32 = 4;
const EXIT_IO_ERROR: i32 = 5;

/// Failure payload for `kujo mcp make`: exit code plus an optional
/// user-facing message. The message may be empty when the delegated generator
/// already reported the error on stderr and the CLI only forwards its status.
pub type McpMakeError = (i32, String);

/// First-party Kujo MCP commands.
#[derive(Subcommand, Debug)]
pub enum McpCommands {
    /// Generate a repo-specific MCP server and review artifacts
    Make {
        /// Repository to analyze [default: current directory]
        repo: Option<PathBuf>,

        /// Directory for the generated MCP server [default: <repo>/.mcp/generated-server]
        #[arg(long)]
        out: Option<PathBuf>,

        /// Directory for review artifacts [default: <repo>/.mcp/artifacts]
        #[arg(long)]
        artifacts: Option<PathBuf>,

        /// Explicit path to the Kujo AI SDK for optional profile enrichment
        #[arg(long)]
        ai_sdk_path: Option<PathBuf>,

        /// Generate only the repository profile and review artifacts
        #[arg(long, conflicts_with = "artifacts_only")]
        profile_only: bool,

        /// Generate the repository profile and review artifacts without the server scaffold
        #[arg(long)]
        artifacts_only: bool,

        /// Skip optional AI enrichment and profile deterministically
        #[arg(long)]
        no_ai: bool,

        /// Validate the generated server after generation
        #[arg(long)]
        validate: bool,

        /// Analyze the repository without writing any files
        #[arg(long)]
        dry_run: bool,
    },
}

/// Dispatches a first-party MCP command. `kujo mcp` help/usage errors are
/// handled by clap; `make` failures return the CLI exit code to forward plus
/// an optional message when the generator already reported the failure.
pub fn execute(command: McpCommands) -> Result<(), McpMakeError> {
    match command {
        McpCommands::Make {
            repo,
            out,
            artifacts,
            ai_sdk_path,
            profile_only,
            artifacts_only,
            no_ai,
            validate,
            dry_run,
        } => run_make_command(MakeOptions {
            repo,
            out,
            artifacts,
            ai_sdk_path,
            profile_only,
            artifacts_only,
            no_ai,
            validate,
            dry_run,
        }),
    }
}

#[derive(Debug)]
pub struct MakeOptions {
    pub repo: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub artifacts: Option<PathBuf>,
    pub ai_sdk_path: Option<PathBuf>,
    pub profile_only: bool,
    pub artifacts_only: bool,
    pub no_ai: bool,
    pub validate: bool,
    pub dry_run: bool,
}

fn run_make_command(options: MakeOptions) -> Result<(), McpMakeError> {
    let repo_abs = resolve_target_repository(options.repo.as_deref())?;
    let package_root = find_mcp_package_root(&repo_abs).map_err(|message| {
        (EXIT_RUNTIME_ERROR, format!("Cannot run `kujo mcp make`: {message}"))
    })?;
    let entry = package_root.join(MCP_ENTRY_FILE);
    delegate_to_generator(&entry, &repo_abs, &options)
}

/// Resolves the repository that the MCP generator will profile.
///
/// No repository argument means the current directory. Relative paths resolve
/// against the caller's working directory. The path must exist and be a
/// directory; deeper safety behavior (bounded traversal, symlink handling,
/// secret detection) stays inside the canonical generator.
fn resolve_target_repository(explicit: Option<&Path>) -> Result<PathBuf, McpMakeError> {
    let cwd = std::env::current_dir()
        .map_err(|error| (EXIT_IO_ERROR, format!("Cannot read the current directory: {error}")))?;
    resolve_target_repository_with_cwd(explicit, &cwd)
}

fn resolve_target_repository_with_cwd(
    explicit: Option<&Path>,
    cwd: &Path,
) -> Result<PathBuf, McpMakeError> {
    let raw = match explicit {
        Some(path) if !path.as_os_str().is_empty() => path.to_path_buf(),
        _ => PathBuf::from("."),
    };

    let absolute = if raw.is_absolute() { raw.clone() } else { cwd.join(&raw) };

    // `.` and `./repo` forms are normalized so delegated paths and generated
    // artifacts avoid redundant dot segments.
    let absolute = normalize_dot_components(&absolute);

    let metadata = std::fs::metadata(&absolute).map_err(|error| {
        (EXIT_USAGE_ERROR, format!("Repository path '{}' does not exist: {error}", raw.display()))
    })?;
    if !metadata.is_dir() {
        return Err((
            EXIT_USAGE_ERROR,
            format!("Repository path '{}' is not a directory", raw.display()),
        ));
    }

    Ok(absolute)
}

/// Removes redundant `.` segments introduced by defaulting and `./` inputs so
/// delegated paths stay clean absolute directory paths.
fn normalize_dot_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::CurDir) {
            continue;
        }
        normalized.push(component.as_os_str());
    }
    normalized
}

/// Resolves the canonical Kujo MCP package that implements `make`.
///
/// Resolution order:
///   1. `KUJO_MCP_PATH` - explicit package-root override for development,
///      tests, and nonstandard installations. An invalid value fails loudly
///      so typos cannot silently fall through to another location.
///   2. Walk-up dogfood discovery: the nearest ancestor of the target
///      repository whose `kennel.toml` declares `[package] name = "mcp"`.
///   3. The target repository's Kennel lockfile: a locked `[[package]]` entry
///      named `mcp`, resolved safely inside `kennel_packages`.
///   4. The first-party ecosystem installation root
///      (`$KUJO_INSTALL_ROOT/sources/mcp`, default `$HOME/.kujo/sources/mcp`),
///      which the official Kujo installer populates.
fn find_mcp_package_root(target_repo: &Path) -> Result<PathBuf, String> {
    if let Some(root) = mcp_package_root_from_env()? {
        return Ok(root);
    }
    if let Some(root) = mcp_package_root_from_dogfood(target_repo) {
        return Ok(root);
    }
    if let Some(root) = mcp_package_root_from_kennel(target_repo) {
        return Ok(root);
    }
    if let Some(root) = mcp_package_root_from_ecosystem() {
        return Ok(root);
    }

    Err(format!(
        "the canonical '{MCP_PACKAGE_NAME}' package was not found. Install it with the Kujo \
         ecosystem installer, which provides '~/.kujo/sources/{MCP_PACKAGE_NAME}', install the \
         '{MCP_PACKAGE_NAME}' Kennel package in the analyzed repository, or set \
         {MCP_PACKAGE_PATH_ENV} to the '{MCP_PACKAGE_NAME}' package root."
    ))
}

fn mcp_package_root_from_env() -> Result<Option<PathBuf>, String> {
    let Some(raw) = std::env::var_os(MCP_PACKAGE_PATH_ENV) else {
        return Ok(None);
    };
    if raw.is_empty() {
        return Ok(None);
    }

    let root = PathBuf::from(&raw);
    if !root.join(MCP_ENTRY_FILE).is_file() {
        return Err(format!(
            "{MCP_PACKAGE_PATH_ENV} must point at the '{MCP_PACKAGE_NAME}' package root \
             containing '{MCP_ENTRY_FILE}', got '{}'",
            root.display()
        ));
    }
    Ok(Some(root))
}

fn mcp_package_root_from_dogfood(target_repo: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(target_repo).ok()?;
    for ancestor in canonical.ancestors() {
        if is_canonical_mcp_package_root(ancestor) {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// Recognizes the canonical MCP package root by its installed package identity
/// (`kennel.toml` `[package] name = "mcp"`) plus the stable `make` entrypoint.
/// Random repositories that merely contain a same-named script are ignored.
fn is_canonical_mcp_package_root(path: &Path) -> bool {
    let kennel_toml = path.join("kennel.toml");
    if !kennel_toml.is_file() || !path.join(MCP_ENTRY_FILE).is_file() {
        return false;
    }
    let Ok(source) = std::fs::read_to_string(&kennel_toml) else {
        return false;
    };
    let Ok(value) = source.parse::<toml::Value>() else {
        return false;
    };
    value.get("package").and_then(|package| package.get("name")).and_then(toml::Value::as_str)
        == Some(MCP_PACKAGE_NAME)
}

fn mcp_package_root_from_kennel(target_repo: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(target_repo).ok()?;
    let root = crate::module::locked_kennel_package_path(&canonical, MCP_PACKAGE_NAME)?;
    if root.join(MCP_ENTRY_FILE).is_file() {
        Some(root)
    } else {
        None
    }
}

fn mcp_package_root_from_ecosystem() -> Option<PathBuf> {
    let install_root = match std::env::var_os(ECOSYSTEM_INSTALL_ROOT_ENV) {
        Some(raw) => PathBuf::from(&raw),
        None => {
            let Some(home) = std::env::var_os("HOME") else {
                return None;
            };
            PathBuf::from(home).join(DEFAULT_ECOSYSTEM_INSTALL_DIR)
        }
    };

    let package_root = install_root.join("sources").join(MCP_PACKAGE_NAME);
    if package_root.join(MCP_ENTRY_FILE).is_file() {
        Some(package_root)
    } else {
        None
    }
}

/// Generates the MCP scaffold by delegating to the canonical MCP package
/// through `kujo run`. VM-first is intentional: the generator does not depend
/// on interpreter-only behavior, so this command stays free of
/// `--interpreter` plumbing.
fn delegate_to_generator(
    entry: &Path,
    repo_abs: &Path,
    options: &MakeOptions,
) -> Result<(), McpMakeError> {
    let executable = std::env::current_exe().map_err(|error| {
        (EXIT_IO_ERROR, format!("Cannot locate the current Kujo executable: {error}"))
    })?;

    let mut command = Command::new(executable);
    command.arg("run").arg(entry);
    command.arg("--");
    command.arg("make").arg(repo_abs);

    if let Some(out) = options.out.as_deref() {
        command.arg("--out").arg(out);
    }
    if let Some(artifacts) = options.artifacts.as_deref() {
        command.arg("--artifacts").arg(artifacts);
    }
    if let Some(ai_sdk_path) = options.ai_sdk_path.as_deref() {
        command.arg("--ai-sdk-path").arg(ai_sdk_path);
    }
    if options.profile_only {
        command.arg("--profile-only");
    }
    if options.artifacts_only {
        command.arg("--artifacts-only");
    }
    if options.no_ai {
        command.arg("--no-ai");
    }
    if options.validate {
        command.arg("--validate");
    }
    if options.dry_run {
        command.arg("--dry-run");
    }

    let status = command
        .status()
        .map_err(|error| (EXIT_IO_ERROR, format!("Failed to launch the MCP generator: {error}")))?;

    if status.success() {
        return Ok(());
    }

    if let Some(code) = status.code() {
        return Err((code, String::new()));
    }
    Err((EXIT_RUNTIME_ERROR, "MCP generator terminated by signal".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("kujo_mcp_make_{label}_{nanos}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_canonical_mcp_package(root: &Path) {
        fs::create_dir_all(root).unwrap();
        fs::write(root.join("kennel.toml"), "[package]\nname = \"mcp\"\nversion = \"1.0.0\"\n")
            .unwrap();
        fs::write(root.join(MCP_ENTRY_FILE), "# canonical mcp make entry\n").unwrap();
    }

    #[test]
    fn resolves_default_repo_to_current_directory() {
        let cwd = unique_temp_root("default_repo");
        let result = resolve_target_repository_with_cwd(None, &cwd);
        assert_eq!(result.expect("cwd repository should resolve"), cwd);
        fs::remove_dir_all(&cwd).unwrap();
    }

    #[test]
    fn normalizes_dot_and_relative_repo_paths() {
        let cwd = unique_temp_root("dot_repo");
        let sub = cwd.join("sub-repo");
        fs::create_dir_all(&sub).unwrap();

        let dot = resolve_target_repository_with_cwd(Some(Path::new(".")), &cwd);
        let relative = resolve_target_repository_with_cwd(Some(Path::new("./sub-repo")), &cwd);

        assert_eq!(dot.expect("dot repo should resolve"), cwd);
        assert_eq!(relative.expect("relative repo should resolve"), sub);
        fs::remove_dir_all(&cwd).unwrap();
    }

    #[test]
    fn rejects_missing_repo_paths_with_usage_error() {
        let cwd = unique_temp_root("missing_repo");
        let missing = resolve_target_repository_with_cwd(Some(Path::new("./does-not-exist")), &cwd);

        let (code, message) = missing.expect_err("missing repo must fail");
        assert_eq!(code, EXIT_USAGE_ERROR);
        assert!(message.contains("does not exist"), "{message}");
        fs::remove_dir_all(&cwd).unwrap();
    }

    #[test]
    fn rejects_file_targets_with_usage_error() {
        let cwd = unique_temp_root("file_repo");
        let file_target = cwd.join("README.md");
        fs::write(&file_target, "not a repository\n").unwrap();

        let (code, message) = resolve_target_repository_with_cwd(Some(&file_target), &cwd)
            .expect_err("file must fail");
        assert_eq!(code, EXIT_USAGE_ERROR);
        assert!(message.contains("is not a directory"), "{message}");
        fs::remove_dir_all(&cwd).unwrap();
    }

    #[test]
    fn finds_package_root_from_dogfood_walk_up() {
        let temp_root = unique_temp_root("dogfood");
        let package_root = temp_root.join("mcp");
        write_canonical_mcp_package(&package_root);
        let target = package_root.join("nested-repo");
        fs::create_dir_all(&target).unwrap();

        let found =
            mcp_package_root_from_dogfood(&target).expect("walk-up should find canonical root");
        assert_eq!(found, package_root.canonicalize().unwrap());
        fs::remove_dir_all(&temp_root).unwrap();
    }

    #[test]
    fn dogfood_walk_ignores_non_mcp_kennel_files() {
        let temp_root = unique_temp_root("not_mcp");
        let package_root = temp_root.join("other-package");
        fs::create_dir_all(&package_root).unwrap();
        fs::write(package_root.join("kennel.toml"), "[package]\nname = \"other-package\"\n")
            .unwrap();
        fs::write(package_root.join(MCP_ENTRY_FILE), "# same-named entry\n").unwrap();
        let target = temp_root.join("repo");
        fs::create_dir_all(&target).unwrap();

        assert!(mcp_package_root_from_dogfood(&target).is_none());
        fs::remove_dir_all(&temp_root).unwrap();
    }

    #[test]
    fn finds_package_root_from_locked_kennel_installs() {
        let temp_root = unique_temp_root("kennel_locked");
        let project_root = temp_root.join("consumer");
        let packages_root = project_root.join("kennel_packages");
        let package_root = packages_root.join(MCP_PACKAGE_NAME);
        fs::create_dir_all(&package_root).unwrap();
        fs::create_dir_all(packages_root.join("unlocked-package")).unwrap();
        write_canonical_mcp_package(&package_root);
        fs::write(
            project_root.join("kennel.lock"),
            "version = 1\n\n[[package]]\nname = \"mcp\"\ninstall_path = \"kennel_packages/mcp\"\n",
        )
        .unwrap();

        let found =
            mcp_package_root_from_kennel(&project_root).expect("locked package should resolve");
        assert_eq!(found, package_root.canonicalize().unwrap());
        fs::remove_dir_all(&temp_root).unwrap();
    }

    #[test]
    fn ignores_unlocked_kennel_package_staging() {
        let temp_root = unique_temp_root("kennel_unlocked");
        let project_root = temp_root.join("consumer");
        let package_root = project_root.join("kennel_packages").join(MCP_PACKAGE_NAME);
        fs::create_dir_all(&package_root).unwrap();
        write_canonical_mcp_package(&package_root);

        assert!(mcp_package_root_from_kennel(&project_root).is_none());
        fs::remove_dir_all(&temp_root).unwrap();
    }

    #[test]
    fn finds_package_root_from_ecosystem_install_layout() {
        let _guard =
            crate::network_policy::test_env_lock().lock().expect("environment lock poisoned");
        let temp_root = unique_temp_root("ecosystem");
        let package_root = temp_root.join("sources").join(MCP_PACKAGE_NAME);
        fs::create_dir_all(&package_root).unwrap();
        fs::write(package_root.join(MCP_ENTRY_FILE), "# canonical entry\n").unwrap();

        std::env::set_var(ECOSYSTEM_INSTALL_ROOT_ENV, &temp_root);
        let found = mcp_package_root_from_ecosystem();
        std::env::remove_var(ECOSYSTEM_INSTALL_ROOT_ENV);

        assert_eq!(found.expect("ecosystem layout should resolve"), package_root);
        fs::remove_dir_all(&temp_root).unwrap();
    }

    #[test]
    fn explicit_env_override_is_validated() {
        let _guard =
            crate::network_policy::test_env_lock().lock().expect("environment lock poisoned");
        let temp_root = unique_temp_root("env_invalid");
        std::env::set_var(MCP_PACKAGE_PATH_ENV, temp_root.join("not-the-mcp"));
        let outcome = mcp_package_root_from_env();
        std::env::remove_var(MCP_PACKAGE_PATH_ENV);

        let message = outcome.expect_err("invalid override must fail loudly");
        assert!(message.contains(MCP_PACKAGE_PATH_ENV), "{message}");
        fs::remove_dir_all(&temp_root).unwrap();
    }
}
