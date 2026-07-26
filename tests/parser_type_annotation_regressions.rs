use kujo::ast::{Stmt, TypeAnnotation};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn parse_without_panic(source: &str) {
    let parse_result = catch_unwind(AssertUnwindSafe(|| {
        let tokens = tokenize(source).expect("regression source should tokenize");
        let mut parser = Parser::new(tokens);
        parser.parse()
    }));

    assert!(parse_result.is_ok(), "parser panicked for malformed user input: {source}");
}

#[test]
fn malformed_result_type_annotation_missing_comma_does_not_panic() {
    let source = "func broken(value: Result<int string>) { return value }";
    parse_without_panic(source);
}

#[test]
fn malformed_result_type_annotation_missing_closer_does_not_panic() {
    let source = "func broken(value: Result<int, string) { return value }";
    parse_without_panic(source);
}

#[test]
fn malformed_option_type_annotation_missing_closer_does_not_panic() {
    let source = "func broken(value: Option<int) { return value }";
    parse_without_panic(source);
}

#[test]
fn result_and_option_type_annotations_parse_as_identifier_tokens() {
    let source = "func typed(value: Result<int, string>, maybe: Option<int>) { return value }";
    let tokens = tokenize(source).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse();

    let Some(Stmt::FuncDef { param_types, .. }) = program.first() else {
        panic!("expected typed function definition");
    };

    assert!(matches!(
        param_types.first(),
        Some(Some(TypeAnnotation::Result { ok_type, err_type }))
            if matches!(**ok_type, TypeAnnotation::Int)
                && matches!(**err_type, TypeAnnotation::String)
    ));
    assert!(matches!(
        param_types.get(1),
        Some(Some(TypeAnnotation::Option { inner_type }))
            if matches!(**inner_type, TypeAnnotation::Int)
    ));
}
