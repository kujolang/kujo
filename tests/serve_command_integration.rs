use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

const TEST_HOST: &str = "127.0.0.1";

fn serve_spawn_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct HttpResponse {
    status_code: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct ServeProcess {
    child: Child,
    port: u16,
}

impl Drop for ServeProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_{prefix}_{nanos}"));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn kujo_binary() -> String {
    env!("CARGO_BIN_EXE_kujo").to_string()
}

fn find_free_port() -> Option<u16> {
    let listener = match TcpListener::bind((TEST_HOST, 0)) {
        Ok(listener) => listener,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => return None,
        Err(error) => panic!("failed to allocate test port: {error}"),
    };
    Some(listener.local_addr().expect("listener should have local addr").port())
}

fn spawn_serve_process(root: &Path) -> Option<ServeProcess> {
    spawn_serve_process_with_extra_args(root, &[])
}

fn spawn_serve_process_with_extra_args(root: &Path, extra_args: &[&str]) -> Option<ServeProcess> {
    // Avoid TOCTOU port-allocation races across parallel tests:
    // reserve a port and launch the process while holding a shared lock.
    let _spawn_guard = serve_spawn_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut last_port = None;
    for _attempt in 0..5 {
        let port = find_free_port()?;
        last_port = Some(port);

        let mut command = Command::new(kujo_binary());
        command.current_dir(root);
        command.args([
            "serve",
            root.to_str().expect("path should be utf-8"),
            "--host",
            TEST_HOST,
            "--port",
            &port.to_string(),
            "--index",
            "index.html",
            "--cache-max-age",
            "120",
        ]);
        command.args(extra_args);

        let mut child = command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn kujo serve process");

        let mut reached = false;
        for _ in 0..100 {
            if child.try_wait().expect("failed to poll child process").is_some() {
                break;
            }

            if TcpStream::connect((TEST_HOST, port)).is_ok() {
                reached = true;
                break;
            }

            thread::sleep(Duration::from_millis(50));
        }

        if reached {
            if let Some(status) = child.try_wait().expect("failed to poll child process") {
                panic!("kujo serve exited before readiness check completed: {status}");
            }
            return Some(ServeProcess { child, port });
        }

        let _ = child.kill();
        let _ = child.wait();
    }

    panic!(
        "kujo serve did not become reachable after retries (last port: {})",
        last_port.unwrap_or(0)
    );
}

macro_rules! spawn_server_or_skip {
    ($spawn_expr:expr) => {{
        let Some(server) = $spawn_expr else {
            eprintln!(
                "skipping {}: local TCP bind not permitted in this environment",
                stringify!($spawn_expr)
            );
            return;
        };
        server
    }};
}

fn send_http_request(port: u16, request: &str) -> HttpResponse {
    // Under CI contention, the server subprocess can occasionally reset the first connection.
    // Retry with a wider budget so the test reflects response-shape correctness, not socket jitter.
    const RETRIES: usize = 20;
    const RETRY_DELAY_MS: u64 = 50;
    let mut last_error = None;
    for _ in 0..RETRIES {
        match send_http_request_once(port, request) {
            Ok(response) => return response,
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
            }
        }
    }

    panic!(
        "failed to read HTTP response after retries: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    );
}

fn send_http_request_once(port: u16, request: &str) -> Result<HttpResponse, String> {
    let mut stream = TcpStream::connect((TEST_HOST, port))
        .map_err(|error| format!("failed to connect to serve process: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to set read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to set write timeout: {error}"))?;

    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write HTTP request: {error}"))?;
    let _ = stream.shutdown(Shutdown::Write);

    let mut response_bytes = Vec::new();
    match stream.read_to_end(&mut response_bytes) {
        Ok(_) => Ok(parse_http_response(&response_bytes)),
        Err(error) if error.kind() == ErrorKind::ConnectionReset => {
            if response_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                Ok(parse_http_response(&response_bytes))
            } else {
                Err(format!("connection reset before full response: {error}"))
            }
        }
        Err(error) => Err(format!("failed to read HTTP response: {error}")),
    }
}

fn parse_http_response(response_bytes: &[u8]) -> HttpResponse {
    let Some(headers_end) = response_bytes.windows(4).position(|window| window == b"\r\n\r\n")
    else {
        panic!("invalid HTTP response: missing header terminator");
    };

    let headers_blob = &response_bytes[..headers_end + 4];
    let body = response_bytes[headers_end + 4..].to_vec();
    let headers_text = String::from_utf8_lossy(headers_blob);
    let mut lines = headers_text.lines();

    let status_line = lines.next().expect("HTTP response should include status line");
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .expect("status line should include numeric status code")
        .parse::<u16>()
        .expect("status code should be valid u16");

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    HttpResponse { status_code, headers, body }
}

#[test]
fn serve_head_returns_headers_without_body() {
    let root = unique_temp_dir("serve_head");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "HEAD / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(200, response.status_code);
    assert_eq!(
        "text/html; charset=utf-8",
        response.headers.get("content-type").expect("expected Content-Type header")
    );
    assert_eq!(0, response.body.len(), "HEAD responses should not include body bytes");
    assert_eq!(
        "14",
        response.headers.get("content-length").expect("expected Content-Length header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_get_returns_length_type_and_safe_default_headers() {
    let root = unique_temp_dir("serve_get_headers");
    let body = "<h1>Hello</h1>";
    fs::write(root.join("index.html"), body).expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(200, response.status_code);
    assert_eq!(body.as_bytes(), response.body.as_slice());
    assert_eq!(
        "text/html; charset=utf-8",
        response.headers.get("content-type").expect("expected Content-Type header")
    );
    assert_eq!(
        body.len().to_string(),
        *response.headers.get("content-length").expect("expected Content-Length header")
    );
    assert_eq!(
        "nosniff",
        response
            .headers
            .get("x-content-type-options")
            .expect("expected X-Content-Type-Options header")
    );
    assert_eq!(
        "no-referrer",
        response.headers.get("referrer-policy").expect("expected Referrer-Policy header")
    );
    assert_eq!(
        "public, max-age=120",
        response.headers.get("cache-control").expect("expected Cache-Control header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_method_not_allowed_returns_allow_header() {
    let root = unique_temp_dir("serve_method_not_allowed");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(405, response.status_code);
    assert_eq!("GET, HEAD", response.headers.get("allow").expect("expected Allow header"));
    assert_eq!(
        "text/plain; charset=utf-8",
        response.headers.get("content-type").expect("expected Content-Type header")
    );
    assert_eq!(
        "no-store",
        response.headers.get("cache-control").expect("expected Cache-Control header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_non_standard_method_returns_501() {
    let root = unique_temp_dir("serve_non_standard_method");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "BREW / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(501, response.status_code);
    assert_eq!(
        "text/plain; charset=utf-8",
        response.headers.get("content-type").expect("expected Content-Type header")
    );
    assert_eq!(
        "no-store",
        response.headers.get("cache-control").expect("expected Cache-Control header")
    );
    assert_eq!(
        "nosniff",
        response
            .headers
            .get("x-content-type-options")
            .expect("expected X-Content-Type-Options header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_range_returns_partial_content_and_content_range_header() {
    let root = unique_temp_dir("serve_range");
    fs::write(root.join("index.html"), b"0123456789").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
		server.port,
		"GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nRange: bytes=2-5\r\nConnection: close\r\n\r\n",
	);

    assert_eq!(206, response.status_code);
    assert_eq!(
        "bytes 2-5/10",
        response.headers.get("content-range").expect("expected Content-Range header")
    );
    assert_eq!(b"2345", response.body.as_slice());

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_invalid_range_returns_416_with_content_range_header() {
    let root = unique_temp_dir("serve_invalid_range");
    fs::write(root.join("index.html"), b"0123456789").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nRange: bytes=50-60\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(416, response.status_code);
    assert_eq!(
        "bytes */10",
        response.headers.get("content-range").expect("expected Content-Range header")
    );
    assert_eq!(b"Range Not Satisfiable", response.body.as_slice());

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_large_file_get_returns_expected_content_length_and_body() {
    let root = unique_temp_dir("serve_large_file_get");
    let expected_body = vec![b'x'; 2 * 1024 * 1024 + 333];
    fs::write(root.join("index.html"), &expected_body).expect("failed to write large index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(200, response.status_code);
    assert_eq!(expected_body.len(), response.body.len());
    assert_eq!(expected_body.as_slice(), response.body.as_slice());
    assert_eq!(
        expected_body.len().to_string(),
        *response.headers.get("content-length").expect("expected Content-Length header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_large_file_head_returns_content_length_without_body() {
    let root = unique_temp_dir("serve_large_file_head");
    let expected_len = 2 * 1024 * 1024 + 333;
    let expected_body = vec![b'y'; expected_len];
    fs::write(root.join("index.html"), &expected_body).expect("failed to write large index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "HEAD /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(200, response.status_code);
    assert_eq!(0, response.body.len(), "HEAD responses should not include body bytes");
    assert_eq!(
        expected_len.to_string(),
        *response.headers.get("content-length").expect("expected Content-Length header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_if_none_match_returns_304_for_matching_etag() {
    let root = unique_temp_dir("serve_etag");
    fs::write(root.join("index.html"), b"etag-body").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let first = send_http_request(
        server.port,
        "GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(200, first.status_code);
    let etag = first.headers.get("etag").expect("expected ETag header").to_string();

    let second = send_http_request(
        server.port,
        &format!(
			"GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nIf-None-Match: {etag}\r\nConnection: close\r\n\r\n"
		),
    );

    assert_eq!(304, second.status_code);
    assert_eq!(0, second.body.len());

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_accept_encoding_prefers_gzip_sibling_asset() {
    let root = unique_temp_dir("serve_encoding");
    fs::write(root.join("index.html"), b"plain-body").expect("failed to write index.html");
    let gzip_bytes = vec![0x1f, 0x8b, 0x08, 0x00, 0x13, 0x37, 0x00, 0x00];
    fs::write(root.join("index.html.gz"), &gzip_bytes).expect("failed to write gzip sibling");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
		server.port,
		"GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept-Encoding: gzip\r\nConnection: close\r\n\r\n",
	);

    assert_eq!(200, response.status_code);
    assert_eq!(
        "gzip",
        response.headers.get("content-encoding").expect("expected Content-Encoding header")
    );
    assert_eq!(gzip_bytes, response.body);

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_mime_policy_covers_known_unknown_and_extensionless_assets() {
    let root = unique_temp_dir("serve_mime_policy");
    fs::write(root.join("index.html"), "<h1>home</h1>").expect("failed to write html");
    fs::write(root.join("styles.css"), "body { color: black; }").expect("failed to write css");
    fs::write(root.join("app.js"), "console.log('kujo');").expect("failed to write js");
    fs::write(root.join("feed.xml"), "<root/>").expect("failed to write xml");
    fs::write(root.join("data.json"), "{\"ok\":true}").expect("failed to write json");
    fs::write(root.join("image.png"), [137, 80, 78, 71, 13, 10, 26, 10])
        .expect("failed to write png");
    fs::write(root.join("photo.jpg"), [0xff, 0xd8, 0xff, 0xd9]).expect("failed to write jpg");
    fs::write(root.join("scan.tif"), [0x49, 0x49, 0x2a, 0x00]).expect("failed to write tif");
    fs::write(root.join("vector.SVG"), "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>")
        .expect("failed to write svg");
    fs::write(root.join("mod.wasm"), [0x00, 0x61, 0x73, 0x6d]).expect("failed to write wasm");
    fs::write(root.join("legacy.eot"), [0x4c, 0x50]).expect("failed to write eot");
    fs::write(root.join("font.woff2"), [0x77, 0x4f, 0x46, 0x32]).expect("failed to write woff2");
    fs::write(root.join("song.mp3"), [0x49, 0x44, 0x33]).expect("failed to write mp3");
    fs::write(root.join("video.MOV"), [0x00, 0x00, 0x00, 0x14]).expect("failed to write mov");
    fs::write(root.join("archive.zip"), [0x50, 0x4b, 0x03, 0x04]).expect("failed to write zip");
    fs::write(root.join("bundle.tar"), [0x75, 0x73, 0x74, 0x61]).expect("failed to write tar");
    fs::write(root.join("bundle.tgz"), [0x1f, 0x8b, 0x08]).expect("failed to write tgz");
    fs::write(root.join("bundle.7z"), [0x37, 0x7a, 0xbc, 0xaf]).expect("failed to write 7z");
    fs::write(root.join("doc.pdf"), b"%PDF-1.4").expect("failed to write pdf");
    fs::write(root.join("notes.txt"), "hello").expect("failed to write txt");
    fs::write(root.join("payload.js.unknown"), "console.log('blocked sniff');")
        .expect("failed to write double extension payload");
    fs::write(root.join("payload.unknown"), "<!DOCTYPE html><script>alert(1)</script>")
        .expect("failed to write unknown");
    fs::write(root.join("LICENSE"), "license text").expect("failed to write extensionless file");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let cases = vec![
        ("/index.html", "text/html; charset=utf-8"),
        ("/styles.css", "text/css; charset=utf-8"),
        ("/app.js", "application/javascript; charset=utf-8"),
        ("/feed.xml", "application/xml; charset=utf-8"),
        ("/data.json", "application/json; charset=utf-8"),
        ("/image.png", "image/png"),
        ("/photo.jpg", "image/jpeg"),
        ("/scan.tif", "image/tiff"),
        ("/vector.SVG", "image/svg+xml"),
        ("/mod.wasm", "application/wasm"),
        ("/legacy.eot", "application/vnd.ms-fontobject"),
        ("/font.woff2", "font/woff2"),
        ("/song.mp3", "audio/mpeg"),
        ("/video.MOV", "video/quicktime"),
        ("/archive.zip", "application/zip"),
        ("/bundle.tar", "application/x-tar"),
        ("/bundle.tgz", "application/gzip"),
        ("/bundle.7z", "application/x-7z-compressed"),
        ("/doc.pdf", "application/pdf"),
        ("/notes.txt", "text/plain; charset=utf-8"),
        ("/payload.js.unknown", "application/octet-stream"),
        ("/payload.unknown", "application/octet-stream"),
        ("/LICENSE", "application/octet-stream"),
    ];

    for (path, expected_content_type) in cases {
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        let response = send_http_request(server.port, &request);
        assert_eq!(200, response.status_code, "unexpected status for {path}");
        assert_eq!(
            expected_content_type,
            response
                .headers
                .get("content-type")
                .unwrap_or_else(|| panic!("expected Content-Type for {path}"))
        );
        assert_eq!(
            "nosniff",
            response
                .headers
                .get("x-content-type-options")
                .unwrap_or_else(|| panic!("expected nosniff header for {path}"))
        );
    }

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_dotfile_blocking_takes_priority_over_known_extension_mime_resolution() {
    let root = unique_temp_dir("serve_mime_dotfile_priority");
    fs::write(root.join("index.html"), "<h1>home</h1>").expect("failed to write index");
    fs::write(root.join(".hidden.json"), "{\"secret\":true}").expect("failed to write dotfile");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /.hidden.json HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(403, response.status_code);
    assert_eq!(
        "text/plain; charset=utf-8",
        response.headers.get("content-type").expect("expected Content-Type header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_rejects_url_encoded_parent_traversal() {
    let root = unique_temp_dir("serve_encoded_traversal");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /%2e%2e/secret.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(403, response.status_code);
    assert_eq!(
        "text/plain; charset=utf-8",
        response.headers.get("content-type").expect("expected Content-Type header")
    );
    assert_eq!(
        "no-store",
        response.headers.get("cache-control").expect("expected Cache-Control header")
    );
    assert_eq!(
        "nosniff",
        response
            .headers
            .get("x-content-type-options")
            .expect("expected X-Content-Type-Options header")
    );
    assert_eq!(
        "no-referrer",
        response.headers.get("referrer-policy").expect("expected Referrer-Policy header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_double_encoded_parent_traversal_does_not_escape_root() {
    let root = unique_temp_dir("serve_double_encoded_traversal");
    let outside = unique_temp_dir("serve_double_encoded_traversal_outside");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");
    fs::write(outside.join("secret.txt"), "outside-secret")
        .expect("failed to write outside secret");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /%252e%252e/secret.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_ne!(
        200, response.status_code,
        "double-encoded traversal should never serve outside-root content"
    );
    assert_ne!(b"outside-secret", response.body.as_slice());

    drop(server);
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
}

#[test]
fn serve_rejects_invalid_percent_encoding_with_400() {
    let root = unique_temp_dir("serve_invalid_percent_encoding");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /bad%2 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(400, response.status_code);

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_rejects_null_byte_in_request_target_with_400() {
    let root = unique_temp_dir("serve_null_byte_request_target");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /%00secret.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(400, response.status_code);

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_rejects_fragment_in_request_target_with_400() {
    let root = unique_temp_dir("serve_fragment_request_target");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /index.html#fragment HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(400, response.status_code);

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_oversized_request_target_returns_414() {
    let root = unique_temp_dir("serve_oversized_request_target");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let oversized_path = format!("/{}", "a".repeat(10_000));
    let request =
        format!("GET {oversized_path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(server.port, &request);

    assert_eq!(414, response.status_code);

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_oversized_request_line_returns_414() {
    let root = unique_temp_dir("serve_oversized_request_line");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let oversized_method = "M".repeat(9_000);
    let request = format!(
        "{oversized_method} /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    );

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(server.port, &request);

    assert_eq!(414, response.status_code);

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_oversized_headers_return_413() {
    let root = unique_temp_dir("serve_oversized_headers");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process_with_extra_args(
        &root,
        &["--max-header-bytes", "256"]
    ));
    let huge_header_value = "x".repeat(1024);
    let request = format!(
        "GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Big: {huge_header_value}\r\nConnection: close\r\n\r\n"
    );
    let response = send_http_request(server.port, &request);

    assert_eq!(413, response.status_code);

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_too_many_headers_return_413() {
    let root = unique_temp_dir("serve_too_many_headers");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process_with_extra_args(
        &root,
        &["--max-header-count", "4"]
    ));

    let mut request = String::from("GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\n");
    for index in 0..6 {
        request.push_str(&format!("X-Header-{index}: value\r\n"));
    }
    request.push_str("Connection: close\r\n\r\n");

    let response = send_http_request(server.port, &request);
    assert_eq!(413, response.status_code);

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_request_body_over_limit_returns_413_before_method_dispatch() {
    let root = unique_temp_dir("serve_request_body_limit");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process_with_extra_args(
        &root,
        &["--max-request-body-bytes", "8"]
    ));
    let oversized_body = "ABCDEFGHIJKL";
    let request = format!(
        "POST /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        oversized_body.len(),
        oversized_body
    );

    let response = send_http_request(server.port, &request);
    assert_eq!(
        413, response.status_code,
        "expected oversized request body to be rejected before method handling"
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_custom_timeout_flags_still_serve_normal_requests() {
    let root = unique_temp_dir("serve_custom_timeouts");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process_with_extra_args(
        &root,
        &["--read-timeout-ms", "25", "--write-timeout-ms", "25"],
    ));
    let response = send_http_request(
        server.port,
        "GET /index.html HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(200, response.status_code);
    assert_eq!(b"<h1>Hello</h1>", response.body.as_slice());

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_query_string_does_not_change_filesystem_resolution() {
    let root = unique_temp_dir("serve_query_string_path_resolution");
    fs::write(root.join("index.html"), "<h1>Hello Query</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /index.html?path=%2e%2e%2fsecret.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(200, response.status_code);
    assert_eq!(b"<h1>Hello Query</h1>", response.body.as_slice());

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_missing_file_returns_404_with_standard_error_headers() {
    let root = unique_temp_dir("serve_missing_file");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /missing.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(404, response.status_code);
    assert_eq!(
        "text/plain; charset=utf-8",
        response.headers.get("content-type").expect("expected Content-Type header")
    );
    assert_eq!(
        "no-store",
        response.headers.get("cache-control").expect("expected Cache-Control header")
    );
    assert_eq!(
        "nosniff",
        response
            .headers
            .get("x-content-type-options")
            .expect("expected X-Content-Type-Options header")
    );
    assert_eq!(
        "no-referrer",
        response.headers.get("referrer-policy").expect("expected Referrer-Policy header")
    );

    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_blocks_dotenv_file_with_403() {
    let root = unique_temp_dir("serve_blocks_dotenv");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");
    fs::write(root.join(".env"), "SECRET=1").expect("failed to write .env");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /.env HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(403, response.status_code);
    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_blocks_dot_git_config_with_403() {
    let root = unique_temp_dir("serve_blocks_dot_git");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");
    fs::create_dir_all(root.join(".git")).expect("failed to create .git directory");
    fs::write(root.join(".git").join("config"), "[core]\nrepositoryformatversion = 0\n")
        .expect("failed to write .git/config");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /.git/config HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(403, response.status_code);
    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_blocks_dot_ds_store_with_403() {
    let root = unique_temp_dir("serve_blocks_ds_store");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");
    fs::write(root.join(".DS_Store"), "mac-metadata").expect("failed to write .DS_Store");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /.DS_Store HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(403, response.status_code);
    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_blocks_backup_and_swap_files_with_403() {
    let root = unique_temp_dir("serve_blocks_backup_swap");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");
    fs::write(root.join("secrets.txt.bak"), "backup copy").expect("failed to write backup file");
    fs::write(root.join("settings.json.swp"), "swap copy").expect("failed to write swap file");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let backup_response = send_http_request(
        server.port,
        "GET /secrets.txt.bak HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    let swap_response = send_http_request(
        server.port,
        "GET /settings.json.swp HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(403, backup_response.status_code);
    assert_eq!(403, swap_response.status_code);
    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn serve_still_serves_normal_public_files() {
    let root = unique_temp_dir("serve_public_files_still_work");
    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");
    fs::write(root.join("public.txt"), "public-data").expect("failed to write public file");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /public.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(200, response.status_code);
    assert_eq!(b"public-data", response.body.as_slice());
    drop(server);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn serve_rejects_symlink_escape_target() {
    let root = unique_temp_dir("serve_symlink_escape");
    let outside = unique_temp_dir("serve_symlink_escape_outside");
    let symlink_path = root.join("linked.txt");
    let outside_file = outside.join("secret.txt");

    fs::write(root.join("index.html"), "<h1>Hello</h1>").expect("failed to write index.html");
    fs::write(&outside_file, "outside").expect("failed to write outside file");
    unix_fs::symlink(&outside_file, &symlink_path).expect("failed to create symlink");

    let server = spawn_server_or_skip!(spawn_serve_process(&root));
    let response = send_http_request(
        server.port,
        "GET /linked.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(403, response.status_code);

    drop(server);
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
}
