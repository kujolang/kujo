use kujo::lexer::tokenize;
use kujo::parser::{ParseOutput, Parser, ParserLimits, DEFAULT_MAX_SOURCE_BYTES};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn parse_output(source: &str) -> ParseOutput {
    let tokens = tokenize(source).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    parser.parse_with_diagnostics()
}

fn parse_output_with_limits(source: &str, limits: ParserLimits) -> ParseOutput {
    let tokens = tokenize(source).expect("test source should tokenize");
    let mut parser = Parser::new_with_limits(tokens, limits);
    parser.parse_with_diagnostics()
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

fn write_fixture(path: &Path, content: &str) {
    fs::write(path, content).expect("failed to write fixture file");
}

fn run_kujo(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(args)
        .output()
        .expect("failed to execute kujo binary")
}

fn nested_parenthesized_expression(depth: usize) -> String {
    format!("value := {}1{}\n", "(".repeat(depth), ")".repeat(depth))
}

fn nested_array_literal(depth: usize) -> String {
    let mut source = String::from("value := ");
    source.push_str(&"[".repeat(depth));
    source.push('1');
    source.push_str(&"]".repeat(depth));
    source.push('\n');
    source
}

fn nested_if_blocks(depth: usize) -> String {
    let mut source = String::new();
    for _ in 0..depth {
        source.push_str("if true {\n");
    }
    source.push_str("value := 1\n");
    for _ in 0..depth {
        source.push_str("}\n");
    }
    source
}

#[test]
fn parser_accepts_valid_program_without_diagnostics() {
    let output = parse_output("let value := 1\nprint(value)\n");
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.stmts.len(), 2);
    assert!(!output.ast_spans.is_empty(), "expected parser to record AST spans");
    assert!(output.ast_spans.iter().all(|node| node.span.end_byte >= node.span.start_byte));
    assert!(output
        .ast_spans
        .iter()
        .any(|node| matches!(node.kind, kujo::parser::AstNodeSpanKind::Statement)));
    assert!(output
        .ast_spans
        .iter()
        .any(|node| matches!(node.kind, kujo::parser::AstNodeSpanKind::Expression)));
}

#[test]
fn parser_accepts_from_import_with_single_level_dotted_module_path() {
    let output = parse_output("from src.util import value\n");
    assert!(
        output.diagnostics.is_empty(),
        "expected dotted from-import to parse, got {:?}",
        output.diagnostics
    );
    assert_eq!(output.stmts.len(), 1);
    match &output.stmts[0] {
        kujo::ast::Stmt::Import { module, symbols } => {
            assert_eq!(module, "src.util");
            let expected = vec!["value".to_string()];
            assert_eq!(symbols.as_ref(), Some(&expected));
        }
        other => panic!("expected import statement, got {other:?}"),
    }
}

#[test]
fn parser_accepts_from_import_with_multi_level_dotted_module_path() {
    let output = parse_output("from src.core.math import add, sub\n");
    assert!(
        output.diagnostics.is_empty(),
        "expected multi-level dotted from-import to parse, got {:?}",
        output.diagnostics
    );
    assert_eq!(output.stmts.len(), 1);
    match &output.stmts[0] {
        kujo::ast::Stmt::Import { module, symbols } => {
            assert_eq!(module, "src.core.math");
            let expected = vec!["add".to_string(), "sub".to_string()];
            assert_eq!(symbols.as_ref(), Some(&expected));
        }
        other => panic!("expected import statement, got {other:?}"),
    }
}

#[test]
fn parser_reports_diagnostic_for_malformed_dotted_from_import_path() {
    let output = parse_output("from src..util import value\n");
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Expected module path segment after '.' in from-import statement")));
}

#[test]
fn parser_reports_diagnostic_for_trailing_dot_in_from_import_path() {
    let output = parse_output("from src. import value\n");
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Expected module path segment after '.' in from-import statement")));
}

#[test]
fn parser_reports_diagnostic_for_invalid_from_import_token_order() {
    let output = parse_output("from src util import value\n");
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Expected 'import' after module name in from-import statement")));
}

#[test]
fn parser_keeps_existing_flat_import_forms_unchanged() {
    let output = parse_output("import math_helper\nfrom utils import helper, formatter\n");
    assert!(
        output.diagnostics.is_empty(),
        "expected existing flat imports to keep parsing, got {:?}",
        output.diagnostics
    );
    assert_eq!(output.stmts.len(), 2);
    match &output.stmts[0] {
        kujo::ast::Stmt::Import { module, symbols } => {
            assert_eq!(module, "math_helper");
            assert!(symbols.is_none());
        }
        other => panic!("expected import statement, got {other:?}"),
    }
    match &output.stmts[1] {
        kujo::ast::Stmt::Import { module, symbols } => {
            assert_eq!(module, "utils");
            let expected = vec!["helper".to_string(), "formatter".to_string()];
            assert_eq!(symbols.as_ref(), Some(&expected));
        }
        other => panic!("expected import statement, got {other:?}"),
    }
}

#[test]
fn parser_accepts_bare_return_before_closing_brace() {
    let output = parse_output("func noop() {\n    return\n}\n");
    assert!(
        output.diagnostics.is_empty(),
        "expected bare return to parse without diagnostics, got {:?}",
        output.diagnostics
    );
    assert_eq!(output.stmts.len(), 1);

    match &output.stmts[0] {
        kujo::ast::Stmt::FuncDef { body, .. } => {
            assert_eq!(body.len(), 1);
            assert!(matches!(body[0], kujo::ast::Stmt::Return(None)));
        }
        other => panic!("expected function statement, got {other:?}"),
    }
}

#[test]
fn parser_accepts_bare_return_at_eof() {
    let output = parse_output("return");
    assert!(
        output.diagnostics.is_empty(),
        "expected bare return at EOF to parse cleanly, got {:?}",
        output.diagnostics
    );
    assert_eq!(output.stmts.len(), 1);
    assert!(matches!(output.stmts[0], kujo::ast::Stmt::Return(None)));
}

#[test]
fn parser_reports_missing_closing_parenthesis() {
    let output = parse_output("print((1 + 2)\n");
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Expected ')'")));
}

#[test]
fn parser_diagnostic_span_matches_legacy_location_fields() {
    let output = parse_output("print((1 + 2)\n");
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("Expected ')'"))
        .expect("expected missing parenthesis diagnostic");

    assert_eq!(diagnostic.line, diagnostic.span.start.line);
    assert_eq!(diagnostic.column, diagnostic.span.start.column);
    assert!(diagnostic.span.end_byte >= diagnostic.span.start_byte);
}

#[test]
fn parser_reports_missing_closing_bracket() {
    let output = parse_output("values := [1, 2\n");
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Expected ']'")));
}

#[test]
fn parser_reports_missing_closing_brace() {
    let output = parse_output("if true {\n  print(1)\n");
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Expected '}'")));
}

#[test]
fn parser_reports_invalid_assignment_target() {
    let output = parse_output("foo() := 1\n");
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Invalid assignment target")));
}

#[test]
fn parser_reports_unexpected_eof_in_function_body() {
    let output = parse_output("func greet(name) {\n");
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Expected '}'")));
}

#[test]
fn parser_recovery_reports_multiple_independent_errors() {
    let output = parse_output("print((1 + 2\nvalues := [1, 2\nok := 1\n");

    let messages: Vec<&str> =
        output.diagnostics.iter().map(|diagnostic| diagnostic.message.as_str()).collect();
    assert!(messages.iter().any(|message| message.contains("Expected ')'")));
    assert!(messages.iter().any(|message| message.contains("Expected ']'")));
    assert!(output.stmts.iter().any(|stmt| matches!(stmt, kujo::ast::Stmt::Assign { .. })));
    assert!(output
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.span.end_byte >= diagnostic.span.start_byte));
}

#[test]
fn parser_reports_expression_depth_limit_for_parenthesized_expressions() {
    let limits =
        ParserLimits { max_expression_depth: 8, max_block_depth: 64, ..ParserLimits::default() };
    let output = parse_output_with_limits(&nested_parenthesized_expression(16), limits);
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Maximum expression nesting depth of 8 exceeded")));
}

#[test]
fn parser_reports_expression_depth_limit_for_nested_array_literals() {
    let limits =
        ParserLimits { max_expression_depth: 6, max_block_depth: 64, ..ParserLimits::default() };
    let output = parse_output_with_limits(&nested_array_literal(12), limits);
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Maximum expression nesting depth of 6 exceeded")));
}

#[test]
fn parser_reports_block_depth_limit_for_nested_if_blocks() {
    let limits =
        ParserLimits { max_expression_depth: 64, max_block_depth: 4, ..ParserLimits::default() };
    let output = parse_output_with_limits(&nested_if_blocks(8), limits);
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Maximum block nesting depth of 4 exceeded")));
}

#[test]
fn parser_accepts_expression_depth_at_limit_boundary() {
    let limits =
        ParserLimits { max_expression_depth: 16, max_block_depth: 64, ..ParserLimits::default() };
    let output = parse_output_with_limits(&nested_parenthesized_expression(6), limits);
    assert!(
        output.diagnostics.is_empty(),
        "expected no diagnostics at boundary-safe expression depth, got {:?}",
        output.diagnostics
    );
}

#[test]
fn parser_reports_collection_literal_limit_for_array() {
    let limits = ParserLimits { max_collection_literal_items: 4, ..ParserLimits::default() };
    let output = parse_output_with_limits("[1, 2, 3, 4, 5]\n", limits);
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Array literal exceeds maximum element count of 4")));
}

#[test]
fn parser_reports_collection_literal_limit_for_dictionary() {
    let limits = ParserLimits { max_collection_literal_items: 3, ..ParserLimits::default() };
    let output = parse_output_with_limits("{\"a\": 1, \"b\": 2, \"c\": 3, \"d\": 4}\n", limits);
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Dictionary literal exceeds maximum item count of 3")));
}

#[test]
fn parser_accepts_collection_literal_at_limit_boundary() {
    let limits = ParserLimits { max_collection_literal_items: 5, ..ParserLimits::default() };
    let output = parse_output_with_limits("[1, 2, 3, 4, 5]\n", limits);
    assert!(
        output.diagnostics.is_empty(),
        "expected no diagnostics at collection literal boundary, got {:?}",
        output.diagnostics
    );
}

#[test]
fn cli_run_exits_non_zero_on_parse_diagnostics() {
    let dir = unique_temp_dir("cli_run_parse_error");
    let file = dir.join("broken.kujo");
    write_fixture(&file, "print((1 + 2)\n");

    let output = run_kujo(&["run", file.to_str().expect("path should be utf-8")]);
    assert_eq!(output.status.code(), Some(3));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("[KUJOPARSE001]"));
    assert!(stderr.contains("Expected ')'"));
}

#[test]
fn cli_test_run_exits_non_zero_on_parse_diagnostics() {
    let dir = unique_temp_dir("cli_test_run_parse_error");
    let file = dir.join("broken_test.kujo");
    write_fixture(&file, "test \"broken\" {\n    print((1 + 2)\n}\n");

    let output = run_kujo(&["test-run", file.to_str().expect("path should be utf-8")]);
    assert_eq!(output.status.code(), Some(3));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("[KUJOPARSE001]"));
    assert!(stderr.contains("Expected ')'"));
}

#[test]
fn cli_run_exits_non_zero_when_source_exceeds_max_size() {
    let dir = unique_temp_dir("cli_run_source_size_limit");
    let file = dir.join("oversized.kujo");
    let oversized = " ".repeat(DEFAULT_MAX_SOURCE_BYTES + 1);
    write_fixture(&file, &oversized);

    let output = run_kujo(&["run", file.to_str().expect("path should be utf-8")]);
    assert_eq!(output.status.code(), Some(3));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("[KUJOPARSE001]"));
    assert!(stderr.contains("Source size"));
    assert!(stderr.contains("exceeds maximum"));
}

#[test]
fn cli_run_accepts_source_at_max_size_boundary() {
    let dir = unique_temp_dir("cli_run_source_size_boundary");
    let file = dir.join("boundary.kujo");
    let boundary = " ".repeat(DEFAULT_MAX_SOURCE_BYTES);
    write_fixture(&file, &boundary);

    let output = run_kujo(&["run", file.to_str().expect("path should be utf-8")]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success for source at byte boundary, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}
