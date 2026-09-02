use kujo::runtime_limits;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);
const FS_MAX_READ_BYTES_FOR_TEST: usize = runtime_limits::MAX_FILE_IO_BYTES;
const FS_MAX_WRITE_BYTES_FOR_TEST: usize = runtime_limits::MAX_FILE_IO_BYTES;
const NETWORK_MAX_BODY_BYTES_FOR_TEST: usize = runtime_limits::MAX_NETWORK_BODY_BYTES;

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

fn run_kujo(args: &[&str], current_dir: &Path) -> Output {
    Command::new(kujo_binary())
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("failed to execute kujo binary")
}

fn run_kujo_with_env(args: &[&str], current_dir: &Path, env_pairs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(kujo_binary());
    command.current_dir(current_dir).args(args);
    for (key, value) in env_pairs {
        command.env(key, value);
    }
    command.output().expect("failed to execute kujo binary")
}

fn read_http_request(stream: &mut TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut request = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read_size) => {
                request.extend_from_slice(&chunk[..read_size]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

fn spawn_one_shot_http_server(
    body: Vec<u8>,
    response_delay: Duration,
) -> Option<(u16, thread::JoinHandle<()>)> {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return None,
        Err(error) => panic!("failed to bind local HTTP test listener: {error}"),
    };
    let port = listener.local_addr().expect("local addr should resolve").port();

    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            read_http_request(&mut stream);
            if !response_delay.is_zero() {
                thread::sleep(response_delay);
            }
            let response_headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(response_headers.as_bytes());
            let _ = stream.write_all(&body);
            let _ = stream.flush();
        }
    });

    Some((port, handle))
}

fn spawn_one_shot_redirect_server(location: String) -> Option<(u16, thread::JoinHandle<()>)> {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return None,
        Err(error) => panic!("failed to bind local HTTP redirect listener: {error}"),
    };
    let port = listener.local_addr().expect("local addr should resolve").port();
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            read_http_request(&mut stream);
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    Some((port, handle))
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

fn escape_kujo_string(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

fn run_unzip_script_with_archive<F>(prefix: &str, archive_builder: F) -> (PathBuf, PathBuf, Output)
where
    F: FnOnce(&Path),
{
    let project_root = unique_temp_dir(prefix);
    let zip_path = project_root.join("payload.zip");
    let output_dir = project_root.join("unzipped");
    archive_builder(&zip_path);

    let script_path = project_root.join("boundary.kujo");
    let script_source = format!(
        "unzip(\"{}\", \"{}\")\n",
        escape_kujo_string(zip_path.to_str().expect("zip path should be utf-8")),
        escape_kujo_string(output_dir.to_str().expect("output path should be utf-8")),
    );
    fs::write(&script_path, script_source).expect("failed to write unzip script");

    let output = run_kujo(
        &["run", script_path.to_str().expect("script path should be utf-8"), "--interpreter"],
        &project_root,
    );

    (project_root, output_dir, output)
}

fn assert_unzip_failure(output: &Output, expected_runtime_error: &str) {
    assert_eq!(
        output.status.code(),
        Some(4),
        "expected unzip boundary failure with exit code 4, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(output),
        stderr_text(output)
    );

    let combined_output = format!("{}\n{}", stdout_text(output), stderr_text(output));
    assert!(
        combined_output.contains(expected_runtime_error),
        "expected runtime error text '{}' in output, got stdout={} stderr={}",
        expected_runtime_error,
        stdout_text(output),
        stderr_text(output)
    );
}

fn write_zip_file_entry(
    writer: &mut ZipWriter<fs::File>,
    entry_name: &str,
    contents: &[u8],
    unix_mode: Option<u32>,
) {
    let mut options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    if let Some(mode) = unix_mode {
        options = options.unix_permissions(mode);
    }
    writer.start_file(entry_name, options).expect("failed to start zip file entry");
    writer.write_all(contents).expect("failed to write zip file entry contents");
}

fn create_zip_archive<F>(zip_path: &Path, builder: F)
where
    F: FnOnce(&mut ZipWriter<fs::File>),
{
    let file = fs::File::create(zip_path).expect("failed to create zip archive");
    let mut writer = ZipWriter::new(file);
    builder(&mut writer);
    writer.finish().expect("failed to finalize zip archive");
}

fn mark_first_zip_entry_as_symlink(zip_path: &Path) {
    let mut archive_bytes = fs::read(zip_path).expect("failed to read zip archive bytes");
    let central_directory_signature = [0x50, 0x4b, 0x01, 0x02];
    let Some(header_start) =
        archive_bytes.windows(4).position(|window| window == central_directory_signature)
    else {
        panic!("expected central directory header in zip archive");
    };

    // Mark as Unix host so unix_mode() is populated by zip::read::ZipFile.
    archive_bytes[header_start + 5] = 3;

    // Central directory external attributes field (offset 38) stores unix mode in the upper 16 bits.
    let symlink_mode_external_attrs = (0o120777_u32) << 16;
    archive_bytes[header_start + 38..header_start + 42]
        .copy_from_slice(&symlink_mode_external_attrs.to_le_bytes());

    fs::write(zip_path, archive_bytes).expect("failed to write patched zip archive bytes");
}

fn assert_runtime_boundary_failure(script_source: &str, expected_runtime_error: &str) {
    let project_root = unique_temp_dir("native_api_security_boundary");
    let script_path = project_root.join("boundary.kujo");
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &["run", script_path.to_str().expect("script path should be utf-8"), "--interpreter"],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected runtime misuse to exit with code 4, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );

    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains(expected_runtime_error),
        "expected runtime error text '{}' in output, got stdout={} stderr={}",
        expected_runtime_error,
        stdout_text(&output),
        stderr_text(&output)
    );
}

fn assert_runtime_boundary_failure_with_args(
    script_source: &str,
    expected_runtime_error: &str,
    run_args: &[&str],
) {
    let project_root = unique_temp_dir("native_api_security_boundary");
    let script_path = project_root.join("boundary.kujo");
    fs::write(&script_path, script_source).expect("failed to write script");

    let mut args = vec!["run"];
    args.extend_from_slice(run_args);
    args.push(script_path.to_str().expect("script path should be utf-8"));

    let output = run_kujo(&args, &project_root);

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected runtime boundary failure with exit code 4, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );

    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains(expected_runtime_error),
        "expected runtime error text '{}' in output, got stdout={} stderr={}",
        expected_runtime_error,
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn process_native_api_misuse_reports_deterministic_error() {
    assert_runtime_boundary_failure("execute(123)\n", "execute() requires a string command");
}

#[test]
fn process_execute_rejects_empty_shell_command() {
    assert_runtime_boundary_failure(
        "execute(\"   \")\n",
        "execute() command must not be empty; use spawn_process([...]) for structured argv execution",
    );
}

#[test]
fn process_execute_status_rejects_newline_shell_command() {
    assert_runtime_boundary_failure(
        "execute_status(\"echo ok\\nwhoami\")\n",
        "execute_status() command contains newline; use spawn_process([...]) for structured argv execution",
    );
}

#[test]
fn network_native_api_misuse_reports_deterministic_error() {
    assert_runtime_boundary_failure(
        "tcp_receive(1, 10)\n",
        "tcp_receive requires (TcpStream, int_size) arguments",
    );
}

#[test]
fn filesystem_native_api_misuse_reports_deterministic_error() {
    assert_runtime_boundary_failure("write_file(1, 2)\n", "write_file requires string arguments");
}

#[test]
fn crypto_native_api_misuse_reports_deterministic_error() {
    assert_runtime_boundary_failure(
        "rsa_generate_keypair(1024)\n",
        "RSA key size must be 2048 or 4096 bits",
    );
}

#[test]
fn database_native_api_misuse_reports_deterministic_error() {
    assert_runtime_boundary_failure(
        "db_connect(\"sqlite\")\n",
        "db_connect requires database type ('sqlite'|'postgres'|'mysql') and connection string",
    );
}

#[test]
fn native_capability_untrusted_denies_filesystem_write() {
    assert_runtime_boundary_failure_with_args(
        "write_file(\"blocked.txt\", \"data\")\n",
        "Capability denied: filesystem-write required for write_file",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn filesystem_beneath_is_filesystem_read_gated_in_vm_and_interpreter() {
    let script = "read_file_beneath(\".\", \"blocked.txt\", 64)\n";
    for runtime_args in [vec!["--untrusted"], vec!["--interpreter", "--untrusted"]] {
        assert_runtime_boundary_failure_with_args(
            script,
            "Capability denied: filesystem-read required for read_file_beneath",
            &runtime_args,
        );
    }

    let script = "read_binary_file_beneath(\".\", \"blocked.txt\", 64)\n";
    for runtime_args in [vec!["--untrusted"], vec!["--interpreter", "--untrusted"]] {
        assert_runtime_boundary_failure_with_args(
            script,
            "Capability denied: filesystem-read required for read_binary_file_beneath",
            &runtime_args,
        );
    }
}

#[test]
fn filesystem_beneath_has_vm_interpreter_parity() {
    let project_root = unique_temp_dir("filesystem_beneath_runtime_parity");
    let script_path = project_root.join("beneath.kujo");
    fs::create_dir(project_root.join("trusted")).expect("trusted directory");
    fs::write(project_root.join("trusted/note.txt"), b"hello").expect("text fixture");
    fs::write(
        &script_path,
        "let text := read_file_beneath(\"trusted\", \"note.txt\", 5)\nlet blob := read_binary_file_beneath(\"trusted\", \"note.txt\", 5)\nprint(text + \":\" + to_string(len(blob)))\n",
    )
    .expect("script fixture");

    let mut outputs = Vec::new();
    for runtime_args in [vec![], vec!["--interpreter"]] {
        let mut args = vec!["run"];
        args.extend(runtime_args);
        args.push(script_path.to_str().expect("utf-8 script path"));
        let output = run_kujo(&args, &project_root);
        assert_eq!(
            output.status.code(),
            Some(0),
            "runtime failed: stdout={} stderr={}",
            stdout_text(&output),
            stderr_text(&output)
        );
        outputs.push(stdout_text(&output));
    }
    assert_eq!(outputs[0], outputs[1]);
    assert!(outputs[0].contains("hello:5"));
}

#[test]
fn native_capability_untrusted_denies_private_spool_in_vm_and_interpreter() {
    let script = "io_private_spool_open(\"blocked.eml\", 1024, 384)\n";
    assert_runtime_boundary_failure_with_args(
        script,
        "Capability denied: filesystem-write required for io_private_spool_open",
        &["--untrusted"],
    );
    assert_runtime_boundary_failure_with_args(
        script,
        "Capability denied: filesystem-write required for io_private_spool_open",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn decode_file_range_info_is_filesystem_read_gated_in_vm_and_interpreter() {
    let script = "decode_file_range_info(\"blocked.eml\",0,0,\"identity\",0,0)\n";
    for runtime_args in [vec!["--untrusted"], vec!["--interpreter", "--untrusted"]] {
        assert_runtime_boundary_failure_with_args(
            script,
            "Capability denied: filesystem-read required for decode_file_range_info",
            &runtime_args,
        );
    }
}

#[test]
fn decode_file_range_info_has_vm_interpreter_parity() {
    let project_root = unique_temp_dir("decode_file_range_runtime_parity");
    let script_path = project_root.join("decode.kujo");
    let input_path = project_root.join("message.txt");
    fs::write(&input_path, b"SGVsbG8sIHdvcmxkIQ==\r\n").expect("decode input should be written");
    let input_literal =
        escape_kujo_string(input_path.to_str().expect("decode input path should be utf-8"));
    let script = format!(
        "let info := decode_file_range_info(\"{input_literal}\",0,22,\"base64\",64,5)\n\
         print(to_json({{\"schema\":info[\"schema\"],\"output_bytes\":info[\"output_bytes\"],\"sha256\":info[\"sha256\"],\"prefix_first\":info[\"prefix\"][0]}}))\n"
    );
    fs::write(&script_path, script).expect("decode script should be written");

    let mut outputs = Vec::new();
    for runtime_args in [vec![], vec!["--interpreter"]] {
        let mut args = vec!["run"];
        args.extend(runtime_args);
        args.push(script_path.to_str().expect("decode script path should be utf-8"));
        let output = run_kujo(&args, &project_root);
        assert!(
            output.status.success(),
            "decode runtime failed: stdout={} stderr={}",
            stdout_text(&output),
            stderr_text(&output)
        );
        let stdout = stdout_text(&output);
        assert!(stdout.contains("\"schema\":\"kujo.file.decode.v1\""));
        assert!(stdout.contains("\"output_bytes\":13"));
        assert!(stdout.contains("\"prefix_first\":72"));
        assert!(!stdout.contains("Hello, world!"));
        outputs.push(stdout);
    }
    assert_eq!(outputs[0], outputs[1]);

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn private_spool_round_trip_has_vm_interpreter_parity() {
    let project_root = unique_temp_dir("private_spool_runtime_parity");
    let script_path = project_root.join("private_spool.kujo");
    let output_path = project_root.join("message.eml");
    let output_literal = escape_kujo_string(
        output_path.to_str().expect("private spool output path should be utf-8"),
    );
    let script = format!(
        "let spool := io_private_spool_open(\"{output_literal}\",16,384)\n\
         io_private_spool_write(spool,\"Subject: x\\r\\n\")\n\
         io_private_spool_write(spool,\"\\r\\nhi\")\n\
         let receipt := io_private_spool_finish(spool)\n\
         print(to_json(receipt))\n"
    );
    fs::write(&script_path, script).expect("private spool script should be written");

    for runtime_args in [vec![], vec!["--interpreter"]] {
        let _ = fs::remove_file(&output_path);
        let mut args = vec!["run"];
        args.extend(runtime_args);
        args.push(script_path.to_str().expect("private spool script path should be utf-8"));
        let output = run_kujo(&args, &project_root);
        assert!(
            output.status.success(),
            "private spool runtime failed: stdout={} stderr={}",
            stdout_text(&output),
            stderr_text(&output)
        );
        let stdout = stdout_text(&output);
        assert!(stdout.contains("\"verified\":true"));
        assert!(stdout.contains("\"published\":true"));
        assert!(stdout.contains("\"temporary_removed\":true"));
        assert!(stdout.contains("\"directory_synced\":true"));
        assert!(stdout.contains("\"bytes_written\":16"));
        assert_eq!(fs::read(&output_path).unwrap(), b"Subject: x\r\n\r\nhi");
        #[cfg(unix)]
        assert_eq!(fs::metadata(&output_path).unwrap().permissions().mode() & 0o777, 0o600);
    }

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn native_capability_untrusted_denies_filesystem_delete() {
    let project_root = unique_temp_dir("native_api_capability_deny_fs_delete");
    let script_path = project_root.join("deny_fs_delete.kujo");
    let target_path = project_root.join("blocked-delete.txt");
    fs::write(&target_path, "blocked").expect("failed to write delete target file");

    let script_source = format!(
        "delete_file(\"{}\")\n",
        escape_kujo_string(target_path.to_str().expect("target path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected delete_file to be denied without fs-delete capability, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("Capability denied: filesystem-delete required for delete_file"),
        "expected filesystem-delete capability denial, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        target_path.exists(),
        "delete target should remain because delete capability is denied"
    );
}

#[test]
fn native_capability_allow_fs_delete_enables_delete_file() {
    let project_root = unique_temp_dir("native_api_capability_allow_fs_delete");
    let script_path = project_root.join("allow_fs_delete.kujo");
    let target_path = project_root.join("allowed-delete.txt");
    fs::write(&target_path, "allowed").expect("failed to write delete target file");

    let script_source = format!(
        "delete_file(\"{}\")\n",
        escape_kujo_string(target_path.to_str().expect("target path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-fs-delete",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected delete_file to succeed when fs-delete is allowed, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        !target_path.exists(),
        "delete target should be removed when delete capability is allowed"
    );
}

#[test]
fn native_capability_untrusted_denies_process_exec() {
    assert_runtime_boundary_failure_with_args(
        "spawn_process([\"echo\", \"ok\"])\n",
        "Capability denied: process-exec required for spawn_process",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_untrusted_denies_shell_exec() {
    assert_runtime_boundary_failure_with_args(
        "execute(\"echo ok\")\n",
        "Capability denied: shell-exec required for execute",
        &["--interpreter", "--untrusted", "--allow-process-exec"],
    );
}

#[test]
fn native_capability_untrusted_allows_shell_exec_when_enabled() {
    let project_root = unique_temp_dir("native_api_capability_allow_shell_exec");
    let script_path = project_root.join("allow_shell_exec.kujo");
    fs::write(&script_path, "print(execute(\"echo shell-allowed\"))\n")
        .expect("failed to write script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-shell-exec",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected execute() to succeed when shell-exec is allowed, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("shell-allowed"),
        "expected shell command output in stdout, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn native_capability_untrusted_denies_env_read() {
    assert_runtime_boundary_failure_with_args(
        "env(\"PATH\")\n",
        "Capability denied: env-read required for env",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_untrusted_denies_env_write() {
    assert_runtime_boundary_failure_with_args(
        "env_set(\"KUJO_CAP_TEST\", \"1\")\n",
        "Capability denied: env-write required for env_set",
        &["--interpreter", "--untrusted", "--allow-env-read"],
    );
}

#[test]
fn native_capability_untrusted_denies_network_client() {
    assert_runtime_boundary_failure_with_args(
        "http_get(\"http://127.0.0.1:1\")\n",
        "Capability denied: network-client required for http_get",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_untrusted_denies_dns_lookup() {
    for function in ["dns_lookup_a", "dns_lookup_aaaa", "dns_lookup_mx"] {
        let source = format!("{}(\"example.com\")\n", function);
        let expected = format!("Capability denied: network-client required for {}", function);
        assert_runtime_boundary_failure_with_args(
            &source,
            &expected,
            &["--interpreter", "--untrusted"],
        );
    }
}

#[test]
fn native_capability_untrusted_denies_tls_client() {
    assert_runtime_boundary_failure_with_args(
        "tls_connect(\"127.0.0.1\", 1)\n",
        "Capability denied: network-client required for tls_connect",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_file_range_sends_require_network_and_filesystem_read() {
    for function in ["tcp_send_file_range", "tls_send_file_range"] {
        let source = format!("{}(null, \"message.eml\", 0, 1)\n", function);
        assert_runtime_boundary_failure_with_args(
            &source,
            &format!("Capability denied: network-client required for {}", function),
            &["--interpreter", "--untrusted"],
        );
        assert_runtime_boundary_failure_with_args(
            &source,
            &format!("Capability denied: filesystem-read required for {}", function),
            &["--interpreter", "--untrusted", "--allow-net-client"],
        );
    }
}

#[test]
fn native_capability_tls_acceptor_requires_network_server_and_filesystem_read() {
    assert_runtime_boundary_failure_with_args(
        "tls_acceptor(\"certificate.pem\", \"private-key.pem\")\n",
        "Capability denied: filesystem-read required for tls_acceptor",
        &["--interpreter", "--untrusted", "--allow-net-server"],
    );
}

fn ai_replay_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ai_cassettes")
}

fn ai_replay_script(endpoint: &str, structured_errors: bool) -> String {
    format!(
        "let result := ai_chat(\"Hello model\", {{\"endpoint\": \"{}\", \"model\": \"gpt-replay\", \"structured_errors\": {}, \"cassette\": {{\"mode\": \"replay\", \"dir\": \"{}\"}}}})\nprint(to_string(result))\n",
        escape_kujo_string(endpoint),
        if structured_errors { "true" } else { "false" },
        escape_kujo_string(ai_replay_fixture_dir().to_str().expect("fixture path should be utf-8")),
    )
}

#[test]
fn native_capability_untrusted_denies_ai_without_allow_ai() {
    assert_runtime_boundary_failure_with_args(
        &ai_replay_script("http://127.0.0.1:1/v1/chat/completions", false),
        "Capability denied: network-ai required for ai_chat",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_untrusted_denies_ai_with_only_network_client() {
    assert_runtime_boundary_failure_with_args(
        &ai_replay_script("http://127.0.0.1:1/v1/chat/completions", false),
        "Capability denied: network-ai required for ai_chat",
        &["--interpreter", "--untrusted", "--allow-net-client"],
    );
}

#[test]
fn native_capability_untrusted_allows_ai_when_enabled() {
    let project_root = unique_temp_dir("native_api_capability_allow_ai");
    let script_path = project_root.join("allow_ai.kujo");
    fs::write(&script_path, ai_replay_script("http://127.0.0.1:1/v1/chat/completions", false))
        .expect("failed to write allow-ai script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-ai",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected ai_chat to succeed with --allow-ai and replay, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("hello from cassette"),
        "expected replayed AI response in stdout, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn native_capability_ai_endpoint_allowlist_denies_miss_with_structured_error() {
    let project_root = unique_temp_dir("native_api_ai_allowlist_miss");
    let script_path = project_root.join("ai_allowlist_miss.kujo");
    fs::write(&script_path, ai_replay_script("http://127.0.0.1:1/v1/chat/completions", true))
        .expect("failed to write AI allowlist script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-ai",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_AI_ALLOWED_ENDPOINTS", "https://api.example.test/v1")],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected structured AI endpoint denial result, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let output_text = stdout_text(&output);
    assert!(
        output_text.contains("endpoint_denied")
            && output_text.contains("KUJO_AI_ALLOWED_ENDPOINTS"),
        "expected endpoint_denied structured error in stdout, got stdout={} stderr={}",
        output_text,
        stderr_text(&output)
    );
}

#[test]
fn native_capability_untrusted_denies_network_server() {
    assert_runtime_boundary_failure_with_args(
        "let server := http_server(8123)\nserver.listen()\n",
        "Capability denied: network-server required for http_server.listen",
        &["--interpreter", "--untrusted", "--allow-net-client"],
    );
}

#[test]
fn network_http_get_rejects_oversized_response_body() {
    let body = vec![b'Z'; NETWORK_MAX_BODY_BYTES_FOR_TEST + 1];
    let Some((port, _server_handle)) = spawn_one_shot_http_server(body, Duration::from_millis(0))
    else {
        eprintln!(
            "Skipping oversized HTTP body boundary test: sandbox denied local TCP bind permissions"
        );
        return;
    };

    let project_root = unique_temp_dir("network_http_get_oversized_body");
    let script_path = project_root.join("oversized_http_body.kujo");
    let script_source = format!("http_get(\"http://127.0.0.1:{port}/payload\")\n");
    fs::write(&script_path, script_source).expect("failed to write oversized http script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1")],
    );
    assert_eq!(
        output.status.code(),
        Some(4),
        "expected oversized HTTP response to fail, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("response body exceeds maximum network body size"),
        "expected oversized response boundary error, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_http_request_timeout_is_reported_deterministically() {
    let body = b"slow-response".to_vec();
    let Some((port, _server_handle)) = spawn_one_shot_http_server(body, Duration::from_millis(250))
    else {
        eprintln!("Skipping HTTP timeout boundary test: sandbox denied local TCP bind permissions");
        return;
    };

    let project_root = unique_temp_dir("network_http_request_timeout");
    let script_path = project_root.join("http_timeout_boundary.kujo");
    let script_source = format!(
        "let result := http_request(\"http://127.0.0.1:{port}/timeout\", {{\"timeout\": 0.05}})\nprint(result)\n"
    );
    fs::write(&script_path, script_source).expect("failed to write timeout script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1")],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected timeout to be surfaced as an http_request Result error, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let output_text = stdout_text(&output).to_lowercase();
    assert!(
        output_text.contains("timed out") || output_text.contains("timeout"),
        "expected timeout details in result output, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_http_client_rejects_unsupported_url_scheme_before_request_execution() {
    let project_root = unique_temp_dir("network_http_client_rejects_unsupported_scheme");
    let script_path = project_root.join("unsupported_url_scheme.kujo");
    fs::write(&script_path, "http_get(\"ftp://127.0.0.1\")\n")
        .expect("failed to write unsupported scheme test script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[],
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected unsupported URL scheme to fail with runtime error, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("unsupported URL scheme 'ftp'"),
        "expected unsupported URL scheme diagnostic, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_http_client_rejects_malformed_url_before_request_execution() {
    let project_root = unique_temp_dir("network_http_client_rejects_malformed_url");
    let script_path = project_root.join("malformed_url.kujo");
    fs::write(&script_path, "http_get(\"http://\")\n")
        .expect("failed to write malformed URL test script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[],
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected malformed URL to fail with runtime error, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("invalid URL"),
        "expected malformed URL diagnostic, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_http_client_rejects_invalid_port_before_request_execution() {
    let project_root = unique_temp_dir("network_http_client_rejects_invalid_port");
    let script_path = project_root.join("invalid_port.kujo");
    fs::write(&script_path, "http_get(\"http://127.0.0.1:99999\")\n")
        .expect("failed to write invalid port test script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[],
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected invalid port to fail with runtime error, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("invalid URL") || combined_output.contains("port"),
        "expected invalid port diagnostic, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_destination_policy_deny_private_blocks_loopback_http_client() {
    let project_root = unique_temp_dir("network_destination_policy_blocks_loopback_http");
    let script_path = project_root.join("destination_policy_http_block.kujo");
    fs::write(&script_path, "http_get(\"http://127.0.0.1:1\")\n")
        .expect("failed to write destination policy script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_NET_DESTINATION_POLICY", "deny_private")],
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected strict destination policy to block loopback HTTP destination, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("blocked by outbound destination policy"),
        "expected outbound destination policy rejection text, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_destination_policy_deny_private_blocks_loopback_tcp_client() {
    let project_root = unique_temp_dir("network_destination_policy_blocks_loopback_tcp");
    let script_path = project_root.join("destination_policy_tcp_block.kujo");
    fs::write(&script_path, "tcp_connect(\"127.0.0.1\", 1)\n")
        .expect("failed to write destination policy script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_NET_DESTINATION_POLICY", "deny_private")],
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected strict destination policy to block loopback TCP destination, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("blocked by outbound destination policy"),
        "expected outbound destination policy rejection text, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_destination_policy_override_allows_trusted_loopback_http_client() {
    let body = b"ok".to_vec();
    let Some((port, _server_handle)) = spawn_one_shot_http_server(body, Duration::from_millis(0))
    else {
        eprintln!(
            "Skipping destination policy override test: sandbox denied local TCP bind permissions"
        );
        return;
    };

    let project_root = unique_temp_dir("network_destination_policy_override_allows_loopback_http");
    let script_path = project_root.join("destination_policy_http_override.kujo");
    let script_source = format!("http_get(\"http://127.0.0.1:{port}/ok\")\n");
    fs::write(&script_path, script_source).expect("failed to write destination policy script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[
            ("KUJO_NET_DESTINATION_POLICY", "deny_private"),
            ("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1"),
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected explicit override to allow trusted loopback destination, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_tcp_connect_bound_uses_requested_source_address() {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("Skipping source-bound TCP test: sandbox denied local TCP bind permissions");
            return;
        }
        Err(error) => panic!("failed to bind source-bound TCP listener: {error}"),
    };
    let port = listener.local_addr().expect("listener address").port();
    let server = thread::spawn(move || {
        let (_stream, peer) = listener.accept().expect("source-bound client should connect");
        assert!(peer.ip().is_loopback());
        thread::sleep(Duration::from_millis(250));
    });

    let project_root = unique_temp_dir("network_tcp_connect_bound");
    let script_path = project_root.join("tcp_connect_bound.kujo");
    let script_source = format!(
        "let conn := tcp_connect_bound(\"127.0.0.1\", {port}, \"127.0.0.1\")\nlet info := tcp_info(conn)\nprint(to_json(info))\ntcp_close(conn)\n"
    );
    fs::write(&script_path, script_source).expect("failed to write source-bound TCP script");
    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1")],
    );
    server.join().expect("source-bound TCP server should finish");
    assert_eq!(
        output.status.code(),
        Some(0),
        "source-bound connect failed: stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("\"local_address\":\"127.0.0.1:"),
        "tcp_info should expose requested local source: {}",
        stdout_text(&output)
    );
    assert!(
        stdout_text(&output).contains("\"peer_ip\":\"127.0.0.1\""),
        "tcp_info should expose the socket-derived peer IP: {}",
        stdout_text(&output)
    );
    assert!(
        stdout_text(&output).contains(&format!("\"peer_port\":{port}")),
        "tcp_info should expose the socket-derived peer port: {}",
        stdout_text(&output)
    );
    assert!(
        stdout_text(&output).contains("\"local_ip\":\"127.0.0.1\""),
        "tcp_info should expose the socket-derived local IP: {}",
        stdout_text(&output)
    );
}

#[test]
fn network_tcp_connect_bound_rejects_non_unicast_source() {
    let project_root = unique_temp_dir("network_tcp_connect_bound_invalid_source");
    let script_path = project_root.join("tcp_connect_bound_invalid.kujo");
    fs::write(&script_path, "tcp_connect_bound(\"127.0.0.1\", 25, \"0.0.0.0\")\n")
        .expect("failed to write invalid source-bound TCP script");
    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1")],
    );
    assert_eq!(output.status.code(), Some(4));
    assert!(format!("{}\n{}", stdout_text(&output), stderr_text(&output))
        .contains("source_ip must be a unicast local address"));
}

#[test]
fn network_ip_classification_is_fail_closed_and_capability_free() {
    let project_root = unique_temp_dir("network_ip_classify");
    let script_path = project_root.join("ip_classify.kujo");
    fs::write(
        &script_path,
        "print(to_json([ip_classify(\"8.8.8.8\"),ip_classify(\"127.0.0.1\"),ip_classify(\"192.0.2.1\"),ip_classify(\"2001:4860:4860::8888\"),ip_classify(\"2001:db8::1\")]))\n",
    )
    .expect("failed to write IP classification script");
    for mode in [vec![], vec!["--interpreter"]] {
        let mut args = vec!["run"];
        args.extend(mode);
        args.push("--untrusted");
        args.push(script_path.to_str().expect("script path should be utf-8"));
        let output = run_kujo(&args, &project_root);
        assert_eq!(
            output.status.code(),
            Some(0),
            "IP classification failed: stdout={} stderr={}",
            stdout_text(&output),
            stderr_text(&output)
        );
        let stdout = stdout_text(&output);
        assert!(
            stdout.contains("\"address\":\"8.8.8.8\"")
                && stdout.contains("\"publicly_routable\":true")
        );
        assert!(
            stdout.contains("\"scope\":\"loopback\"")
                && stdout.contains("\"scope\":\"documentation\"")
        );
        assert!(!stdout.contains("capability denied"));
    }
}

#[test]
fn network_cidr_membership_is_family_aware_capability_free_and_runtime_equivalent() {
    let project_root = unique_temp_dir("network_ip_cidr_contains");
    let script_path = project_root.join("ip_cidr_contains.kujo");
    fs::write(
        &script_path,
        "print(to_json([ip_cidr_contains(\"192.0.2.4\",\"192.0.2.0/24\"),ip_cidr_contains(\"192.0.3.4\",\"192.0.2.0/24\"),ip_cidr_contains(\"2001:db8::1\",\"2001:db8::/32\"),ip_cidr_contains(\"192.0.2.1\",\"2001:db8::/32\")]))\n",
    )
    .expect("failed to write CIDR membership script");
    let mut outputs = Vec::new();
    for mode in [vec![], vec!["--interpreter"]] {
        let mut args = vec!["run"];
        args.extend(mode);
        args.push("--untrusted");
        args.push(script_path.to_str().expect("script path should be utf-8"));
        let output = run_kujo(&args, &project_root);
        assert_eq!(
            output.status.code(),
            Some(0),
            "CIDR membership failed: stdout={} stderr={}",
            stdout_text(&output),
            stderr_text(&output)
        );
        let stdout = stdout_text(&output);
        assert_eq!(stdout.trim(), "[true,false,true,false]");
        assert!(!stdout.contains("capability denied"));
        outputs.push(stdout);
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn compression_in_memory_readers_are_bounded_and_capability_free() {
    let project_root = unique_temp_dir("compression_in_memory");
    let script_path = project_root.join("compression.kujo");
    fs::write(
        &script_path,
        r#"
let expected := "<feedback><report_id>x</report_id></feedback>"
let gzip_data := decode_base64("H4sIAAAAAAAC/7NJS01NSUpMzrazKUotyC8qic9Msauw0UdwbPThSgA6Q8SWLQAAAA==")
let gzip_plain := gzip_decompress(gzip_data,1024)
assert(decode_base64_utf8(encode_base64(gzip_plain)) == expected,"gzip output")
let zip_data := decode_base64("UEsDBBQAAAAIADFyH106Q8SWHwAAAC0AAAAKAAAAcmVwb3J0LnhtbLNJS01NSUpMzrazKUotyC8qic9Msauw0UdwbPThSgBQSwECFAMUAAAACAAxch9dOkPElh8AAAAtAAAACgAAAAAAAAAAAAAAgAEAAAAAcmVwb3J0LnhtbFBLBQYAAAAAAQABADgAAABHAAAAAAA=")
let entry := zip_single_file_read(zip_data,1024)
assert(entry["name"] == "report.xml","zip name")
assert(decode_base64_utf8(encode_base64(entry["bytes"])) == expected,"zip output")
mut bounded := false
try { gzip_decompress(gzip_data,4) }
except error { bounded = contains(to_string(error),"exceeds max_output_bytes") }
assert(bounded,"gzip bound")
print("COMPRESSION_OK")
"#,
    )
    .expect("failed to write compression script");
    for mode in [vec![], vec!["--interpreter"]] {
        let mut args = vec!["run"];
        args.extend(mode);
        args.push("--untrusted");
        args.push(script_path.to_str().expect("script path should be utf-8"));
        let output = run_kujo(&args, &project_root);
        assert_eq!(
            output.status.code(),
            Some(0),
            "compression primitive failed: stdout={} stderr={}",
            stdout_text(&output),
            stderr_text(&output)
        );
        assert!(stdout_text(&output).contains("COMPRESSION_OK"));
        assert!(!stdout_text(&output).contains("capability denied"));
    }
}

#[test]
fn network_tcp_bind_probe_is_structured_and_server_capability_gated() {
    let project_root = unique_temp_dir("network_tcp_bind_probe");
    let script_path = project_root.join("tcp_bind_probe.kujo");
    fs::write(
        &script_path,
        "print(to_json([tcp_bind_probe(\"127.0.0.1\"),tcp_bind_probe(\"192.0.2.10\"),tcp_bind_probe(\"invalid\")]))\n",
    )
    .expect("failed to write bind probe script");
    let denied =
        run_kujo(&["run", "--untrusted", script_path.to_str().expect("utf-8 path")], &project_root);
    assert_ne!(denied.status.code(), Some(0));
    assert!(stderr_text(&denied).contains("network-server"));
    for mode in [vec![], vec!["--interpreter"]] {
        let mut args = vec!["run"];
        args.extend(mode);
        args.push("--untrusted");
        args.push("--allow-net-server");
        args.push(script_path.to_str().expect("utf-8 path"));
        let output = run_kujo(&args, &project_root);
        assert_eq!(output.status.code(), Some(0), "stderr={}", stderr_text(&output));
        let stdout = stdout_text(&output);
        assert!(stdout.contains("TCP_BIND_PROBE_AVAILABLE"));
        assert!(stdout.contains("TCP_BIND_PROBE_UNAVAILABLE"));
        assert!(stdout.contains("TCP_BIND_PROBE_ADDRESS_INVALID"));
        assert!(!stdout.contains("os error"));
    }
}

#[test]
fn network_http_request_explicit_deny_private_ignores_global_override() {
    let project_root = unique_temp_dir("network_http_request_explicit_deny_private");
    let script_path = project_root.join("explicit_destination_policy.kujo");
    fs::write(
        &script_path,
        "let result := http_request(\"http://127.0.0.1:1/callback\", {\"destination_policy\": \"deny_private\", \"pin_dns\": true, \"redirects\": \"none\"})\nprint(result)\n",
    )
    .expect("failed to write explicit destination policy script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1")],
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout_text(&output).contains("explicit outbound destination policy 'deny_private'"),
        "explicit callback policy should fail closed: stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn network_http_request_can_pin_resolution_and_disable_redirects() {
    let Some((port, server_handle)) =
        spawn_one_shot_redirect_server("http://127.0.0.1:1/private".to_string())
    else {
        eprintln!("Skipping redirect policy test: sandbox denied local TCP bind permissions");
        return;
    };
    let project_root = unique_temp_dir("network_http_request_no_redirect");
    let script_path = project_root.join("no_redirect.kujo");
    let script_source = format!(
        "let result := http_request(\"http://127.0.0.1:{port}/callback\", {{\"pin_dns\": true, \"redirects\": \"none\"}})\nprint(result)\n"
    );
    fs::write(&script_path, script_source).expect("failed to write no-redirect script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1")],
    );
    server_handle.join().expect("redirect server should finish");
    assert_eq!(
        output.status.code(),
        Some(0),
        "no-redirect request failed: stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("302"),
        "redirect response should be returned without following Location: {}",
        stdout_text(&output)
    );
}

#[test]
fn network_http_request_enforces_per_request_response_limit() {
    let body = vec![b'x'; 65];
    let Some((port, server_handle)) = spawn_one_shot_http_server(body, Duration::from_millis(0))
    else {
        eprintln!("Skipping response-limit test: sandbox denied local TCP bind permissions");
        return;
    };
    let project_root = unique_temp_dir("network_http_request_response_limit");
    let script_path = project_root.join("response_limit.kujo");
    let script_source = format!(
        "let result := http_request(\"http://127.0.0.1:{port}/callback\", {{\"max_response_bytes\": 64}})\nprint(result)\n"
    );
    fs::write(&script_path, script_source).expect("failed to write response-limit script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-net-client",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS", "1")],
    );
    server_handle.join().expect("response-limit server should finish");
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout_text(&output).contains("65 bytes > 64 bytes"),
        "per-request response limit should be enforced: stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn native_capability_untrusted_denies_database() {
    assert_runtime_boundary_failure_with_args(
        "db_connect(\"sqlite\", \"tmp.db\")\n",
        "Capability denied: database required for db_connect",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_untrusted_denies_image_file_read() {
    assert_runtime_boundary_failure_with_args(
        "load_image(\"photo.png\")\n",
        "Capability denied: filesystem-read required for load_image",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_untrusted_denies_image_conversion_write() {
    assert_runtime_boundary_failure_with_args(
        "gif_to_webp(\"in.gif\", \"out.webp\")\n",
        "Capability denied: filesystem-write required for gif_to_webp",
        &["--interpreter", "--untrusted", "--allow-fs-read"],
    );
}

#[test]
fn native_capability_untrusted_denies_clock() {
    assert_runtime_boundary_failure_with_args(
        "now()\n",
        "Capability denied: clock required for now",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_untrusted_denies_random() {
    assert_runtime_boundary_failure_with_args(
        "random()\n",
        "Capability denied: random required for random",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_allow_fs_write_enables_write_file() {
    let project_root = unique_temp_dir("native_api_capability_allow_fs_write");
    let script_path = project_root.join("allow_fs_write.kujo");
    let output_path = project_root.join("written.txt");
    let script_source = format!(
        "write_file(\"{}\", \"allowed\")\n",
        escape_kujo_string(output_path.to_str().expect("output path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-fs-write",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected write_file to succeed when fs-write is allowed, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let written = fs::read_to_string(&output_path).expect("expected write_file output file");
    assert_eq!(written, "allowed");
}

#[test]
fn native_capability_allows_only_requested_capability() {
    let project_root = unique_temp_dir("native_api_capability_only_requested");
    let script_path = project_root.join("allow_only_requested.kujo");
    let output_path = project_root.join("written.txt");
    let script_source = format!(
        "write_file(\"{}\", \"allowed\")\nenv(\"PATH\")\n",
        escape_kujo_string(output_path.to_str().expect("output path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-fs-write",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected env() to remain blocked when only fs-write is allowed, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );

    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("Capability denied: env-read required for env"),
        "expected env-read capability denial, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    let written = fs::read_to_string(&output_path).expect("expected write_file output file");
    assert_eq!(written, "allowed");
}

#[test]
fn native_capability_vm_and_interpreter_both_enforce_denial() {
    let script = "write_file(\"blocked.txt\", \"data\")\n";
    assert_runtime_boundary_failure_with_args(
        script,
        "Capability denied: filesystem-write required for write_file",
        &["--untrusted"],
    );
    assert_runtime_boundary_failure_with_args(
        script,
        "Capability denied: filesystem-write required for write_file",
        &["--interpreter", "--untrusted"],
    );
}

#[test]
fn native_capability_spawned_interpreter_inherits_policy() {
    let project_root = unique_temp_dir("native_api_capability_spawn_inherit");
    let script_path = project_root.join("spawn_policy.kujo");
    let output_path = project_root.join("spawn_blocked.txt");
    let script_source = format!(
        "spawn {{\n    write_file(\"{}\", \"blocked\")\n}}\nsleep(100)\n",
        escape_kujo_string(output_path.to_str().expect("output path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-clock",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "spawn script should complete while blocked write remains denied, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        !output_path.exists(),
        "spawned interpreter should not bypass filesystem-write capability policy"
    );
}

#[test]
fn filesystem_write_overwrite_requires_explicit_flag() {
    let project_root = unique_temp_dir("filesystem_write_overwrite_requires_flag");
    let script_path = project_root.join("overwrite_requires_flag.kujo");
    let target_path = project_root.join("overwrite.txt");
    fs::write(&target_path, "original").expect("failed to seed overwrite target file");

    let script_source = format!(
        "write_file(\"{}\", \"replacement\")\n",
        escape_kujo_string(target_path.to_str().expect("target path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &["run", "--interpreter", script_path.to_str().expect("script path should be utf-8")],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected overwrite without explicit flag to fail, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("already exists") && combined_output.contains("overwrite"),
        "expected overwrite safeguard error, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    let written = fs::read_to_string(&target_path).expect("overwrite target should still exist");
    assert_eq!(
        written, "original",
        "file content should remain unchanged when overwrite is denied"
    );
}

#[test]
fn filesystem_write_overwrite_succeeds_with_explicit_flag() {
    let project_root = unique_temp_dir("filesystem_write_overwrite_with_flag");
    let script_path = project_root.join("overwrite_with_flag.kujo");
    let target_path = project_root.join("overwrite.txt");
    fs::write(&target_path, "original").expect("failed to seed overwrite target file");

    let script_source = format!(
        "write_file(\"{}\", \"replacement\", true)\n",
        escape_kujo_string(target_path.to_str().expect("target path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &["run", "--interpreter", script_path.to_str().expect("script path should be utf-8")],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected overwrite with explicit flag to succeed, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let written = fs::read_to_string(&target_path).expect("overwrite target should still exist");
    assert_eq!(written, "replacement");
}

#[test]
fn filesystem_read_file_rejects_payload_over_limit() {
    let project_root = unique_temp_dir("filesystem_read_over_limit");
    let script_path = project_root.join("read_over_limit.kujo");
    let target_path = project_root.join("too-large.txt");
    fs::write(&target_path, vec![b'A'; FS_MAX_READ_BYTES_FOR_TEST + 1])
        .expect("failed to write oversized read fixture");

    let script_source = format!(
        "read_file(\"{}\")\n",
        escape_kujo_string(target_path.to_str().expect("target path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &["run", "--interpreter", script_path.to_str().expect("script path should be utf-8")],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected oversized read to fail, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("exceeds maximum read size"),
        "expected read-size limit error, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn filesystem_write_file_rejects_payload_over_limit() {
    let project_root = unique_temp_dir("filesystem_write_over_limit");
    let script_path = project_root.join("write_over_limit.kujo");
    let target_path = project_root.join("too-large-write.txt");

    let script_source = format!(
        "let payload := repeat(\"A\", {})\nwrite_file(\"{}\", payload)\n",
        FS_MAX_WRITE_BYTES_FOR_TEST + 1,
        escape_kujo_string(target_path.to_str().expect("target path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &["run", "--interpreter", script_path.to_str().expect("script path should be utf-8")],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected oversized write to fail, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("exceeds maximum write size"),
        "expected write-size limit error, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        !target_path.exists(),
        "write target should not exist when oversized write is rejected"
    );
}

#[test]
fn filesystem_write_and_read_succeeds_at_size_limit_boundary() {
    let project_root = unique_temp_dir("filesystem_size_limit_boundary_success");
    let script_path = project_root.join("size_limit_boundary_success.kujo");
    let target_path = project_root.join("at-limit.txt");

    let script_source = format!(
        "let payload := repeat(\"B\", {})\nwrite_file(\"{}\", payload)\nlet content := read_file(\"{}\")\nprint(len(content))\n",
        FS_MAX_WRITE_BYTES_FOR_TEST,
        escape_kujo_string(target_path.to_str().expect("target path should be utf-8")),
        escape_kujo_string(target_path.to_str().expect("target path should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &["run", "--interpreter", script_path.to_str().expect("script path should be utf-8")],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected at-limit write/read to succeed, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains(FS_MAX_WRITE_BYTES_FOR_TEST.to_string().as_str()),
        "expected script output to include boundary payload length, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn filesystem_directory_delete_behavior_is_non_recursive() {
    let project_root = unique_temp_dir("filesystem_directory_delete_non_recursive");
    let script_path = project_root.join("directory_delete_non_recursive.kujo");
    let target_dir = project_root.join("non_empty");
    fs::create_dir_all(&target_dir).expect("failed to create non-empty directory fixture");
    fs::write(target_dir.join("child.txt"), "child")
        .expect("failed to seed non-empty directory fixture");

    let script_source = format!(
        "os_rmdir(\"{}\")\n",
        escape_kujo_string(target_dir.to_str().expect("target dir should be utf-8"))
    );
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &["run", "--interpreter", script_path.to_str().expect("script path should be utf-8")],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected non-empty directory delete to fail, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let combined_output = format!("{}\n{}", stdout_text(&output), stderr_text(&output));
    assert!(
        combined_output.contains("Cannot remove directory"),
        "expected non-recursive directory delete error, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        target_dir.exists(),
        "non-empty directory should remain after failed non-recursive delete"
    );
}

#[test]
fn process_direct_exec_does_not_expand_shell_tokens() {
    let project_root = unique_temp_dir("native_api_process_no_shell_expand");
    let script_path = project_root.join("no_shell_expand.kujo");
    let script_source =
        "let result := spawn_process([\"echo\", \"$HOME\"])\nprint(result.stdout)\n";
    fs::write(&script_path, script_source).expect("failed to write script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-process-exec",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected spawn_process direct argv execution to avoid shell expansion, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("$HOME"),
        "expected direct argv process output to preserve literal shell token, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn process_timeout_kills_long_running_process() {
    let project_root = unique_temp_dir("native_api_process_timeout");
    let child_script_path = project_root.join("slow_child.kujo");
    fs::write(&child_script_path, "sleep(250)\nprint(\"done\")\n")
        .expect("failed to write child script");

    let script_path = project_root.join("timeout_boundary.kujo");
    let script_source = format!(
        "let result := spawn_process([\"{}\", \"run\", \"--interpreter\", \"{}\"], {{\"timeout_ms\": 25}})\nprint(result.timed_out)\nprint(result.success)\n",
        escape_kujo_string(kujo_binary().as_str()),
        escape_kujo_string(child_script_path.to_str().expect("child script path should be utf-8")),
    );
    fs::write(&script_path, script_source).expect("failed to write timeout script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-process-exec",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected timed process execution to be reported deterministically, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("true") && stdout_text(&output).contains("false"),
        "expected timeout result to report timed_out=true and success=false, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn process_output_limit_sets_truncation_flags() {
    let project_root = unique_temp_dir("native_api_process_output_limit");
    let child_script_path = project_root.join("large_output_child.kujo");
    fs::write(&child_script_path, "print(repeat(\"A\", 4096))\n")
        .expect("failed to write child script");

    let script_path = project_root.join("output_limit_boundary.kujo");
    let script_source = format!(
        "let result := spawn_process([\"{}\", \"run\", \"--interpreter\", \"{}\"], {{\"max_output_bytes\": 64}})\nprint(result.stdout_truncated)\nprint(len(result.stdout))\n",
        escape_kujo_string(kujo_binary().as_str()),
        escape_kujo_string(child_script_path.to_str().expect("child script path should be utf-8")),
    );
    fs::write(&script_path, script_source).expect("failed to write output-limit script");

    let output = run_kujo(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-process-exec",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected output truncation metadata to be reported, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("true"),
        "expected stdout_truncated=true when process output exceeds limit, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn process_env_allow_and_deny_policy_is_enforced() {
    let project_root = unique_temp_dir("native_api_process_env_policy");
    let child_script_path = project_root.join("env_child.kujo");
    fs::write(
        &child_script_path,
        "print(env_or(\"KUJO_ALLOWED\", \"missing-allowed\"))\nprint(env_or(\"KUJO_DENIED\", \"missing-denied\"))\nprint(env_or(\"KUJO_INJECTED\", \"missing-injected\"))\n",
    )
    .expect("failed to write child script");

    let script_path = project_root.join("env_policy_boundary.kujo");
    let script_source = format!(
        "let result := spawn_process([\"{}\", \"run\", \"--interpreter\", \"{}\"], {{\"inherit_env\": false, \"env_allow\": [\"KUJO_ALLOWED\", \"KUJO_DENIED\"], \"env_deny\": [\"KUJO_DENIED\"], \"env\": {{\"KUJO_INJECTED\": \"injected-value\"}}}})\nprint(result.stdout)\n",
        escape_kujo_string(kujo_binary().as_str()),
        escape_kujo_string(child_script_path.to_str().expect("child script path should be utf-8")),
    );
    fs::write(&script_path, script_source).expect("failed to write env-policy script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-process-exec",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOWED", "allowed-value"), ("KUJO_DENIED", "denied-value")],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected process env allow/deny policy to be enforced, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let output_text = stdout_text(&output);
    assert!(
        output_text.contains("allowed-value")
            && output_text.contains("missing-denied")
            && output_text.contains("injected-value"),
        "expected allow/deny env policy effects in process stdout, got stdout={} stderr={}",
        output_text,
        stderr_text(&output)
    );
}

#[test]
fn process_env_deny_overrides_allow_for_inherited_values() {
    let project_root = unique_temp_dir("native_api_process_env_deny_overrides_allow");
    let child_script_path = project_root.join("env_child_deny_override.kujo");
    fs::write(
        &child_script_path,
        "print(env_or(\"KUJO_ALLOWED\", \"missing-allowed\"))\nprint(env_or(\"KUJO_DENIED\", \"missing-denied\"))\n",
    )
    .expect("failed to write child script");

    let script_path = project_root.join("env_deny_override_boundary.kujo");
    let script_source = format!(
        "let result := spawn_process([\"{}\", \"run\", \"--interpreter\", \"{}\"], {{\"inherit_env\": true, \"env_allow\": [\"KUJO_ALLOWED\", \"KUJO_DENIED\"], \"env_deny\": [\"KUJO_DENIED\"]}})\nprint(result.stdout)\n",
        escape_kujo_string(kujo_binary().as_str()),
        escape_kujo_string(child_script_path.to_str().expect("child script path should be utf-8")),
    );
    fs::write(&script_path, script_source).expect("failed to write env deny override script");

    let output = run_kujo_with_env(
        &[
            "run",
            "--interpreter",
            "--untrusted",
            "--allow-process-exec",
            script_path.to_str().expect("script path should be utf-8"),
        ],
        &project_root,
        &[("KUJO_ALLOWED", "allowed-value"), ("KUJO_DENIED", "denied-value")],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected process env deny override policy to be enforced, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );
    let output_text = stdout_text(&output);
    assert!(
        output_text.contains("allowed-value") && output_text.contains("missing-denied"),
        "expected inherited env allow/deny precedence to be reflected in process stdout, got stdout={} stderr={}",
        output_text,
        stderr_text(&output)
    );
}

#[test]
fn unzip_rejects_parent_traversal_entries() {
    let (project_root, _, output) =
        run_unzip_script_with_archive("native_api_unzip_parent_traversal", |zip_path| {
            create_zip_archive(zip_path, |writer| {
                write_zip_file_entry(writer, "../escape.txt", b"escape", None);
            });
        });

    assert_unzip_failure(&output, "parent directory traversal component");
    assert!(
        !project_root.join("escape.txt").exists(),
        "zip traversal entry should not write files outside extraction root"
    );
}

#[test]
fn unzip_rejects_absolute_entries() {
    let (_, _, output) =
        run_unzip_script_with_archive("native_api_unzip_absolute_path", |zip_path| {
            create_zip_archive(zip_path, |writer| {
                write_zip_file_entry(writer, "/tmp/escape.txt", b"escape", None);
            });
        });

    assert_unzip_failure(&output, "absolute path");
}

#[test]
fn unzip_rejects_windows_drive_prefixed_entries() {
    let (_, _, output) =
        run_unzip_script_with_archive("native_api_unzip_drive_prefix", |zip_path| {
            create_zip_archive(zip_path, |writer| {
                write_zip_file_entry(writer, "C:/escape.txt", b"escape", None);
            });
        });

    assert_unzip_failure(&output, "drive-prefixed path");
}

#[test]
fn unzip_rejects_null_byte_entries() {
    let (_, _, output) = run_unzip_script_with_archive("native_api_unzip_null_byte", |zip_path| {
        create_zip_archive(zip_path, |writer| {
            write_zip_file_entry(writer, "bad\0name.txt", b"escape", None);
        });
    });

    assert_unzip_failure(&output, "null byte");
}

#[test]
fn unzip_rejects_symlink_entries() {
    let (_, _, output) = run_unzip_script_with_archive("native_api_unzip_symlink", |zip_path| {
        create_zip_archive(zip_path, |writer| {
            write_zip_file_entry(writer, "symlink-entry", b"target.txt", None);
        });
        mark_first_zip_entry_as_symlink(zip_path);
    });

    assert_unzip_failure(&output, "symbolic links are not allowed");
}

#[test]
fn unzip_rejects_archives_exceeding_single_entry_limit() {
    let (_, _, output) =
        run_unzip_script_with_archive("native_api_unzip_single_limit", |zip_path| {
            create_zip_archive(zip_path, |writer| {
                let oversized = vec![b'x'; 17 * 1024 * 1024];
                write_zip_file_entry(writer, "oversized.bin", &oversized, None);
            });
        });

    assert_unzip_failure(&output, "exceeds maximum per-entry size");
}

#[test]
fn unzip_rejects_archives_exceeding_total_size_limit() {
    let (_, _, output) =
        run_unzip_script_with_archive("native_api_unzip_total_limit", |zip_path| {
            create_zip_archive(zip_path, |writer| {
                let payload = vec![b'y'; 14 * 1024 * 1024];
                for index in 0..5 {
                    write_zip_file_entry(writer, &format!("bulk-{index}.bin"), &payload, None);
                }
            });
        });

    assert_unzip_failure(&output, "exceeds maximum total extraction size");
}

#[test]
fn unzip_rejects_archives_exceeding_entry_count_limit() {
    let (_, _, output) =
        run_unzip_script_with_archive("native_api_unzip_entry_count_limit", |zip_path| {
            create_zip_archive(zip_path, |writer| {
                for index in 0..1025 {
                    write_zip_file_entry(writer, &format!("entry-{index}.txt"), b"ok", None);
                }
            });
        });

    assert_unzip_failure(&output, "exceeds maximum entry count");
}

#[test]
fn unzip_extracts_safe_nested_entries() {
    let (project_root, output_dir, output) =
        run_unzip_script_with_archive("native_api_unzip_safe_nested", |zip_path| {
            create_zip_archive(zip_path, |writer| {
                write_zip_file_entry(writer, "safe/nested/file.txt", b"hello", None);
                write_zip_file_entry(writer, "safe/nested/second.txt", b"world", None);
            });
        });

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected unzip success, got status={:?}, stdout={}, stderr={}",
        output.status.code(),
        stdout_text(&output),
        stderr_text(&output)
    );

    let first_file = output_dir.join("safe/nested/file.txt");
    let second_file = output_dir.join("safe/nested/second.txt");
    assert!(
        first_file.exists() && second_file.exists(),
        "expected safe nested files to be extracted under output directory; output root={} stdout={} stderr={}",
        project_root.display(),
        stdout_text(&output),
        stderr_text(&output)
    );
    assert_eq!(fs::read_to_string(first_file).expect("expected first extracted file"), "hello");
    assert_eq!(fs::read_to_string(second_file).expect("expected second extracted file"), "world");
}
