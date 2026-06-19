use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const EXIT_USAGE_ERROR: i32 = 2;
const EXIT_LEX_PARSE_ERROR: i32 = 3;
const EXIT_VERIFICATION_ERROR: i32 = 3;
const EXIT_RUNTIME_ERROR: i32 = 4;
const EXIT_IO_ERROR: i32 = 5;

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_{}_{}", prefix, nanos));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn kujo_binary() -> String {
    env!("CARGO_BIN_EXE_kujo").to_string()
}

fn run_kujo(args: &[&str]) -> std::process::Output {
    Command::new(kujo_binary()).args(args).output().expect("failed to execute kujo binary")
}

fn run_kujo_in_dir(args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new(kujo_binary())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to execute kujo binary")
}

fn write_fixture(path: &Path, content: &str) {
    fs::write(path, content).expect("failed to write fixture file");
}

#[test]
fn cli_help_exits_zero() {
    let output = run_kujo(&["--help"]);
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Kujo: A modern programming language"));
}

#[test]
fn cli_version_exits_zero_and_prints_crate_version() {
    let output = run_kujo(&["--version"]);
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn cli_run_missing_file_exits_with_io_error_code() {
    let dir = unique_temp_dir("cli_run_missing_file");
    let missing = dir.join("missing.kujo");

    let output = run_kujo(&["run", missing.to_str().expect("path should be utf-8")]);
    assert_eq!(output.status.code(), Some(EXIT_IO_ERROR));
    assert!(output.stdout.is_empty(), "run missing-file failure should not write stdout");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("Failed to read .kujo file"));
}

#[test]
fn cli_run_parse_error_exits_with_lex_parse_code() {
    let dir = unique_temp_dir("cli_run_parse_exit_code");
    let file = dir.join("broken.kujo");
    write_fixture(&file, "print((1 + 2)\n");

    let output = run_kujo(&["run", file.to_str().expect("path should be utf-8")]);
    assert_eq!(output.status.code(), Some(EXIT_LEX_PARSE_ERROR));
    assert!(output.stdout.is_empty(), "parse failure should not write stdout");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("[RUFPARSE001]"));
}

#[test]
fn cli_run_runtime_error_exits_with_runtime_code() {
    let dir = unique_temp_dir("cli_run_runtime_exit_code");
    let file = dir.join("runtime_error.kujo");
    write_fixture(&file, "let denom := 0\nprint(1 / denom)\n");

    let output = run_kujo(&["run", file.to_str().expect("path should be utf-8")]);
    assert_eq!(output.status.code(), Some(EXIT_RUNTIME_ERROR));
    assert!(output.stdout.is_empty(), "runtime failure should not write stdout");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("Division by zero") || stderr.contains("divide by zero"));
}

#[test]
fn cli_run_runtime_error_json_mode_emits_stdout_payload() {
    let dir = unique_temp_dir("cli_run_runtime_json_error");
    let file = dir.join("runtime_error_json.kujo");
    write_fixture(&file, "denom := 0\nprint(1 / denom)\n");

    let output = run_kujo(&[
        "run",
        file.to_str().expect("path should be utf-8"),
        "--json-runtime-diagnostics",
    ]);
    assert_eq!(output.status.code(), Some(EXIT_RUNTIME_ERROR));
    assert!(output.stderr.is_empty(), "json runtime diagnostics should suppress stderr");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let parsed: Value =
        serde_json::from_str(&stdout).expect("run --json-runtime-diagnostics should emit JSON");
    assert_eq!(parsed["command"], "run");
    assert_eq!(parsed["status"], "error");
    assert_eq!(parsed["kind"], "runtime_diagnostic");
    assert_eq!(parsed["exit_code"], EXIT_RUNTIME_ERROR);
    assert!(parsed["diagnostic"].is_object());
}

#[test]
fn cli_usage_errors_use_usage_exit_code() {
    let output = run_kujo(&["run"]);
    assert_eq!(output.status.code(), Some(EXIT_USAGE_ERROR));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("Usage:"));
}

#[test]
fn cli_lsp_diagnostics_json_is_valid_json() {
    let dir = unique_temp_dir("cli_lsp_diagnostics_json");
    let file = dir.join("broken.kujo");
    write_fixture(&file, "print((1 + 2)\n");

    let output =
        run_kujo(&["lsp-diagnostics", file.to_str().expect("path should be utf-8"), "--json"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let parsed: Value = serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_array());
    assert!(output.stderr.is_empty(), "successful JSON diagnostics should not write stderr");
}

#[test]
fn cli_lsp_complete_plain_and_json_modes_are_stable() {
    let dir = unique_temp_dir("cli_lsp_complete_modes");
    let file = dir.join("complete.kujo");
    write_fixture(&file, "let alpha := 1\nprint(al)\n");

    let plain = run_kujo(&[
        "lsp-complete",
        file.to_str().expect("path should be utf-8"),
        "--line",
        "2",
        "--column",
        "8",
    ]);
    assert_eq!(plain.status.code(), Some(0));
    assert!(plain.stderr.is_empty(), "plain completion should not write stderr");
    let plain_stdout = String::from_utf8(plain.stdout).expect("stdout should be utf-8");
    assert_eq!(
        plain_stdout,
        [
            "abs\tbuiltin",
            "aes_decrypt\tbuiltin",
            "aes_decrypt_bytes\tbuiltin",
            "aes_encrypt\tbuiltin",
            "aes_encrypt_bytes\tbuiltin",
            "ai_chat\tbuiltin",
            "ai_embedding\tbuiltin",
            "ai_stream_chat\tbuiltin",
            "ai_tool_loop\tbuiltin",
            "all\tbuiltin",
            "alpha\tvariable",
            "any\tbuiltin",
            "append\tbuiltin",
            "append_file\tbuiltin",
            "arg_parser\tbuiltin",
            "args\tbuiltin",
            "array\tbuiltin",
            "assert\tbuiltin",
            "assert_contains\tbuiltin",
            "assert_equal\tbuiltin",
            "assert_false\tbuiltin",
            "assert_true\tbuiltin",
            "async_http_get\tbuiltin",
            "async_http_post\tbuiltin",
            "async_read_file\tbuiltin",
            "async_read_files\tbuiltin",
            "async_sleep\tbuiltin",
            "async_timeout\tbuiltin",
            "async_write_file\tbuiltin",
            "async_write_files\tbuiltin",
            "await_all\tbuiltin",
            "await_task\tbuiltin",
            "",
        ]
        .join("\n"),
        "plain completion rows should remain byte-compatible"
    );

    let json = run_kujo(&[
        "lsp-complete",
        file.to_str().expect("path should be utf-8"),
        "--line",
        "2",
        "--column",
        "8",
        "--json",
    ]);
    assert_eq!(json.status.code(), Some(0));
    assert!(json.stderr.is_empty(), "json completion should not write stderr");
    let json_stdout = String::from_utf8(json.stdout).expect("stdout should be utf-8");
    let parsed: Value =
        serde_json::from_str(&json_stdout).expect("json completion should be valid");
    assert!(parsed.is_array(), "json completion should be an array");
}

#[test]
fn cli_lsp_rename_invalid_identifier_uses_runtime_error_stderr() {
    let dir = unique_temp_dir("cli_lsp_rename_invalid_identifier");
    let file = dir.join("rename.kujo");
    write_fixture(&file, "let value := 1\nprint(value)\n");

    let output = run_kujo(&[
        "lsp-rename",
        file.to_str().expect("path should be utf-8"),
        "--line",
        "1",
        "--column",
        "5",
        "--new-name",
        "123bad",
    ]);
    assert_eq!(output.status.code(), Some(EXIT_RUNTIME_ERROR));
    assert!(output.stdout.is_empty(), "rename validation failure should not write stdout");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains("start with a letter or underscore"),
        "rename validation message should be surfaced on stderr"
    );
}

#[test]
fn cli_lsp_rename_keyword_identifier_uses_runtime_error_stderr() {
    let dir = unique_temp_dir("cli_lsp_rename_keyword_identifier");
    let file = dir.join("rename_keyword.kujo");
    write_fixture(&file, "let value := 1\nprint(value)\n");

    let output = run_kujo(&[
        "lsp-rename",
        file.to_str().expect("path should be utf-8"),
        "--line",
        "1",
        "--column",
        "5",
        "--new-name",
        "if",
    ]);
    assert_eq!(output.status.code(), Some(EXIT_RUNTIME_ERROR));
    assert!(output.stdout.is_empty(), "rename validation failure should not write stdout");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains("must not be a reserved keyword"),
        "keyword rename validation message should be surfaced on stderr"
    );
}

#[test]
fn cli_lsp_diagnostics_missing_file_uses_io_error_and_stderr() {
    let dir = unique_temp_dir("cli_lsp_missing_file");
    let missing = dir.join("missing.kujo");
    let output =
        run_kujo(&["lsp-diagnostics", missing.to_str().expect("path should be utf-8"), "--json"]);

    assert_eq!(output.status.code(), Some(EXIT_IO_ERROR));
    assert!(output.stdout.is_empty(), "missing-file diagnostics failure should not write stdout");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("Failed to read .kujo file"));
}

#[test]
fn cli_check_does_not_execute_script_side_effects() {
    let dir = unique_temp_dir("cli_check_no_side_effects");
    let file = dir.join("check_only.kujo");
    let marker = dir.join("marker.txt");
    let source = format!("write_file(\"{}\", \"created\", true)\n", marker.to_string_lossy());
    write_fixture(&file, &source);

    let output = run_kujo(&["check", file.to_str().expect("path should be utf-8")]);

    assert_eq!(output.status.code(), Some(0));
    assert!(!marker.exists(), "check command must not execute runtime side effects");
}

#[test]
fn cli_run_executes_program_output() {
    let dir = unique_temp_dir("cli_run_executes_output");
    let file = dir.join("prints.kujo");
    write_fixture(&file, "print(\"run-ok\")\n");

    let output = run_kujo(&["run", file.to_str().expect("path should be utf-8")]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "successful run should not emit stderr");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("run-ok"), "run should execute and print script output");
}

#[test]
fn cli_test_discovers_and_runs_expected_fixtures() {
    let workspace = unique_temp_dir("cli_test_discovers_fixtures");
    let tests_dir = workspace.join("tests");
    fs::create_dir_all(&tests_dir).expect("failed to create tests directory");

    let fixture = tests_dir.join("sample.kujo");
    let snapshot = tests_dir.join("sample.out");
    write_fixture(&fixture, "print(\"fixture-ok\")\n");
    write_fixture(&snapshot, "fixture-ok\n");
    write_fixture(
        &tests_dir.join("framework.kujo"),
        "# Run with: kujo test-run tests/framework.kujo\nprint(\"not-discovered-here\")\n",
    );

    let output = run_kujo_in_dir(&["test"], &workspace);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "test command should report results on stdout");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Passed 1/1 tests"), "test should discover and run fixture files");
    assert!(stdout.contains("[✓]"), "test should report passing fixture");
    assert!(
        stdout.contains("Fixture outcomes: passed=1, failed=0, skipped=1, expected_fail=0"),
        "test summary should distinguish passed, failed, skipped, and expected-fail counts"
    );
    assert!(
        stdout.contains("Runtime strategy: dual"),
        "test command without explicit --runtime should default to dual strategy"
    );
    assert!(
        stdout.contains("interpreter_fallback=0"),
        "dual default should report zero fallback usage for VM-clean fixtures"
    );
}

#[test]
fn cli_test_runtime_vm_mode_executes_vm_drift_fixture_without_snapshot_mismatch() {
    let workspace = unique_temp_dir("cli_test_runtime_vm_mode");
    let tests_dir = workspace.join("tests");
    fs::create_dir_all(&tests_dir).expect("failed to create tests directory");

    let fixture = tests_dir.join("sample.kujo");
    let snapshot = tests_dir.join("sample.out");
    write_fixture(
        &fixture,
        "print(\"start\")\nresult := assert_equal(5, 5)\nprint(\"after first\")\ntry {\n    result := assert_equal(5, 10)\n    print(\"unexpected\")\n} except error {\n    print(\"caught\")\n}\nprint(\"end\")\n",
    );
    write_fixture(&snapshot, "start\nafter first\ncaught\nend\n");

    let output = run_kujo_in_dir(&["test", "--runtime", "vm"], &workspace);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "test command should report results on stdout");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Passed 1/1 tests"), "vm mode should pass snapshot for this fixture");
    assert!(stdout.contains("[✓]"), "vm mode should mark fixture as passed");
    assert!(
        stdout.contains("Runtime strategy: vm"),
        "vm mode should print runtime strategy summary"
    );
    assert!(
        !stdout.contains("[dual fallback: interpreter]"),
        "vm mode should not emit dual fallback marker"
    );
}

#[test]
fn cli_test_runtime_dual_mode_keeps_vm_primary_for_vm_drift_fixture() {
    let workspace = unique_temp_dir("cli_test_runtime_dual_mode");
    let tests_dir = workspace.join("tests");
    fs::create_dir_all(&tests_dir).expect("failed to create tests directory");

    let fixture = tests_dir.join("sample.kujo");
    let snapshot = tests_dir.join("sample.out");
    write_fixture(
        &fixture,
        "print(\"start\")\nresult := assert_equal(5, 5)\nprint(\"after first\")\ntry {\n    result := assert_equal(5, 10)\n    print(\"unexpected\")\n} except error {\n    print(\"caught\")\n}\nprint(\"end\")\n",
    );
    write_fixture(&snapshot, "start\nafter first\ncaught\nend\n");

    let output = run_kujo_in_dir(&["test", "--runtime", "dual"], &workspace);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "test command should report results on stdout");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("Passed 1/1 tests"),
        "dual mode should pass fixture on primary VM path"
    );
    assert!(stdout.contains("[✓]"), "dual mode should report passing fixture");
    assert!(
        stdout.contains("Runtime strategy: dual"),
        "dual mode should print runtime strategy summary"
    );
    assert!(
        stdout.contains("interpreter_fallback=0"),
        "dual mode should report zero fallback when VM output matches snapshot"
    );
    assert!(
        !stdout.contains("[dual fallback: interpreter]"),
        "dual mode should not emit fallback marker when interpreter fallback is unused"
    );
}

#[test]
fn cli_test_reports_fixture_failures_with_verification_exit_code() {
    for runtime in ["vm", "dual"] {
        let workspace = unique_temp_dir(&format!("cli_test_exit_code_{runtime}"));
        let tests_dir = workspace.join("tests");
        fs::create_dir_all(&tests_dir).expect("failed to create tests directory");

        let fixture = tests_dir.join("failing.kujo");
        let snapshot = tests_dir.join("failing.out");
        write_fixture(&fixture, "print(\"actual-output\")\n");
        write_fixture(&snapshot, "expected-output\n");

        let output = run_kujo_in_dir(&["test", "--runtime", runtime], &workspace);
        assert_eq!(
            output.status.code(),
            Some(EXIT_VERIFICATION_ERROR),
            "kujo test --runtime {runtime} should return verification failure exit code"
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert!(
            stdout.contains("[✗]"),
            "kujo test --runtime {runtime} should report failing fixtures"
        );
        assert!(
            stdout.contains("Fixture outcomes: passed=0, failed=1, skipped=0, expected_fail=0"),
            "kujo test --runtime {runtime} should summarize failed fixtures"
        );
        assert!(
            stdout.contains("Expected:"),
            "kujo test --runtime {runtime} should print expected output on mismatch"
        );
    }
}

#[test]
fn cli_test_reports_nonzero_fixture_exit_codes_as_failures() {
    let workspace = unique_temp_dir("cli_test_nonzero_fixture_exit");
    let tests_dir = workspace.join("tests");
    fs::create_dir_all(&tests_dir).expect("failed to create tests directory");

    let fixture = tests_dir.join("runtime_error.kujo");
    let snapshot = tests_dir.join("runtime_error.out");
    write_fixture(
        &fixture,
        "print(\"before runtime failure\")\n\
db := db_connect(\"sqlite\", \":memory:\")\n\
db_execute(db, \"CREATE TABLE test (id INTEGER)\", [])\n\
db_begin(db)\n\
db_begin(db)\n",
    );
    write_fixture(&snapshot, "before runtime failure\n");

    let output = run_kujo_in_dir(&["test", "--runtime", "vm"], &workspace);
    assert_eq!(
        output.status.code(),
        Some(EXIT_VERIFICATION_ERROR),
        "kujo test should surface nonzero fixture exits as verification failures"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("runtime_error.kujo"),
        "kujo test should report the failing fixture name"
    );
}

#[test]
fn cli_check_verbose_and_quiet_output_are_deterministic() {
    let dir = unique_temp_dir("cli_check_verbosity");
    let file = dir.join("valid.kujo");
    write_fixture(&file, "let value := args()\n");

    let quiet = run_kujo(&["check", file.to_str().expect("path should be utf-8"), "--quiet"]);
    assert_eq!(quiet.status.code(), Some(0));
    assert!(quiet.stdout.is_empty(), "check --quiet should suppress success output on stdout");
    assert!(quiet.stderr.is_empty(), "check --quiet success should not write stderr");

    let verbose = run_kujo(&["check", file.to_str().expect("path should be utf-8"), "--verbose"]);
    assert_eq!(verbose.status.code(), Some(0));
    assert!(verbose.stderr.is_empty(), "check --verbose success should not write stderr");
    let verbose_stdout = String::from_utf8(verbose.stdout).expect("stdout should be utf-8");
    assert!(
        verbose_stdout.contains("check passed"),
        "check --verbose should emit deterministic success summary"
    );
    assert!(
        verbose_stdout.contains("statements="),
        "check --verbose should include statement-count metadata"
    );
}

#[test]
fn cli_check_json_success_is_valid_json() {
    let dir = unique_temp_dir("cli_check_json");
    let file = dir.join("valid.kujo");
    write_fixture(&file, "print(\"json-check\")\n");

    let output = run_kujo(&["check", file.to_str().expect("path should be utf-8"), "--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "check --json success should not write stderr");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let parsed: Value = serde_json::from_str(&stdout).expect("check --json output should be JSON");
    assert_eq!(parsed["command"], "check");
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["file"], file.display().to_string());
    assert!(parsed["statement_count"].is_u64());
    assert!(parsed["bytecode_instruction_count"].is_u64());
}

#[test]
fn cli_pack_run_executes_pack_local_command_and_supports_json() {
    let workspace = unique_temp_dir("cli_pack_run_json");
    let pack_dir = workspace.join(".kujo").join("packs").join("acme-tools");
    let commands_dir = pack_dir.join("commands");
    fs::create_dir_all(&commands_dir).expect("failed to create pack command directory");

    write_fixture(
        &pack_dir.join("kujo-pack.yaml"),
        r#"id: acme-tools
namespace: acme
name: Acme Tools
version: 0.1.0
commands:
  - name: status
    summary: Status check.
    entry: commands/status.kujo
"#,
    );
    write_fixture(
        &commands_dir.join("status.kujo"),
        "print(\"{\\\"pack\\\":\\\"acme-tools\\\",\\\"namespace\\\":\\\"acme\\\",\\\"command\\\":\\\"status\\\",\\\"status\\\":\\\"pass\\\",\\\"summary\\\":{\\\"pass\\\":1,\\\"warn\\\":0,\\\"fail\\\":0,\\\"skip\\\":0,\\\"info\\\":0},\\\"checks\\\":[]}\")\n",
    );

    let output = run_kujo_in_dir(&["pack", "run", "acme", "status", "--json"], &workspace);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "pack run --json success should not write stderr");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let parsed: Value =
        serde_json::from_str(&stdout).expect("pack run --json output should be JSON");
    assert_eq!(parsed["namespace"], "acme");
    assert_eq!(parsed["command"], "status");
    assert_eq!(parsed["status"], "pass");
}

#[test]
fn cli_reserved_alias_name_is_rejected_before_workflow_routing() {
    let output = run_kujo(&["dev"]);
    assert_eq!(output.status.code(), Some(EXIT_USAGE_ERROR));
    assert!(output.stdout.is_empty(), "reserved alias rejection should not write stdout");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("reserved"), "expected reserved-name rejection, got: {}", stderr);
}

#[test]
fn cli_pack_run_rejects_reserved_namespace() {
    let output = run_kujo(&["pack", "run", "kujo", "status"]);
    assert_eq!(output.status.code(), Some(EXIT_USAGE_ERROR));
    assert!(output.stdout.is_empty(), "reserved namespace rejection should not write stdout");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("Namespace 'kujo' is reserved"));
}

#[test]
fn cli_doctor_json_contract_is_stable() {
    let workspace = unique_temp_dir("cli_doctor_json_contract");
    let output = run_kujo_in_dir(&["doctor", "--json"], &workspace);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "doctor --json should not write stderr on success");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let parsed: Value = serde_json::from_str(&stdout).expect("doctor --json must emit valid JSON");
    assert_eq!(parsed["tool"], "kujo-doctor");
    assert_eq!(parsed["command"], "doctor");
    assert_eq!(parsed["profile"], "generic");
    assert_eq!(parsed["schema_version"], "0.1.0");
    assert!(parsed["checks"].is_array());
    assert!(parsed["summary"].is_object());
}

#[test]
fn cli_doctor_lists_profiles() {
    let workspace = unique_temp_dir("cli_doctor_profiles");
    let output = run_kujo_in_dir(&["doctor", "--list-profiles"], &workspace);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "doctor --list-profiles should not write stderr");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("generic"), "expected default doctor profile listing");
}

#[test]
fn cli_doctor_rejects_unknown_profile() {
    let workspace = unique_temp_dir("cli_doctor_unknown_profile");
    let output = run_kujo_in_dir(&["doctor", "unknown-profile"], &workspace);
    assert_eq!(output.status.code(), Some(EXIT_USAGE_ERROR));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("Unknown doctor profile"));
}
