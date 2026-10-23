use std::process::Command;

#[test]
fn native_predicates_preserve_boolean_return_types_and_call_arguments() {
    let source = r#"
        func positive(x) { return x > 0 }
        func literal(x) { if x > 0 { return true } return false }
        func mixed(x) { if x > 0 { return true } return 7 }
        func negate(x) { return !(x > 0) }
        func relay(x) { return positive(x) }
        func merged(x) { mut r := 0 if x > 0 { r = true } return r }
        func check(x) { assert_equal(positive(x), true) return 7 }
        mut i := 0
        while i < 200 {
            assert_equal(positive(7), true)
            assert_equal(positive(-7), false)
            assert_equal(literal(7), true)
            assert_equal(literal(-7), false)
            assert_equal(mixed(7), true)
            assert_equal(mixed(-7), 7)
            assert_equal(negate(0), true)
            assert_equal(negate(1), false)
            assert_equal(relay(7), true)
            assert_equal(merged(7), true)
            assert_equal(merged(-7), 0)
            assert_equal(check(7), 7)
            i += 1
        }
        print("PASS")
    "#;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bool.kujo");
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
            for name in ["positive", "literal", "mixed", "negate", "relay", "check", "merged"] {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
        }
    }
}

#[test]
fn direct_native_return_tags_survive_recursion_and_reset_between_calls() {
    use kujo::{
        compiler::Compiler,
        interpreter::Value,
        jit::{JitCompiler, VMContext},
        lexer,
        parser::Parser,
        vm::VM,
    };
    use std::{
        collections::HashMap,
        hash::{Hash, Hasher},
    };
    for (name, source, checks) in [
        ("mixed", "func mixed(x) { if x > 0 { return true } return 7 } return mixed", vec![(2, Value::Bool(true)), (-2, Value::Int(7)), (3, Value::Bool(true)), (-3, Value::Int(7))]),
        ("positive", "func positive(x) { return x > 0 } return positive", vec![(2, Value::Bool(true)), (-2, Value::Bool(false))]),
        ("short_recursive", "func short_recursive(x) { return x == 0 || short_recursive(x - 1) } return short_recursive", vec![(4, Value::Bool(true)), (0, Value::Bool(true))]),
        ("recursive", "func recursive(x) { if x > 0 { return recursive(x - 1) } return true } return recursive", vec![(4, Value::Bool(true)), (0, Value::Bool(true))]),
    ] {
        let mut parser = Parser::new(lexer::tokenize(source).unwrap());
        let chunk = Compiler::new().compile(&parser.parse()).unwrap();
        let mut vm = VM::new();
        let Value::BytecodeFunction { chunk, .. } = vm.execute(chunk).unwrap() else { panic!("function expected") };
        let mut jit = JitCompiler::new().unwrap();
        let compiled = jit.compile_function_with_direct_arg(&chunk, name).unwrap();
        let mut stack = Vec::new();
        let mut locals = HashMap::new();
        let mut globals = HashMap::new();
        let mut names = HashMap::new();
        for variable in ["x", name] {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            variable.hash(&mut hash);
            names.insert(hash.finish(), variable.to_owned());
        }
        let mut ctx = VMContext::new(&mut stack, &mut locals, &mut globals);
        ctx.var_names_ptr = &mut names;
        for (argument, expected) in checks {
            // SAFETY: code/context storage stay live with exclusive access during the call.
            let result = unsafe { compiled(&mut ctx, argument) };
            assert!(ctx.runtime_error.is_none(), "{name}: {:?}", ctx.runtime_error);
            assert_eq!(format!("{:?}", ctx.scalar_value(result)), format!("{expected:?}"), "{name}({argument})");
        }
    }
}
