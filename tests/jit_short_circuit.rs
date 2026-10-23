use std::process::Command;

#[test]
fn native_short_circuit_preserves_results_skipped_effects_and_traps() {
    let source = r#"
        mut calls := 0
        func rhs(x) { calls += 1 return x > 0 }
        func within(x) { return x > 0 && x < 10 }
        func outside(x) { return x < 0 || x > 10 }
        func nested(x) { return (x > 0 && x < 10) || x == 20 }
        func effect_and(x) { return x > 0 && rhs(x) }
        func effect_or(x) { return x == 0 || rhs(x) }
        func safe_divide(x) { return x != 0 && (10 / x) > 1 }
        func looping(x) { mut n := 0 while n < x && n < 10 { n += 1 } return n }
        func recursive(x) { return x == 0 || recursive(x - 1) }
        mut i := 0
        while i < 200 {
            assert_equal(within(-1), false)
            assert_equal(within(5), true)
            assert_equal(within(11), false)
            assert_equal(outside(-1), true)
            assert_equal(outside(5), false)
            assert_equal(outside(11), true)
            assert_equal(nested(-1), false)
            assert_equal(nested(5), true)
            assert_equal(nested(11), false)
            assert_equal(nested(20), true)
            assert_equal(effect_and(0), false)
            assert_equal(effect_and(1), true)
            assert_equal(effect_or(0), true)
            assert_equal(effect_or(-1), false)
            assert_equal(safe_divide(0), false)
            assert_equal(safe_divide(2), true)
            assert_equal(safe_divide(10), false)
            assert_equal(looping(4), 4)
            assert_equal(looping(20), 10)
            assert_equal(looping(0), 0)
            assert_equal(recursive(4), true)
            i += 1
        }
        assert_equal(calls, 400)
        print("PASS")
    "#;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("short_circuit.kujo");
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
            for name in [
                "within",
                "outside",
                "nested",
                "effect_and",
                "effect_or",
                "safe_divide",
                "recursive",
                "looping",
            ] {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{name}: {stderr}"
                );
            }
        }
    }
}

#[test]
fn native_stack_joins_preserve_mixed_scalar_types() {
    use kujo::{
        bytecode::{BytecodeBindingKind, BytecodeChunk, Constant, OpCode},
        interpreter::Value,
        jit::{JitCompiler, VMContext},
        vm::VM,
    };
    use std::collections::HashMap;
    let mut chunk = BytecodeChunk::new();
    chunk.name = Some("merged".into());
    chunk.params = vec!["x".into()];
    chunk.local_names = chunk.params.clone();
    chunk.local_count = 1;
    chunk.local_binding_kinds = vec![BytecodeBindingKind::Mutable];
    chunk.constants = vec![Constant::Int(7), Constant::Bool(false)];
    for instruction in [
        OpCode::LoadLocal(0),
        OpCode::JumpIfFalse(5),
        OpCode::Pop,
        OpCode::LoadConst(0),
        OpCode::Jump(8),
        OpCode::Pop,
        OpCode::LoadConst(1),
        OpCode::Jump(8),
        OpCode::Return,
    ] {
        chunk.emit(instruction);
    }
    let function = Value::BytecodeFunction {
        chunk: chunk.clone(),
        captured: HashMap::new(),
        captured_binding_kinds: HashMap::new(),
    };
    for native in [false, true] {
        let mut vm = VM::new();
        vm.set_jit_enabled(native);
        if native {
            assert!(vm.jit_compile_bytecode_function(&function).unwrap());
        }
        for argument in [0, 1, 0, -1] {
            let actual =
                vm.call_function_from_jit(function.clone(), vec![Value::Int(argument)]).unwrap();
            if argument == 0 {
                assert!(matches!(actual, Value::Bool(false)), "{actual:?}");
            } else {
                assert!(matches!(actual, Value::Int(7)), "{actual:?}");
            }
        }
    }
    // Scripts have no call-frame arguments. Use an input constant there;
    // the loop entrypoint reads the same input from its VM local slot.
    for script in [false, true] {
        for argument in [0, 1, 0, -1] {
            let mut jit = JitCompiler::new().unwrap();
            let compiled = if script {
                let mut script_chunk = chunk.clone();
                script_chunk.params.clear();
                script_chunk.local_names.clear();
                script_chunk.local_count = 0;
                script_chunk.local_binding_kinds.clear();
                let input = script_chunk.add_constant(Constant::Int(argument));
                script_chunk.instructions[0] = OpCode::LoadConst(input);
                jit.compile_script(&script_chunk, "merged_script")
            } else {
                jit.compile(&chunk, 0)
            }
            .unwrap();
            let mut slots = vec![Value::Int(argument)];
            let mut stack = Vec::new();
            let mut locals = HashMap::new();
            let mut globals = HashMap::new();
            let mut ctx = VMContext::new(&mut stack, &mut locals, &mut globals);
            ctx.local_slots_ptr = &mut slots;
            // SAFETY: code and exclusively borrowed context storage remain live.
            assert_eq!(unsafe { compiled(&mut ctx) }, 0);
            assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
            assert!(ctx.has_return_value);
            let actual = ctx.scalar_value(ctx.return_value);
            if argument == 0 {
                assert!(matches!(actual, Value::Bool(false)), "script={script}: {actual:?}");
            } else {
                assert!(matches!(actual, Value::Int(7)), "script={script}: {actual:?}");
            }
        }
    }
    let mut jit = JitCompiler::new().unwrap();
    let compiled = jit.compile_function_with_direct_arg(&chunk, "merged").unwrap();
    let mut stack = Vec::new();
    let mut locals = HashMap::new();
    let mut globals = HashMap::new();
    let mut ctx = VMContext::new(&mut stack, &mut locals, &mut globals);
    for argument in [0, 1, 0, -1] {
        // SAFETY: JIT code and exclusively borrowed context storage remain live.
        let result = unsafe { compiled(&mut ctx, argument) };
        assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
        let actual = ctx.scalar_value(result);
        if argument == 0 {
            assert!(matches!(actual, Value::Bool(false)), "{actual:?}");
        } else {
            assert!(matches!(actual, Value::Int(7)), "{actual:?}");
        }
    }
}
