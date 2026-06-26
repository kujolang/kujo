use kujo::errors::{
    Diagnostic, DiagnosticSeverity, DiagnosticSubsystem, KujoError, SourceLocation,
    DIAGNOSTIC_CODE_CLI, DIAGNOSTIC_CODE_LEXER, DIAGNOSTIC_CODE_PARSER,
};
use kujo::lexer::tokenize_with_file;
use kujo::parser::Parser;

#[test]
fn diagnostic_human_render_includes_code_subsystem_and_location() {
    let diagnostic = Diagnostic::new(
        DIAGNOSTIC_CODE_CLI,
        DiagnosticSeverity::Error,
        DiagnosticSubsystem::Cli,
        "Invalid CLI invocation",
    )
    .with_location(Some("script.kujo".to_string()), 2, 8)
    .with_help("Use `kujo run <file>`");

    let rendered = diagnostic.render_human();
    assert!(rendered.contains("[KUJOCLI001]"));
    assert!(rendered.contains("[cli]"));
    assert!(rendered.contains("script.kujo:2:8"));
    assert!(rendered.contains("help: Use `kujo run <file>`"));
}

#[test]
fn diagnostic_json_shape_includes_required_fields() {
    let diagnostic = Diagnostic::new(
        DIAGNOSTIC_CODE_CLI,
        DiagnosticSeverity::Error,
        DiagnosticSubsystem::Cli,
        "Invalid CLI invocation",
    )
    .with_location(Some("script.kujo".to_string()), 2, 8)
    .with_help("Use `kujo run <file>`");

    let json = diagnostic.to_json_value();
    assert_eq!(json["code"], "KUJOCLI001");
    assert_eq!(json["severity"], "error");
    assert_eq!(json["subsystem"], "cli");
    assert_eq!(json["message"], "Invalid CLI invocation");
    assert_eq!(json["help"], "Use `kujo run <file>`");
    assert_eq!(json["file"], "script.kujo");
    assert_eq!(json["line"], 2);
    assert_eq!(json["column"], 8);
}

#[test]
fn runtime_error_display_keeps_location_when_available() {
    let runtime_error = KujoError::runtime_error(
        "boom".to_string(),
        SourceLocation::with_file(3, 7, "main.kujo".to_string()),
    );

    let rendered = runtime_error.to_string();
    assert!(rendered.contains("[KUJORUN001]"));
    assert!(rendered.contains("main.kujo:3:7"));
}

#[test]
fn lexer_diagnostic_converts_to_stable_code() {
    let diagnostics = tokenize_with_file("let value := @", Some("fixture.kujo"))
        .expect_err("source should produce lexical diagnostics");
    let first = diagnostics.first().expect("diagnostics should not be empty");
    let converted = first.to_diagnostic();

    assert!(converted.code.starts_with(DIAGNOSTIC_CODE_LEXER));
    assert_eq!(converted.subsystem, DiagnosticSubsystem::Lexer);
    assert_eq!(converted.file.as_deref(), Some("fixture.kujo"));
}

#[test]
fn parser_diagnostic_converts_to_stable_code() {
    let tokens =
        tokenize_with_file("print((1 + 2", Some("broken.kujo")).expect("source should tokenize");
    let mut parser = Parser::new(tokens);
    let parse_output = parser.parse_with_diagnostics();
    let first = parse_output.diagnostics.first().expect("parse diagnostics should not be empty");
    let converted = first.to_diagnostic(Some("broken.kujo"));

    assert_eq!(converted.code, DIAGNOSTIC_CODE_PARSER);
    assert_eq!(converted.subsystem, DiagnosticSubsystem::Parser);
    assert_eq!(converted.file.as_deref(), Some("broken.kujo"));
}
