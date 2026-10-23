use std::process::Command;

fn check(source: &str, names: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("global_types.kujo");
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
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            for name in names {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
            if names.contains(&"walk") {
                assert!(
                    stderr.contains("Compiled 'walk' with direct-arg optimization"),
                    "{stderr}"
                );
                assert!(stderr.contains("Interpreter direct-arg call to 'walk'"), "{stderr}");
            }
        }
    }
}

#[test]
fn native_global_reads_preserve_boolean_and_integer_types_between_calls() {
    check(
        r#"
        mut state := true
        func read_state(x) { return state }
        func walk(x) { if x == 0 { return state } return walk(x - 1) }
        func relay_walk(x) { return walk(x) }
        func numeric(x) { return state + x }
        func same(x) { return state == x }
        func positive(x) { return x > 0 }
        func relay(x) { return positive(x) }
        mut i := 0
        while i < 200 {
            state = true
            assert_equal(read_state(0), true)
            assert_equal(walk(3), true)
            assert_equal(relay_walk(3), true)
            assert_equal(same(1), false)
            state = false
            assert_equal(read_state(0), false)
            assert_equal(walk(3), false)
            assert_equal(relay_walk(3), false)
            assert_equal(same(0), false)
            state = -1
            assert_equal(read_state(0), -1)
            assert_equal(walk(3), -1)
            assert_equal(relay_walk(3), -1)
            assert_equal(numeric(2), 1)
            assert_equal(same(-1), true)
            assert_equal(relay(1), true)
            i += 1
        }
        state = true
        mut caught := 0
        try { numeric(2) } except error { caught += 1 }
        assert_equal(caught, 1)
        state = -1
        assert_equal(numeric(2), 1)
        print("PASS")
    "#,
        &["read_state", "relay", "walk", "relay_walk", "numeric", "same"],
    );
}

#[test]
fn native_global_assignments_preserve_types_and_readonly_errors() {
    check(
        r#"
        mut state := 7
        const locked := true
        mut before := 0
        mut after := 0
        func write_state(x) { state = x > 0 return state }
        func write_number(x) { state = x return state }
        func fail_write(x) { before += 1 locked = x > 0 after += 1 return x }
        mut i := 0
        while i < 200 {
            assert_equal(write_state(1), true)
            assert_equal(state, true)
            assert_equal(write_state(0), false)
            assert_equal(state, false)
            assert_equal(write_number(-1), -1)
            assert_equal(state, -1)
            try { fail_write(1) } except error { }
            i += 1
        }
        assert_equal(locked, true)
        assert_equal(before, 200)
        assert_equal(after, 0)
        print("PASS")
    "#,
        &["write_state", "write_number", "fail_write"],
    );
}

#[test]
fn explicit_global_bytecodes_ignore_same_named_parameters_and_check_bindings() {
    use kujo::{
        bytecode::{Constant, OpCode},
        compiler::Compiler,
        interpreter::Value,
        lexer,
        parser::Parser,
        vm::VM,
    };
    for binding in ["mut", "const", "missing"] {
        for native in [false, true] {
            let declaration = if binding == "missing" {
                String::new()
            } else {
                format!("{binding} state := true")
            };
            let source =
                format!("{declaration} func raw(state, a, b, c, d) {{ return state }} return raw");
            let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
            let code = Compiler::new().compile(&parser.parse()).unwrap();
            let mut vm = VM::new();
            vm.set_jit_enabled(native);
            let mut function = vm.execute(code).unwrap();
            let Value::BytecodeFunction { chunk, .. } = &mut function else {
                panic!("function expected")
            };
            chunk.instructions.clear();
            chunk.constants = vec![Constant::Bool(false)];
            for opcode in [
                OpCode::LoadConst(0),
                OpCode::StoreGlobal("state".into()),
                OpCode::Pop,
                OpCode::LoadGlobal("state".into()),
                OpCode::Return,
            ] {
                chunk.emit(opcode);
            }
            if native {
                assert!(vm.jit_compile_bytecode_function(&function).unwrap());
            }
            let result = vm.call_function_from_jit(
                function,
                vec![Value::Int(7), Value::Int(0), Value::Int(0), Value::Int(0), Value::Int(0)],
            );
            if binding != "const" {
                assert!(matches!(result, Ok(Value::Bool(false))), "native={native}: {result:?}");
            } else {
                assert!(result.is_err(), "native={native}: readonly write succeeded: {result:?}");
            }
        }
    }
}
