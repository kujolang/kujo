#[path = "support/native_api.rs"]
mod support;

const MODULE: &str = r#"
export func expression(n) { return term(n) }
func term(n) { return factor(n) }
func factor(n) { if n == 0 { return 17 } return expression(n - 1) }
"#;

#[test]
fn imported_mutual_recursion_preserves_module_bindings_in_both_runtimes() {
    for interpreter in [false, true] {
        let out = support::run_modules(
            r#"
            from recursive import expression
            func term(n) { return -1 }
            assert_equal(expression(10), 17)
            assert_equal(expression(60), 17)
            import recursive
            assert_equal(recursive.expression(60), 17)
            "#,
            interpreter,
            &[("recursive.kujo", MODULE)],
        );
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
}

#[test]
fn excessive_imported_depth_is_catchable_and_recovers() {
    for interpreter in [false, true] {
        let out = support::run_modules(
            r#"
            from recursive import expression
            mut caught := false
            try { expression(100) } except e { caught = true }
            assert_equal(caught, true)
            assert_equal(expression(60), 17)
            "#,
            interpreter,
            &[("recursive.kujo", MODULE)],
        );
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
}

#[test]
fn direct_recursion_has_the_same_supported_depth() {
    support::success(&format!("{MODULE}\nassert_equal(expression(60), 17)"));
}

#[test]
fn ast_recursion_grows_a_small_embedding_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let tokens =
                kujo::lexer::tokenize(&format!("{MODULE}\nassert_equal(expression(60), 17)"))
                    .unwrap();
            let mut parser = kujo::parser::Parser::new(tokens);
            let stmts = parser.parse();
            let mut interpreter = kujo::interpreter::Interpreter::new();
            interpreter.eval_stmts(&stmts);
            assert!(interpreter.return_value.is_none());
        })
        .unwrap()
        .join()
        .unwrap();
}
