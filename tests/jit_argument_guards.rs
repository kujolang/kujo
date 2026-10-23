#[path = "support/native_api.rs"]
mod support;
use std::process::Command;

#[test]
fn compiled_callback_bridge_guards_arguments_without_discarding_integer_code() {
    use kujo::{compiler::Compiler, interpreter::Value, lexer, parser::Parser, vm::VM};
    let mut parser = Parser::new(
        lexer::tokenize("func plus_one(value) { return value + 1 } return plus_one").unwrap(),
    );
    let chunk = Compiler::new().compile(&parser.parse()).unwrap();
    let mut vm = VM::new();
    vm.set_jit_enabled(true);
    let function = vm.execute(chunk).unwrap();
    assert!(vm.jit_compile_bytecode_function(&function).unwrap());
    for _ in 0..3 {
        assert!(matches!(
            vm.call_function_from_jit(function.clone(), vec![Value::Float(2.5)]).unwrap(),
            Value::Float(value) if value == 3.5
        ));
        assert!(matches!(
            vm.call_function_from_jit(function.clone(), vec![Value::Int(2)]).unwrap(),
            Value::Int(3)
        ));
    }
}

#[test]
fn integer_compiled_calls_preserve_other_argument_types_and_reuse_integer_code() {
    let source = r#"
        func identity(value) { return value }
        func plus_one(value) { return value + 1 }
        func sum_five(a, b, c, d, e) { return a + b + c + d + e }
        mut i := 0
        while i < 200 {
            assert_equal(identity(i), i)
            assert_equal(plus_one(i), i + 1)
            assert_equal(sum_five(1, 2, 3, 4, 5), 15)
            i += 1
        }
        for value in [2.5, true, false, null, "hello", [1, 2], {"x": 3}, bytes([0, 255])] {
            assert_equal(identity(value), value)
        }
        assert_equal(plus_one(2.5), 3.5)
        assert_equal(sum_five(1, 2, 3, 4, 5.5), 15.5)
        i = 0
        while i < 200 {
            assert_equal(plus_one(2.5), 3.5)
            assert_equal(identity(i), i)
            assert_equal(plus_one(i), i + 1)
            i += 1
        }
        print("PASS")
    "#;
    support::success(source);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("arguments.kujo");
    std::fs::write(&file, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["run", "--jit", "--scheduler-no-timeout"])
        .arg(&file)
        .env("DEBUG_JIT", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    assert!(stderr.contains("Successfully compiled function 'plus_one'"), "{stderr}");
}

#[test]
fn float_only_hot_calls_keep_float_values() {
    let source = r#"
        func plus_one(value) { return value + 1 }
        mut i := 0
        while i < 200 { assert_equal(plus_one(2.5), 3.5) i += 1 }
    "#;
    support::success(source);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("floats.kujo");
    std::fs::write(&file, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["run", "--jit", "--scheduler-no-timeout"])
        .arg(file)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
