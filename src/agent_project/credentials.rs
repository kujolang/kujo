use super::*;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, OpenOptions};
use fs2::FileExt;
use std::io::{self, IsTerminal, Read, Write};

#[cfg(feature = "runtime-network")]
const KEYRING_SERVICE: &str = "ai.kujo.agent";
const TEST_STORE_ENV: &str = "KUJO_AGENT_TEST_CREDENTIAL_STORE";
const PROJECT_ENV_FILE: &str = ".env.local";
pub(super) const CREDENTIAL_TEMP_PREFIX: &str = ".kujo-credential-";
const MAX_CREDENTIAL_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CredentialSource {
    Environment,
    Project,
    Keyring,
}

impl CredentialSource {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::Project => "project .env.local",
            Self::Keyring => "OS credential store",
        }
    }
}

pub(super) struct ResolvedCredential {
    pub(super) value: String,
    pub(super) source: CredentialSource,
}

pub(super) fn execute_auth(command: AuthCommands) -> Result<(), AgentError> {
    match command {
        AuthCommands::Set(args) => set(args),
        AuthCommands::Status(args) => status(args),
        AuthCommands::Remove(args) => remove(args),
    }
}

fn set(args: AuthSetArgs) -> Result<(), AgentError> {
    let target = credential_target(args.provider.as_deref(), args.name.as_deref())?;
    let credential = if args.from_stdin {
        read_stdin_secret()?
    } else if args.from_env {
        std::env::var(&target.name).map_err(|_| {
            fail(format!(
                "{} is not set. Remove --from-env to enter the credential securely.",
                target.name
            ))
        })?
    } else {
        if !io::stdin().is_terminal() {
            return Err(usage(
                "Interactive credential entry requires a terminal. Use --from-stdin for automation or --from-env to import the configured provider variable.",
            ));
        }
        rpassword::prompt_password(format!("Enter {} credential (input hidden): ", target.label))
            .map_err(|e| ioerr(format!("Could not read credential: {e}")))?
    };
    validate_secret(&credential)?;
    let scope = if args.project {
        let root = current_project_root()?;
        write_project_secret(&root, &target.name, &credential)?;
        "project"
    } else {
        keyring_set(&target.name, &credential)?;
        "user"
    };
    let payload = json!({
        "contract":"kujo-agent-auth/v1",
        "status":"configured",
        "target":target.id,
        "credential_name":target.name,
        "scope":scope,
        "storage":if args.project { "project .env.local" } else { "OS credential store" }
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else {
        println!(
            "Configured {} for {} in the {}.",
            target.name,
            target.label,
            payload["storage"].as_str().unwrap()
        );
    }
    Ok(())
}

fn status(args: AuthProjectArgs) -> Result<(), AgentError> {
    let provider_target = args.provider.is_some();
    let target = credential_target(args.provider.as_deref(), args.name.as_deref())?;
    let root = if args.project { Some(current_project_root()?) } else { None };
    let resolved = resolve(&target.name, root.as_deref())?;
    let payload = json!({
        "contract":"kujo-agent-auth/v1",
        "status":if resolved.is_some() { "configured" } else { "missing" },
        "target":target.id,
        "credential_name":target.name,
        "source":resolved.as_ref().map(|value| value.source.label())
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else if let Some(value) = resolved {
        println!(
            "{} is configured for {} via {}.",
            target.name,
            target.label,
            value.source.label()
        );
    } else {
        println!(
            "{} is not configured for {}. Run `kujo agent auth set {}`.",
            target.name,
            target.label,
            if provider_target { target.id.clone() } else { format!("--name {}", target.name) }
        );
    }
    Ok(())
}

fn remove(args: AuthProjectArgs) -> Result<(), AgentError> {
    let target = credential_target(args.provider.as_deref(), args.name.as_deref())?;
    let removed = if args.project {
        let root = current_project_root()?;
        remove_project_secret(&root, &target.name)?
    } else {
        keyring_remove(&target.name)?
    };
    let payload = json!({
        "contract":"kujo-agent-auth/v1",
        "status":if removed { "removed" } else { "not_found" },
        "target":target.id,
        "credential_name":target.name,
        "scope":if args.project { "project" } else { "user" }
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else if removed {
        println!("Removed {} credential.", target.label);
    } else {
        println!("No stored {} credential was found.", target.label);
    }
    Ok(())
}

pub(super) fn configure_new_project_credential(args: &NewArgs) -> Result<(), AgentError> {
    let settings = provider_settings(&args.provider)
        .ok_or_else(|| usage(format!("Unsupported provider '{}'.", args.provider)))?;
    if settings.mode == "fixture" || std::env::var(settings.api_key_env).is_ok() {
        return Ok(());
    }
    if keyring_get(settings.api_key_env)?.is_some() {
        return Ok(());
    }
    if args.no_credential {
        return Ok(());
    }
    let credential = if args.credential_stdin {
        read_stdin_secret()?
    } else if io::stdin().is_terminal() && !args.json {
        rpassword::prompt_password(format!(
            "No saved {} credential was found. Enter it once to store it securely (input hidden): ",
            display_provider(&args.provider)
        ))
        .map_err(|e| ioerr(format!("Could not read credential: {e}")))?
    } else {
        return Err(fail(format!(
            "{} is not configured. Run `kujo agent auth set {}` first, use --credential-stdin, or use --no-credential to scaffold without a runnable live provider.",
            settings.api_key_env, args.provider
        )));
    };
    validate_secret(&credential)?;
    keyring_set(settings.api_key_env, &credential)
}

pub(super) fn preferred_provider(json_output: bool) -> Result<String, AgentError> {
    if json_output {
        return Ok("fixture".into());
    }
    for provider in ["openai", "openrouter", "deepseek"] {
        let settings = provider_settings(provider).unwrap();
        if std::env::var(settings.api_key_env).is_ok()
            || keyring_get(settings.api_key_env).unwrap_or(None).is_some()
        {
            return Ok(provider.to_string());
        }
    }
    if !io::stdin().is_terminal() {
        return Ok("fixture".into());
    }
    eprintln!("Choose a model provider:");
    eprintln!("  1. OpenAI (recommended)");
    eprintln!("  2. OpenRouter");
    eprintln!("  3. DeepSeek");
    eprintln!("  4. Offline fixture");
    eprint!("Provider [1]: ");
    io::stderr().flush().map_err(|e| ioerr(e.to_string()))?;
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).map_err(|e| ioerr(e.to_string()))?;
    match choice.trim() {
        "" | "1" | "openai" => Ok("openai".into()),
        "2" | "openrouter" => Ok("openrouter".into()),
        "3" | "deepseek" => Ok("deepseek".into()),
        "4" | "fixture" => Ok("fixture".into()),
        _ => Err(usage("Unknown provider choice. Select 1, 2, 3, or 4.")),
    }
}

pub(super) fn resolve_for_project(
    root: &Path,
    credential_name: &str,
) -> Result<Option<ResolvedCredential>, AgentError> {
    resolve(credential_name, Some(root))
}

fn resolve(
    credential_name: &str,
    root: Option<&Path>,
) -> Result<Option<ResolvedCredential>, AgentError> {
    if let Ok(value) = std::env::var(credential_name) {
        if !value.trim().is_empty() {
            return Ok(Some(ResolvedCredential { value, source: CredentialSource::Environment }));
        }
    }
    if let Some(root) = root {
        if let Some(value) = read_project_secret(root, credential_name)? {
            return Ok(Some(ResolvedCredential { value, source: CredentialSource::Project }));
        }
    }
    Ok(keyring_get(credential_name)?
        .map(|value| ResolvedCredential { value, source: CredentialSource::Keyring }))
}

struct CredentialTarget {
    id: String,
    name: String,
    label: String,
}

fn credential_target(
    provider: Option<&str>,
    explicit_name: Option<&str>,
) -> Result<CredentialTarget, AgentError> {
    if let Some(name) = explicit_name {
        if !valid_credential_name(name) {
            return Err(usage(
                "--name must be an uppercase environment-variable name such as LINEAR_API_TOKEN.",
            ));
        }
        return Ok(CredentialTarget {
            id: name.to_string(),
            name: name.to_string(),
            label: format!("connector {name}"),
        });
    }
    let provider = provider.ok_or_else(|| {
        usage("Provide a built-in provider name or --name <CREDENTIAL_ENV> for a connector.")
    })?;
    let settings = live_provider(provider)?;
    Ok(CredentialTarget {
        id: provider.to_string(),
        name: settings.api_key_env.to_string(),
        label: display_provider(provider).to_string(),
    })
}

fn live_provider(provider: &str) -> Result<ProviderSettings, AgentError> {
    let settings = provider_settings(provider).ok_or_else(|| {
        usage(format!(
            "Unknown provider '{provider}'. Expected one of: openai, openrouter, deepseek, custom."
        ))
    })?;
    if settings.mode == "fixture" {
        return Err(usage("Fixture mode does not use a credential."));
    }
    Ok(settings)
}

fn display_provider(provider: &str) -> &str {
    match provider {
        "openai" => "OpenAI",
        "openrouter" => "OpenRouter",
        "deepseek" => "DeepSeek",
        "custom" => "custom provider",
        _ => provider,
    }
}

fn validate_secret(value: &str) -> Result<(), AgentError> {
    if value.trim().is_empty() {
        return Err(usage("Credential may not be empty."));
    }
    if value.len() > MAX_CREDENTIAL_BYTES {
        return Err(usage("Credential exceeds the 64 KiB safety limit."));
    }
    if value.contains('\0') || value.contains('\n') || value.contains('\r') {
        return Err(usage("Credential may not contain NUL or newline characters."));
    }
    Ok(())
}

fn read_stdin_secret() -> Result<String, AgentError> {
    let mut input = String::new();
    io::stdin()
        .take((MAX_CREDENTIAL_BYTES + 2) as u64)
        .read_to_string(&mut input)
        .map_err(|e| ioerr(format!("Could not read credential from stdin: {e}")))?;
    let value = input.trim_end_matches(['\r', '\n']).to_string();
    validate_secret(&value)?;
    Ok(value)
}

fn current_project_root() -> Result<PathBuf, AgentError> {
    let cwd = std::env::current_dir().map_err(|e| ioerr(e.to_string()))?;
    discover(&cwd)
}

#[cfg(feature = "runtime-network")]
fn keyring_entry(credential_name: &str) -> Result<keyring_core::Entry, AgentError> {
    credential_store::entry(KEYRING_SERVICE, credential_name)
        .map_err(|e| fail(format!("OS credential store is unavailable: {e}")))
}

#[cfg(feature = "runtime-network")]
fn keyring_set(credential_name: &str, value: &str) -> Result<(), AgentError> {
    if let Some(path) = test_store_path() {
        return update_test_store(&path, credential_name, Some(value));
    }
    keyring_entry(credential_name)?
        .set_password(value)
        .map_err(|e| fail(format!("Could not save credential in the OS credential store: {e}")))
}

#[cfg(feature = "runtime-network")]
fn keyring_get(credential_name: &str) -> Result<Option<String>, AgentError> {
    if let Some(path) = test_store_path() {
        return Ok(read_test_store(&path)?.remove(credential_name));
    }
    match keyring_entry(credential_name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(error) => Err(fail(format!("Could not read the OS credential store: {error}"))),
    }
}

#[cfg(feature = "runtime-network")]
fn keyring_remove(credential_name: &str) -> Result<bool, AgentError> {
    if let Some(path) = test_store_path() {
        let existed = read_test_store(&path)?.contains_key(credential_name);
        update_test_store(&path, credential_name, None)?;
        return Ok(existed);
    }
    match keyring_entry(credential_name)?.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring_core::Error::NoEntry) => Ok(false),
        Err(error) => Err(fail(format!("Could not remove credential: {error}"))),
    }
}

#[cfg(not(feature = "runtime-network"))]
fn keyring_set(credential_name: &str, value: &str) -> Result<(), AgentError> {
    if let Some(path) = test_store_path() {
        return update_test_store(&path, credential_name, Some(value));
    }
    Err(fail(
        "OS credential store is disabled in this build (enable the 'runtime-network' feature)",
    ))
}

#[cfg(not(feature = "runtime-network"))]
fn keyring_get(credential_name: &str) -> Result<Option<String>, AgentError> {
    if let Some(path) = test_store_path() {
        return Ok(read_test_store(&path)?.remove(credential_name));
    }
    Err(fail(
        "OS credential store is disabled in this build (enable the 'runtime-network' feature)",
    ))
}

#[cfg(not(feature = "runtime-network"))]
fn keyring_remove(credential_name: &str) -> Result<bool, AgentError> {
    if let Some(path) = test_store_path() {
        let existed = read_test_store(&path)?.contains_key(credential_name);
        update_test_store(&path, credential_name, None)?;
        return Ok(existed);
    }
    Err(fail(
        "OS credential store is disabled in this build (enable the 'runtime-network' feature)",
    ))
}

fn test_store_path() -> Option<PathBuf> {
    std::env::var_os(TEST_STORE_ENV).filter(|value| !value.is_empty()).map(PathBuf::from)
}

fn read_test_store(path: &Path) -> Result<BTreeMap<String, String>, AgentError> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    reject_symlink(path, "credential test store")?;
    let source = fs::read_to_string(path).map_err(|e| ioerr(e.to_string()))?;
    serde_json::from_str(&source)
        .map_err(|e| usage(format!("Malformed credential test store: {e}")))
}

fn update_test_store(path: &Path, name: &str, value: Option<&str>) -> Result<(), AgentError> {
    let mut entries = read_test_store(path)?;
    if let Some(value) = value {
        entries.insert(name.to_string(), value.to_string());
    } else {
        entries.remove(name);
    }
    secure_atomic_write(path, &format!("{}\n", serde_json::to_string_pretty(&entries).unwrap()))
}

fn project_directory(root: &Path) -> Result<Dir, AgentError> {
    Dir::open_ambient_dir(root, cap_std::ambient_authority()).map_err(|e| ioerr(e.to_string()))
}

fn lock_project_credentials(root: &Dir) -> Result<std::fs::File, AgentError> {
    const LOCK_DIR: &str = ".kujo-agent";
    match root.create_dir(LOCK_DIR) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(ioerr(format!("Could not create credential lock directory: {e}"))),
    }
    let directory = root
        .open_dir_nofollow(LOCK_DIR)
        .map_err(|e| ioerr(format!("Could not open credential lock directory: {e}")))?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true).follow(FollowSymlinks::No).nonblock(true);
    let lock = match directory.open_with("credential.lock", &options) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            directory.open_with("credential.lock", options.create_new(false))
        }
        result => result,
    }
    .map_err(|e| ioerr(format!("Could not open project credential lock: {e}")))?;
    if !lock.metadata().map_err(|e| ioerr(e.to_string()))?.is_file() {
        return Err(ioerr("Project credential lock must be a regular file."));
    }
    // Keep the lock file in place so concurrent waiters always lock the same inode.
    let lock = lock.into_std();
    FileExt::lock_exclusive(&lock)
        .map_err(|e| ioerr(format!("Could not lock project credentials: {e}")))?;
    Ok(lock)
}

fn read_project_entries(root: &Dir, path: &Path) -> Result<BTreeMap<String, String>, AgentError> {
    match root.symlink_metadata(PROJECT_ENV_FILE) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(metadata) if metadata.is_symlink() => {
            return Err(usage(format!("Refusing symlinked {PROJECT_ENV_FILE}.")));
        }
        Ok(_) => return Err(usage(format!("{PROJECT_ENV_FILE} must be a regular file."))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(ioerr(e.to_string())),
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true);
    let mut file = root.open_with(PROJECT_ENV_FILE, &options).map_err(|e| ioerr(e.to_string()))?;
    let metadata = file.metadata().map_err(|e| ioerr(e.to_string()))?;
    if !metadata.is_file() {
        return Err(usage(format!("{PROJECT_ENV_FILE} must be a regular file.")));
    }
    require_private_permissions(path, &metadata)?;
    let mut source = String::new();
    file.read_to_string(&mut source).map_err(|e| ioerr(e.to_string()))?;
    parse_env_file(&source)
}

fn read_project_secret(root: &Path, name: &str) -> Result<Option<String>, AgentError> {
    Ok(read_project_entries(&project_directory(root)?, &root.join(PROJECT_ENV_FILE))?.remove(name))
}

fn write_project_secret(root: &Path, name: &str, value: &str) -> Result<(), AgentError> {
    let path = root.join(PROJECT_ENV_FILE);
    let directory = project_directory(root)?;
    let _lock = lock_project_credentials(&directory)?;
    let mut entries = read_project_entries(&directory, &path)?;
    entries.insert(name.to_string(), value.to_string());
    secure_atomic_write(&path, &render_env_file(&entries))
}

fn remove_project_secret(root: &Path, name: &str) -> Result<bool, AgentError> {
    let path = root.join(PROJECT_ENV_FILE);
    let directory = project_directory(root)?;
    let _lock = lock_project_credentials(&directory)?;
    let mut entries = read_project_entries(&directory, &path)?;
    let existed = entries.remove(name).is_some();
    if existed {
        secure_atomic_write(&path, &render_env_file(&entries))?;
    }
    Ok(existed)
}

fn parse_env_file(source: &str) -> Result<BTreeMap<String, String>, AgentError> {
    let mut entries = BTreeMap::new();
    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line
            .split_once('=')
            .ok_or_else(|| usage(format!("Malformed {PROJECT_ENV_FILE} line {}.", index + 1)))?;
        if !valid_credential_name(name) {
            return Err(usage(format!(
                "Invalid credential name in {PROJECT_ENV_FILE} line {}.",
                index + 1
            )));
        }
        validate_secret(value)?;
        entries.insert(name.to_string(), value.to_string());
    }
    Ok(entries)
}

fn render_env_file(entries: &BTreeMap<String, String>) -> String {
    entries.iter().map(|(name, value)| format!("{name}={value}\n")).collect()
}

fn valid_credential_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), AgentError> {
    if fs::symlink_metadata(path).map_err(|e| ioerr(e.to_string()))?.file_type().is_symlink() {
        return Err(usage(format!("Refusing symlinked {label}.")));
    }
    Ok(())
}

#[cfg(unix)]
fn require_private_permissions(
    path: &Path,
    metadata: &cap_std::fs::Metadata,
) -> Result<(), AgentError> {
    use cap_std::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(fail(format!(
            "{} must be private (chmod 600) before Kujo will read it.",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_private_permissions(
    _path: &Path,
    _metadata: &cap_std::fs::Metadata,
) -> Result<(), AgentError> {
    Ok(())
}

fn secure_atomic_write(path: &Path, body: &str) -> Result<(), AgentError> {
    let parent = path.parent().ok_or_else(|| usage("Credential path has no parent directory."))?;
    fs::create_dir_all(parent).map_err(|e| ioerr(e.to_string()))?;
    let mut temp = tempfile::Builder::new()
        .prefix(CREDENTIAL_TEMP_PREFIX)
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|e| ioerr(e.to_string()))?;
    temp.write_all(body.as_bytes()).map_err(|e| ioerr(e.to_string()))?;
    temp.as_file().sync_all().map_err(|e| ioerr(e.to_string()))?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err(usage("Credential file must be a regular file.")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(ioerr(e.to_string())),
    }
    temp.persist(path)
        .map_err(|e| ioerr(format!("Could not promote credential file: {}", e.error)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_project_credential_updates_preserve_every_entry() {
        let root = tempfile::tempdir().unwrap();
        let barrier = std::sync::Barrier::new(12);
        std::thread::scope(|scope| {
            for i in 0..12 {
                let root = root.path();
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    write_project_secret(root, &format!("TEST_{i}"), "placeholder")
                        .unwrap_or_else(|error| panic!("{}", error.message));
                });
            }
        });
        let root = root.path();
        let directory = project_directory(root).unwrap_or_else(|error| panic!("{}", error.message));
        let entries = read_project_entries(&directory, &root.join(PROJECT_ENV_FILE))
            .unwrap_or_else(|error| panic!("{}", error.message));
        assert_eq!(entries.len(), 12);
        assert!(entries.values().all(|value| value == "placeholder"));
    }

    #[test]
    fn failed_credential_publication_cleans_up_staging_file() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("existing");
        fs::create_dir(&destination).unwrap();
        assert!(secure_atomic_write(&destination, "placeholder").is_err());
        assert!(destination.is_dir());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
