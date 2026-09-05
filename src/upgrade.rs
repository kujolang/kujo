//! Runtime-only upgrades. Transport injection is private and used only by fixtures.
use fs2::FileExt;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, String>;
const METADATA_LIMIT: u64 = 2 * 1024 * 1024;
const ARCHIVE_LIMIT: u64 = 128 * 1024 * 1024;
const BINARY_LIMIT: u64 = 256 * 1024 * 1024;
const API: &str = "https://api.github.com/repos/kujolang/kujo/releases";

pub fn parse_version(value: &str) -> Result<String> {
    let value = value.strip_prefix('v').unwrap_or(value);
    let version = Version::parse(value).map_err(|_| {
        "use an exact stable version such as 1.2.4 (ranges are unsupported)".to_string()
    })?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err(
            "prerelease and build-metadata versions are unsupported; use an exact stable release"
                .into(),
        );
    }
    Ok(version.to_string())
}

#[derive(Debug, Serialize)]
pub struct Outcome {
    pub current_version: String,
    pub target_version: String,
    pub status: String,
    pub changed: bool,
    pub platform: String,
    pub destination: PathBuf,
    pub installation: String,
    pub upgrade_available: bool,
    pub backup: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
}

trait Transport {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>>;
}
struct Http(reqwest::blocking::Client);
impl Http {
    fn new() -> Result<Self> {
        reqwest::blocking::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(concat!("kujo-upgrade/", env!("CARGO_PKG_VERSION")))
            .build()
            .map(Self)
            .map_err(|e| e.to_string())
    }
}
fn bounded(mut reader: impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("interrupted or invalid download: {e}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!("input exceeds {limit} byte limit"));
    }
    Ok(bytes)
}
impl Transport for Http {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>> {
        let response =
            self.0.get(url).send().map_err(|e| {
                format!("release request failed (check connectivity/timeouts): {e}")
            })?;
        match response.status().as_u16() {
            403 | 429 => return Err("GitHub denied the request or rate limit was reached; retry later".into()),
            404 => return Err("published release or required asset not found; check the version on kujolang/kujo releases".into()),
            _ => (),
        }
        let response = response
            .error_for_status()
            .map_err(|e| format!("GitHub release request failed: {e}"))?;
        let length = response.content_length();
        if length.is_some_and(|n| n > limit) {
            return Err("release response is too large".into());
        }
        let bytes = bounded(response, limit)?;
        if length.is_some_and(|n| n != bytes.len() as u64) {
            return Err("truncated release response".into());
        }
        Ok(bytes)
    }
}
fn platform(os: &str, arch: &str) -> Result<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Ok("linux-x64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("macos", "x86_64") => Ok("macos-x64"),
        ("macos", "aarch64") => Ok("macos-arm64"),
        ("windows", "x86_64") => Ok("windows-x64"),
        _ => Err(format!("no official release binary for {os}-{arch}")),
    }
}
struct ResolvedRelease {
    version: Version,
    name: String,
    archive_url: String,
    checksum_url: String,
    size: u64,
}
fn resolve(
    transport: &impl Transport,
    exact: Option<&str>,
    platform: &str,
) -> Result<ResolvedRelease> {
    let url = match exact {
        Some(v) => format!("{API}/tags/v{}", parse_version(v)?),
        None => format!("{API}/latest"),
    };
    let release: Release = serde_json::from_slice(&transport.get(&url, METADATA_LIMIT)?)
        .map_err(|e| format!("invalid release metadata: {e}"))?;
    let version = Version::parse(&parse_version(&release.tag_name)?).map_err(|e| e.to_string())?;
    if release.draft
        || release.prerelease
        || release.published_at.as_deref().is_none_or(str::is_empty)
        || release.tag_name != format!("v{version}")
    {
        return Err("target is not a published stable vMAJOR.MINOR.PATCH release".into());
    }
    if exact.is_some_and(|v| parse_version(v).ok().as_deref() != Some(version.to_string().as_str()))
    {
        return Err("release tag does not match requested version".into());
    }
    let ext = if platform == "windows-x64" { "zip" } else { "tar.gz" };
    let name = format!("kujo-v{version}-{platform}.{ext}");
    let asset = |name: &str, limit| -> Result<(String, u64)> {
        let entries: Vec<_> = release.assets.iter().filter(|a| a.name == name).collect();
        if entries.len() != 1 {
            return Err(format!("release must contain exactly one {name} asset"));
        }
        let a = entries[0];
        let expected =
            format!("https://github.com/kujolang/kujo/releases/download/v{version}/{name}");
        if a.browser_download_url != expected || a.size == 0 || a.size > limit {
            return Err(format!("invalid official asset URL or size: {name}"));
        }
        Ok((a.browser_download_url.clone(), a.size))
    };
    let (archive, size) = asset(&name, ARCHIVE_LIMIT)?;
    let (checksum, _) = asset(&format!("{name}.sha256"), 4096)?;
    Ok(ResolvedRelease { version, name, archive_url: archive, checksum_url: checksum, size })
}
fn verify(bytes: &[u8], checksum: &[u8], name: &str) -> Result<()> {
    let text = std::str::from_utf8(checksum).map_err(|_| "checksum is not UTF-8")?;
    let fields: Vec<_> = text.split_whitespace().collect();
    if fields.len() != 2
        || fields[1].trim_start_matches('*') != name
        || fields[0].len() != 64
        || !fields[0].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("malformed SHA-256 checksum or mismatched archive filename".into());
    }
    if format!("{:x}", Sha256::digest(bytes)) != fields[0].to_ascii_lowercase() {
        return Err("archive SHA-256 mismatch; nothing was executed or installed".into());
    }
    Ok(())
}
struct LimitedReader<R> {
    inner: R,
    remaining: u64,
}
impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut extra = [0];
            return if self.inner.read(&mut extra)? == 0 {
                Ok(0)
            } else {
                Err(io::Error::other("expanded archive exceeds size limit"))
            };
        }
        let limit = buf.len().min(self.remaining as usize);
        let count = self.inner.read(&mut buf[..limit])?;
        self.remaining -= count as u64;
        Ok(count)
    }
}
// ZipArchive indexes entries by name and collapses duplicates. Validate the
// single-entry, non-ZIP64 envelope independently before asking it to decompress.
// Official Windows artifacts are below ZIP64 limits and have no ZIP comment.
fn zip_envelope(bytes: &[u8]) -> Result<usize> {
    let end = bytes.len().checked_sub(22).ok_or("truncated ZIP footer")?;
    let footer = &bytes[end..];
    if &footer[..4] != b"PK\x05\x06"
        || footer[4..8] != [0, 0, 0, 0]
        || footer[8..12] != [1, 0, 1, 0]
        || footer[20..22] != [0, 0]
    {
        return Err(
            "ZIP must contain exactly one entry, without ZIP64, spanning, or comments".into()
        );
    }
    let size = u32::from_le_bytes(footer[12..16].try_into().unwrap()) as usize;
    let start = u32::from_le_bytes(footer[16..20].try_into().unwrap()) as usize;
    if start.checked_add(size) != Some(end) || !bytes.starts_with(b"PK\x03\x04") {
        return Err("invalid ZIP directory envelope".into());
    }
    Ok(start)
}
fn extract(bytes: &[u8], windows: bool) -> Result<Vec<u8>> {
    let expected = if windows { "kujo.exe" } else { "kujo" };
    let mut binary = None;
    if windows {
        let directory_start = zip_envelope(bytes)?;
        let mut archive =
            zip::ZipArchive::new(io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
        if archive.len() != 1 {
            return Err("archive must contain exactly one regular executable".into());
        }
        let entry = archive.by_index(0).map_err(|e| e.to_string())?;
        let local = bytes.get(..30).ok_or("truncated ZIP local header")?;
        let name_len = u16::from_le_bytes([local[26], local[27]]) as usize;
        let has_descriptor = local[6] & 8 != 0;
        let payload_end =
            entry.data_start().checked_add(entry.compressed_size()).ok_or("invalid ZIP size")?;
        let gap =
            (directory_start as u64).checked_sub(payload_end).ok_or("overlapping ZIP directory")?;
        if entry.header_start() != 0
            || entry.central_header_start() != directory_start as u64
            || bytes.get(30..30 + name_len) != Some(expected.as_bytes())
            || if has_descriptor { !matches!(gap, 12 | 16) } else { gap != 0 }
        {
            return Err("ZIP has unexpected local entries or payload layout".into());
        }
        let mode = entry.unix_mode().unwrap_or(0o100644) & 0o170000;
        if entry.name() != expected
            || !matches!(mode, 0 | 0o100000)
            || entry.is_dir()
            || entry.size() > BINARY_LIMIT
        {
            return Err("unsafe ZIP entry".into());
        }
        binary = Some(bounded(entry, BINARY_LIMIT)?);
    } else {
        let decoder = flate2::read::MultiGzDecoder::new(bytes);
        // Read through terminators and concatenated members so hidden duplicate
        // entries, gzip checksum failures, and excessive padding are rejected.
        let mut archive = tar::Archive::new(LimitedReader {
            inner: decoder,
            remaining: BINARY_LIMIT + 1024 * 1024,
        });
        archive.set_ignore_zeros(true);
        for entry in archive.entries().map_err(|e| e.to_string())?.raw(true) {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path_bytes().as_ref() != expected.as_bytes()
                || !entry.header().entry_type().is_file()
                || binary.is_some()
                || entry.size() > BINARY_LIMIT
            {
                return Err("unsafe, duplicate, or oversized TAR entry".into());
            }
            binary = Some(bounded(entry, BINARY_LIMIT)?);
        }
    }
    binary.filter(|b| !b.is_empty()).ok_or_else(|| "archive has no executable".into())
}

fn classify(path: &Path) -> Result<String> {
    let components: Vec<_> =
        path.iter().map(|c| c.to_string_lossy().to_ascii_lowercase()).collect();
    if components.iter().any(|c| c == "node_modules") {
        return Ok("npm".into());
    }
    for ancestor in path.ancestors().skip(1).take(4) {
        let manifest = ancestor.join("package.json");
        if manifest.is_file() {
            let data = bounded(File::open(&manifest).map_err(|e| e.to_string())?, 64 * 1024)?;
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&data) {
                if value["name"].as_str().is_some_and(|n| n.starts_with("@kujolang/kujo-")) {
                    return Ok("npm".into());
                }
            }
        }
        if ancestor.join(".crates.toml").exists() || ancestor.join(".crates2.json").exists() {
            return Ok("cargo".into());
        }
    }
    if components.iter().any(|c| c == ".cargo") {
        return Ok("cargo".into());
    }
    if components.windows(2).any(|c| c[0] == "target" && (c[1] == "debug" || c[1] == "release"))
        || components.iter().any(|c| c == "target")
    {
        return Ok("development".into());
    }
    if components.iter().any(|c| {
        matches!(
            c.as_str(),
            "cellar"
                | "homebrew"
                | "nix"
                | "scoop"
                | "chocolatey"
                | "snap"
                | "winget"
                | "windowsapps"
        )
    }) || path.starts_with("/usr/bin")
        || path.starts_with("/bin")
    {
        return Ok("managed".into());
    }
    if path.file_name().and_then(|n| n.to_str())
        != Some(if cfg!(windows) { "kujo.exe" } else { "kujo" })
    {
        return Err("unrecognized executable name; use your original installer".into());
    }
    Ok("standalone".into())
}
fn guidance(kind: &str) -> String {
    match kind {
        "npm" => "npm-managed runtime: use npm install --global @kujolang/kujo-runtime@VERSION (or your original project's package manager)",
        "cargo" => "Cargo-managed runtime: use cargo install kujolang --version VERSION --locked --force",
        "development" => "development target binary: rebuild the checkout; install an official standalone binary to use upgrade",
        _ => "managed installation: upgrade with its original package manager",
    }.into()
}
fn fingerprint(path: &Path) -> Result<(fs::Metadata, Vec<u8>, same_file::Handle)> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > BINARY_LIMIT {
        return Err("destination is not a regular bounded executable".into());
    }
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let copied = io::copy(&mut Read::by_ref(&mut file).take(BINARY_LIMIT + 1), &mut hash)
        .map_err(|e| e.to_string())?;
    if copied > BINARY_LIMIT {
        return Err("destination exceeds size limit".into());
    }
    Ok((
        metadata,
        hash.finalize().to_vec(),
        same_file::Handle::from_path(path).map_err(|e| e.to_string())?,
    ))
}
fn unchanged(path: &Path, old: &(fs::Metadata, Vec<u8>, same_file::Handle)) -> Result<()> {
    let now = fingerprint(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if now.0.dev() != old.0.dev() || now.0.ino() != old.0.ino() {
            return Err("destination identity changed during upgrade".into());
        }
    }
    if now.2 != old.2
        || now.0.len() != old.0.len()
        || now.0.modified().ok() != old.0.modified().ok()
        || now.1 != old.1
    {
        return Err("destination changed during upgrade".into());
    }
    Ok(())
}
fn check_binary(path: &Path, version: &Version, timeout: Duration) -> Result<()> {
    let mut child = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot execute staged binary: {e}"))?;
    let stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || bounded(stdout, 4096));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("staged --version failed or timed out: {result:?}"));
            }
        }
    };
    // Official Kujo --version does not spawn descendants. Do not wait indefinitely
    // for a pipe retained by an unexpected descendant.
    while !reader.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if !reader.is_finished() {
        return Err("staged version output timed out".into());
    }
    let output = reader.join().map_err(|_| "version reader failed")??;
    if !status.success()
        || std::str::from_utf8(&output).ok().map(str::trim)
            != Some(format!("kujo {version}").as_str())
    {
        return Err("staged binary version does not match release metadata".into());
    }
    Ok(())
}

pub fn run(exact: Option<String>, check: bool, allow_downgrade: bool) -> Result<Outcome> {
    let destination = std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|e| format!("cannot resolve running executable: {e}"))?;
    execute(
        &Http::new()?,
        exact.as_deref(),
        check,
        allow_downgrade,
        &destination,
        env!("CARGO_PKG_VERSION"),
    )
}
fn execute(
    transport: &impl Transport,
    exact: Option<&str>,
    check: bool,
    allow_downgrade: bool,
    destination: &Path,
    current: &str,
) -> Result<Outcome> {
    let platform = platform(std::env::consts::OS, std::env::consts::ARCH)?;
    let current = Version::parse(current).map_err(|e| e.to_string())?;
    let installation = classify(destination)?;
    if !check && installation != "standalone" {
        return Err(guidance(&installation));
    }
    let original = fingerprint(destination)?;
    let ResolvedRelease { version: target, name, archive_url, checksum_url, size } =
        resolve(transport, exact, platform)?;
    let status = if target == current {
        "up_to_date"
    } else if target < current {
        "newer_local"
    } else {
        "upgrade_available"
    };
    let mut outcome = Outcome {
        current_version: current.to_string(),
        target_version: target.to_string(),
        status: status.into(),
        changed: false,
        platform: platform.into(),
        destination: destination.into(),
        installation,
        upgrade_available: target > current,
        backup: None,
    };
    if check || target == current || (target < current && exact.is_none()) {
        return Ok(outcome);
    }
    if target < current && !allow_downgrade {
        return Err("explicit downgrade requires --allow-downgrade".into());
    }
    let parent = destination.parent().ok_or("destination has no parent")?;
    // Persistent advisory lock: unlinking after unlock races with waiting upgraders.
    let lock_path = parent.join(".kujo-upgrade.lock");
    if fs::symlink_metadata(&lock_path).is_ok_and(|m| !m.is_file()) {
        return Err("unsafe upgrade lock path".into());
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|e| format!("cannot lock destination directory: {e}"))?;
    lock.try_lock_exclusive().map_err(|_| "another upgrade holds the destination lock")?;
    unchanged(destination, &original)?;
    eprintln!("Downloading {name}; destination {}", destination.display());
    let checksum = transport.get(&checksum_url, 4096)?;
    let archive = transport.get(&archive_url, ARCHIVE_LIMIT)?;
    if archive.len() as u64 != size {
        return Err("archive download size differs from release metadata".into());
    }
    verify(&archive, &checksum, &name)?;
    let binary = extract(&archive, platform == "windows-x64")?;
    let mut staged = tempfile::Builder::new()
        .prefix(".kujo-upgrade-")
        .suffix(if cfg!(windows) { ".exe" } else { "" })
        .tempfile_in(parent)
        .map_err(|e| format!("cannot stage beside destination: {e}"))?;
    staged
        .write_all(&binary)
        .and_then(|_| staged.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        staged
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    let staged = staged.into_temp_path(); // Close writable handle before execution.
    check_binary(&staged, &target, Duration::from_secs(10))?;
    unchanged(destination, &original)?;
    if classify(destination)? != "standalone" {
        return Err("installation ownership changed during upgrade".into());
    }
    let backup = replace(&staged, destination, &original)?;
    outcome.status = if target < current { "downgraded" } else { "upgraded" }.into();
    outcome.changed = true;
    outcome.backup = Some(backup);
    Ok(outcome)
}
fn replace(
    staged: &Path,
    destination: &Path,
    original: &(fs::Metadata, Vec<u8>, same_file::Handle),
) -> Result<PathBuf> {
    let backup = destination.with_file_name(format!(
        "kujo-backup-{}{}",
        uuid::Uuid::new_v4(),
        if cfg!(windows) { ".exe" } else { "" }
    ));
    #[cfg(unix)]
    {
        let mut prior = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)
            .map_err(|e| e.to_string())?;
        let result = (|| -> Result<()> {
            io::copy(&mut File::open(destination).map_err(|e| e.to_string())?, &mut prior)
                .map_err(|e| e.to_string())?;
            prior
                .set_permissions(original.0.permissions())
                .and_then(|_| prior.sync_all())
                .map_err(|e| e.to_string())?;
            unchanged(destination, original)?;
            fs::rename(staged, destination)
                .map_err(|e| format!("replacement failed; prior runtime remains installed: {e}"))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&backup);
        }
        result?;
        // Rename is atomic on this filesystem; power-loss durability is not universal.
        if let Some(parent) = destination.parent() {
            let _ = File::open(parent).and_then(|f| f.sync_all());
        }
    }
    #[cfg(windows)]
    {
        unchanged(destination, original)?;
        fs::rename(destination, &backup)
            .map_err(|e| format!("cannot move running executable to backup: {e}"))?;
        if let Err(error) = fs::rename(staged, destination) {
            return match fs::rename(&backup, destination) {
                Ok(()) => Err(format!("replacement failed; prior runtime restored: {error}")),
                Err(restore) => Err(format!(
                    "replacement failed: {error}; restore {} to {} after exiting Kujo: {restore}",
                    backup.display(),
                    destination.display()
                )),
            };
        }
    }
    Ok(backup)
}

#[cfg(test)]
mod tests;
