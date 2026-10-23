use std::process::Command;

fn run(source: &str, names: &[&str]) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("immediate.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]).arg(&path);
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
                    "{name}: {stderr}"
                );
            }
        }
    }
}

#[test]
fn native_float_and_null_values_survive_globals_callbacks_and_recursion() {
    run(
        r#"
        mut state := null
        func read_state(x) { return state }
        func assign_state(x) { state = x / 2.0 return state }
        func empty(x) { }
        func relay_empty(x) { return empty(x) }
        func recursive(x) { if x == 0 { return state } return recursive(x - 1) }
        func identity(x) { return x }
        func relay(x) { return identity(state) }
        func choose(x) { mut value := null if x > 0 { value = 2.5 } return value }
        mut i := 0
        while i < 200 {
            state = null
            assert_equal(read_state(0), null)
            assert_equal(relay(0), null)
            assert_equal(recursive(3), null)
            assert_equal(relay_empty(0), null)
            assert_equal(empty(0), null)
            assert_equal(choose(0), null)
            assert_equal(choose(1), 2.5)
            assert_equal(assign_state(7), 3.5)
            assert_equal(state, 3.5)
            assert_equal(read_state(0), 3.5)
            assert_equal(relay(0), 3.5)
            assert_equal(recursive(3), 3.5)
            i += 1
        }
        print("PASS")
    "#,
        &["read_state", "assign_state", "empty", "relay_empty", "recursive", "relay", "choose"],
    );
}

#[test]
fn native_float_operations_match_reference_values_and_error_recovery() {
    run(
        r#"
        mut left := 7.5
        mut right := 2.0
        func add(x) { return left + right }
        func sub(x) { return left - right }
        func mul(x) { return left * right }
        func div(x) { return left / right }
        func rem(x) { return left % right }
        func neg(x) { return -left }
        func less(x) { return left < right }
        func equal(x) { return left == right }
        func unequal(x) { return left != right }
        func truth(x) { if left { return 7 } return 9 }
        func not_value(x) { return !left }
        mut i := 0
        while i < 200 {
            left = 7.5 right = 2.0
            assert_equal(add(0), 9.5)
            assert_equal(sub(0), 5.5)
            assert_equal(mul(0), 15.0)
            assert_equal(div(0), 3.75)
            assert_equal(rem(0), 1.5)
            assert_equal(neg(0), -7.5)
            assert_equal(less(0), false)
            assert_equal(equal(0), false)
            assert_equal(unequal(0), true)
            left = 2 right = 2.0
            assert_equal(add(0), 4.0)
            assert_equal(equal(0), true)
            left = -0.0
            assert_equal(truth(0), 9)
            left = 0.5
            assert_equal(truth(0), 7)
            left = false
            assert_equal(not_value(0), true)
            i += 1
        }
        left = 7.5 right = 0.0
        mut caught := 0
        try { div(0) } except error { caught += 1 }
        try { rem(0) } except error { caught += 1 }
        try { not_value(0) } except error { caught += 1 }
        left = null
        try { add(0) } except error { caught += 1 }
        try { neg(0) } except error { caught += 1 }
        assert_equal(caught, 5)
        assert_equal(equal(0), false)
        left = 7.5 right = 2
        assert_equal(div(0), 3.75)
        assert_equal(rem(0), 1.5)
        print("PASS")
    "#,
        &[
            "add",
            "sub",
            "mul",
            "div",
            "rem",
            "neg",
            "less",
            "equal",
            "unequal",
            "truth",
            "not_value",
        ],
    );
}

#[test]
fn native_loop_entry_and_exit_preserve_immediate_local_types() {
    use kujo::{
        bytecode::{BytecodeBindingKind, BytecodeChunk, Constant, OpCode},
        interpreter::Value,
        jit::{JitCompiler, VMContext},
    };
    use std::collections::HashMap;
    let mut chunk = BytecodeChunk::new();
    chunk.local_names = vec!["value".into(), "remaining".into()];
    chunk.local_count = 2;
    chunk.local_binding_kinds = vec![BytecodeBindingKind::Mutable; 2];
    chunk.constants = vec![Constant::Int(0), Constant::Float(0.5), Constant::Int(1)];
    for op in [
        OpCode::LoadLocal(1),
        OpCode::LoadConst(0),
        OpCode::GreaterThan,
        OpCode::JumpIfFalse(16),
        OpCode::Pop,
        OpCode::LoadLocal(0),
        OpCode::LoadConst(1),
        OpCode::Add,
        OpCode::StoreLocal(0),
        OpCode::Pop,
        OpCode::LoadLocal(1),
        OpCode::LoadConst(2),
        OpCode::Sub,
        OpCode::StoreLocal(1),
        OpCode::Pop,
        OpCode::JumpBack(0),
        OpCode::Pop,
        OpCode::LoadLocal(0),
        OpCode::Return,
    ] {
        chunk.emit(op);
    }
    let mut jit = JitCompiler::new().unwrap();
    let compiled = jit.compile(&chunk, 0).unwrap();
    let mut stack = Vec::new();
    let mut locals = HashMap::new();
    let mut globals = HashMap::new();
    for initial in [Value::Int(1), Value::Float(1.0)] {
        let mut slots = vec![initial, Value::Int(3)];
        let mut ctx = VMContext::new(&mut stack, &mut locals, &mut globals);
        ctx.local_slots_ptr = &mut slots;
        // SAFETY: compiled code and all context storage remain live and exclusive.
        assert_eq!(unsafe { compiled(&mut ctx) }, 0);
        assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
        assert!(matches!(ctx.scalar_value(ctx.return_value), Value::Float(x) if x == 2.5));
        assert!(matches!(slots[0], Value::Float(x) if x == 2.5), "{:?}", slots[0]);
        assert!(matches!(slots[1], Value::Int(0)));
    }
}

#[test]
fn native_immediate_operations_match_vm_at_ieee_and_integer_boundaries() {
    use kujo::{
        compiler::Compiler,
        interpreter::{Environment, Value},
        lexer,
        parser::Parser,
        vm::VM,
    };
    use std::sync::{Arc, Mutex};
    let values = vec![
        Value::Int(i64::MIN),
        Value::Int(i64::MAX),
        Value::Int(0),
        Value::Int(9_007_199_254_740_993),
        Value::Float(-0.0),
        Value::Float(0.0),
        Value::Float(1.5),
        Value::Float(f64::EPSILON / 2.0),
        Value::Float(f64::NAN),
        Value::Float(f64::INFINITY),
        Value::Float(f64::NEG_INFINITY),
        Value::Float(f64::MAX),
        Value::Bool(true),
        Value::Null,
    ];
    for operator in ["+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "neg", "not"] {
        let expression = match operator {
            "neg" => "-left".to_owned(),
            "not" => "!left".to_owned(),
            op => format!("left {op} right"),
        };
        let source = format!("func calculate(x) {{ return {expression} }} return calculate");
        let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
        let chunk = Compiler::new().compile(&parser.parse()).unwrap();
        let mut reference = VM::new();
        let mut native = VM::new();
        native.set_jit_enabled(true);
        let reference_env = Arc::new(Mutex::new(Environment::new()));
        let native_env = Arc::new(Mutex::new(Environment::new()));
        reference.set_globals(Arc::clone(&reference_env));
        native.set_globals(Arc::clone(&native_env));
        let function = reference.execute(chunk.clone()).unwrap();
        let native_function = native.execute(chunk).unwrap();
        assert!(native.jit_compile_bytecode_function(&native_function).unwrap());
        for left in &values {
            for right in &values {
                for env in [&reference_env, &native_env] {
                    let mut env = env.lock().unwrap();
                    env.define("left".into(), left.clone());
                    env.define("right".into(), right.clone());
                }
                let expected = reference
                    .call_bytecode_function_from_interpreter(function.clone(), vec![Value::Int(0)]);
                let actual = native.call_bytecode_function_from_interpreter(
                    native_function.clone(),
                    vec![Value::Int(0)],
                );
                match (&expected, &actual) {
                    (Ok(Value::Float(a)), Ok(Value::Float(b))) => {
                        assert!(
                            (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits(),
                            "{left:?} {operator} {right:?}: {expected:?} vs {actual:?}"
                        );
                    }
                    _ => assert_eq!(
                        format!("{actual:?}"),
                        format!("{expected:?}"),
                        "{left:?} {operator} {right:?}"
                    ),
                }
            }
        }
    }
}
