use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn run_kujo(args: &[&str]) -> std::process::Output {
    Command::new(kujo_binary()).args(args).output().expect("failed to execute kujo binary")
}

fn run_kujo_with_env(args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(kujo_binary());
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("failed to execute kujo binary")
}

fn parse_stdout_json(output: &std::process::Output) -> Value {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8");
    serde_json::from_str(&stdout).expect("stdout should be valid json")
}

fn write_fixture(path: &Path, content: &str) {
    fs::write(path, content).expect("failed to write fixture file");
}

fn read_fixture(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read fixture '{}': {}", path.display(), err))
}

fn docgen_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("docgen")
        .join(name)
}

fn normalize_docgen_contract_paths(mut payload: Value) -> Value {
    for key in [
        "file",
        "output_dir",
        "module_doc_path",
        "project_json_path",
        "gaps_json_path",
        "capabilities_json_path",
    ] {
        payload[key] = Value::String(format!("<{}>", key.to_ascii_uppercase()));
    }

    payload["builtin_doc_path"] = Value::Null;
    payload["ai_tasks_path"] = Value::Null;
    payload
}

#[test]
fn format_json_contract_is_stable() {
    let dir = unique_temp_dir("format_json_contract");
    let file = dir.join("format_input.kujo");
    write_fixture(&file, "let value:=1\n");

    let output = run_kujo(&["format", file.to_str().expect("path should be utf-8"), "--json"]);

    assert!(output.status.success(), "format --json should succeed");
    let body = parse_stdout_json(&output);

    assert_eq!(body["command"], "format");
    assert_eq!(body["status"], "preview");
    assert!(body["changed"].is_boolean());
    assert!(body["file"].is_string());
    assert!(body["options"].is_object());
    assert!(body["formatted_source"].is_string());
}

#[test]
fn lint_json_contract_is_stable() {
    let dir = unique_temp_dir("lint_json_contract");
    let file = dir.join("lint_input.kujo");
    write_fixture(&file, "let unused_value := 42\n");

    let output = run_kujo(&["lint", file.to_str().expect("path should be utf-8"), "--json"]);

    assert!(output.status.success(), "lint --json should succeed for warning-only input");
    let body = parse_stdout_json(&output);

    assert!(body.is_array());
    let first = body
        .as_array()
        .and_then(|items| items.first())
        .expect("lint output should include at least one issue");

    assert!(first.get("rule_id").is_some());
    assert!(first.get("line").is_some());
    assert!(first.get("column").is_some());
    assert!(first.get("severity").is_some());
    assert!(first.get("message").is_some());
    assert!(first.get("fix").is_some());
}

#[test]
fn docgen_json_contract_is_stable() {
    let dir = unique_temp_dir("docgen_json_contract");
    let file = dir.join("docgen_input.kujo");
    let out_dir = dir.join("docs_out");
    write_fixture(&file, "/// Adds one\nfunc add_one(value) {\n    return value + 1\n}\n");

    let output = run_kujo(&[
        "docgen",
        file.to_str().expect("path should be utf-8"),
        "--out-dir",
        out_dir.to_str().expect("path should be utf-8"),
        "--json",
    ]);

    assert!(output.status.success(), "docgen --json should succeed");
    let body = parse_stdout_json(&output);

    assert_eq!(body["command"], "docgen");
    assert!(body["file"].is_string());
    assert!(body["output_dir"].is_string());
    assert!(body["module_doc_path"].is_string());
    assert!(body.get("builtin_doc_path").is_some());
    assert!(body["item_count"].is_number());
    assert!(body["project_symbol_count"].is_number());
    assert!(body["builtin_symbol_count"].is_number());
    assert_eq!(
        body["item_count"].as_u64().expect("item_count should be u64"),
        body["project_symbol_count"].as_u64().expect("project_symbol_count should be u64")
            + body["builtin_symbol_count"].as_u64().expect("builtin_symbol_count should be u64")
    );
    assert!(body["symbol_kind_counts"].is_object());
    let kind_total: u64 = body["symbol_kind_counts"]
        .as_object()
        .expect("symbol_kind_counts should be an object")
        .values()
        .map(|value| value.as_u64().expect("kind count should be u64"))
        .sum();
    assert_eq!(kind_total, body["item_count"].as_u64().expect("item_count should be u64"));
    assert!(body["summary"].is_object());
    assert_eq!(body["summary"]["schema_version"], "docgen-summary/v1");
    assert_eq!(
        body["summary"]["item_count"].as_u64().expect("summary item_count should be u64"),
        body["item_count"].as_u64().expect("item_count should be u64")
    );
    assert_eq!(
        body["summary"]["project_symbol_count"]
            .as_u64()
            .expect("summary project_symbol_count should be u64"),
        body["project_symbol_count"].as_u64().expect("project_symbol_count should be u64")
    );
    assert_eq!(
        body["summary"]["builtin_symbol_count"]
            .as_u64()
            .expect("summary builtin_symbol_count should be u64"),
        body["builtin_symbol_count"].as_u64().expect("builtin_symbol_count should be u64")
    );
    assert!(body["discovery_skip_counts"].is_object());
    assert!(body["discovery_skip_counts"]["max_file_size"].is_number());
    assert!(body["discovery_skip_counts"]["max_depth"].is_number());
    assert!(body["discovery_skip_counts"]["max_files"].is_number());
    assert!(body["discovery_skip_counts"]["invalid_encoding"].is_number());
    assert_eq!(body["discovery_limits"]["max_file_size_bytes"], 2 * 1024 * 1024);
    assert_eq!(body["discovery_limits"]["max_depth"], 64);
    assert_eq!(body["discovery_limits"]["max_files"], 20_000);
    assert!(body["link_validation_skip_counts"].is_object());
    assert!(body["link_validation_skip_counts"]["max_link_checks"].is_number());
    assert!(body["link_validation_skip_counts"]["max_external_checks"].is_number());
    assert!(body["link_validation_skip_counts"]["max_total_time"].is_number());
    assert!(body["summary"]["link_validation_skip_counts"].is_object());
    assert!(body["summary"]["link_validation_skip_counts"]["max_link_checks"].is_number());
    assert!(body["summary"]["link_validation_skip_counts"]["max_external_checks"].is_number());
    assert!(body["summary"]["link_validation_skip_counts"]["max_total_time"].is_number());
    assert_eq!(body["summary"]["discovery_limits"]["max_file_size_bytes"], 2 * 1024 * 1024);
    assert_eq!(body["summary"]["discovery_limits"]["max_depth"], 64);
    assert_eq!(body["summary"]["discovery_limits"]["max_files"], 20_000);
    assert!(body["adapter_health"].is_object());
    assert!(body["adapter_health"]["kujo"].is_object());
    assert!(body["adapter_health"]["kujo"]["files_scanned"].is_number());
    assert!(body["adapter_health"]["kujo"]["symbols_extracted"].is_number());
    assert!(body["adapter_health"]["kujo"]["doc_blocks_attached"].is_number());
    assert!(body["adapter_health"]["kujo"]["placeholders_emitted"].is_number());
    assert!(body["summary"]["adapter_health"].is_object());
    assert!(body["summary"]["adapter_health"]["kujo"].is_object());
    assert!(body["summary"]["adapter_health"]["kujo"]["files_scanned"].is_number());
    assert!(body["summary"]["adapter_health"]["kujo"]["symbols_extracted"].is_number());
    assert!(body["summary"]["adapter_health"]["kujo"]["doc_blocks_attached"].is_number());
    assert!(body["summary"]["adapter_health"]["kujo"]["placeholders_emitted"].is_number());
    assert_eq!(body["cache_stats"]["hits"], 0);
    assert_eq!(body["cache_stats"]["misses"], 0);
    assert_eq!(body["summary"]["cache_stats"]["hits"], 0);
    assert_eq!(body["summary"]["cache_stats"]["misses"], 0);
}

#[test]
fn docgen_json_contract_snapshot_is_stable() {
    let dir = unique_temp_dir("docgen_json_contract_snapshot");
    let file = dir.join("docgen_snapshot_input.kujo");
    let out_dir = dir.join("docs_out");
    write_fixture(&file, "/// Adds one\npub func add_one(value) {\n    return value + 1\n}\n");

    let output = run_kujo(&[
        "docgen",
        file.to_str().expect("path should be utf-8"),
        "--out-dir",
        out_dir.to_str().expect("path should be utf-8"),
        "--language",
        "kujo",
        "--no-builtins",
        "--json",
    ]);

    assert!(output.status.success(), "docgen --json should succeed");
    let actual = normalize_docgen_contract_paths(parse_stdout_json(&output));
    let expected: Value = serde_json::from_str(&read_fixture(&docgen_fixture_path(
        "docgen_json_contract_snapshot.expected.json",
    )))
    .expect("snapshot fixture should be valid json");

    assert_eq!(actual, expected, "docgen json contract snapshot drift");
}

#[test]
fn docgen_json_discovery_limits_support_cli_and_env_overrides() {
    let dir = unique_temp_dir("docgen_json_discovery_limits");
    let file = dir.join("docgen_limits_input.kujo");
    let out_dir = dir.join("docs_out");
    write_fixture(&file, "pub func api() { return 1 }\n");

    let file_str = file.to_str().expect("path should be utf-8");
    let out_dir_str = out_dir.to_str().expect("path should be utf-8");

    let env_overridden = run_kujo_with_env(
        &["docgen", file_str, "--out-dir", out_dir_str, "--json"],
        &[
            ("KUJO_DOCGEN_MAX_FILE_SIZE_BYTES", "8192"),
            ("KUJO_DOCGEN_MAX_FILES", "1234"),
            ("KUJO_DOCGEN_MAX_DEPTH", "7"),
        ],
    );
    assert!(env_overridden.status.success(), "docgen with env discovery overrides should succeed");
    let env_body = parse_stdout_json(&env_overridden);
    assert_eq!(env_body["discovery_limits"]["max_file_size_bytes"], 8192);
    assert_eq!(env_body["discovery_limits"]["max_files"], 1234);
    assert_eq!(env_body["discovery_limits"]["max_depth"], 7);
    assert_eq!(env_body["summary"]["discovery_limits"]["max_file_size_bytes"], 8192);
    assert_eq!(env_body["summary"]["discovery_limits"]["max_files"], 1234);
    assert_eq!(env_body["summary"]["discovery_limits"]["max_depth"], 7);

    let cli_overridden = run_kujo_with_env(
        &[
            "docgen",
            file_str,
            "--out-dir",
            out_dir_str,
            "--max-discovery-file-size-bytes",
            "4096",
            "--max-discovery-files",
            "222",
            "--max-discovery-depth",
            "5",
            "--json",
        ],
        &[
            ("KUJO_DOCGEN_MAX_FILE_SIZE_BYTES", "8192"),
            ("KUJO_DOCGEN_MAX_FILES", "1234"),
            ("KUJO_DOCGEN_MAX_DEPTH", "7"),
        ],
    );
    assert!(cli_overridden.status.success(), "docgen with cli discovery overrides should succeed");
    let cli_body = parse_stdout_json(&cli_overridden);
    assert_eq!(cli_body["discovery_limits"]["max_file_size_bytes"], 4096);
    assert_eq!(cli_body["discovery_limits"]["max_files"], 222);
    assert_eq!(cli_body["discovery_limits"]["max_depth"], 5);
    assert_eq!(cli_body["summary"]["discovery_limits"]["max_file_size_bytes"], 4096);
    assert_eq!(cli_body["summary"]["discovery_limits"]["max_files"], 222);
    assert_eq!(cli_body["summary"]["discovery_limits"]["max_depth"], 5);
}

#[test]
fn docgen_json_discovery_limits_fail_on_invalid_env_values() {
    let dir = unique_temp_dir("docgen_json_discovery_limits_invalid_env");
    let file = dir.join("docgen_invalid_env.kujo");
    let out_dir = dir.join("docs_out");
    write_fixture(&file, "pub func api() { return 1 }\n");

    let output = run_kujo_with_env(
        &[
            "docgen",
            file.to_str().expect("path should be utf-8"),
            "--out-dir",
            out_dir.to_str().expect("path should be utf-8"),
            "--json",
        ],
        &[("KUJO_DOCGEN_MAX_DEPTH", "not-a-number")],
    );
    assert!(!output.status.success(), "docgen should fail for invalid env limit values");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains(
            "KUJO_DOCGEN_MAX_DEPTH environment value 'not-a-number' is not a valid integer"
        ),
        "stderr did not include invalid env limit diagnostic: {stderr}"
    );
}

#[test]
fn docgen_json_discovery_limits_fail_on_zero_cli_values() {
    let dir = unique_temp_dir("docgen_json_discovery_limits_zero_cli");
    let file = dir.join("docgen_zero_cli.kujo");
    let out_dir = dir.join("docs_out");
    write_fixture(&file, "pub func api() { return 1 }\n");

    let output = run_kujo(&[
        "docgen",
        file.to_str().expect("path should be utf-8"),
        "--out-dir",
        out_dir.to_str().expect("path should be utf-8"),
        "--max-discovery-files",
        "0",
        "--json",
    ]);
    assert!(!output.status.success(), "docgen should fail for zero CLI discovery limits");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains("max discovery files must be greater than 0"),
        "stderr did not include zero-limit diagnostic: {stderr}"
    );
}

#[test]
fn docgen_json_cache_mode_reports_hit_miss_counters() {
    let dir = unique_temp_dir("docgen_json_cache_hits_misses");
    let file = dir.join("docgen_cache_input.kujo");
    let out_dir_first = dir.join("docs_out_first");
    let out_dir_second = dir.join("docs_out_second");
    let cache_dir = dir.join(".docgen-cache");
    write_fixture(&file, "pub func api() { return 1 }\n");

    let file_str = file.to_str().expect("path should be utf-8");
    let cache_dir_str = cache_dir.to_str().expect("path should be utf-8");

    let first = run_kujo(&[
        "docgen",
        file_str,
        "--out-dir",
        out_dir_first.to_str().expect("path should be utf-8"),
        "--cache-dir",
        cache_dir_str,
        "--json",
    ]);
    assert!(first.status.success(), "first cached docgen run should succeed");
    let first_body = parse_stdout_json(&first);
    assert_eq!(first_body["cache_stats"]["hits"], 0);
    assert_eq!(first_body["cache_stats"]["misses"], 1);

    let second = run_kujo(&[
        "docgen",
        file_str,
        "--out-dir",
        out_dir_second.to_str().expect("path should be utf-8"),
        "--cache-dir",
        cache_dir_str,
        "--json",
    ]);
    assert!(second.status.success(), "second cached docgen run should succeed");
    let second_body = parse_stdout_json(&second);
    assert_eq!(second_body["cache_stats"]["hits"], 1);
    assert_eq!(second_body["cache_stats"]["misses"], 0);
    assert_eq!(second_body["summary"]["cache_stats"]["hits"], 1);
    assert_eq!(second_body["summary"]["cache_stats"]["misses"], 0);
}

#[test]
fn docgen_json_link_mode_failure_breakdown_is_stable() {
    let dir = unique_temp_dir("docgen_json_link_mode_breakdown");
    let file = dir.join("docgen_links_input.kujo");
    let out_dir = dir.join("docs_out");
    write_fixture(
        &file,
        "/// Missing local doc: [Missing](missing.md)\npub func linked_api() { return 1 }\n",
    );

    let output = run_kujo(&[
        "docgen",
        file.to_str().expect("path should be utf-8"),
        "--out-dir",
        out_dir.to_str().expect("path should be utf-8"),
        "--public-only",
        "--fail-on-broken-links",
        "--json",
    ]);

    assert!(output.status.success(), "docgen --json should emit payload even with gate failures");
    let body = parse_stdout_json(&output);
    let failures = body["gate_failures"].as_array().expect("gate_failures should be an array");
    assert!(
        failures.iter().any(|entry| {
            entry.as_str().is_some_and(|message| {
                message.starts_with("1 broken links detected")
                    && message.contains("local_file=1")
                    && message.contains("local_anchor=0")
                    && message.contains("external=0")
            })
        }),
        "broken link gate failures should include per-mode counts"
    );
}

#[test]
fn docgen_json_warns_for_external_mode_without_allowlist() {
    let dir = unique_temp_dir("docgen_json_external_allowlist_warning");
    let file = dir.join("docgen_external_input.kujo");
    let out_dir = dir.join("docs_out");
    write_fixture(
        &file,
        "/// External link: [Docs](https://example.com/reference)\npub func linked_api() { return 1 }\n",
    );

    let output = run_kujo(&[
        "docgen",
        file.to_str().expect("path should be utf-8"),
        "--out-dir",
        out_dir.to_str().expect("path should be utf-8"),
        "--public-only",
        "--validate-external-links",
        "--fail-on-warnings",
        "--json",
    ]);

    assert!(output.status.success(), "docgen --json should succeed and report warnings in payload");
    let body = parse_stdout_json(&output);
    assert_eq!(body["warning_count"], 1);
    let failures = body["gate_failures"].as_array().expect("gate_failures should be an array");
    assert!(failures
        .iter()
        .any(|entry| entry.as_str().is_some_and(|message| message == "1 warnings detected")));
}

#[test]
fn lsp_cli_json_contracts_are_stable() {
    let dir = unique_temp_dir("lsp_json_contract");
    let file = dir.join("lsp_input.kujo");
    write_fixture(&file, "func greet(name) {\n    return name\n}\nlet value := greet(\"kujo\")\n");

    let file_str = file.to_str().expect("path should be utf-8");

    let complete = run_kujo(&["lsp-complete", file_str, "--line", "4", "--column", "18", "--json"]);
    assert!(complete.status.success());
    let complete_body = parse_stdout_json(&complete);
    assert!(complete_body.is_array());

    let definition =
        run_kujo(&["lsp-definition", file_str, "--line", "4", "--column", "14", "--json"]);
    assert!(definition.status.success());
    let definition_body = parse_stdout_json(&definition);
    assert!(definition_body.is_object() || definition_body.is_null());

    let references =
        run_kujo(&["lsp-references", file_str, "--line", "4", "--column", "14", "--json"]);
    assert!(references.status.success());
    let references_body = parse_stdout_json(&references);
    assert!(references_body.is_array());

    let hover = run_kujo(&["lsp-hover", file_str, "--line", "4", "--column", "14", "--json"]);
    assert!(hover.status.success());
    let hover_body = parse_stdout_json(&hover);
    assert!(hover_body.is_object() || hover_body.is_null());

    let diagnostics_input = dir.join("lsp_bad_input.kujo");
    write_fixture(&diagnostics_input, "print((1 + 2)\n");
    let diagnostics = run_kujo(&[
        "lsp-diagnostics",
        diagnostics_input.to_str().expect("path should be utf-8"),
        "--json",
    ]);
    assert!(diagnostics.status.success());
    let diagnostics_body = parse_stdout_json(&diagnostics);
    assert!(diagnostics_body.is_array());
    let first_diagnostic = diagnostics_body
        .as_array()
        .and_then(|items| items.first())
        .expect("lsp-diagnostics should emit at least one diagnostic item");
    assert!(first_diagnostic["line"].is_number());
    assert!(first_diagnostic["column"].is_number());
    assert!(first_diagnostic["severity"].is_string());
    assert!(first_diagnostic["message"].is_string());
    assert!(first_diagnostic["code"].is_string());
    assert!(first_diagnostic["subsystem"].is_string());

    let rename = run_kujo(&[
        "lsp-rename",
        file_str,
        "--line",
        "4",
        "--column",
        "14",
        "--new-name",
        "welcome",
        "--json",
    ]);
    assert!(rename.status.success());
    let rename_body = parse_stdout_json(&rename);
    assert!(rename_body.get("edit_count").is_some());
    assert!(rename_body.get("edits").is_some());
    assert!(rename_body.get("updated_source").is_some());

    let code_actions = run_kujo(&[
        "lsp-code-actions",
        diagnostics_input.to_str().expect("path should be utf-8"),
        "--json",
    ]);
    assert!(code_actions.status.success());
    let code_actions_body = parse_stdout_json(&code_actions);
    assert!(code_actions_body.is_array());
}

#[test]
fn cli_json_negative_paths_have_stable_failure_signals() {
    let dir = unique_temp_dir("cli_json_negative_paths");
    let file = dir.join("negative_input.kujo");
    write_fixture(&file, "func greet(name) {\n    return name\n}\nlet value := greet(\"kujo\")\n");

    let missing_file = dir.join("missing.kujo");
    let missing_file_str = missing_file.to_str().expect("path should be utf-8");

    let format_missing = run_kujo(&["format", missing_file_str, "--json"]);
    assert_eq!(format_missing.status.code(), Some(5));
    assert!(format_missing.stdout.is_empty(), "format failure should not emit JSON stdout");
    let format_stderr =
        String::from_utf8(format_missing.stderr).expect("stderr should be valid utf-8");
    assert!(format_stderr.contains("Failed to read .kujo file"));

    let lint_missing = run_kujo(&["lint", missing_file_str, "--json"]);
    assert_eq!(lint_missing.status.code(), Some(5));
    assert!(lint_missing.stdout.is_empty(), "lint failure should not emit JSON stdout");
    let lint_stderr = String::from_utf8(lint_missing.stderr).expect("stderr should be valid utf-8");
    assert!(lint_stderr.contains("Failed to read .kujo file"));

    let malformed_line = run_kujo(&[
        "lsp-definition",
        file.to_str().expect("path should be utf-8"),
        "--line",
        "nope",
        "--column",
        "1",
        "--json",
    ]);
    assert_eq!(malformed_line.status.code(), Some(2));
    assert!(malformed_line.stdout.is_empty());
    let malformed_stderr =
        String::from_utf8(malformed_line.stderr).expect("stderr should be valid utf-8");
    assert!(malformed_stderr.contains("invalid value 'nope'"));
}

#[test]
fn lsp_rename_json_failure_contract_is_stable() {
    let dir = unique_temp_dir("lsp_rename_json_failure_contract");
    let file = dir.join("rename_failure.kujo");
    write_fixture(&file, "func greet(name) {\n    return name\n}\nlet value := greet(\"kujo\")\n");

    let unknown_symbol = run_kujo(&[
        "lsp-rename",
        file.to_str().expect("path should be utf-8"),
        "--line",
        "4",
        "--column",
        "1",
        "--new-name",
        "renamed",
        "--json",
    ]);

    assert_eq!(unknown_symbol.status.code(), Some(4));
    assert!(
        unknown_symbol.stderr.is_empty(),
        "lsp-rename --json failure should emit JSON payload on stdout only"
    );

    let body = parse_stdout_json(&unknown_symbol);
    assert_eq!(body["command"], "lsp-rename");
    assert_eq!(body["status"], "error");
    assert_eq!(body["kind"], "runtime_error");
    assert_eq!(body["contract_version"], "1.0.0-draft");
    assert_eq!(body["exit_code"], 4);
    let message = body["message"].as_str().expect("message should be a string");
    assert!(message.contains("No identifier found at cursor location"));
}

#[test]
fn run_runtime_json_diagnostic_contract_is_stable() {
    let dir = unique_temp_dir("run_runtime_json_diagnostic");
    let file = dir.join("runtime_error.kujo");
    write_fixture(&file, "denom := 0\nprint(1 / denom)\n");

    let output = run_kujo(&[
        "run",
        file.to_str().expect("path should be utf-8"),
        "--json-runtime-diagnostics",
    ]);

    assert_eq!(output.status.code(), Some(4));
    assert!(
        output.stderr.is_empty(),
        "run --json-runtime-diagnostics should emit failure payload on stdout only"
    );

    let body = parse_stdout_json(&output);
    assert_eq!(body["command"], "run");
    assert_eq!(body["status"], "error");
    assert_eq!(body["kind"], "runtime_diagnostic");
    assert_eq!(body["contract_version"], "1.0.0-draft");
    assert_eq!(body["exit_code"], 4);
    assert!(body["call_stack"].is_array());

    let diagnostic = &body["diagnostic"];
    assert_eq!(diagnostic["code"], "KUJOVM001");
    assert_eq!(diagnostic["subsystem"], "vm");
    assert_eq!(diagnostic["severity"], "error");
    assert!(diagnostic["message"].as_str().is_some());
}

#[test]
fn run_runtime_json_diagnostic_contract_includes_missing_module_help() {
    let dir = unique_temp_dir("run_runtime_json_missing_module");
    let file = dir.join("missing_module_entry.kujo");
    write_fixture(&file, "import missing_module\n");

    let output = run_kujo(&[
        "run",
        file.to_str().expect("path should be utf-8"),
        "--json-runtime-diagnostics",
    ]);

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stderr.is_empty(), "runtime diagnostics should emit stdout-only JSON");

    let body = parse_stdout_json(&output);
    let diagnostic = &body["diagnostic"];
    let message = diagnostic["message"].as_str().expect("message should be a string");
    let help = diagnostic["help"].as_str().expect("help should be a string");
    assert!(message.contains("Module not found: missing_module"));
    assert!(
        message.contains("flat <module>.kujo file") && message.contains("src/..."),
        "expected module resolution help text, got: {message}"
    );
    assert!(help.contains("<module>.kujo") && help.contains("src/..."));
}

#[test]
fn run_runtime_json_diagnostic_contract_reports_capability_hint() {
    let dir = unique_temp_dir("run_runtime_json_capability_hint");
    let file = dir.join("capability_denied.kujo");
    write_fixture(&file, "write_file(\"blocked.txt\", \"data\")\n");

    let output = run_kujo(&[
        "run",
        "--untrusted",
        file.to_str().expect("path should be utf-8"),
        "--json-runtime-diagnostics",
    ]);

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stderr.is_empty(), "runtime diagnostics should emit stdout-only JSON");

    let body = parse_stdout_json(&output);
    let diagnostic = &body["diagnostic"];
    let message = diagnostic["message"].as_str().expect("message should be a string");
    let help = diagnostic["help"].as_str().expect("help should be a string");
    assert!(message.contains("Capability denied: filesystem-write required for write_file"));
    assert!(
        message.contains("--allow-fs-write"),
        "expected capability hint in runtime diagnostic message, got: {message}"
    );
    assert!(help.contains("--allow-*"));
}

#[test]
fn run_runtime_json_diagnostic_contract_reports_non_callable_call_hint() {
    let dir = unique_temp_dir("run_runtime_json_non_callable");
    let file = dir.join("non_callable.kujo");
    write_fixture(&file, "value := 1\nvalue()\n");

    let output = run_kujo(&[
        "run",
        file.to_str().expect("path should be utf-8"),
        "--json-runtime-diagnostics",
    ]);

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stderr.is_empty(), "runtime diagnostics should emit stdout-only JSON");

    let body = parse_stdout_json(&output);
    let diagnostic = &body["diagnostic"];
    let message = diagnostic["message"].as_str().expect("message should be a string");
    let help = diagnostic["help"].as_str().expect("help should be a string");
    assert!(message.contains("Cannot call non-function"));
    assert!(
        message.contains("callable value"),
        "expected callable remediation hint, got: {message}"
    );
    assert!(help.contains("functions") && help.contains("callable values"));
}

#[test]
fn run_runtime_json_diagnostic_contract_reports_invalid_binary_help() {
    let dir = unique_temp_dir("run_runtime_json_invalid_binary");
    let file = dir.join("invalid_binary.kujo");
    write_fixture(&file, "print(true + 1)\n");

    let output = run_kujo(&[
        "run",
        file.to_str().expect("path should be utf-8"),
        "--json-runtime-diagnostics",
    ]);

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stderr.is_empty(), "runtime diagnostics should emit stdout-only JSON");

    let body = parse_stdout_json(&output);
    let diagnostic = &body["diagnostic"];
    let message = diagnostic["message"].as_str().expect("message should be a string");
    let help = diagnostic["help"].as_str().expect("help should be a string");
    assert!(message.contains("Invalid binary operation"));
    assert!(help.contains("operands") && help.contains("convert values"));
}
