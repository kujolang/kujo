use super::*;
use std::sync::{Mutex, OnceLock};

struct Fixture {
    release: Vec<u8>,
    archive: Vec<u8>,
    checksum: Vec<u8>,
    calls: Mutex<Vec<String>>,
}
impl Transport for Fixture {
    fn get(&self, url: &str, _: u64) -> Result<Vec<u8>> {
        self.calls.lock().unwrap().push(url.into());
        if url.starts_with(API) {
            Ok(self.release.clone())
        } else if url.ends_with(".sha256") {
            Ok(self.checksum.clone())
        } else {
            Ok(self.archive.clone())
        }
    }
}
fn native_binary() -> &'static [u8] {
    static BINARY: OnceLock<Vec<u8>> = OnceLock::new();
    BINARY.get_or_init(|| {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("fixture.rs");
        fs::write(&source, r#"fn main() {
 let name = std::env::current_exe().unwrap();
 if name.file_stem().unwrap() == "slow" { loop { std::thread::sleep(std::time::Duration::from_secs(1)); } }
 if name.file_stem().unwrap() == "flood" { for _ in 0..10000 { println!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"); } }
 println!("kujo 9.0.0");
}"#).unwrap();
        let binary = temp.path().join(if cfg!(windows) { "fixture.exe" } else { "fixture" });
        assert!(Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .status()
            .unwrap()
            .success());
        fs::read(binary).unwrap()
    })
}
fn tar_bytes(entries: &[(&str, &[u8], u8)]) -> Vec<u8> {
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(gzip);
    for (name, bytes, kind) in entries {
        let mut header = tar::Header::new_gnu();
        // Raw header deliberately permits traversal fixtures rejected by builders.
        header.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_entry_type(tar::EntryType::new(*kind));
        header.set_cksum();
        tar.append(&header, *bytes).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}
fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
fn fixture(version: &str, bytes: &[u8]) -> Fixture {
    let platform = platform(std::env::consts::OS, std::env::consts::ARCH).unwrap();
    let archive = if cfg!(windows) {
        zip_bytes(&[("kujo.exe", bytes)])
    } else {
        tar_bytes(&[("kujo", bytes, b'0')])
    };
    let name =
        format!("kujo-v{version}-{platform}.{}", if cfg!(windows) { "zip" } else { "tar.gz" });
    let checksum = format!("{:x}  {name}\n", Sha256::digest(&archive)).into_bytes();
    let release = serde_json::json!({"tag_name": format!("v{version}"), "draft":false,"prerelease":false,"published_at":"2026-09-05T00:00:00Z", "assets":[
        {"name":name,"browser_download_url":format!("https://github.com/kujolang/kujo/releases/download/v{version}/{name}"),"size":archive.len()},
        {"name":format!("{name}.sha256"),"browser_download_url":format!("https://github.com/kujolang/kujo/releases/download/v{version}/{name}.sha256"),"size":checksum.len()}
    ]});
    Fixture {
        release: serde_json::to_vec(&release).unwrap(),
        archive,
        checksum,
        calls: Mutex::new(vec![]),
    }
}
fn destination(temp: &tempfile::TempDir) -> PathBuf {
    let path = temp.path().join(if cfg!(windows) { "kujo.exe" } else { "kujo" });
    fs::write(&path, native_binary()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}
#[test]
fn versions_and_platforms() {
    assert_eq!(parse_version("v1.2.4").unwrap(), "1.2.4");
    for value in [
        "1.2",
        "01.2.3",
        "^1.2.3",
        ">=1.0.0",
        "1.2.3-rc.1",
        "1.2.3+build",
        "latest",
        " v1.2.3",
        "vv1.2.3",
        "1.2.3/other",
    ] {
        assert!(parse_version(value).is_err(), "{value}");
    }
    assert!(Version::parse("1.10.0").unwrap() > Version::parse("1.9.0").unwrap());
    for (os, arch, expected) in [
        ("linux", "x86_64", "linux-x64"),
        ("linux", "aarch64", "linux-arm64"),
        ("macos", "x86_64", "macos-x64"),
        ("macos", "aarch64", "macos-arm64"),
        ("windows", "x86_64", "windows-x64"),
    ] {
        assert_eq!(platform(os, arch).unwrap(), expected);
    }
    assert!(platform("windows", "aarch64").is_err());
}
#[test]
fn release_resolution_and_rejections() {
    let f = fixture("9.0.0", b"binary");
    let p = platform(std::env::consts::OS, std::env::consts::ARCH).unwrap();
    resolve(&f, None, p).unwrap();
    resolve(&f, Some("v9.0.0"), p).unwrap();
    assert_eq!(*f.calls.lock().unwrap(), [format!("{API}/latest"), format!("{API}/tags/v9.0.0")]);
    assert!(resolve(&f, Some("8.0.0"), p).is_err());
    for (field, value) in [
        ("draft", serde_json::json!(true)),
        ("prerelease", serde_json::json!(true)),
        ("published_at", serde_json::Value::Null),
        ("assets", serde_json::json!([])),
        ("tag_name", serde_json::json!("v9.0.0-rc.1")),
    ] {
        let mut f = fixture("9.0.0", b"binary");
        let mut meta: serde_json::Value = serde_json::from_slice(&f.release).unwrap();
        meta[field] = value;
        f.release = serde_json::to_vec(&meta).unwrap();
        assert!(resolve(&f, None, p).is_err());
    }
    let mut f = fixture("9.0.0", b"binary");
    f.release = b"not JSON".to_vec();
    assert!(resolve(&f, None, p).is_err());
}
#[test]
fn integrity_and_archive_safety() {
    let bytes = b"archive";
    let checksum = format!("{:x}  asset\n", Sha256::digest(bytes));
    verify(bytes, checksum.as_bytes(), "asset").unwrap();
    for checksum in [
        "bad".into(),
        format!("{}  asset", "0".repeat(64)),
        format!("{:x}  other", Sha256::digest(bytes)),
        format!("{checksum}extra"),
    ] {
        assert!(verify(bytes, checksum.as_bytes(), "asset").is_err());
    }
    assert_eq!(extract(&tar_bytes(&[("kujo", b"bin", b'0')]), false).unwrap(), b"bin");
    for name in ["../kujo", "/kujo", "./kujo", "dir/kujo", "kujo.exe", "C:\\kujo"] {
        assert!(extract(&tar_bytes(&[(name, b"bin", b'0')]), false).is_err());
    }
    for kind in [b'1', b'2', b'5', b'6'] {
        assert!(extract(&tar_bytes(&[("kujo", b"bin", kind)]), false).is_err());
    }
    assert!(extract(&tar_bytes(&[("kujo", b"a", b'0'), ("kujo", b"b", b'0')]), false).is_err());
    assert!(extract(&tar_bytes(&[]), false).is_err());
    assert!(extract(b"broken", false).is_err());
    assert_eq!(extract(&zip_bytes(&[("kujo.exe", b"bin")]), true).unwrap(), b"bin");
    for name in ["../kujo.exe", "/kujo.exe", "dir/kujo.exe", "C:\\kujo.exe"] {
        assert!(extract(&zip_bytes(&[(name, b"bin")]), true).is_err());
    }
    assert!(extract(&zip_bytes(&[("kujo.exe", b"a"), ("extra", b"b")]), true).is_err());
    assert!(bounded(io::Cursor::new(b"12345"), 4).is_err());
}
#[test]
fn checks_and_noops_write_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let path = destination(&temp);
    for (target, current, exact, check, status) in [
        ("9.0.0", "8.0.0", None, true, "upgrade_available"),
        ("9.0.0", "9.0.0", Some("9.0.0"), false, "up_to_date"),
        ("9.0.0", "10.0.0", None, false, "newer_local"),
        ("9.0.0", "10.0.0", Some("9.0.0"), true, "newer_local"),
    ] {
        let f = fixture(target, native_binary());
        let result = execute(&f, exact, check, false, &path, current).unwrap();
        assert_eq!(result.status, status);
        assert!(!result.changed);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        assert_eq!(f.calls.lock().unwrap().len(), 1);
    }
    assert!(execute(
        &fixture("9.0.0", native_binary()),
        Some("9.0.0"),
        false,
        false,
        &path,
        "10.0.0"
    )
    .unwrap_err()
    .contains("--allow-downgrade"));
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}
#[test]
fn managed_installs() {
    let temp = tempfile::tempdir().unwrap();
    let path = destination(&temp);
    fs::write(temp.path().join(".crates2.json"), "{}").unwrap();
    assert_eq!(classify(&path).unwrap(), "cargo");
    let f = fixture("9.0.0", native_binary());
    assert!(execute(&f, None, false, false, &path, "8.0.0").unwrap_err().contains("cargo install"));
    assert!(f.calls.lock().unwrap().is_empty());
    let check = execute(&f, None, true, false, &path, "8.0.0").unwrap();
    assert_eq!(check.installation, "cargo");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
    fs::remove_file(temp.path().join(".crates2.json")).unwrap();
    fs::write(temp.path().join("package.json"), r#"{"name":"@kujolang/kujo-linux-x64"}"#).unwrap();
    assert_eq!(classify(&path).unwrap(), "npm");
    for (path, kind) in [
        ("/tmp/node_modules/pkg/bin/kujo", "npm"),
        ("/tmp/.cargo/bin/kujo", "cargo"),
        ("/tmp/project/target/release/kujo", "development"),
        ("/opt/homebrew/Cellar/kujo/bin/kujo", "managed"),
        ("/nix/store/bin/kujo", "managed"),
    ] {
        assert_eq!(classify(Path::new(path)).unwrap(), kind);
    }
}
#[test]
fn replacement_and_backup() {
    let temp = tempfile::tempdir().unwrap();
    let path = destination(&temp);
    let prior = fs::read(&path).unwrap();
    let result =
        execute(&fixture("9.0.0", native_binary()), None, false, false, &path, "8.0.0").unwrap();
    assert!(result.changed);
    assert_eq!(result.status, "upgraded");
    assert_eq!(fs::read(result.backup.unwrap()).unwrap(), prior);
    check_binary(&path, &Version::parse("9.0.0").unwrap(), Duration::from_secs(5)).unwrap();
    assert!(!fs::read_dir(temp.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".kujo-upgrade-")));
    let result =
        execute(&fixture("9.0.0", native_binary()), Some("9.0.0"), false, true, &path, "10.0.0")
            .unwrap();
    assert_eq!(result.status, "downgraded");
}
#[test]
fn concurrent_identity_and_failure_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let path = destination(&temp);
    let original = fingerprint(&path).unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(temp.path().join(".kujo-upgrade.lock"))
        .unwrap();
    lock.try_lock_exclusive().unwrap();
    assert!(execute(&fixture("9.0.0", native_binary()), None, false, false, &path, "8.0.0")
        .unwrap_err()
        .contains("another upgrade"));
    drop(lock);
    assert!(replace(&temp.path().join("missing"), &path, &original).is_err());
    assert_eq!(fingerprint(&path).unwrap().1, original.1);
    fs::write(&path, b"changed").unwrap();
    assert!(unchanged(&path, &original).is_err());
}
#[test]
fn failed_verification_preserves_installation() {
    let temp = tempfile::tempdir().unwrap();
    let path = destination(&temp);
    let prior = fs::read(&path).unwrap();
    let mut f = fixture("9.0.0", native_binary());
    f.archive.pop();
    assert!(execute(&f, None, false, false, &path, "8.0.0").unwrap_err().contains("size"));
    let mut f = fixture("9.0.0", native_binary());
    f.checksum = b"bad".to_vec();
    assert!(execute(&f, None, false, false, &path, "8.0.0").is_err());
    assert!(execute(&fixture("8.0.0", native_binary()), None, false, false, &path, "7.0.0")
        .unwrap_err()
        .contains("version"));
    assert_eq!(fs::read(&path).unwrap(), prior);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2); // runtime + persistent lock
}
#[cfg(unix)]
#[test]
fn symlink_and_subprocess_limits() {
    use std::os::unix::{fs::symlink, fs::PermissionsExt};
    let temp = tempfile::tempdir().unwrap();
    let path = destination(&temp);
    let link = temp.path().join("alias");
    symlink(&path, &link).unwrap();
    assert!(fingerprint(&link).is_err());
    assert_eq!(link.canonicalize().unwrap(), path.canonicalize().unwrap());
    let script = temp.path().join("slow");
    fs::write(&script, "#!/bin/sh\nwhile :; do :; done\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(check_binary(&script, &Version::parse("9.0.0").unwrap(), Duration::from_millis(100))
        .unwrap_err()
        .contains("timed out"));
    fs::write(&script,"#!/bin/sh\nwhile :; do echo aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa; done\n").unwrap();
    assert!(check_binary(&script, &Version::parse("9.0.0").unwrap(), Duration::from_millis(100))
        .is_err());
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o555)).unwrap();
    if unsafe { libc::geteuid() } != 0 {
        assert!(execute(&fixture("9.0.0", native_binary()), None, false, false, &path, "8.0.0")
            .is_err());
    }
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
}
// This helper runs only inside a copied test executable, never the test runner.
#[test]
fn disposable_running_executable_child() {
    if std::env::var_os("KUJO_UPGRADE_TEST_CHILD").is_none() {
        return;
    }
    let path = std::env::current_exe().unwrap().canonicalize().unwrap();
    assert!(path.parent().unwrap().join("disposable-marker").exists());
    let result =
        execute(&fixture("9.0.0", native_binary()), None, false, false, &path, "8.0.0").unwrap();
    assert!(result.changed);
    assert!(result.backup.unwrap().exists());
}
#[test]
fn running_executable_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(if cfg!(windows) { "kujo.exe" } else { "kujo" });
    fs::copy(std::env::current_exe().unwrap(), &path).unwrap();
    // Linux debug test runners exceed release-binary limits. Strip only this
    // disposable copy, retaining exactly the production installation bounds.
    #[cfg(target_os = "linux")]
    assert!(Command::new("strip").arg("--strip-debug").arg(&path).status().unwrap().success());
    fs::write(temp.path().join("disposable-marker"), "").unwrap();
    let status = Command::new(&path)
        .args(["--exact", "upgrade::tests::disposable_running_executable_child", "--nocapture"])
        .env("KUJO_UPGRADE_TEST_CHILD", "1")
        .status()
        .unwrap();
    assert!(status.success());
    check_binary(&path, &Version::parse("9.0.0").unwrap(), Duration::from_secs(5)).unwrap();
}

#[test]
fn local_http_errors_limits_and_timeouts() {
    use std::net::TcpListener;
    fn serve(response: Vec<u8>, delay: Duration) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    other => panic!("fixture accept failed: {other:?}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
            let mut request = Vec::new();
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0 && request.len() + count <= 8192);
                request.extend_from_slice(&buffer[..count]);
            }
            std::thread::sleep(delay);
            let _ = stream.write_all(&response);
        });
        (format!("http://{address}/fixture"), handle)
    }
    // HTTP is enabled only on this test-owned client; production is HTTPS-only.
    let http = Http(
        reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap(),
    );
    for (status, expected) in
        [(404, "not found"), (403, "rate limit"), (429, "rate limit"), (500, "failed")]
    {
        let (url, server) = serve(
            format!("HTTP/1.1 {status} Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .into_bytes(),
            Duration::ZERO,
        );
        let error = http.get(&url, 100).unwrap_err();
        assert!(error.contains(expected), "HTTP {status}: {error}");
        server.join().unwrap();
    }
    for response in [
        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nshort".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n".to_vec(),
    ] {
        let (url, server) = serve(response, Duration::ZERO);
        assert!(http.get(&url, 100).is_err());
        server.join().unwrap();
    }
    let (url, server) = serve(
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok".to_vec(),
        Duration::from_millis(500),
    );
    let slow_http = Http(
        reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap(),
    );
    assert!(slow_http.get(&url, 100).unwrap_err().contains("timeout"));
    server.join().unwrap();
    let (url, server) =
        serve(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok".to_vec(), Duration::ZERO);
    assert_eq!(http.get(&url, 100).unwrap(), b"ok");
    server.join().unwrap();
    assert!(Http::new().unwrap().get("http://127.0.0.1:1/", 100).is_err());
}

#[test]
fn success_json_schema() {
    let temp = tempfile::tempdir().unwrap();
    let path = destination(&temp);
    let outcome =
        execute(&fixture("9.0.0", native_binary()), Some("v9.0.0"), true, false, &path, "8.0.0")
            .unwrap();
    let value = serde_json::to_value(outcome).unwrap();
    let mut keys: Vec<_> = value.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "backup",
            "changed",
            "current_version",
            "destination",
            "installation",
            "platform",
            "status",
            "target_version",
            "upgrade_available"
        ]
    );
    assert_eq!(value["current_version"], "8.0.0");
    assert_eq!(value["target_version"], "9.0.0");
    assert_eq!(value["changed"], false);
    assert_eq!(value["upgrade_available"], true);
    assert!(value["backup"].is_null());
}

#[test]
fn native_subprocess_timeout_and_output_limit() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["slow", "flood"] {
        let path = temp.path().join(format!("{name}{}", if cfg!(windows) { ".exe" } else { "" }));
        fs::write(&path, native_binary()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(check_binary(&path, &Version::parse("9.0.0").unwrap(), Duration::from_millis(200))
            .is_err());
    }
}

#[test]
fn archive_expansion_and_hidden_entries() {
    let mut reader = LimitedReader { inner: io::Cursor::new(b"12345"), remaining: 4 };
    assert!(io::copy(&mut reader, &mut io::sink()).is_err());
    let mut header = tar::Header::new_gnu();
    header.set_path("kujo").unwrap();
    header.set_size(BINARY_LIMIT + 1);
    header.set_mode(0o755);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gzip.write_all(header.as_bytes()).unwrap();
    assert!(extract(&gzip.finish().unwrap(), false).is_err());
    let mut archive = tar_bytes(&[("kujo", b"bin", b'0')]);
    archive.extend(tar_bytes(&[("../unsafe", b"evil", b'0')]));
    assert!(extract(&archive, false).is_err());
    let mut corrupt = tar_bytes(&[("kujo", b"bin", b'0')]);
    let end = corrupt.len();
    corrupt[end - 8] ^= 0xff;
    assert!(extract(&corrupt, false).is_err());
    let mut zip = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
    zip.add_symlink("kujo.exe", "elsewhere", zip::write::SimpleFileOptions::default()).unwrap();
    assert!(extract(&zip.finish().unwrap().into_inner(), true).is_err());
    let mut zip = zip_bytes(&[("kujo.exe", b"a"), ("dupe.exe", b"b")]);
    for i in 0..zip.len() - 8 {
        if &zip[i..i + 8] == b"dupe.exe" {
            zip[i..i + 8].copy_from_slice(b"kujo.exe");
        }
    }
    assert!(extract(&zip, true).is_err());
}

#[test]
fn destination_replaced_during_resolution_is_rejected() {
    struct Mutating<'a> {
        fixture: Fixture,
        path: &'a Path,
    }
    impl Transport for Mutating<'_> {
        fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>> {
            let result = self.fixture.get(url, limit)?;
            if url.starts_with(API) {
                let next = self.path.with_extension("replacement");
                fs::copy(self.path, &next).unwrap();
                fs::rename(next, self.path).unwrap();
            }
            Ok(result)
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let path = destination(&temp);
    let transport = Mutating { fixture: fixture("9.0.0", native_binary()), path: &path };
    assert!(execute(&transport, None, false, false, &path, "8.0.0")
        .unwrap_err()
        .contains("changed"));
    assert_eq!(transport.fixture.calls.lock().unwrap().len(), 1);
}

#[cfg(windows)]
#[test]
fn powershell_release_archive_layout() {
    let temp = tempfile::tempdir().unwrap();
    let binary = destination(&temp);
    let archive = temp.path().join("release.zip");
    assert!(Command::new("pwsh").args(["-NoLogo","-NoProfile","-Command","Compress-Archive -LiteralPath $env:KUJO_UPGRADE_TEST_BINARY -DestinationPath $env:KUJO_UPGRADE_TEST_ARCHIVE"]).env("KUJO_UPGRADE_TEST_BINARY",&binary).env("KUJO_UPGRADE_TEST_ARCHIVE",&archive).status().unwrap().success());
    assert_eq!(extract(&fs::read(archive).unwrap(), true).unwrap(), native_binary());
}
