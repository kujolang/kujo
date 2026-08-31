use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "kujo_{}_{}_{}_{}",
        prefix,
        std::process::id(),
        nanos,
        counter
    ));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn kujo_binary() -> String {
    env!("CARGO_BIN_EXE_kujo").to_string()
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

fn reserve_local_port() -> Option<u16> {
    match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener.local_addr().ok().map(|addr| addr.port()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => None,
        Err(error) => panic!("failed to reserve local port: {error}"),
    }
}

fn spawn_interpreter(script_path: &Path, current_dir: &Path) -> Child {
    Command::new(kujo_binary())
        .current_dir(current_dir)
        .args([
            "run",
            script_path.to_str().expect("script path should be utf-8"),
            "--interpreter",
            "--allow-net-server",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch interpreter process")
}

fn spawn_runtime_with_read_timeout(
    script_path: &Path,
    current_dir: &Path,
    interpreter: bool,
) -> Child {
    let mut command = Command::new(kujo_binary());
    command.current_dir(current_dir).arg("run").arg(script_path);
    if interpreter {
        command.arg("--interpreter");
    }
    command
        .arg("--allow-net-server")
        .env("KUJO_HTTP_SERVER_READ_TIMEOUT_MS", "150")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch routed HTTP runtime")
}

fn terminate_child(mut child: Child) -> Output {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    child.wait_with_output().expect("failed to collect interpreter output")
}

fn send_http_get(port: u16, path: &str) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().expect("socket addr should parse"),
        Duration::from_millis(80),
    )?;

    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;

    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read_size) => {
                response.extend_from_slice(&chunk[..read_size]);
                if response_has_complete_payload(&response) {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                if response.is_empty() {
                    return Err(error);
                }
                break;
            }
            Err(error) => return Err(error),
        }
    }

    if response.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "empty HTTP response"));
    }

    parse_http_response(&response)
}

fn response_has_complete_payload(raw: &[u8]) -> bool {
    let marker = b"\r\n\r\n";
    let Some(headers_end) = raw.windows(marker.len()).position(|window| window == marker) else {
        return false;
    };

    let payload_start = headers_end + marker.len();
    let headers = String::from_utf8_lossy(&raw[..headers_end]);
    let mut expected_len: Option<usize> = None;

    for line in headers.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            expected_len = value.trim().parse::<usize>().ok();
            break;
        }
    }

    match expected_len {
        Some(len) => raw.len() >= payload_start + len,
        None => true,
    }
}

fn parse_http_response(raw: &[u8]) -> std::io::Result<(u16, String)> {
    let text = String::from_utf8_lossy(raw);
    let mut sections = text.splitn(2, "\r\n\r\n");
    let headers = sections.next().unwrap_or("");
    let body = sections.next().unwrap_or("").to_string();

    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid response"))?;

    Ok((status, body))
}

fn wait_for_response(child: &mut Child, port: u16, path: &str) -> Result<(u16, String), String> {
    for _ in 0..300 {
        if let Some(status) = child.try_wait().expect("child status check should succeed") {
            return Err(format!("interpreter exited before serving request: {status}"));
        }

        match send_http_get(port, path) {
            Ok(response) => return Ok(response),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::InvalidData
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::NotConnected
                ) =>
            {
                thread::sleep(Duration::from_millis(40));
            }
            Err(error) => return Err(format!("request failed: {error}")),
        }
    }

    Err("timed out waiting for server response".to_string())
}

fn send_incomplete_http_body(port: u16) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = format!(
        "POST /body HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 4\r\nConnection: close\r\n\r\nx"
    );
    stream.write_all(request.as_bytes())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    parse_http_response(&response)
}

fn send_oversized_http_body(port: u16) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let body = vec![b'x'; 8 * 1024 * 1024 + 1];
    let headers = format!(
        "POST /body HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(&body)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    parse_http_response(&response)
}

#[test]
fn routed_http_request_exposes_socket_peer_in_vm_and_interpreter() {
    for interpreter in [false, true] {
        let Some(port) = reserve_local_port() else {
            eprintln!("Skipping peer identity test: unable to reserve localhost test port");
            return;
        };
        let project_root =
            unique_temp_dir(if interpreter { "http_peer_interpreter" } else { "http_peer_vm" });
        let script_path = project_root.join("main.kujo");
        let script_source = format!(
            "server := http_server({port})\nserver = server.route(\"GET\", \"/peer\", func(req) {{\n    payload := req[\"peer_address\"] + \"|\" + req[\"peer_ip\"] + \"|\" + to_string(req[\"peer_port\"]) + \"|\" + req[\"peer_transport\"]\n    return http_response(200, payload)\n}})\nserver.listen()\n"
        );
        fs::write(&script_path, script_source).expect("failed to write peer identity script");

        let mut child = spawn_runtime_with_read_timeout(&script_path, &project_root, interpreter);
        let response = match wait_for_response(&mut child, port, "/peer") {
            Ok(response) => response,
            Err(message) => {
                let output = terminate_child(child);
                panic!(
                    "{message}; stdout={}; stderr={}",
                    stdout_text(&output),
                    stderr_text(&output)
                );
            }
        };
        let output = terminate_child(child);
        let fields: Vec<&str> = response.1.trim().split('|').collect();
        assert_eq!(
            response.0,
            200,
            "stdout={}; stderr={}",
            stdout_text(&output),
            stderr_text(&output)
        );
        assert_eq!(fields.len(), 4, "unexpected peer payload: {}", response.1);
        assert!(fields[0].starts_with("127.0.0.1:"), "unexpected peer address: {}", fields[0]);
        assert_eq!(fields[1], "127.0.0.1");
        assert!(fields[2].parse::<u16>().is_ok(), "unexpected peer port: {}", fields[2]);
        assert_eq!(fields[3], "tcp");
    }
}

#[test]
fn routed_http_incomplete_body_times_out_before_dispatch_in_vm_and_interpreter() {
    for interpreter in [false, true] {
        let Some(port) = reserve_local_port() else {
            eprintln!("Skipping read deadline test: unable to reserve localhost test port");
            return;
        };
        let project_root = unique_temp_dir(if interpreter {
            "http_timeout_interpreter"
        } else {
            "http_timeout_vm"
        });
        let script_path = project_root.join("main.kujo");
        let script_source = format!(
            "server := http_server({port})\nserver = server.route(\"GET\", \"/health\", func(req) {{ return http_response(200, \"ok\") }})\nserver = server.route(\"POST\", \"/body\", func(req) {{ return http_response(200, \"dispatched\") }})\nserver.listen()\n"
        );
        fs::write(&script_path, script_source).expect("failed to write timeout script");

        let mut child = spawn_runtime_with_read_timeout(&script_path, &project_root, interpreter);
        if let Err(message) = wait_for_response(&mut child, port, "/health") {
            let output = terminate_child(child);
            panic!("{message}; stdout={}; stderr={}", stdout_text(&output), stderr_text(&output));
        }
        let response = match send_incomplete_http_body(port) {
            Ok(response) => response,
            Err(error) => {
                let output = terminate_child(child);
                panic!(
                    "incomplete body request failed: {error}; stdout={}; stderr={}",
                    stdout_text(&output),
                    stderr_text(&output)
                );
            }
        };
        let output = terminate_child(child);
        assert_eq!(
            response.0,
            408,
            "body={}; stdout={}; stderr={}",
            response.1,
            stdout_text(&output),
            stderr_text(&output)
        );
        assert!(!response.1.contains("dispatched"));
    }
}

#[test]
fn routed_http_oversized_body_is_rejected_before_dispatch_in_vm_and_interpreter() {
    for interpreter in [false, true] {
        let Some(port) = reserve_local_port() else {
            eprintln!("Skipping body bound test: unable to reserve localhost test port");
            return;
        };
        let project_root =
            unique_temp_dir(if interpreter { "http_bound_interpreter" } else { "http_bound_vm" });
        let script_path = project_root.join("main.kujo");
        let script_source = format!(
            "server := http_server({port})\nserver = server.route(\"GET\", \"/health\", func(req) {{ return http_response(200, \"ok\") }})\nserver = server.route(\"POST\", \"/body\", func(req) {{ return http_response(200, \"dispatched\") }})\nserver.listen()\n"
        );
        fs::write(&script_path, script_source).expect("failed to write body bound script");

        let mut child = spawn_runtime_with_read_timeout(&script_path, &project_root, interpreter);
        if let Err(message) = wait_for_response(&mut child, port, "/health") {
            let output = terminate_child(child);
            panic!("{message}; stdout={}; stderr={}", stdout_text(&output), stderr_text(&output));
        }
        let response = match send_oversized_http_body(port) {
            Ok(response) => response,
            Err(error) => {
                let output = terminate_child(child);
                panic!(
                    "oversized body request failed: {error}; stdout={}; stderr={}",
                    stdout_text(&output),
                    stderr_text(&output)
                );
            }
        };
        let output = terminate_child(child);
        assert_eq!(
            response.0,
            413,
            "body={}; stdout={}; stderr={}",
            response.1,
            stdout_text(&output),
            stderr_text(&output)
        );
        assert!(!response.1.contains("dispatched"));
    }
}

#[test]
fn http_route_callback_resolves_module_local_helper() {
    let Some(port) = reserve_local_port() else {
        eprintln!("Skipping callback closure test: unable to reserve localhost test port");
        return;
    };

    let project_root = unique_temp_dir("http_route_callback_module_helper");
    let script_path = project_root.join("main.kujo");
    let script_source = format!(
        "func build_payload() {{\n    return \"closure-helper-ok\"\n}}\n\nserver := http_server({port})\nserver = server.route(\"GET\", \"/health\", func(req) {{\n    return http_response(200, build_payload())\n}})\nserver.listen()\n"
    );
    fs::write(&script_path, script_source).expect("failed to write test script");

    let mut child = spawn_interpreter(&script_path, &project_root);
    let response = match wait_for_response(&mut child, port, "/health") {
        Ok(response) => response,
        Err(message) => {
            let output = terminate_child(child);
            panic!("{}; stdout={}; stderr={}", message, stdout_text(&output), stderr_text(&output));
        }
    };

    let output = terminate_child(child);
    assert_eq!(
        response.0,
        200,
        "expected 200 response from closure helper callback, got status={}, body={}, stdout={}, stderr={}",
        response.0,
        response.1,
        stdout_text(&output),
        stderr_text(&output)
    );
    assert_eq!(
        response.1.trim(),
        "closure-helper-ok",
        "unexpected body from closure helper callback, stdout={}, stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn http_route_callback_resolves_module_helper_chaining_to_import() {
    let Some(port) = reserve_local_port() else {
        eprintln!("Skipping callback import-chain test: unable to reserve localhost test port");
        return;
    };

    let project_root = unique_temp_dir("http_route_callback_import_chain");
    let script_path = project_root.join("main.kujo");
    let helper_path = project_root.join("tenant_utils.kujo");

    let helper_source =
        "export func format_tenant_name(name) {\n    return \"tenant:\" + name\n}\n";
    fs::write(&helper_path, helper_source).expect("failed to write helper module");

    let script_source = format!(
        "import tenant_utils\n\nfunc ensure_global_tenant_access(name) {{\n    return format_tenant_name(name)\n}}\n\nserver := http_server({port})\nserver = server.route(\"GET\", \"/tenant\", func(req) {{\n    return http_response(200, ensure_global_tenant_access(\"alpha\"))\n}})\nserver.listen()\n"
    );
    fs::write(&script_path, script_source).expect("failed to write test script");

    let mut child = spawn_interpreter(&script_path, &project_root);
    let response = match wait_for_response(&mut child, port, "/tenant") {
        Ok(response) => response,
        Err(message) => {
            let output = terminate_child(child);
            panic!("{}; stdout={}; stderr={}", message, stdout_text(&output), stderr_text(&output));
        }
    };

    let output = terminate_child(child);
    assert_eq!(
        response.0,
        200,
        "expected 200 response from import-chain callback, got status={}, body={}, stdout={}, stderr={}",
        response.0,
        response.1,
        stdout_text(&output),
        stderr_text(&output)
    );
    assert_eq!(
        response.1.trim(),
        "tenant:alpha",
        "unexpected body from import-chain callback, stdout={}, stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}
