use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn temp_root(mode: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kujo_http_upload_{}_{}_{}",
        mode,
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn reserve_port() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0").ok()?.local_addr().ok().map(|address| address.port())
}

fn spawn_server(script: &Path, root: &Path, interpreter: bool) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.current_dir(root).arg("run").arg(script);
    if interpreter {
        command.arg("--interpreter");
    }
    command
        .args(["--allow-net-server", "--allow-fs-read", "--allow-fs-write", "--allow-fs-delete"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn stop(mut child: Child) -> Output {
    if child.try_wait().unwrap().is_none() {
        child.kill().unwrap();
    }
    child.wait_with_output().unwrap()
}

fn read_response(mut stream: TcpStream) -> std::io::Result<(u16, String)> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad response"))?;
    Ok((status, body.to_string()))
}

fn request(
    port: u16,
    authorization: &str,
    body: Option<&[u8]>,
    length: usize,
) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    write!(
        stream,
        "POST /upload HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: {authorization}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
    )?;
    if let Some(body) = body {
        stream.write_all(body)?;
    }
    read_response(stream)
}

fn wait_ready(child: &mut Child, port: u16) {
    for _ in 0..300 {
        assert!(child.try_wait().unwrap().is_none(), "server exited before readiness");
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            if stream
                .write_all(
                    format!(
                        "GET /ready HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .is_ok()
                && read_response(stream).is_ok()
            {
                return;
            }
        }
        thread::sleep(Duration::from_millis(30));
    }
    panic!("server did not become ready");
}

#[test]
#[cfg(unix)]
fn routed_upload_preflights_then_streams_private_body_in_vm_and_interpreter() {
    for interpreter in [false, true] {
        let Some(port) = reserve_port() else { return };
        let root = temp_root(if interpreter { "interpreter" } else { "vm" });
        let spool = root.join("spool");
        fs::create_dir(&spool).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&spool, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let script = root.join("main.kujo");
        fs::write(
            &script,
            format!(
                r#"func authorize(req) {{
    return req["body_mode"] == "preflight" && req["headers"]["Authorization"] == "Bearer fixture"
}}
func accept(req) {{
    artifact := req["body_artifact"]
    return http_response(200, to_string(artifact["bytes"]) + "|" + artifact["sha256"] + "|" + req["body_mode"])
}}
server := http_server({port})
server = server.route_upload("POST", "/upload", "{}", 10485760, authorize, accept)
server.listen()
"#,
                spool.to_string_lossy()
            ),
        )
        .unwrap();

        let mut child = spawn_server(&script, &root, interpreter);
        wait_ready(&mut child, port);

        let denied = request(port, "Bearer wrong", None, 9 * 1024 * 1024).unwrap();
        assert_eq!(denied.0, 403);
        assert_eq!(fs::read_dir(&spool).unwrap().count(), 0);

        let body = vec![b'z'; 8 * 1024 * 1024 + 4096];
        let accepted = request(port, "Bearer fixture", Some(&body), body.len()).unwrap();
        let expected_hash = format!("{:x}", Sha256::digest(&body));
        assert_eq!(accepted, (200, format!("{}|{}|private_spool", body.len(), expected_hash)));
        assert_eq!(fs::read_dir(&spool).unwrap().count(), 0);

        let oversized = request(port, "Bearer fixture", None, 10 * 1024 * 1024 + 1).unwrap();
        assert_eq!(oversized.0, 413);
        assert_eq!(fs::read_dir(&spool).unwrap().count(), 0);

        let _output = stop(child);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
#[cfg(not(unix))]
fn routed_upload_fails_closed_without_private_spool_semantics() {
    let Some(port) = reserve_port() else { return };
    let root = temp_root("unsupported_platform");
    let spool = root.join("spool");
    fs::create_dir(&spool).unwrap();
    let script = root.join("main.kujo");
    fs::write(
        &script,
        format!(
            "server := http_server({port})\nserver = server.route_upload(\"POST\", \"/upload\", \"{}\", 1024, func(req) {{ return true }}, func(req) {{ return http_response(200, \"ok\") }})\nserver = server.route(\"GET\", \"/ready\", func(req) {{ return http_response(200, \"ready\") }})\nserver.listen()\n",
            spool.to_string_lossy()
        ),
    )
    .unwrap();
    let mut child = spawn_server(&script, &root, false);
    wait_ready(&mut child, port);
    let response = request(port, "Bearer fixture", Some(b"body"), 4).unwrap();
    assert_eq!(response.0, 500);
    assert_eq!(fs::read_dir(&spool).unwrap().count(), 0);
    let _output = stop(child);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn routed_upload_requires_explicit_filesystem_capabilities() {
    let Some(port) = reserve_port() else { return };
    let root = temp_root("capability");
    let spool = root.join("spool");
    fs::create_dir(&spool).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&spool, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let script = root.join("main.kujo");
    fs::write(
        &script,
        format!(
            "server := http_server({port})\nserver = server.route_upload(\"POST\", \"/upload\", \"{}\", 1024, func(req) {{ return true }}, func(req) {{ return http_response(200, \"ok\") }})\nserver.listen()\n",
            spool.to_string_lossy()
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .current_dir(&root)
        .args(["run", script.to_str().unwrap(), "--allow-net-server"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("filesystem-write"), "{combined}");
    fs::remove_dir_all(root).unwrap();
}
