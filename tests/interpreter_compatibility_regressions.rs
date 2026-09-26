use kujo::{lexer::tokenize, parser::Parser, type_checker::TypeChecker};

fn check(source: &str) -> Result<(), Vec<kujo::errors::KujoError>> {
    let mut parser = Parser::new(tokenize(source).unwrap());
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty());
    TypeChecker::new().check(&parsed.stmts)
}

#[test]
fn gradual_bindings_shadow_builtins_and_can_hold_returned_callables() {
    let source = r#"
        func factory() { return func(value) { return value } }
        let callback := factory()
        callback(42)
        count := 0
        count := count + 1
        assert(count == 1)
        mut changed := false
        changed = contains("abc", "a")
        let result := str(count)
        let position := index_of([1, 2], 2)
        func unknown_result() { return Ok(42) }
        let unwrapped := unknown_result()?
    "#;
    assert!(check(source).is_ok(), "{:?}", check(source));
}

#[test]
fn explicit_annotations_and_unknown_functions_still_report_errors() {
    assert!(check("mut value: int := 1\nvalue = \"wrong\"").is_err());
    assert!(check("definitely_missing_function(1)").is_err());
    assert!(check("let value := 1? ").is_err());
}

#[test]
fn pattern_and_exception_bindings_shadow_builtin_names() {
    assert!(check(
        r#"match Err("bad") { case Err(error): { print("error: " + error) } }
        try { missing := 1 } except error { print("error: " + error) }"#
    )
    .is_ok());
}

#[test]
fn interpreter_preserves_uncaught_frames_and_clears_caught_frames() {
    let mut interpreter = kujo::interpreter::Interpreter::new();
    let mut parser = Parser::new(
        tokenize("func inner() { absent }\nfunc outer() { inner() }\nouter()").unwrap(),
    );
    interpreter.eval_stmts(&parser.parse());
    assert_eq!(interpreter.get_call_stack(), vec!["outer", "inner"]);
    interpreter.return_value = None;
    let mut parser =
        Parser::new(tokenize("try { outer() } except error {}\nanother_absent").unwrap());
    interpreter.eval_stmts(&parser.parse());
    assert!(interpreter.get_call_stack().is_empty());
}

#[test]
fn seek_and_positional_read_arities_match_runtime() {
    assert!(check("io_seek_read(\"file\", 0)").is_ok());
    assert!(check("io_seek_read(\"file\", 0, 1)").is_err());
    assert!(check("io_read_at(\"file\", 0, 1)").is_ok());
    assert!(check("io_read_at(\"file\", 0)").is_err());
}

#[test]
fn collection_inference_keeps_unknown_values_gradual() {
    // Dispatch's provider probe mixes known strings with an unannotated result.
    // Neither that result nor unknown parameters may be narrowed to String.
    let source = r#"
        func unknown_result() { return false }
        let probe := {"provider": "fixture", "ok": unknown_result()}
        assert(probe["ok"] == false)
        func inspect(unknown) {
            let values := ["fixture", unknown]
            assert(values[1] == false)
            let reversed := [unknown, "fixture"]
            assert(reversed[0] == false)
            let nested := [["fixture"], [unknown]]
            assert(nested[1][0] == false)
            let record := {"provider": "fixture", "ok": unknown}
            assert(record["ok"] == false)
        }
    "#;
    assert!(check(source).is_ok(), "{:?}", check(source));
    assert!(check(
        r#"let probe := {"ok": "wrong"}
        assert(probe["ok"] == false)"#
    )
    .is_err());
    assert!(check(r#"let value: bool := ["wrong"][0]"#).is_err());
}
