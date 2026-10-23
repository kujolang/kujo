use std::process::Command;

#[test]
fn compiled_callback_errors_stop_execution_and_reach_handlers() {
    let source = r#"
        func checked_one(x) { assert_equal(x, 1) print(x) return 7 }
        func checked_two(x, y) { assert_equal(x, y) print(x) return 7 }
        func nested(x) { return checked_two(x, 1) }
        mut i := 0
        while i < 200 {
            assert_equal(checked_one(1), 7)
            assert_equal(checked_two(1, 1), 7)
            assert_equal(nested(1), 7)
            i += 1
        }
        mut caught := 0
        try { checked_one(2) } except error { caught += 1 }
        try { checked_two(2, 1) } except error { caught += 1 }
        try { nested(2) } except error { caught += 1 }
        assert_equal(caught, 3)
        assert_equal(checked_one(1), 7)
        print("PASS")
    "#;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("errors.kujo");
    std::fs::write(&file, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]).arg(&file);
        if let Some(flag) = mode {
            command.arg(flag);
        }
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1");
        }
        let output = command.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "mode={mode:?}: {stderr}");
        assert_eq!(stdout.lines().filter(|line| *line == "1").count(), 601);
        assert!(!stdout.lines().any(|line| line == "2"), "ran after failed callback");
        assert_eq!(stdout.lines().last(), Some("PASS"));
        if mode == Some("--jit") {
            assert!(stderr.contains("Successfully compiled function 'checked_two'"), "{stderr}");
        }
    }
}

#[test]
fn failed_compiled_callback_bridge_preserves_message_and_can_be_reused() {
    use kujo::{compiler::Compiler, interpreter::Value, lexer, parser::Parser, vm::VM};
    let mut parser = Parser::new(
        lexer::tokenize("func checked(x, y) { assert_equal(x, y) return 7 } return checked")
            .unwrap(),
    );
    let chunk = Compiler::new().compile(&parser.parse()).unwrap();
    let mut vm = VM::new();
    vm.set_jit_enabled(true);
    let function = vm.execute(chunk).unwrap();
    assert!(vm.jit_compile_bytecode_function(&function).unwrap());
    for _ in 0..3 {
        let error = vm
            .call_function_from_jit(function.clone(), vec![Value::Int(2), Value::Int(1)])
            .unwrap_err();
        assert!(error.contains("Assertion failed"), "{error}");
        assert!(matches!(
            vm.call_function_from_jit(function.clone(), vec![Value::Int(1), Value::Int(1)])
                .unwrap(),
            Value::Int(7)
        ));
    }
}
