use std::process::Command;

#[test]
fn hot_integer_arithmetic_errors_preserve_handlers_and_side_effects() {
    let mut failures = Vec::new();
    for (operator, left, right) in [
        ("neg", "(-9223372036854775807 - 1)", "0"),
        ("+", "9223372036854775807", "1"),
        ("-", "(-9223372036854775807 - 1)", "1"),
        ("*", "9223372036854775807", "2"),
        ("/", "8", "0"),
        ("%", "8", "0"),
        ("/", "(-9223372036854775807 - 1)", "-1"),
        ("%", "(-9223372036854775807 - 1)", "-1"),
    ] {
        let expression =
            if operator == "neg" { "-a".to_string() } else { format!("a {operator} b") };
        let warm_result =
            if operator == "neg" { "-6".to_string() } else { format!("6 {operator} 2") };
        let source = format!(
            r#"
            mut before := 0
            mut after := 0
            func calculate(a, b) {{
                before += 1
                let result := {expression}
                after += 1
                return result
            }}
            func relay(a, b) {{ return calculate(a, b) }}
            mut i := 0
            while i < 200 {{ assert_equal(calculate(6, 2), {warm_result}) i += 1 }}
            i = 0
            while i < 200 {{ assert_equal(relay(6, 2), {warm_result}) i += 1 }}
            assert_equal(before, 400)
            assert_equal(after, 400)
            mut caught := 0
            mut result := 42
            i = 0
            while i < 3 {{
                try {{ result = relay({left}, {right}) }} except error {{ caught += 1 }}
                assert_equal(result, 42)
                assert_equal(relay(6, 2), {warm_result})
                i += 1
            }}
            assert_equal(caught, 3)
            assert_equal(before, 406)
            assert_equal(after, 403)
            print("PASS")
        "#
        );
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("checked.kujo");
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
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != "PASS"
            {
                failures.push(format!(
                    "{left} {operator} {right} mode={mode:?} status={:?}: {stderr}",
                    output.status
                ));
            } else if mode == Some("--jit")
                && !stderr.contains("Successfully compiled function 'calculate'")
            {
                failures.push(format!("{operator}: native calculation not compiled: {stderr}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn direct_native_arithmetic_preserves_exact_errors_and_can_be_reused() {
    use kujo::{
        compiler::Compiler,
        interpreter::Value,
        jit::{JitCompiler, VMContext},
        lexer,
        parser::Parser,
        vm::VM,
    };
    use std::collections::HashMap;
    for (expr, valid, expected, bad, error) in [
        ("-x", 7, -7, i64::MIN, "Integer overflow: -(-9223372036854775808)"),
        ("x + 1", 7, 8, i64::MAX, "Integer overflow: 9223372036854775807 + 1"),
        ("x / -1", 7, -7, i64::MIN, "Integer overflow: -9223372036854775808 / -1"),
        ("x % -1", 7, 0, i64::MIN, "Integer overflow: -9223372036854775808 % -1"),
    ] {
        let mut parser = Parser::new(
            lexer::tokenize(&format!("func calculate(x) {{ return {expr} }} return calculate"))
                .unwrap(),
        );
        let chunk = Compiler::new().compile(&parser.parse()).unwrap();
        let mut vm = VM::new();
        let Value::BytecodeFunction { chunk, .. } = vm.execute(chunk).unwrap() else {
            panic!("function expected")
        };
        let mut jit = JitCompiler::new().unwrap();
        let compiled = jit.compile_function_with_direct_arg(&chunk, "calculate").unwrap();
        let mut stack = Vec::new();
        let mut locals = HashMap::new();
        let mut globals = HashMap::new();
        let mut ctx = VMContext::new(&mut stack, &mut locals, &mut globals);
        for _ in 0..3 {
            // SAFETY: compiled code and all exclusively borrowed context data remain live.
            assert_eq!(unsafe { compiled(&mut ctx, valid) }, expected);
            assert!(ctx.runtime_error.is_none());
            // SAFETY: identical ownership/lifetime guarantees to the valid call.
            unsafe { compiled(&mut ctx, bad) };
            assert_eq!(ctx.runtime_error.take().as_deref(), Some(error), "{expr}");
        }
    }
}

#[test]
fn compiled_binary_arithmetic_matches_checked_contract_at_integer_boundaries() {
    use kujo::{compiler::Compiler, interpreter::Value, lexer, parser::Parser, vm::VM};
    let values = [
        i64::MIN,
        i64::MIN + 1,
        -3037000500,
        -2,
        -1,
        0,
        1,
        2,
        3037000499,
        3037000500,
        i64::MAX - 1,
        i64::MAX,
    ];
    for operator in ["+", "-", "*", "/", "%"] {
        let mut parser = Parser::new(
            lexer::tokenize(&format!(
                "func calculate(a, b) {{ return a {operator} b }} return calculate"
            ))
            .unwrap(),
        );
        let chunk = Compiler::new().compile(&parser.parse()).unwrap();
        let mut vm = VM::new();
        vm.set_jit_enabled(true);
        let function = vm.execute(chunk).unwrap();
        assert!(vm.jit_compile_bytecode_function(&function).unwrap());
        for left in values {
            for right in values {
                let actual = vm.call_function_from_jit(
                    function.clone(),
                    vec![Value::Int(left), Value::Int(right)],
                );
                match Value::checked_int_arithmetic(left, operator, right) {
                    Ok(expected) => assert!(
                        matches!(actual, Ok(Value::Int(value)) if value == expected),
                        "{left} {operator} {right}: {actual:?}"
                    ),
                    Err(expected) => {
                        assert_eq!(actual.unwrap_err(), expected, "{left} {operator} {right}")
                    }
                }
            }
        }
    }
}

#[test]
fn loop_compiler_error_exit_preserves_completed_local_writes() {
    use kujo::{
        bytecode::{BytecodeChunk, Constant, OpCode},
        interpreter::Value,
        jit::{JitCompiler, VMContext},
    };
    use std::collections::HashMap;
    let mut chunk = BytecodeChunk::new();
    chunk.local_names = vec!["before".into(), "after".into()];
    let one = chunk.add_constant(Constant::Int(1));
    let max = chunk.add_constant(Constant::Int(i64::MAX));
    for op in [
        OpCode::LoadLocal(0),
        OpCode::LoadConst(one),
        OpCode::Add,
        OpCode::StoreLocal(0),
        OpCode::Pop,
        OpCode::LoadConst(max),
        OpCode::LoadConst(one),
        OpCode::Add,
        OpCode::StoreLocal(1),
        OpCode::Return,
    ] {
        chunk.emit(op);
    }
    let mut jit = JitCompiler::new().unwrap();
    let compiled = jit.compile(&chunk, 0).unwrap();
    let mut slots = vec![Value::Int(40), Value::Int(99)];
    let mut stack = Vec::new();
    let mut locals = HashMap::new();
    let mut globals = HashMap::new();
    let mut ctx = VMContext::new(&mut stack, &mut locals, &mut globals);
    ctx.local_slots_ptr = &mut slots;
    // SAFETY: the compiler and context storage stay live and are exclusively used.
    assert_ne!(unsafe { compiled(&mut ctx) }, 0);
    assert_eq!(ctx.runtime_error.as_deref(), Some("Integer overflow: 9223372036854775807 + 1"));
    assert!(matches!(slots[0], Value::Int(41)));
    assert!(matches!(slots[1], Value::Int(99)));
}
