use std::process::Command;

#[test]
fn scalar_type_errors_reach_handlers_after_native_warmup() {
    let mut failures = Vec::new();
    for (expression, expected) in [
        ("r + 1", "3"),
        ("1 + r", "3"),
        ("r - 1", "1"),
        ("r * 2", "4"),
        ("r / 1", "2"),
        ("1 / r", "0"),
        ("r % 3", "2"),
        ("1 % r", "1"),
        ("-r", "-2"),
        ("r < 3", "true"),
        ("r > 3", "false"),
        ("r <= 3", "true"),
        ("r >= 3", "false"),
    ] {
        let source = format!(
            r#"
            mut before := 0
            mut after := 0
            func calculate(x) {{
                before += 1
                mut r := 2
                if x > 0 {{ r = x > 1 }}
                let result := {expression}
                after += 1
                return result
            }}
            mut i := 0
            while i < 200 {{ assert_equal(calculate(0), {expected}) i += 1 }}
            mut caught := 0
            mut result := 42
            for x in [1, 2] {{
                try {{ result = calculate(x) }} except error {{ caught += 1 }}
                assert_equal(result, 42)
                assert_equal(calculate(0), {expected})
            }}
            assert_equal(caught, 2)
            assert_equal(before, 204)
            assert_equal(after, 202)
            print("PASS")
        "#
        );
        for mode in [None, Some("--interpreter"), Some("--jit")] {
            if let Err(error) = run(&source, mode, "calculate") {
                failures.push(format!("{expression} {mode:?}: {error}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn not_requires_boolean_and_equality_distinguishes_boolean_from_integer() {
    let source = r#"
        func negate(x) { mut r := x > 0 if x < 0 { r = 1 } return !r }
        func equal(x) { return (x > 0) == 1 }
        func unequal(x) { return (x > 0) != 1 }
        func bool_equal(x) { return (x > 0) == (x < 10) }
        mut i := 0
        while i < 200 {
            assert_equal(negate(1), false)
            assert_equal(negate(0), true)
            assert_equal(equal(1), false)
            assert_equal(unequal(1), true)
            assert_equal(bool_equal(1), true)
            assert_equal(bool_equal(20), false)
            i += 1
        }
        mut caught := false
        try { negate(-1) } except error { caught = true }
        assert_equal(caught, true)
        assert_equal(negate(0), true)
        print("PASS")
    "#;
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        run(source, mode, "negate").unwrap_or_else(|error| panic!("{mode:?}: {error}"));
    }
}

fn run(source: &str, mode: Option<&str>, compiled: &str) -> Result<(), String> {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("operators.kujo");
    std::fs::write(&file, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--scheduler-no-timeout"]).arg(&file);
    if let Some(flag) = mode {
        command.arg(flag);
    }
    if mode == Some("--jit") {
        command.env("DEBUG_JIT", "1");
    }
    let output = command.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(stderr.into_owned());
    }
    if String::from_utf8_lossy(&output.stdout).trim() != "PASS" {
        return Err("missing PASS".into());
    }
    if mode == Some("--jit")
        && !stderr.contains(&format!("Successfully compiled function '{compiled}'"))
    {
        return Err(format!("missing native compilation: {stderr}"));
    }
    Ok(())
}

#[test]
fn native_type_errors_keep_canonical_messages_and_allow_reuse() {
    use kujo::{compiler::Compiler, interpreter::Value, lexer, parser::Parser, vm::VM};
    for (expression, expected) in [
        ("(x > 0) + 1", "Invalid binary operation: bool + int"),
        ("1 + (x > 0)", "Invalid binary operation: int + bool"),
        ("(x > 0) / 0", "Invalid binary operation: bool / int"),
        ("-(x > 0)", "Invalid unary operation: - Bool(true)"),
        ("!x", "Invalid unary operation: ! Int(1)"),
        ("(x > 0) < 1", "Invalid binary operation: bool < int"),
    ] {
        let source = format!(
            "func calculate(x) {{ if x == 0 {{ return 7 }} return {expression} }} return calculate"
        );
        let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
        let chunk = Compiler::new().compile(&parser.parse()).unwrap();
        let mut vm = VM::new();
        vm.set_jit_enabled(true);
        let function = vm.execute(chunk).unwrap();
        assert!(vm.jit_compile_bytecode_function(&function).unwrap());
        for _ in 0..3 {
            assert_eq!(
                vm.call_function_from_jit(function.clone(), vec![Value::Int(1)]).unwrap_err(),
                expected
            );
            assert!(matches!(
                vm.call_function_from_jit(function.clone(), vec![Value::Int(0)]).unwrap(),
                Value::Int(7)
            ));
        }
    }
}

#[test]
fn native_logical_bytecodes_use_truthiness_and_return_boolean() {
    use kujo::{
        bytecode::{BytecodeBindingKind, BytecodeChunk, OpCode},
        interpreter::Value,
        vm::VM,
    };
    use std::collections::HashMap;
    for operator in [OpCode::And, OpCode::Or] {
        let mut chunk = BytecodeChunk::new();
        chunk.name = Some("logical".into());
        chunk.params = vec!["a".into(), "b".into()];
        chunk.local_names = chunk.params.clone();
        chunk.local_count = chunk.local_names.len();
        chunk.local_binding_kinds = vec![BytecodeBindingKind::Mutable; 2];
        for instruction in
            [OpCode::LoadLocal(0), OpCode::LoadLocal(1), operator.clone(), OpCode::Return]
        {
            chunk.emit(instruction);
        }
        let function = Value::BytecodeFunction {
            chunk,
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
        };
        for native in [false, true] {
            let mut vm = VM::new();
            vm.set_jit_enabled(native);
            if native {
                assert!(vm.jit_compile_bytecode_function(&function).unwrap());
            }
            for (a, b) in [(0, 0), (0, 2), (2, 0), (2, 4), (-2, 1), (-2, -4)] {
                let expected = if matches!(operator, OpCode::And) {
                    a != 0 && b != 0
                } else {
                    a != 0 || b != 0
                };
                let actual = vm
                    .call_function_from_jit(function.clone(), vec![Value::Int(a), Value::Int(b)])
                    .unwrap();
                assert!(
                    matches!(actual, Value::Bool(value) if value == expected),
                    "{operator:?} {a} {b}: {actual:?}"
                );
            }
        }
    }
}
