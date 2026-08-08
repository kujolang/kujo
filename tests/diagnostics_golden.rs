use kujo::errors::{
    Diagnostic, DiagnosticSeverity, DiagnosticSubsystem, KujoError, SourceLocation,
    DIAGNOSTIC_CODE_CLI, DIAGNOSTIC_CODE_PARSER, DIAGNOSTIC_CODE_RUNTIME,
};
use kujo::lexer::tokenize_with_file;
use kujo::parser::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/diagnostics")
}

fn normalize_snapshot_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn snapshot_path(base_name: &str, kind: &str) -> PathBuf {
    fixtures_dir().join(format!("{base_name}.{kind}.golden"))
}

fn should_update_goldens() -> bool {
    matches!(std::env::var("KUJO_UPDATE_GOLDENS").as_deref(), Ok("1"))
}

fn assert_or_update_golden(base_name: &str, kind: &str, actual: &str) {
    let actual_normalized = normalize_snapshot_text(actual);
    let path = snapshot_path(base_name, kind);

    if should_update_goldens() {
        fs::write(&path, actual_normalized).expect("failed to update golden snapshot");
        return;
    }

    let expected = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "missing golden snapshot '{}': {} (run with KUJO_UPDATE_GOLDENS=1 to generate)",
            path.display(),
            error
        )
    });
    let expected_normalized = normalize_snapshot_text(&expected);
    assert_eq!(expected_normalized, actual_normalized, "snapshot mismatch for {base_name}.{kind}");
}

fn read_fixture_source(name: &str) -> String {
    fs::read_to_string(fixtures_dir().join(name)).expect("failed to read diagnostic fixture")
}

fn run_runtime_json_diagnostic_fixture(fixture_file: &str, extra_args: &[&str]) -> String {
    let fixture_path = fixtures_dir().join(fixture_file);
    let mut args = vec!["run"];
    args.extend_from_slice(extra_args);
    args.push(fixture_path.to_str().expect("fixture path should be utf-8"));
    args.push("--json-runtime-diagnostics");

    let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to run kujo runtime diagnostic fixture");

    assert!(!output.status.success(), "runtime diagnostic fixture should fail: {fixture_file}");

    String::from_utf8(output.stdout).expect("runtime diagnostic stdout should be utf-8")
}

fn first_lexer_diagnostic_from_fixture(fixture_file: &str) -> Diagnostic {
    let source = read_fixture_source(fixture_file);
    let diagnostics = tokenize_with_file(&source, Some(fixture_file))
        .expect_err("fixture should produce lexer diagnostics");
    diagnostics.first().expect("lexer diagnostics should not be empty").to_diagnostic()
}

fn first_parser_diagnostic_from_fixture(fixture_file: &str) -> Diagnostic {
    let source = read_fixture_source(fixture_file);
    let tokens = tokenize_with_file(&source, Some(fixture_file))
        .expect("fixture should tokenize for parser");
    let mut parser = Parser::new(tokens);
    let parse_output = parser.parse_with_diagnostics();
    parse_output
        .diagnostics
        .first()
        .expect("parser diagnostics should not be empty")
        .to_diagnostic(Some(fixture_file))
}

fn to_human_snapshot(diagnostic: &Diagnostic) -> String {
    diagnostic.render_human()
}

fn to_json_snapshot(diagnostic: &Diagnostic) -> String {
    serde_json::to_string_pretty(&diagnostic.to_json_value())
        .expect("diagnostic json should serialize")
}

fn assert_golden_pair(base_name: &str, diagnostic: &Diagnostic) {
    assert_or_update_golden(base_name, "human", &to_human_snapshot(diagnostic));
    assert_or_update_golden(base_name, "json", &to_json_snapshot(diagnostic));
}

#[test]
fn diagnostics_golden_lexer_parse_semantic_runtime_cli_and_server_contracts() {
    let lexer = first_lexer_diagnostic_from_fixture("lexer_invalid_escape.kujo");
    assert_golden_pair("lexer_invalid_escape", &lexer);

    let parser = first_parser_diagnostic_from_fixture("parser_missing_paren.kujo");
    assert_golden_pair("parser_missing_paren", &parser);

    let semantic = first_parser_diagnostic_from_fixture("semantic_invalid_assignment.kujo");
    assert_eq!(semantic.code, DIAGNOSTIC_CODE_PARSER);
    assert_golden_pair("semantic_invalid_assignment", &semantic);

    let runtime = KujoError::runtime_error(
        "Undefined variable: missing_value".to_string(),
        SourceLocation::with_file(1, 1, "runtime_undefined_identifier.kujo".to_string()),
    )
    .as_diagnostic();
    assert_eq!(runtime.code, DIAGNOSTIC_CODE_RUNTIME);
    assert_golden_pair("runtime_undefined_identifier", &runtime);
    let runtime_run_envelope = kujo::errors::run_runtime_diagnostic_envelope_json(&runtime, 4);
    let runtime_run_envelope_json = serde_json::to_string_pretty(&runtime_run_envelope)
        .expect("runtime run envelope should serialize");
    assert_or_update_golden(
        "runtime_undefined_identifier_run_envelope",
        "json",
        &runtime_run_envelope_json,
    );

    let runtime_invalid_unary_envelope =
        run_runtime_json_diagnostic_fixture("runtime_invalid_unary.kujo", &["--interpreter"]);
    assert_or_update_golden(
        "runtime_invalid_unary_run_envelope",
        "json",
        &runtime_invalid_unary_envelope,
    );
    let runtime_break_outside_loop_envelope =
        run_runtime_json_diagnostic_fixture("runtime_break_outside_loop.kujo", &["--interpreter"]);
    assert_or_update_golden(
        "runtime_break_outside_loop_run_envelope",
        "json",
        &runtime_break_outside_loop_envelope,
    );
    let runtime_missing_module_envelope =
        run_runtime_json_diagnostic_fixture("runtime_missing_module_entry.kujo", &[]);
    assert_or_update_golden(
        "runtime_missing_module_run_envelope",
        "json",
        &runtime_missing_module_envelope,
    );
    let runtime_non_callable_envelope =
        run_runtime_json_diagnostic_fixture("runtime_non_callable_call.kujo", &[]);
    assert_or_update_golden(
        "runtime_non_callable_call_run_envelope",
        "json",
        &runtime_non_callable_envelope,
    );
    let runtime_capability_denied_envelope =
        run_runtime_json_diagnostic_fixture("runtime_capability_denied.kujo", &["--untrusted"]);
    assert_or_update_golden(
        "runtime_capability_denied_run_envelope",
        "json",
        &runtime_capability_denied_envelope,
    );

    let cli = Diagnostic::new(
        DIAGNOSTIC_CODE_CLI,
        DiagnosticSeverity::Error,
        DiagnosticSubsystem::Cli,
        "Invalid CLI invocation",
    )
    .with_location(Some("cli_invalid_flag.kujo".to_string()), 1, 1)
    .with_help("Use `kujo --help` to list valid commands");
    assert_golden_pair("cli_invalid_flag", &cli);

    let server = Diagnostic::new(
        DIAGNOSTIC_CODE_CLI,
        DiagnosticSeverity::Error,
        DiagnosticSubsystem::Cli,
        "serve max header bytes must be greater than 0",
    )
    .with_location(Some("serve_invalid_header_limit.kujo".to_string()), 1, 1)
    .with_help("Pass a positive integer to --max-header-bytes");
    assert_golden_pair("serve_invalid_header_limit", &server);
}

#[test]
fn diagnostics_golden_normalizes_line_endings() {
    let crlf_text = "line1\r\nline2\r\n";
    let normalized = normalize_snapshot_text(crlf_text);
    assert_eq!(normalized, "line1\nline2\n");
}
