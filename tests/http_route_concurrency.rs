use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn reserve_port() -> Option<u16> {
    match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener.local_addr().ok().map(|address| address.port()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => None,
        Err(error) => panic!("failed to reserve local port: {error}"),
    }
}

fn temp_script(label: &str, source: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "kujo_http_concurrency_{label}_{}_{}_{}.kujo",
        std::process::id(),
        nonce,
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, source).expect("failed to write temporary Kujo script");
    path
}

fn spawn_server(script: &Path, interpreter: bool, maximum: usize) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(script);
    if interpreter {
        command.arg("--interpreter");
    }
    command
        .args(["--allow-net-server", "--allow-clock"])
        .env("KUJO_HTTP_SERVER_MAX_IN_FLIGHT", maximum.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to launch routed HTTP server")
}

fn wait_until_ready(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if request(port, "/ready").is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("server on port {port} did not become ready");
}

fn request(port: u16, path: &str) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().expect("valid test address"),
        Duration::from_millis(500),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")?;
    stream.flush()?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let status = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "missing status"))?;
    let body = text.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default();
    Ok((status, body.to_string()))
}

fn terminate(mut child: Child, script: &Path) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    let _ = fs::remove_file(script);
}

fn for_each_runtime(mut check: impl FnMut(bool)) {
    check(false);
    check(true);
}

#[test]
fn slow_handler_does_not_block_fast_handler() {
    for_each_runtime(|interpreter| {
        let Some(port) = reserve_port() else { return };
        let script = temp_script(
            "overlap",
            &format!(
                "server := http_server({port})\nserver = server.route(\"GET\", \"/ready\", func(req) {{ return http_response(200, \"ready\") }})\nserver = server.route(\"GET\", \"/slow\", func(req) {{ sleep(400) return http_response(200, \"slow\") }})\nserver = server.route(\"GET\", \"/fast\", func(req) {{ return http_response(200, \"fast\") }})\nserver.listen()\n"
            ),
        );
        let child = spawn_server(&script, interpreter, 4);
        wait_until_ready(port);

        let slow = thread::spawn(move || request(port, "/slow"));
        thread::sleep(Duration::from_millis(60));
        let started = Instant::now();
        let (status, body) = request(port, "/fast").expect("fast request should complete");
        let fast_elapsed = started.elapsed();
        let slow_result =
            slow.join().expect("slow request thread should join").expect("slow request");

        assert_eq!((status, body.as_str()), (200, "fast"));
        assert_eq!((slow_result.0, slow_result.1.as_str()), (200, "slow"));
        assert!(fast_elapsed < Duration::from_millis(250), "fast request took {fast_elapsed:?}");
        terminate(child, &script);
    });
}

#[test]
fn concurrent_requests_keep_request_state_isolated() {
    for_each_runtime(|interpreter| {
        let Some(port) = reserve_port() else { return };
        let script = temp_script(
            "isolation",
            &format!(
                "server := http_server({port})\nserver = server.route(\"GET\", \"/ready\", func(req) {{ return http_response(200, \"ready\") }})\nserver = server.route(\"GET\", \"/echo/:id\", func(req) {{ sleep(30) return http_response(200, req[\"params\"][\"id\"]) }})\nserver.listen()\n"
            ),
        );
        let child = spawn_server(&script, interpreter, 8);
        wait_until_ready(port);

        let handles: Vec<_> = (0..8)
            .map(|id| thread::spawn(move || (id, request(port, &format!("/echo/{id}")))))
            .collect();
        for handle in handles {
            let (id, result) = handle.join().expect("request thread should join");
            let (status, body) = result.expect("isolated request should complete");
            assert_eq!(status, 200);
            assert_eq!(body, id.to_string());
        }
        terminate(child, &script);
    });
}

#[test]
fn exhausted_capacity_returns_service_unavailable() {
    for_each_runtime(|interpreter| {
        let Some(port) = reserve_port() else { return };
        let script = temp_script(
            "capacity",
            &format!(
                "server := http_server({port})\nserver = server.route(\"GET\", \"/ready\", func(req) {{ return http_response(200, \"ready\") }})\nserver = server.route(\"GET\", \"/hold\", func(req) {{ sleep(400) return http_response(200, \"done\") }})\nserver.listen()\n"
            ),
        );
        let child = spawn_server(&script, interpreter, 1);
        wait_until_ready(port);

        let first = thread::spawn(move || request(port, "/hold"));
        thread::sleep(Duration::from_millis(60));
        let (status, body) = request(port, "/hold").expect("overload response should complete");
        assert_eq!(status, 503);
        assert_eq!(body, "Service Unavailable");
        assert_eq!(first.join().expect("first request joins").expect("first request").0, 200);
        terminate(child, &script);
    });
}

#[test]
#[cfg(unix)]
fn process_shutdown_terminates_in_flight_handlers() {
    for_each_runtime(|interpreter| {
        let Some(port) = reserve_port() else { return };
        let script = temp_script(
            "shutdown",
            &format!(
                "server := http_server({port})\nserver = server.route(\"GET\", \"/ready\", func(req) {{ return http_response(200, \"ready\") }})\nserver = server.route(\"GET\", \"/hold\", func(req) {{ sleep(350) return http_response(200, \"done\") }})\nserver.listen()\n"
            ),
        );
        let mut child = spawn_server(&script, interpreter, 2);
        wait_until_ready(port);
        let pending = thread::spawn(move || request(port, "/hold"));
        thread::sleep(Duration::from_millis(60));

        let started = Instant::now();
        let signal_result = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };
        assert_eq!(signal_result, 0, "SIGTERM should be delivered");
        let status = child.wait().expect("terminated server should be waitable");
        assert!(status.success(), "graceful server shutdown should succeed");
        assert!(started.elapsed() < Duration::from_secs(1));
        let (pending_status, pending_body) = pending
            .join()
            .expect("pending request thread should join")
            .expect("in-flight request should drain");
        assert_eq!((pending_status, pending_body.as_str()), (200, "done"));
        let _ = fs::remove_file(script);
    });
}
