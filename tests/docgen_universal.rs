use kujo::docgen::adapters::kujo::{
    extract_symbols_with_parser_fallback, KujoParserAssistedStrategy,
};
use kujo::docgen::core::{
    run as run_docgen, run_with_link_validation as run_docgen_with_link_validation,
    run_with_link_validation_and_options as run_docgen_with_options, DocOutputFormat, DocgenConfig,
    DocgenExtractionOptions,
};
use kujo::docgen::gaps::LinkValidationOptions;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

fn symbol_visibility<'a>(symbols: &'a [Value], qualified_name: &str) -> &'a str {
    symbols
        .iter()
        .find(|symbol| symbol["qualified_name"] == qualified_name)
        .unwrap_or_else(|| panic!("missing symbol '{qualified_name}'"))["visibility"]
        .as_str()
        .unwrap_or_else(|| panic!("symbol '{qualified_name}' visibility should be string"))
}

fn has_symbol(symbols: &[Value], qualified_name: &str) -> bool {
    symbols.iter().any(|symbol| symbol["qualified_name"] == qualified_name)
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_docgen_universal_{prefix}_{nanos}"));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent directory");
    }
    fs::write(path, content).expect("failed to write fixture file");
}

fn docgen_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("docgen")
        .join(name)
}

fn run_kujo(args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to execute kujo binary")
}

struct TestHttpServer {
    addr: SocketAddr,
}

impl TestHttpServer {
    fn url_with_host(&self, host: &str, path: &str) -> String {
        let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
        format!("http://{}:{}{}", host, self.addr.port(), path)
    }
}

fn spawn_http_server<F>(expected_requests: usize, responder: F) -> Option<TestHttpServer>
where
    F: Fn(&str) -> String + Send + 'static,
{
    spawn_http_server_on("127.0.0.1:0", expected_requests, responder)
}

fn spawn_http_server_on<F>(
    bind_address: &str,
    expected_requests: usize,
    responder: F,
) -> Option<TestHttpServer>
where
    F: Fn(&str) -> String + Send + 'static,
{
    let server = match tiny_http::Server::http(bind_address) {
        Ok(server) => server,
        Err(err) => {
            eprintln!(
                "skipping network-bound docgen test: unable to bind local test server ({err})"
            );
            return None;
        }
    };
    let addr = server.server_addr().to_ip().expect("HTTP test server should use an IP socket");
    thread::spawn(move || {
        for _ in 0..expected_requests {
            let Ok(Some(request)) = server.recv_timeout(Duration::from_secs(4)) else {
                break;
            };
            let raw_response = responder(request.url());
            let response_result = if raw_response.starts_with("HTTP/1.1 302") {
                let location = raw_response
                    .lines()
                    .find_map(|line| line.strip_prefix("Location: "))
                    .expect("redirect response should contain Location");
                let header = tiny_http::Header::from_bytes("Location", location)
                    .expect("redirect Location header should be valid");
                request.respond(
                    tiny_http::Response::empty(tiny_http::StatusCode(302)).with_header(header),
                )
            } else {
                request.respond(tiny_http::Response::from_string("ok"))
            };
            response_result.expect("write test response");
        }
    });
    Some(TestHttpServer { addr })
}

fn http_200_response() -> String {
    "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_string()
}

fn http_302_response(location: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

#[test]
fn docgen_captures_documented_and_undocumented_kujo_symbols() {
    let dir = unique_temp_dir("kujo_symbols");
    let input = dir.join("mod.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "/// Adds one\nfunc add_one(value) {\n    return value + 1\n}\n\nfunc sub_one(value) {\n    return value - 1\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input: input.clone(),
        out_dir: out.clone(),
        format: DocOutputFormat::All,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: true,
        search_index: true,
        source_links: true,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    assert!(summary.item_count >= 2);
    assert!(summary.project_json_path.exists());
    assert!(summary.gaps_json_path.exists());
    assert!(summary.ai_tasks_path.as_ref().expect("ai tasks path").exists());

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "add_one" && symbol["docs"]["placeholder"] == false
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "sub_one" && symbol["docs"]["placeholder"] == true
    }));
}

#[test]
fn docgen_kujo_supports_additional_doc_comment_styles() {
    let dir = unique_temp_dir("kujo_doc_comment_styles");
    let input = dir.join("doc_styles.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "//! Public API docs from bang style\npub func bang_doc_api() {\n    return 1\n}\n\n/**\n * Public API docs from block style\n */\npub func block_doc_api() {\n    return 2\n}\n\n/* Regular block comment should not be treated as docs */\npub func no_doc_api() {\n    return 3\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "bang_doc_api" && symbol["docs"]["placeholder"] == false
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "block_doc_api" && symbol["docs"]["placeholder"] == false
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "no_doc_api" && symbol["docs"]["placeholder"] == true
    }));
}

#[test]
fn docgen_kujo_attaches_docs_across_decorator_lines_without_overreaching() {
    let dir = unique_temp_dir("kujo_doc_attachment_decorators");
    let input = dir.join("doc_attachment.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "/// Decorated API docs\n@rate_limit(10)\npub func decorated_api() {\n    return 1\n}\n\n/// Internal state docs\n@memoized\nlet cached_value := 42\n\npub func missing_docs_api() {\n    return cached_value\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "decorated_api" && symbol["docs"]["placeholder"] == false
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "cached_value" && symbol["docs"]["placeholder"] == false
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "missing_docs_api" && symbol["docs"]["placeholder"] == true
    }));
}

#[test]
fn docgen_kujo_handles_spacing_and_proximity_edge_cases() {
    let dir = unique_temp_dir("kujo_doc_spacing_proximity");
    let input = dir.join("spacing_proximity.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "/// Spaced docs\n\n\npub func spaced_api() {\n    return 1\n}\n\n/// Blocked by regular comment\n// informational comment\npub func blocked_api() {\n    return 2\n}\n\n/// Internal state docs\nlet internal_value := 3\n\npub func missing_docs_api() {\n    return internal_value\n}\n\n/// First block\n\n/// Second block\npub func nearest_block_api() {\n    return 4\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "spaced_api" && symbol["docs"]["placeholder"] == false
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "blocked_api" && symbol["docs"]["placeholder"] == true
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "internal_value" && symbol["docs"]["placeholder"] == false
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "missing_docs_api" && symbol["docs"]["placeholder"] == true
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["qualified_name"] == "nearest_block_api"
            && symbol["docs"]["summary"] == "Second block"
    }));
}

#[test]
fn docgen_emits_discovery_diagnostics_for_oversized_files() {
    let dir = unique_temp_dir("docgen_discovery_max_file_size");
    let oversized = dir.join("oversized.kujo");
    let small = dir.join("small.kujo");
    let out = dir.join("docs");

    write_file(&oversized, &"a".repeat((2 * 1024 * 1024) + 1));
    write_file(&small, "pub func kept_api() {\n    return 1\n}\n");

    let (_project, summary) = run_docgen(&DocgenConfig {
        input: dir.clone(),
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    assert!(summary.diagnostics_count > 0, "expected discovery warning diagnostics");
    assert_eq!(summary.discovery_skip_counts.get("max_file_size").copied().unwrap_or_default(), 1);
    assert_eq!(summary.discovery_skip_counts.get("max_depth").copied().unwrap_or_default(), 0);
    assert_eq!(summary.discovery_skip_counts.get("max_files").copied().unwrap_or_default(), 0);
    assert_eq!(
        summary.discovery_skip_counts.get("invalid_encoding").copied().unwrap_or_default(),
        0
    );

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");
    let diagnostics = project["diagnostics"].as_array().expect("diagnostics should be an array");

    assert!(has_symbol(symbols, "kept_api"));
    assert!(diagnostics.iter().any(|diag| {
        diag["code"] == "DOCGEN_DISCOVERY_MAX_FILE_SIZE"
            && diag["severity"] == "Warning"
            && diag["path"].as_str().unwrap_or_default().ends_with("oversized.kujo")
    }));
}

#[test]
fn docgen_skips_non_utf8_sources_with_deterministic_discovery_diagnostics() {
    let dir = unique_temp_dir("docgen_discovery_invalid_encoding");
    let bad = dir.join("bad.kujo");
    let good = dir.join("good.kujo");
    let out = dir.join("docs");

    fs::write(&bad, vec![0xff, 0xfe, 0xfd]).expect("write invalid utf8 source");
    write_file(&good, "pub func kept_api() {\n    return 1\n}\n");

    let (_project, summary) = run_docgen(&DocgenConfig {
        input: dir.clone(),
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    assert_eq!(summary.discovery_skip_counts.get("max_file_size").copied().unwrap_or_default(), 0);
    assert_eq!(summary.discovery_skip_counts.get("max_depth").copied().unwrap_or_default(), 0);
    assert_eq!(summary.discovery_skip_counts.get("max_files").copied().unwrap_or_default(), 0);
    assert_eq!(
        summary.discovery_skip_counts.get("invalid_encoding").copied().unwrap_or_default(),
        1
    );

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");
    let diagnostics = project["diagnostics"].as_array().expect("diagnostics should be an array");

    assert!(has_symbol(symbols, "kept_api"));
    assert!(
        diagnostics.iter().any(|diag| {
            diag["code"] == "DOCGEN_DISCOVERY_INVALID_ENCODING"
                && diag["severity"] == "Warning"
                && diag["path"].as_str().unwrap_or_default().ends_with("bad.kujo")
        }),
        "expected invalid-encoding discovery warning"
    );
}

#[test]
fn docgen_diagnostics_order_is_deterministic_across_sources() {
    let dir = unique_temp_dir("docgen_diagnostic_ordering");
    let input = dir.join("module.kujo");
    let oversized = dir.join("oversized.kujo");
    let out_a = dir.join("docs_a");
    let out_b = dir.join("docs_b");

    write_file(
        &input,
        "/// Uses a broken doc link: [missing](./does-not-exist.md)\npub func public_api() {\n    return 1\n}\n",
    );
    write_file(&oversized, &"z".repeat((2 * 1024 * 1024) + 1));

    let (_project_a, summary_a) = run_docgen(&DocgenConfig {
        input: dir.clone(),
        out_dir: out_a,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("first docgen run should succeed");

    let (_project_b, summary_b) = run_docgen(&DocgenConfig {
        input: dir,
        out_dir: out_b,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("second docgen run should succeed");

    let project_a: Value = serde_json::from_str(
        &fs::read_to_string(summary_a.project_json_path).expect("read first project json"),
    )
    .expect("first project json should be valid");
    let project_b: Value = serde_json::from_str(
        &fs::read_to_string(summary_b.project_json_path).expect("read second project json"),
    )
    .expect("second project json should be valid");

    let diagnostics_a =
        project_a["diagnostics"].as_array().expect("diagnostics should be an array");
    let diagnostics_b =
        project_b["diagnostics"].as_array().expect("diagnostics should be an array");

    assert_eq!(
        diagnostics_a, diagnostics_b,
        "diagnostic ordering should remain stable across repeated runs"
    );

    let codes: Vec<&str> =
        diagnostics_a.iter().map(|diag| diag["code"].as_str().unwrap_or_default()).collect();
    assert_eq!(
        codes,
        vec!["DOCGEN_DISCOVERY_MAX_FILE_SIZE", "DOCGEN_LINK_BROKEN_LOCAL_FILE"],
        "diagnostics should be deterministically ordered by severity/code/path/line/message"
    );
}

#[test]
fn docgen_html_renderer_source_link_toggle_preserves_current_output_shape() {
    let dir = unique_temp_dir("docgen_html_source_link_toggle");
    let input = dir.join("module.kujo");
    write_file(&input, "/// API docs\npub func api() {\n    return 1\n}\n");

    let out_false = dir.join("docs_false");
    let (_project_false, summary_false) = run_docgen(&DocgenConfig {
        input: input.clone(),
        out_dir: out_false,
        format: DocOutputFormat::Html,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen html run without source links should succeed");

    let out_true = dir.join("docs_true");
    let (_project_true, summary_true) = run_docgen(&DocgenConfig {
        input,
        out_dir: out_true,
        format: DocOutputFormat::Html,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: true,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen html run with source links should succeed");

    let html_false =
        fs::read_to_string(summary_false.module_doc_path).expect("failed to read html output");
    let html_true =
        fs::read_to_string(summary_true.module_doc_path).expect("failed to read html output");

    assert_eq!(
        html_false, html_true,
        "source_links toggle should preserve current html output shape"
    );
}

#[test]
fn docgen_source_link_template_renders_configured_urls() {
    let dir = unique_temp_dir("docgen_source_link_template");
    let input = dir.join("src").join("module with space.kujo");
    write_file(&input, "/// API docs\npub func api() {\n    return 1\n}\n");

    let out = dir.join("docs");
    let (_project, summary) = run_docgen(&DocgenConfig {
        input: dir.clone(),
        out_dir: out,
        format: DocOutputFormat::Html,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: true,
        source_link_template: Some(
            "https://github.com/acme/repo/blob/main/{path}#L{line}".to_string(),
        ),
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let html = fs::read_to_string(summary.module_doc_path).expect("failed to read html output");
    assert!(
        html.contains("https://github.com/acme/repo/blob/main/src/module%20with%20space.kujo#L2"),
        "expected configured source-link template URL to be rendered in HTML output"
    );
}

#[test]
fn docgen_kujo_visibility_tracks_top_level_functions_and_struct_methods() {
    let dir = unique_temp_dir("kujo_visibility_matrix");
    let input = dir.join("visibility.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "func internal_helper() {\n    return 1\n}\n\npub func exported_api() {\n    return 2\n}\n\nstruct Worker {\n    func hidden_method(self) {\n        return 3\n    }\n\n    pub func visible_method(self) {\n        return 4\n    }\n}\n\npub struct PublicWorker {\n    pub func visible_method(self) {\n        return 5\n    }\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input: input.clone(),
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert_eq!(symbol_visibility(symbols, "internal_helper"), "Private");
    assert_eq!(symbol_visibility(symbols, "exported_api"), "Public");
    assert_eq!(symbol_visibility(symbols, "Worker::hidden_method"), "Private");
    assert_eq!(symbol_visibility(symbols, "Worker::visible_method"), "Private");
    assert_eq!(symbol_visibility(symbols, "PublicWorker::visible_method"), "Public");
}

#[test]
fn docgen_strict_public_gate_ignores_private_undocumented_kujo_functions() {
    let dir = unique_temp_dir("kujo_visibility_strict_gate_private");
    let input = dir.join("strict_private.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "func private_helper() {\n    return 1\n}\n\n/// API docs\npub func public_api() {\n    return 2\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("strict docgen run should complete");

    assert_eq!(summary.undocumented_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert_eq!(
        summary.broken_link_count, 0,
        "unexpected broken links: {:?}",
        summary.gate_failures
    );
    assert!(summary.gate_failures.is_empty(), "strict gate should pass");
}

#[test]
fn docgen_strict_public_gate_still_fails_on_undocumented_explicit_public_kujo_function() {
    let dir = unique_temp_dir("kujo_visibility_strict_gate_public");
    let input = dir.join("strict_public.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "func private_helper() {\n    return 1\n}\n\npub func public_missing_docs() {\n    return 2\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("strict docgen run should complete");

    assert_eq!(summary.undocumented_count, 1);
    assert!(
        summary
            .gate_failures
            .iter()
            .any(|failure| failure == "1 undocumented public symbols detected"),
        "strict gate should fail on undocumented explicit public symbols"
    );
}

#[test]
fn docgen_public_only_visibility_matrix_keeps_internal_helpers_out_of_public_gate() {
    let dir = unique_temp_dir("kujo_public_only_visibility_matrix");
    let input = dir.join("visibility_matrix_public_only.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "func internal_helper() {\n    return 1\n}\n\n/// Exported API docs\npub func exported_api() {\n    return 2\n}\n\nstruct InternalWorker {\n    pub func visible_but_private(self) {\n        return 3\n    }\n\n    func hidden_method(self) {\n        return 4\n    }\n}\n\n/// Public worker docs\npub struct PublicWorker {\n    /// Public method docs\n    pub func documented_public_method(self) {\n        return 5\n    }\n\n    func private_method(self) {\n        return 6\n    }\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("strict public-only docgen run should complete");

    assert_eq!(summary.undocumented_count, 0);
    assert!(summary.gate_failures.is_empty(), "strict gate should pass");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert!(has_symbol(symbols, "exported_api"));
    assert!(has_symbol(symbols, "PublicWorker"));
    assert!(has_symbol(symbols, "PublicWorker::documented_public_method"));

    assert!(!has_symbol(symbols, "internal_helper"));
    assert!(!has_symbol(symbols, "InternalWorker"));
    assert!(!has_symbol(symbols, "InternalWorker::visible_but_private"));
    assert!(!has_symbol(symbols, "InternalWorker::hidden_method"));
    assert!(!has_symbol(symbols, "PublicWorker::private_method"));
}

#[test]
fn docgen_public_only_excludes_methods_on_private_structs() {
    let dir = unique_temp_dir("kujo_private_struct_methods_gate");
    let input = dir.join("private_struct_methods.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "struct InternalWorker {\n    pub func internal_api(self) {\n        return 1\n    }\n}\n\n/// Public worker docs\npub struct PublicWorker {\n    /// Public worker API\n    pub func exposed_api(self) {\n        return 2\n    }\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("strict docgen run should complete");

    assert_eq!(summary.undocumented_count, 0);
    assert!(summary.gate_failures.is_empty(), "strict gate should pass");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert!(
        !has_symbol(symbols, "InternalWorker::internal_api"),
        "public-only output should not include methods on private structs"
    );
    assert!(
        has_symbol(symbols, "PublicWorker::exposed_api"),
        "public-only output should include methods on public structs"
    );
}

#[test]
fn docgen_public_only_excludes_variants_of_private_enums() {
    let dir = unique_temp_dir("kujo_private_enum_variants_gate");
    let input = dir.join("private_enum_variants.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "enum InternalState {\n    Idle,\n    Busy,\n}\n\npub enum ApiState {\n    /// Public variant docs\n    Ready,\n    Running,\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("public-only docgen run should complete");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert!(
        !has_symbol(symbols, "InternalState::Idle"),
        "public-only output should not include variants from private enums"
    );
    assert!(
        !has_symbol(symbols, "InternalState::Busy"),
        "public-only output should not include variants from private enums"
    );
    assert!(
        has_symbol(symbols, "ApiState"),
        "public enum should be included in public-only output"
    );
    assert!(
        has_symbol(symbols, "ApiState::Ready"),
        "public enum variants should be included in public-only output"
    );
}

#[test]
fn docgen_extracts_async_kujo_functions_and_methods_with_visibility() {
    let dir = unique_temp_dir("kujo_async_visibility_matrix");
    let input = dir.join("async_visibility.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "async func internal_async() {\n    return 1\n}\n\npub async func exported_async() {\n    return 2\n}\n\nstruct PrivateWorker {\n    pub async func visible_but_private(self) {\n        return 3\n    }\n}\n\npub struct PublicWorker {\n    async func hidden_async(self) {\n        return 4\n    }\n\n    pub async func visible_async(self) {\n        return 5\n    }\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert_eq!(symbol_visibility(symbols, "internal_async"), "Private");
    assert_eq!(symbol_visibility(symbols, "exported_async"), "Public");
    assert_eq!(symbol_visibility(symbols, "PrivateWorker::visible_but_private"), "Private");
    assert_eq!(symbol_visibility(symbols, "PublicWorker::hidden_async"), "Private");
    assert_eq!(symbol_visibility(symbols, "PublicWorker::visible_async"), "Public");
}

#[test]
fn docgen_strict_public_gate_handles_async_kujo_functions() {
    let dir = unique_temp_dir("kujo_async_visibility_strict_gate");
    let input = dir.join("async_strict_public.kujo");
    let out = dir.join("docs");

    write_file(
        &input,
        "async func private_async_helper() {\n    return 1\n}\n\npub async func public_async_missing_docs() {\n    return 2\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("strict docgen run should complete");

    assert_eq!(summary.undocumented_count, 1);
    assert!(
        summary
            .gate_failures
            .iter()
            .any(|failure| failure == "1 undocumented public symbols detected"),
        "strict gate should fail on undocumented explicit public async symbols"
    );
}

#[test]
fn docgen_kujo_extraction_edge_fixture_async_visibility_contract() {
    let dir = unique_temp_dir("kujo_async_fixture_visibility");
    let input = dir.join("fixture_async_visibility.kujo");
    let out = dir.join("docs");

    let source = fs::read_to_string(docgen_fixture_path("kujo_async_visibility.kujo"))
        .expect("failed to read async visibility fixture");
    write_file(&input, &source);

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let expected_json =
        fs::read_to_string(docgen_fixture_path("kujo_async_visibility.expected.json"))
            .expect("failed to read expected async visibility fixture");
    let expected: Value =
        serde_json::from_str(&expected_json).expect("expected visibility fixture should be json");
    let expected_map =
        expected.as_object().expect("expected visibility fixture should be an object map");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    for (qualified_name, visibility) in expected_map {
        let expected_visibility = visibility.as_str().unwrap_or_else(|| {
            panic!("visibility fixture for '{qualified_name}' should be string")
        });
        assert_eq!(
            symbol_visibility(symbols, qualified_name),
            expected_visibility,
            "unexpected visibility for fixture symbol '{qualified_name}'"
        );
    }
}

#[test]
fn docgen_kujo_extraction_edge_fixture_async_strict_gate_contract() {
    let dir = unique_temp_dir("kujo_async_fixture_strict_gate");
    let input = dir.join("fixture_async_strict.kujo");
    let out = dir.join("docs");

    let source = fs::read_to_string(docgen_fixture_path("kujo_async_strict_public.kujo"))
        .expect("failed to read async strict fixture");
    write_file(&input, &source);

    let expected_json =
        fs::read_to_string(docgen_fixture_path("kujo_async_strict_public.expected.json"))
            .expect("failed to read expected async strict fixture");
    let expected: Value =
        serde_json::from_str(&expected_json).expect("expected strict fixture should be json");
    let expected_undocumented =
        expected["undocumented_count"].as_u64().expect("expected undocumented_count should be u64")
            as usize;
    let expected_gate_failure = expected["gate_failure_contains"]
        .as_str()
        .expect("expected gate_failure_contains should be string");

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("strict docgen run should complete");

    assert_eq!(summary.undocumented_count, expected_undocumented);
    assert!(
        summary.gate_failures.iter().any(|failure| failure.contains(expected_gate_failure)),
        "strict gate failures should contain '{expected_gate_failure}'"
    );
}

#[test]
fn docgen_kujo_parser_assisted_fixture_success_contract() {
    let dir = unique_temp_dir("kujo_parser_assisted_fixture_success");
    let input = dir.join("fixture_parser_assisted_success.kujo");
    let out = dir.join("docs");

    let source = fs::read_to_string(docgen_fixture_path("kujo_parser_assisted_success.kujo"))
        .expect("failed to read parser-assisted success fixture");
    write_file(&input, &source);

    let (_project, summary) = run_docgen_with_options(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: false,
            fail_on_broken_links: false,
            fail_on_warnings: false,
            public_only: false,
            include_private: true,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions::default(),
        DocgenExtractionOptions { kujo_parser_assisted: true },
    )
    .expect("docgen should succeed with parser-assisted extraction");

    let expected_json =
        fs::read_to_string(docgen_fixture_path("kujo_parser_assisted_success.expected.json"))
            .expect("failed to read expected parser-assisted fixture");
    let expected: Value = serde_json::from_str(&expected_json)
        .expect("expected parser-assisted fixture should be json");
    let expected_map =
        expected.as_object().expect("expected parser-assisted fixture should be an object map");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    for (qualified_name, visibility) in expected_map {
        let expected_visibility = visibility.as_str().unwrap_or_else(|| {
            panic!("parser-assisted fixture visibility for '{qualified_name}' should be string")
        });
        assert_eq!(
            symbol_visibility(symbols, qualified_name),
            expected_visibility,
            "unexpected visibility for parser-assisted fixture symbol '{qualified_name}'"
        );
    }
}

#[test]
fn docgen_kujo_parser_assisted_fixture_fallback_contract() {
    let path = docgen_fixture_path("kujo_parser_assisted_fallback.kujo");
    let source = fs::read_to_string(&path).expect("failed to read parser-fallback fixture");
    let expected_json =
        fs::read_to_string(docgen_fixture_path("kujo_parser_assisted_fallback.expected.json"))
            .expect("failed to read expected parser-fallback fixture");
    let expected: Value = serde_json::from_str(&expected_json)
        .expect("expected parser-fallback fixture should be json");
    let expected_map =
        expected.as_object().expect("expected parser-fallback fixture should be an object map");

    let extraction = extract_symbols_with_parser_fallback(&source, &path)
        .expect("parser-assisted extraction should complete");
    assert_eq!(
        extraction.strategy,
        KujoParserAssistedStrategy::RegexFallbackParserDiagnostics,
        "fixture should trigger parser-diagnostic fallback"
    );

    for (qualified_name, visibility) in expected_map {
        let expected_visibility = visibility.as_str().unwrap_or_else(|| {
            panic!("fallback fixture visibility for '{qualified_name}' should be string")
        });
        let actual = extraction
            .symbols
            .iter()
            .find(|symbol| symbol.qualified_name == qualified_name.as_str())
            .unwrap_or_else(|| panic!("missing fallback symbol '{qualified_name}'"));
        assert_eq!(
            format!("{:?}", actual.visibility),
            expected_visibility,
            "unexpected fallback visibility for symbol '{}'",
            qualified_name
        );
    }
}

#[test]
fn docgen_typescript_visibility_matrix_preserves_modifier_and_export_semantics() {
    let dir = unique_temp_dir("typescript_visibility_matrix");
    let input = dir.join("visibility.ts");
    let out = dir.join("docs");

    write_file(
        &input,
        "class InternalClass {\n  public explicitly_open(input: string): string {\n    return input;\n  }\n\n  protected partly_open(input: string): string {\n    return input;\n  }\n\n  private closed(input: string): string {\n    return input;\n  }\n}\n\nexport class PublicClass {\n  run(input: string): string {\n    return input;\n  }\n\n  protected gate(input: string): string {\n    return input;\n  }\n\n  private lock(input: string): string {\n    return input;\n  }\n}\n\nfunction internalHelper(input: string) {\n  return input;\n}\n\nexport function exportedApi(input: string) {\n  return input;\n}\n\ninterface InternalShape {\n  value: string;\n}\n\nexport interface PublicShape {\n  value: string;\n}\n\ntype InternalAlias = string;\nexport type PublicAlias = string;\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("typescript".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("typescript docgen should succeed");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert_eq!(symbol_visibility(symbols, "InternalClass"), "Private");
    assert_eq!(symbol_visibility(symbols, "PublicClass"), "Public");
    assert_eq!(symbol_visibility(symbols, "InternalClass.explicitly_open"), "Public");
    assert_eq!(symbol_visibility(symbols, "InternalClass.partly_open"), "Protected");
    assert_eq!(symbol_visibility(symbols, "InternalClass.closed"), "Private");
    assert_eq!(symbol_visibility(symbols, "PublicClass.run"), "Public");
    assert_eq!(symbol_visibility(symbols, "PublicClass.gate"), "Protected");
    assert_eq!(symbol_visibility(symbols, "PublicClass.lock"), "Private");
    assert_eq!(symbol_visibility(symbols, "internalHelper"), "Private");
    assert_eq!(symbol_visibility(symbols, "exportedApi"), "Public");
    assert_eq!(symbol_visibility(symbols, "InternalShape"), "Private");
    assert_eq!(symbol_visibility(symbols, "PublicShape"), "Public");
    assert_eq!(symbol_visibility(symbols, "InternalAlias"), "Private");
    assert_eq!(symbol_visibility(symbols, "PublicAlias"), "Public");
}

#[test]
fn docgen_typescript_public_only_keeps_public_methods_even_under_private_classes() {
    let dir = unique_temp_dir("typescript_public_only_private_class_methods");
    let input = dir.join("public_only.ts");
    let out = dir.join("docs");

    write_file(
        &input,
        "class InternalClass {\n  public documentedPublicMethod(input: string): string {\n    return input;\n  }\n\n  private hiddenMethod(input: string): string {\n    return input;\n  }\n}\n\nexport class PublicClass {\n  public exposedMethod(input: string): string {\n    return input;\n  }\n}\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("typescript".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("typescript public-only docgen should succeed");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read docgen json");
    let project: Value =
        serde_json::from_str(&project_json).expect("docgen.json should be valid json");
    let symbols = project["symbols"].as_array().expect("symbols should be an array");

    assert!(!has_symbol(symbols, "InternalClass"));
    assert!(!has_symbol(symbols, "InternalClass.hiddenMethod"));
    assert!(has_symbol(symbols, "InternalClass.documentedPublicMethod"));
    assert!(has_symbol(symbols, "PublicClass"));
    assert!(has_symbol(symbols, "PublicClass.exposedMethod"));
}

#[test]
fn docgen_cli_json_contract_preserves_legacy_fields() {
    let dir = unique_temp_dir("cli_contract");
    let input = dir.join("file.kujo");
    let out = dir.join("docs");

    write_file(&input, "func hello() {\n    return 1\n}\n");

    let output = run_kujo(
        &[
            "docgen",
            input.to_str().expect("path utf-8"),
            "--out-dir",
            out.to_str().expect("path utf-8"),
            "--json",
        ],
        &dir,
    );

    assert!(output.status.success(), "docgen cli should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf-8");
    let json: Value = serde_json::from_str(&stdout).expect("stdout should be json");

    assert_eq!(json["command"], "docgen");
    assert!(json["file"].is_string());
    assert!(json["output_dir"].is_string());
    assert!(json["module_doc_path"].is_string());
    assert!(json["item_count"].is_number());
    assert!(json["project_symbol_count"].is_number());
    assert!(json["builtin_symbol_count"].is_number());
    assert_eq!(
        json["item_count"].as_u64().expect("item_count should be u64"),
        json["project_symbol_count"].as_u64().expect("project_symbol_count should be u64")
            + json["builtin_symbol_count"].as_u64().expect("builtin_symbol_count should be u64")
    );
    assert!(json["symbol_kind_counts"].is_object());
    assert!(json["summary"].is_object());
    assert_eq!(json["summary"]["schema_version"], "docgen-summary/v1");
    assert_eq!(
        json["summary"]["item_count"].as_u64().expect("summary item_count should be u64"),
        json["item_count"].as_u64().expect("item_count should be u64")
    );
    assert!(json["project_json_path"].is_string());
    assert!(json["gaps_json_path"].is_string());
}

#[test]
fn docgen_supports_mixed_language_projects_deterministically() {
    let dir = unique_temp_dir("mixed");
    let out_a = dir.join("docs_a");
    let out_b = dir.join("docs_b");

    write_file(&dir.join("main.kujo"), "func kujo_fn() { return 1 }\n");
    write_file(&dir.join("a.py"), "def py_fn(x):\n    return x\n");
    write_file(&dir.join("b.php"), "<?php\nfunction php_fn($x) { return $x; }\n");
    write_file(&dir.join("c.ts"), "export function tsFn(x: number): number { return x; }\n");
    write_file(&dir.join("d.js"), "export function jsFn(x) { return x; }\n");
    write_file(&dir.join("e.rb"), "def rb_fn(x)\n  x\nend\n");
    write_file(&dir.join("f.go"), "package main\nfunc GoFn(x int) int { return x }\n");
    write_file(&dir.join("g.hs"), "-- | doc\nhFn :: Int -> Int\nhFn x = x\n");
    write_file(&dir.join("h.zig"), "pub fn zigFn(x: i32) i32 { return x; }\n");

    let run_config = |out_dir: PathBuf| DocgenConfig {
        input: dir.clone(),
        out_dir,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: None,
        languages: Some("kujo,php,python,typescript,javascript,ruby,go,haskell,zig".to_string()),
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    };

    let (_, summary_a) =
        run_docgen(&run_config(out_a.clone())).expect("first docgen run should succeed");
    let (_, summary_b) =
        run_docgen(&run_config(out_b.clone())).expect("second docgen run should succeed");

    let json_a = fs::read_to_string(summary_a.project_json_path).expect("read first project json");
    let json_b = fs::read_to_string(summary_b.project_json_path).expect("read second project json");
    assert_eq!(json_a, json_b, "docgen output should be deterministic");

    let parsed: Value = serde_json::from_str(&json_a).expect("valid json");
    let langs: BTreeSet<String> = parsed["languages"]
        .as_array()
        .expect("languages should be array")
        .iter()
        .filter_map(|entry| entry.as_str().map(ToOwned::to_owned))
        .collect();

    for expected in
        ["kujo", "php", "python", "typescript", "javascript", "ruby", "go", "haskell", "zig"]
    {
        assert!(langs.contains(expected), "missing language {expected}");
    }
}

#[test]
fn docgen_does_not_follow_symlink_escape() {
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;

        let root = unique_temp_dir("symlink_root");
        let outside = unique_temp_dir("symlink_outside");
        let out_dir = root.join("docs");
        let outside_file = outside.join("escape.py");
        write_file(&outside_file, "def stolen():\n    return 1\n");
        let symlink = root.join("linked_escape.py");
        unix_fs::symlink(&outside_file, &symlink).expect("failed to create symlink");

        let (_project, summary) = run_docgen(&DocgenConfig {
            input: root.clone(),
            out_dir,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: None,
            languages: Some("python".to_string()),
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: false,
            fail_on_broken_links: false,
            fail_on_warnings: false,
            public_only: false,
            include_private: true,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        })
        .expect("docgen should succeed");

        let json = fs::read_to_string(summary.project_json_path).expect("read json");
        assert!(!json.contains("stolen"), "symlink escape file should not be discovered");
    }
}

#[test]
fn docgen_never_executes_source_code() {
    let dir = unique_temp_dir("no_exec");
    let out = dir.join("docs");
    let marker = dir.join("should_not_exist.txt");

    write_file(
        &dir.join("danger.kujo"),
        &format!("write_file(\"{}\", \"owned\")\nfunc safe() {{ return 1 }}\n", marker.display()),
    );

    let output = run_kujo(
        &[
            "docgen",
            dir.to_str().expect("dir utf-8"),
            "--language",
            "kujo",
            "--out-dir",
            out.to_str().expect("out utf-8"),
        ],
        &dir,
    );

    assert!(output.status.success(), "docgen should parse without executing code");
    assert!(
        !marker.exists(),
        "marker file should not exist because docgen must not execute source"
    );
}

#[test]
fn docgen_html_escapes_untrusted_docs() {
    let dir = unique_temp_dir("escape_html");
    let out = dir.join("docs");
    let input = dir.join("x.kujo");
    write_file(&input, "/// <script>alert('xss')</script>\nfunc test() { return 1 }\n");

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Html,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let html = fs::read_to_string(summary.module_doc_path).expect("read html");
    assert!(html.contains("&lt;script&gt;alert"), "html output should be escaped");
    assert!(!html.contains("<script>alert"), "raw script should not be present");
}

#[test]
fn docgen_adapter_conformance_smoke_extracts_symbols_for_all_languages() {
    let dir = unique_temp_dir("adapter_conformance");
    let out = dir.join("docs");

    write_file(&dir.join("sample.kujo"), "func kujo_fn() { return 1 }\n");
    write_file(&dir.join("sample.php"), "<?php\nfunction php_fn($x) { return $x; }\n");
    write_file(&dir.join("sample.py"), "def py_fn(x):\n    return x\n");
    write_file(&dir.join("sample.ts"), "export function tsFn(x: number): number { return x; }\n");
    write_file(&dir.join("sample.js"), "export function jsFn(x) { return x; }\n");
    write_file(&dir.join("sample.rb"), "def rb_fn(x)\n  x\nend\n");
    write_file(&dir.join("sample.go"), "package main\nfunc GoFn(x int) int { return x }\n");
    write_file(&dir.join("sample.hs"), "hFn :: Int -> Int\nhFn x = x\n");
    write_file(&dir.join("sample.zig"), "pub fn zigFn(x: i32) i32 { return x; }\n");

    let (project, _summary) = run_docgen(&DocgenConfig {
        input: dir.clone(),
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: None,
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let present: BTreeSet<String> =
        project.symbols.iter().map(|symbol| symbol.language.clone()).collect();
    for language in
        ["kujo", "php", "python", "typescript", "javascript", "ruby", "go", "haskell", "zig"]
    {
        assert!(
            present.contains(language),
            "expected at least one extracted symbol for language {language}"
        );
    }
}

#[test]
fn docgen_adapter_conformance_edge_fixtures_preserve_shape_and_visibility_contracts() {
    let dir = unique_temp_dir("adapter_conformance_edges");
    let out = dir.join("docs");
    let fixtures = [
        ("conformance_edges.kujo", "sample.kujo"),
        ("conformance_edges.php", "sample.php"),
        ("conformance_edges.py", "sample.py"),
        ("conformance_edges.ts", "sample.ts"),
        ("conformance_edges.js", "sample.js"),
        ("conformance_edges.rb", "sample.rb"),
        ("conformance_edges.go", "sample.go"),
        ("conformance_edges.hs", "sample.hs"),
        ("conformance_edges.zig", "sample.zig"),
    ];

    for (fixture_name, output_name) in fixtures {
        let source =
            fs::read_to_string(docgen_fixture_path(fixture_name)).expect("failed to read fixture");
        write_file(&dir.join(output_name), &source);
    }

    let (_project, summary) = run_docgen(&DocgenConfig {
        input: dir.clone(),
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: None,
        languages: Some("kujo,php,python,typescript,javascript,ruby,go,haskell,zig".to_string()),
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let project_json =
        fs::read_to_string(summary.project_json_path).expect("failed to read project json");
    let project: Value = serde_json::from_str(&project_json).expect("project json should be valid");
    let symbols = project["symbols"].as_array().expect("symbols should be array");

    let present_languages: BTreeSet<String> = symbols
        .iter()
        .filter_map(|symbol| symbol["language"].as_str().map(ToOwned::to_owned))
        .collect();
    for language in
        ["kujo", "php", "python", "typescript", "javascript", "ruby", "go", "haskell", "zig"]
    {
        assert!(present_languages.contains(language), "expected symbols for language {language}");
    }

    for symbol in symbols {
        assert!(
            symbol.get("id").is_some_and(Value::is_string),
            "symbol is missing string id: {symbol:?}"
        );
        assert!(
            symbol.get("language").is_some_and(Value::is_string),
            "symbol is missing string language: {symbol:?}"
        );
        assert!(
            symbol.get("kind").is_some_and(Value::is_string),
            "symbol is missing string kind: {symbol:?}"
        );
        assert!(
            symbol.get("name").is_some_and(Value::is_string),
            "symbol is missing string name: {symbol:?}"
        );
        assert!(
            symbol.get("qualified_name").is_some_and(Value::is_string),
            "symbol is missing string qualified_name: {symbol:?}"
        );
        assert!(
            symbol.get("visibility").is_some_and(Value::is_string),
            "symbol is missing string visibility: {symbol:?}"
        );
        assert!(
            symbol.get("source_path").is_some_and(Value::is_string),
            "symbol is missing string source_path: {symbol:?}"
        );
        assert!(
            symbol.get("line").is_some_and(Value::is_number),
            "symbol is missing numeric line: {symbol:?}"
        );
        assert!(
            symbol.get("docs").is_some_and(Value::is_object),
            "symbol is missing docs object: {symbol:?}"
        );
        assert!(
            symbol["docs"].get("placeholder").is_some_and(Value::is_boolean),
            "symbol docs missing placeholder boolean: {symbol:?}"
        );
    }

    for documented_symbol in [
        "exported_async_task",
        "HiddenWorker::promoted",
        "JsClient.fetchData",
        "public_helper",
        "phpPublicHelper",
        "Runner#run",
        "PublicHelper",
        "valueFn",
        "zigPublic",
    ] {
        let symbol = symbols
            .iter()
            .find(|candidate| candidate["qualified_name"] == documented_symbol)
            .unwrap_or_else(|| panic!("missing documented symbol '{documented_symbol}'"));
        assert!(
            !symbol["docs"]["placeholder"].as_bool().expect("docs.placeholder should be bool"),
            "expected docs to attach for symbol {documented_symbol}"
        );
    }

    for undocumented_symbol in [
        "public_without_docs_kujo",
        "tsPublicWithoutDocs",
        "jsPublicWithoutDocs",
        "public_without_docs_py",
        "phpPublicWithoutDocs",
        "ruby_without_docs",
        "publicWithoutDocs",
        "zigWithoutDocs",
    ] {
        let symbol = symbols
            .iter()
            .find(|candidate| candidate["qualified_name"] == undocumented_symbol)
            .unwrap_or_else(|| panic!("missing undocumented symbol '{undocumented_symbol}'"));
        assert!(
            symbol["docs"]["placeholder"].as_bool().expect("docs.placeholder should be bool"),
            "expected undocumented placeholder for symbol {undocumented_symbol}"
        );
    }

    assert_eq!(symbol_visibility(symbols, "HiddenWorker::promoted"), "Private");
    assert_eq!(symbol_visibility(symbols, "TsClient.cacheData"), "Private");
    assert_eq!(symbol_visibility(symbols, "_private_helper"), "Private");
    assert_eq!(symbol_visibility(symbols, "zigPrivate"), "Private");
    assert!(
        !has_symbol(symbols, "tsAsyncHelper"),
        "TypeScript adapter currently does not extract `export async function` declarations"
    );
    assert!(
        !has_symbol(symbols, "TsClient.fetchData"),
        "TypeScript adapter currently does not extract `async` class methods"
    );
}

#[test]
fn docgen_adapter_conformance_edge_fixtures_strict_public_gate_reports_undocumented_symbols() {
    let dir = unique_temp_dir("adapter_conformance_edges_strict");
    let out = dir.join("docs");
    let fixtures = [
        ("conformance_edges.kujo", "sample.kujo"),
        ("conformance_edges.php", "sample.php"),
        ("conformance_edges.py", "sample.py"),
        ("conformance_edges.ts", "sample.ts"),
        ("conformance_edges.js", "sample.js"),
        ("conformance_edges.rb", "sample.rb"),
        ("conformance_edges.go", "sample.go"),
        ("conformance_edges.hs", "sample.hs"),
        ("conformance_edges.zig", "sample.zig"),
    ];

    for (fixture_name, output_name) in fixtures {
        let source =
            fs::read_to_string(docgen_fixture_path(fixture_name)).expect("failed to read fixture");
        write_file(&dir.join(output_name), &source);
    }

    let (_project, summary) = run_docgen(&DocgenConfig {
        input: dir,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: None,
        languages: Some("kujo,php,python,typescript,javascript,ruby,go,haskell,zig".to_string()),
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("strict docgen run should complete");

    assert!(summary.undocumented_count > 0, "strict gate should detect undocumented symbols");
    assert!(
        summary.gate_failures.iter().any(|entry| entry.contains("undocumented")),
        "strict gate should emit undocumented failure message"
    );
}

#[test]
fn docgen_strict_gates_fail_as_expected() {
    let dir = unique_temp_dir("strict_gates");
    let out = dir.join("docs");
    let input = dir.join("gate.kujo");
    write_file(
        &input,
        "/// See [Missing](missing.md)\npub func documented() { return 1 }\npub func undocumented() { return 2 }\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen run itself should complete and report gate failures");

    assert!(!summary.gate_failures.is_empty(), "strict gates should report failure messages");
    assert!(
        summary.gate_failures.iter().any(|entry| entry.contains("undocumented")),
        "expected undocumented gate failure"
    );
    assert!(
        summary.gate_failures.iter().any(|entry| entry.contains("broken links")),
        "expected broken-link gate failure"
    );
}

#[test]
fn docgen_default_link_check_allows_existing_local_anchor_links_and_external_links() {
    let dir = unique_temp_dir("default_link_check_success");
    let out = dir.join("docs");
    let input = dir.join("links_ok.kujo");
    let local_target = dir.join("guide.md");
    write_file(&local_target, "# Guide\n\n## intro\n");
    write_file(
        &input,
        "/// Local anchor: [Guide](guide.md#intro)\n/// External link: [Example](https://example.com/docs)\n/// Mail link: [Mail](mailto:team@example.com)\npub func linked_api() { return 1 }\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen run should complete");

    assert_eq!(summary.undocumented_count, 0);
    assert_eq!(summary.broken_link_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(
        summary.gate_failures.is_empty(),
        "default link checking should only enforce local file existence and ignore anchor/external validation"
    );
}

#[test]
fn docgen_default_link_check_reports_missing_local_links() {
    let dir = unique_temp_dir("default_link_check_missing_local");
    let out = dir.join("docs");
    let input = dir.join("links_missing.kujo");
    write_file(
        &input,
        "/// Missing local doc: [Missing](missing.md)\npub func linked_api() { return 1 }\n",
    );

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: true,
        fail_on_broken_links: true,
        fail_on_warnings: true,
        public_only: true,
        include_private: false,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen run should complete");

    assert_eq!(summary.undocumented_count, 0);
    assert_eq!(summary.broken_link_count, 1);
    assert_eq!(summary.warning_count, 1);
    assert!(
        summary.gate_failures.iter().any(|entry| entry.starts_with("1 broken links detected")),
        "missing local links should fail default link validation"
    );
}

#[test]
fn docgen_optional_local_anchor_validation_passes_for_existing_anchor() {
    let dir = unique_temp_dir("local_anchor_validation_pass");
    let out = dir.join("docs");
    let input = dir.join("anchors_ok.kujo");
    write_file(&dir.join("guide.md"), "# Guide\n\n## intro section\n");
    write_file(
        &input,
        "/// Anchor link: [Guide](guide.md#intro-section)\npub func linked_api() { return 1 }\n",
    );

    let (_project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: true,
            fail_on_broken_links: true,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions { validate_local_anchors: true, ..LinkValidationOptions::default() },
    )
    .expect("docgen run should complete");

    assert_eq!(
        summary.broken_link_count, 0,
        "unexpected broken links: {:?}",
        summary.gate_failures
    );
    assert!(
        summary.gate_failures.is_empty(),
        "unexpected gate failures: {:?}",
        summary.gate_failures
    );
}

#[test]
fn docgen_optional_local_anchor_validation_fails_for_missing_anchor() {
    let dir = unique_temp_dir("local_anchor_validation_fail");
    let out = dir.join("docs");
    let input = dir.join("anchors_missing.kujo");
    write_file(&dir.join("guide.md"), "# Guide\n\n## intro section\n");
    write_file(
        &input,
        "/// Missing anchor: [Guide](guide.md#does-not-exist)\npub func linked_api() { return 1 }\n",
    );

    let (_project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: true,
            fail_on_broken_links: true,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions { validate_local_anchors: true, ..LinkValidationOptions::default() },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 1);
    assert!(
        summary.gate_failures.iter().any(|entry| entry.starts_with("1 broken links detected")),
        "missing anchors should fail when local anchor validation mode is enabled"
    );
}

#[test]
fn docgen_cli_optional_local_anchor_validation_flag_enforces_anchor_checks() {
    let dir = unique_temp_dir("local_anchor_validation_cli_flag");
    let out = dir.join("docs");
    let input = dir.join("anchors_cli.kujo");
    write_file(&dir.join("guide.md"), "# Guide\n\n## intro section\n");
    write_file(
        &input,
        "/// Missing anchor: [Guide](guide.md#does-not-exist)\npub func linked_api() { return 1 }\n",
    );

    let output = run_kujo(
        &[
            "docgen",
            dir.to_str().expect("dir path utf-8"),
            "--language",
            "kujo",
            "--out-dir",
            out.to_str().expect("out path utf-8"),
            "--public-only",
            "--fail-on-broken-links",
            "--validate-local-anchors",
        ],
        &dir,
    );

    assert!(!output.status.success(), "missing anchors should fail when anchor mode is enabled");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("docgen gate failed: 1 broken links detected"));
}

#[test]
fn docgen_optional_external_validation_skips_non_allowlisted_hosts() {
    let dir = unique_temp_dir("external_validation_allowlist_skip");
    let out = dir.join("docs");
    let input = dir.join("external_skip.kujo");
    write_file(
        &input,
        "/// External link: [Docs](https://example.com/missing)\npub func linked_api() { return 1 }\n",
    );

    let (_project, summary) = match std::panic::catch_unwind(|| {
        run_docgen_with_link_validation(
            &DocgenConfig {
                input,
                out_dir: out,
                format: DocOutputFormat::Json,
                include_builtins: false,
                language: Some("kujo".to_string()),
                languages: None,
                emit_ai_tasks: false,
                search_index: false,
                source_links: false,
                source_link_template: None,
                fail_on_undocumented: true,
                fail_on_broken_links: true,
                fail_on_warnings: true,
                public_only: true,
                include_private: false,
                max_discovery_file_size_bytes: None,
                max_discovery_files: None,
                max_discovery_depth: None,
                cache_dir: None,
            },
            LinkValidationOptions {
                validate_local_anchors: false,
                validate_external_links: true,
                external_link_timeout_ms: 100,
                external_link_allowlist: BTreeSet::from(["localhost".to_string()]),
                allow_private_network_links: false,
                ..LinkValidationOptions::default()
            },
        )
    }) {
        Ok(result) => result.expect("docgen run should complete"),
        Err(_) => {
            eprintln!(
                "skipping docgen_optional_external_validation_skips_non_allowlisted_hosts: \
                     host runtime does not support reqwest/system-configuration initialization"
            );
            return;
        }
    };

    assert_eq!(summary.broken_link_count, 0);
    assert!(summary.gate_failures.is_empty());
}

#[test]
fn docgen_optional_external_validation_fails_allowlisted_unreachable_hosts() {
    let dir = unique_temp_dir("external_validation_unreachable");
    let out = dir.join("docs");
    let input = dir.join("external_fail.kujo");
    write_file(
        &input,
        "/// Localhost link: [Docs](http://127.0.0.1:9/docs)\npub func linked_api() { return 1 }\n",
    );

    let (_project, summary) = match std::panic::catch_unwind(|| {
        run_docgen_with_link_validation(
            &DocgenConfig {
                input,
                out_dir: out,
                format: DocOutputFormat::Json,
                include_builtins: false,
                language: Some("kujo".to_string()),
                languages: None,
                emit_ai_tasks: false,
                search_index: false,
                source_links: false,
                source_link_template: None,
                fail_on_undocumented: true,
                fail_on_broken_links: true,
                fail_on_warnings: true,
                public_only: true,
                include_private: false,
                max_discovery_file_size_bytes: None,
                max_discovery_files: None,
                max_discovery_depth: None,
                cache_dir: None,
            },
            LinkValidationOptions {
                validate_local_anchors: false,
                validate_external_links: true,
                external_link_timeout_ms: 100,
                external_link_allowlist: BTreeSet::from(["127.0.0.1".to_string()]),
                allow_private_network_links: true,
                ..LinkValidationOptions::default()
            },
        )
    }) {
        Ok(result) => result.expect("docgen run should complete"),
        Err(_) => {
            eprintln!(
                    "skipping docgen_optional_external_validation_fails_allowlisted_unreachable_hosts: \
                     host runtime does not support reqwest/system-configuration initialization"
                );
            return;
        }
    };

    assert_eq!(summary.broken_link_count, 1);
    assert!(summary.gate_failures.iter().any(|entry| entry.starts_with("1 broken links detected")));
}

#[test]
fn docgen_external_validation_blocks_direct_private_ip_by_default() {
    let dir = unique_temp_dir("external_validation_blocks_direct_private_ip");
    let out = dir.join("docs");
    let input = dir.join("external_private_ip.kujo");
    write_file(
        &input,
        "/// Private IP link: [Docs](http://127.0.0.1:9/docs)\npub func linked_api() { return 1 }\n",
    );

    let (project, summary) = match std::panic::catch_unwind(|| {
        run_docgen_with_link_validation(
            &DocgenConfig {
                input,
                out_dir: out,
                format: DocOutputFormat::Json,
                include_builtins: false,
                language: Some("kujo".to_string()),
                languages: None,
                emit_ai_tasks: false,
                search_index: false,
                source_links: false,
                source_link_template: None,
                fail_on_undocumented: true,
                fail_on_broken_links: true,
                fail_on_warnings: true,
                public_only: true,
                include_private: false,
                max_discovery_file_size_bytes: None,
                max_discovery_files: None,
                max_discovery_depth: None,
                cache_dir: None,
            },
            LinkValidationOptions {
                validate_local_anchors: false,
                validate_external_links: true,
                external_link_timeout_ms: 100,
                external_link_allowlist: BTreeSet::from(["127.0.0.1".to_string()]),
                allow_private_network_links: false,
                ..LinkValidationOptions::default()
            },
        )
    }) {
        Ok(result) => result.expect("docgen run should complete"),
        Err(_) => {
            eprintln!(
                "skipping docgen_external_validation_blocks_direct_private_ip_by_default: \
                     host runtime does not support reqwest/system-configuration initialization"
            );
            return;
        }
    };

    assert_eq!(summary.broken_link_count, 1);
    assert!(summary.gate_failures.iter().any(|entry| entry.starts_with("1 broken links detected")));
    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_LINK_BROKEN_EXTERNAL_PRIVATE_ADDRESS"
                && diag.message.contains("127.0.0.1")
                && diag.message.contains("linked_api")
        }),
        "private-address rejection should emit deterministic diagnostics"
    );
}

#[test]
fn docgen_external_validation_blocks_dns_hosts_resolving_to_private_ranges_by_default() {
    let dir = unique_temp_dir("external_validation_blocks_dns_private_range");
    let out = dir.join("docs");
    let input = dir.join("external_localhost.kujo");
    write_file(
        &input,
        "/// Localhost link: [Docs](http://localhost:80/docs)\npub func linked_api() { return 1 }\n",
    );

    let (project, summary) = match std::panic::catch_unwind(|| {
        run_docgen_with_link_validation(
            &DocgenConfig {
                input,
                out_dir: out,
                format: DocOutputFormat::Json,
                include_builtins: false,
                language: Some("kujo".to_string()),
                languages: None,
                emit_ai_tasks: false,
                search_index: false,
                source_links: false,
                source_link_template: None,
                fail_on_undocumented: true,
                fail_on_broken_links: true,
                fail_on_warnings: true,
                public_only: true,
                include_private: false,
                max_discovery_file_size_bytes: None,
                max_discovery_files: None,
                max_discovery_depth: None,
                cache_dir: None,
            },
            LinkValidationOptions {
                validate_local_anchors: false,
                validate_external_links: true,
                external_link_timeout_ms: 100,
                external_link_allowlist: BTreeSet::from(["localhost".to_string()]),
                allow_private_network_links: false,
                ..LinkValidationOptions::default()
            },
        )
    }) {
        Ok(result) => result.expect("docgen run should complete"),
        Err(_) => {
            eprintln!(
                    "skipping docgen_external_validation_blocks_dns_hosts_resolving_to_private_ranges_by_default: \
                     host runtime does not support reqwest/system-configuration initialization"
                );
            return;
        }
    };

    assert_eq!(summary.broken_link_count, 1);
    assert!(summary.gate_failures.iter().any(|entry| entry.starts_with("1 broken links detected")));
    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_LINK_BROKEN_EXTERNAL_PRIVATE_ADDRESS"
                && diag.message.contains("localhost")
                && (diag.message.contains("127.0.0.1") || diag.message.contains("::1"))
        }),
        "dns hostnames resolving to blocked private ranges should be rejected by default"
    );
}

#[test]
fn docgen_external_validation_allows_same_host_redirect_hops() {
    let Some(redirect_server) = spawn_http_server(2, |path| {
        if path == "/start" {
            return http_302_response("/final");
        }
        http_200_response()
    }) else {
        return;
    };
    let dir = unique_temp_dir("external_validation_same_host_redirect");
    let out = dir.join("docs");
    let input = dir.join("external_same_host_redirect.kujo");
    write_file(
        &input,
        &format!(
            "/// Redirect link: [Docs]({})\npub func linked_api() {{ return 1 }}\n",
            redirect_server.url_with_host("127.0.0.1", "/start")
        ),
    );

    let (_project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: true,
            fail_on_broken_links: true,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions {
            validate_local_anchors: false,
            validate_external_links: true,
            external_link_timeout_ms: 500,
            external_link_allowlist: BTreeSet::from(["127.0.0.1".to_string()]),
            allow_private_network_links: true,
            ..LinkValidationOptions::default()
        },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 0);
    assert!(summary.gate_failures.is_empty());
}

#[test]
fn docgen_external_validation_allows_cross_host_redirect_when_hosts_are_allowlisted() {
    // Allow extra requests because redirect validation may issue retries under load.
    let Some(destination_server) = spawn_http_server(16, |_path| http_200_response()) else {
        return;
    };
    let destination_url = destination_server.url_with_host("localhost", "/final");
    let Some(redirect_server) =
        spawn_http_server(16, move |_path| http_302_response(&destination_url))
    else {
        return;
    };

    let dir = unique_temp_dir("external_validation_cross_host_allowlisted_redirect");
    let out = dir.join("docs");
    let input = dir.join("external_cross_host_redirect.kujo");
    write_file(
        &input,
        &format!(
            "/// Redirect link: [Docs]({})\npub func linked_api() {{ return 1 }}\n",
            redirect_server.url_with_host("127.0.0.1", "/start")
        ),
    );

    let (_project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: true,
            fail_on_broken_links: true,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions {
            validate_local_anchors: false,
            validate_external_links: true,
            external_link_timeout_ms: 2_000,
            external_link_allowlist: BTreeSet::from([
                "127.0.0.1".to_string(),
                "localhost".to_string(),
            ]),
            allow_private_network_links: true,
            ..LinkValidationOptions::default()
        },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 0);
    assert!(summary.gate_failures.is_empty());
}

#[test]
fn docgen_external_validation_blocks_redirects_to_non_allowlisted_hosts() {
    let Some(destination_server) = spawn_http_server_on("[::1]:0", 4, |_path| http_200_response())
    else {
        return;
    };
    let blocked_target = destination_server.url_with_host("::1", "/blocked");
    let Some(redirect_server) =
        spawn_http_server(4, move |_path| http_302_response(&blocked_target))
    else {
        return;
    };

    let dir = unique_temp_dir("external_validation_blocked_redirect_host");
    let out = dir.join("docs");
    let input = dir.join("external_blocked_redirect.kujo");
    write_file(
        &input,
        &format!(
            "/// Redirect link: [Docs]({})\npub func linked_api() {{ return 1 }}\n",
            redirect_server.url_with_host("127.0.0.1", "/start")
        ),
    );

    let (project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: true,
            fail_on_broken_links: true,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions {
            validate_local_anchors: false,
            validate_external_links: true,
            external_link_timeout_ms: 500,
            external_link_allowlist: BTreeSet::from(["127.0.0.1".to_string()]),
            allow_private_network_links: true,
            ..LinkValidationOptions::default()
        },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 1);
    assert!(summary.gate_failures.iter().any(|entry| entry.starts_with("1 broken links detected")));
    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_LINK_BROKEN_EXTERNAL_REDIRECT_ALLOWLIST"
                && diag.message.contains("::1")
                && diag.message.contains("non-allowlisted host")
                && diag.message.contains("linked_api")
        }),
        "blocked redirect host should produce deterministic external broken-link diagnostics"
    );
}

#[test]
fn docgen_external_validation_warns_when_allowlist_is_empty() {
    let dir = unique_temp_dir("external_validation_empty_allowlist_warning");
    let out = dir.join("docs");
    let input = dir.join("external_warning.kujo");
    write_file(
        &input,
        "/// External link: [Docs](https://example.com/reference)\npub func linked_api() { return 1 }\n",
    );

    let (project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: false,
            fail_on_broken_links: false,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions {
            validate_external_links: true,
            external_link_allowlist: BTreeSet::new(),
            ..LinkValidationOptions::default()
        },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 0);
    assert_eq!(summary.warning_count, 1);
    assert!(summary.gate_failures.iter().any(|entry| entry == "1 warnings detected"));
    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_LINK_EXTERNAL_ALLOWLIST_EMPTY"
                && diag.message.contains("external link validation is enabled")
        }),
        "external mode should emit an explicit warning when allowlist is empty"
    );
}

#[test]
fn docgen_external_allowlist_warns_when_external_validation_is_disabled() {
    let dir = unique_temp_dir("external_allowlist_ignored_warning");
    let out = dir.join("docs");
    let input = dir.join("external_ignored_warning.kujo");
    write_file(
        &input,
        "/// External link: [Docs](https://example.com/reference)\npub func linked_api() { return 1 }\n",
    );

    let (project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: false,
            fail_on_broken_links: false,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions {
            validate_external_links: false,
            external_link_allowlist: BTreeSet::from(["example.com".to_string()]),
            ..LinkValidationOptions::default()
        },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 0);
    assert_eq!(summary.warning_count, 1);
    assert!(summary.gate_failures.iter().any(|entry| entry == "1 warnings detected"));
    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_LINK_EXTERNAL_ALLOWLIST_IGNORED"
                && diag.message.contains("without --validate-external-links")
        }),
        "allowlist without external validation should emit a warning"
    );
}

#[test]
fn docgen_cli_exposes_external_link_validation_flags() {
    let dir = unique_temp_dir("docgen_cli_external_flag_help");
    let output = run_kujo(&["docgen", "--help"], &dir);
    assert!(output.status.success(), "docgen --help should succeed");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("--validate-external-links"));
    assert!(stdout.contains("--external-link-timeout-ms"));
    assert!(stdout.contains("--external-link-allowlist"));
    assert!(stdout.contains("--allow-private-network-links"));
    assert!(stdout.contains("--kujo-parser-assisted"));
    assert!(stdout.contains("--max-link-checks"));
    assert!(stdout.contains("--max-external-link-checks"));
    assert!(stdout.contains("--max-total-validation-time-ms"));
}

#[test]
fn docgen_link_validation_budget_max_link_checks_truncates_deterministically() {
    let dir = unique_temp_dir("docgen_link_budget_max_link_checks");
    let out = dir.join("docs");
    let input = dir.join("budget_max_links.kujo");
    write_file(
        &input,
        "/// Missing local A: [A](missing-a.md)\n/// Missing local B: [B](missing-b.md)\n/// Missing local C: [C](missing-c.md)\npub func linked_api() { return 1 }\n",
    );

    let (project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: true,
            fail_on_broken_links: true,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions { max_link_checks: Some(1), ..LinkValidationOptions::default() },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 1);
    assert_eq!(summary.link_validation_skip_counts.get("max_link_checks"), Some(&2usize));
    assert_eq!(summary.link_validation_skip_counts.get("max_external_checks"), Some(&0usize));
    assert_eq!(summary.link_validation_skip_counts.get("max_total_time"), Some(&0usize));
    assert!(summary.gate_failures.iter().any(|entry| entry.starts_with("1 broken links detected")));
    assert!(summary.gate_failures.iter().any(|entry| entry == "2 warnings detected"));
    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_LINK_VALIDATION_BUDGET_MAX_LINK_CHECKS"
                && diag.message.contains("skipped 2 links")
        }),
        "budget-truncation diagnostics should be deterministic"
    );
}

#[test]
fn docgen_link_validation_budget_max_external_checks_truncates_deterministically() {
    let Some(server) = spawn_http_server(1, |_path| http_200_response()) else {
        return;
    };
    let dir = unique_temp_dir("docgen_link_budget_max_external_checks");
    let out = dir.join("docs");
    let input = dir.join("budget_max_external.kujo");
    write_file(
        &input,
        &format!(
            "/// External A: [A]({})\n/// External B: [B]({})\npub func linked_api() {{ return 1 }}\n",
            server.url_with_host("127.0.0.1", "/one"),
            server.url_with_host("127.0.0.1", "/two")
        ),
    );

    let (project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: true,
            fail_on_broken_links: true,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions {
            validate_external_links: true,
            external_link_allowlist: BTreeSet::from(["127.0.0.1".to_string()]),
            allow_private_network_links: true,
            max_external_link_checks: Some(1),
            ..LinkValidationOptions::default()
        },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 0);
    assert_eq!(summary.link_validation_skip_counts.get("max_link_checks"), Some(&0usize));
    assert_eq!(summary.link_validation_skip_counts.get("max_external_checks"), Some(&1usize));
    assert_eq!(summary.link_validation_skip_counts.get("max_total_time"), Some(&0usize));
    assert!(summary.gate_failures.iter().any(|entry| entry == "1 warnings detected"));
    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_LINK_VALIDATION_BUDGET_MAX_EXTERNAL_CHECKS"
                && diag.message.contains("skipped 1 external links")
        }),
        "external-budget truncation diagnostics should be deterministic"
    );
}

#[test]
fn docgen_link_validation_budget_total_time_truncates_deterministically() {
    let dir = unique_temp_dir("docgen_link_budget_total_time");
    let out = dir.join("docs");
    let input = dir.join("budget_total_time.kujo");
    write_file(
        &input,
        "/// Missing local A: [A](missing-a.md)\n/// Missing local B: [B](missing-b.md)\npub func linked_api() { return 1 }\n",
    );

    let (project, summary) = run_docgen_with_link_validation(
        &DocgenConfig {
            input,
            out_dir: out,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: true,
            fail_on_broken_links: true,
            fail_on_warnings: true,
            public_only: true,
            include_private: false,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: None,
        },
        LinkValidationOptions {
            max_total_validation_time_ms: Some(0),
            ..LinkValidationOptions::default()
        },
    )
    .expect("docgen run should complete");

    assert_eq!(summary.broken_link_count, 0);
    assert_eq!(summary.link_validation_skip_counts.get("max_link_checks"), Some(&0usize));
    assert_eq!(summary.link_validation_skip_counts.get("max_external_checks"), Some(&0usize));
    assert_eq!(summary.link_validation_skip_counts.get("max_total_time"), Some(&2usize));
    assert!(summary.gate_failures.iter().any(|entry| entry == "1 warnings detected"));
    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_LINK_VALIDATION_BUDGET_TOTAL_TIME"
                && diag.message.contains("skipped 2 links")
        }),
        "time-budget truncation diagnostics should be deterministic"
    );
}

#[test]
fn docgen_large_repo_smoke_completes_with_deterministic_counts() {
    let dir = unique_temp_dir("large_repo");
    let out = dir.join("docs");

    for idx in 0..250usize {
        write_file(
            &dir.join(format!("pkg/mod_{idx}.py")),
            &format!("def fn_{idx}(x):\n    return x\n"),
        );
    }

    let (_project, summary) = run_docgen(&DocgenConfig {
        input: dir.clone(),
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: Some("python".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("large repo docgen should succeed");

    assert_eq!(summary.item_count, 250);
    assert_eq!(summary.project_symbol_count, 250);
    assert_eq!(summary.builtin_symbol_count, 0);
    assert_eq!(summary.symbol_kind_counts.get("function").copied().unwrap_or_default(), 250);
}

#[test]
fn docgen_summary_splits_project_and_builtin_counts() {
    let dir = unique_temp_dir("summary_split_project_builtin");
    let out = dir.join("docs");
    let input = dir.join("counts.kujo");
    write_file(&input, "pub func alpha() {\n    return 1\n}\n");

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: true,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    assert!(summary.builtin_symbol_count > 0);
    assert_eq!(summary.item_count, summary.project_symbol_count + summary.builtin_symbol_count);
    assert_eq!(summary.project_symbol_count, 1);
    assert_eq!(summary.symbol_kind_counts.get("function").copied().unwrap_or_default(), 1);
    assert_eq!(
        summary.symbol_kind_counts.get("builtin").copied().unwrap_or_default(),
        summary.builtin_symbol_count
    );
    assert_eq!(summary.dashboard_summary.schema_version, "docgen-summary/v1");
    assert_eq!(summary.dashboard_summary.item_count, summary.item_count);
    assert_eq!(summary.dashboard_summary.project_symbol_count, summary.project_symbol_count);
    assert_eq!(summary.dashboard_summary.builtin_symbol_count, summary.builtin_symbol_count);
}

#[test]
fn docgen_adapter_health_counters_and_low_yield_warnings_are_emitted() {
    let dir = unique_temp_dir("adapter_health_low_yield");
    let out = dir.join("docs");
    let kujo_file = dir.join("api.kujo");
    write_file(
        &kujo_file,
        "/// Primary API.\npub func documented_api() { return 1 }\n\n// Break doc attachment to the next symbol.\npub func undocumented_api() { return 2 }\n",
    );
    write_file(&dir.join("empty_a.js"), "// intentionally empty\n");
    write_file(&dir.join("empty_b.js"), "// intentionally empty\n");
    write_file(&dir.join("empty_c.js"), "// intentionally empty\n");

    let (project, summary) = run_docgen(&DocgenConfig {
        input: dir.clone(),
        out_dir: out,
        format: DocOutputFormat::Json,
        include_builtins: false,
        language: None,
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: false,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("docgen should succeed");

    let kujo_health = summary.adapter_health.get("kujo").expect("kujo adapter health should exist");
    assert_eq!(kujo_health.files_scanned, 1);
    assert_eq!(kujo_health.symbols_extracted, 2);
    let documented_kujo_symbols = project
        .symbols
        .iter()
        .filter(|symbol| symbol.language == "kujo" && !symbol.docs.lines.is_empty())
        .count();
    let undocumented_kujo_symbols = project
        .symbols
        .iter()
        .filter(|symbol| symbol.language == "kujo" && symbol.docs.lines.is_empty())
        .count();
    assert_eq!(kujo_health.doc_blocks_attached, documented_kujo_symbols);
    assert_eq!(kujo_health.placeholders_emitted, undocumented_kujo_symbols);

    let js_health =
        summary.adapter_health.get("javascript").expect("javascript adapter health should exist");
    assert_eq!(js_health.files_scanned, 3);
    assert_eq!(js_health.symbols_extracted, 0);
    assert_eq!(js_health.doc_blocks_attached, 0);
    assert_eq!(js_health.placeholders_emitted, 0);

    assert!(
        project.diagnostics.iter().any(|diag| {
            diag.code == "DOCGEN_ADAPTER_LOW_YIELD"
                && diag.message.contains("language 'javascript'")
                && diag.message.contains("files_scanned=3")
                && diag.message.contains("symbols_extracted=0")
        }),
        "expected deterministic low-yield diagnostic for javascript adapter"
    );
    assert_eq!(
        summary
            .dashboard_summary
            .adapter_health
            .get("kujo")
            .map(|entry| entry.placeholders_emitted),
        Some(undocumented_kujo_symbols)
    );
}

#[test]
fn docgen_cache_mode_tracks_hits_and_misses_for_changed_files() {
    let dir = unique_temp_dir("docgen_cache_hits_misses");
    let cache_dir = dir.join(".docgen-cache");
    let input = dir.join("module.kujo");
    write_file(&input, "pub func api() { return 1 }\n");

    let run_once = |out_dir: PathBuf| {
        run_docgen(&DocgenConfig {
            input: input.clone(),
            out_dir,
            format: DocOutputFormat::Json,
            include_builtins: false,
            language: Some("kujo".to_string()),
            languages: None,
            emit_ai_tasks: false,
            search_index: false,
            source_links: false,
            source_link_template: None,
            fail_on_undocumented: false,
            fail_on_broken_links: false,
            fail_on_warnings: false,
            public_only: false,
            include_private: true,
            max_discovery_file_size_bytes: None,
            max_discovery_files: None,
            max_discovery_depth: None,
            cache_dir: Some(cache_dir.clone()),
        })
        .expect("docgen should succeed")
        .1
    };

    let first = run_once(dir.join("docs_first"));
    assert_eq!(first.cache_stats.hits, 0);
    assert_eq!(first.cache_stats.misses, 1);
    assert_eq!(first.item_count, 1);

    let second = run_once(dir.join("docs_second"));
    assert_eq!(second.cache_stats.hits, 1);
    assert_eq!(second.cache_stats.misses, 0);
    assert_eq!(second.item_count, 1);

    write_file(&input, "pub func api() { return 1 }\npub func api_two() { return 2 }\n");
    let third = run_once(dir.join("docs_third"));
    assert_eq!(third.cache_stats.hits, 0);
    assert_eq!(third.cache_stats.misses, 1);
    assert_eq!(third.item_count, 2);
}

#[test]
fn docgen_snapshot_stable_core_outputs() {
    let dir = unique_temp_dir("snapshot");
    let out = dir.join("docs");
    let input = dir.join("snap.kujo");
    write_file(&input, "/// Add\nfunc add(a, b) { return a + b }\nconst VALUE := 1\n");

    let (_project, summary) = run_docgen(&DocgenConfig {
        input,
        out_dir: out,
        format: DocOutputFormat::All,
        include_builtins: false,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: true,
        search_index: true,
        source_links: true,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .expect("snapshot docgen should succeed");

    let json = fs::read_to_string(summary.project_json_path).expect("read project json");
    assert!(json.contains("\"qualified_name\": \"add\""));
    assert!(json.contains("\"qualified_name\": \"VALUE\""));

    let html = fs::read_to_string(summary.module_doc_path).expect("read html");
    assert!(html.contains("Universal Kujo DocGen"));

    let markdown = fs::read_to_string(summary.output_dir.join("docgen.md")).expect("read markdown");
    assert!(markdown.contains("# Kujo DocGen"));
}
