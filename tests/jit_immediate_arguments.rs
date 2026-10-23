use std::process::Command;

fn check(source: &str, native_functions: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("immediate_arguments.kujo");
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
            if native_functions.contains(&"walk") {
                assert!(
                    stderr.contains("Compiled 'walk' with direct-arg optimization"),
                    "{stderr}"
                );
                assert!(stderr.contains("Interpreter direct-arg call to 'walk'"), "{stderr}");
            }
            for name in native_functions {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
        }
    }
}

#[test]
fn hot_immediate_arguments_compile_and_preserve_types_across_arity_and_loops() {
    check(
        r#"
        func one(x) { return x }
        func four(a,b,c,d) { if c { return a + b } return d }
        func five(a,b,c,d,e) { if d { return a + b + c } return e }
        func copied_loop(x) { mut result := x mut n := 0 while n < 3 { result += 0.5 n += 1 } return result }
        func looping(x) { mut result := 0.0 mut n := 0 while n < 3 { result += x n += 1 } return result }
        func relay(x) { return one(x) }
        func walk(x) { if x < 1 { return x } return walk(x - 0.5) }
        func branch(x) { if x < 1 { return x } let result := branch(x - 0.5) return result + x }
        mut i := 0
        while i < 200 {
            assert_equal(one(2.5), 2.5)
            assert_equal(relay(3.5), 3.5)
            assert_equal(four(1.5,2.5,true,null),4.0)
            assert_equal(five(1.5,2.5,3.5,true,null),7.5)
            assert_equal(copied_loop(2.5),4.0)
            assert_equal(looping(2.5),7.5)
            assert_equal(walk(3),0.5)
            assert_equal(walk(2.5),0.5)
            assert_equal(branch(3),10.5)
            i += 1
        }
        for value in [true,false,null,-0.0,2.5,7,"hello",[1,2],{"x":3}] {
            assert_equal(one(value),value)
            assert_equal(relay(value),value)
        }
        assert_equal(four(1,2,false,null),null)
        assert_equal(five(1,2,3,false,null),null)
        assert_equal(looping(2),6.0)
        assert_equal(copied_loop(2),3.5)
        print("PASS")
    "#,
        &["one", "four", "five", "copied_loop", "looping", "relay", "walk", "branch"],
    );
}

#[test]
fn native_immediate_argument_errors_remain_catchable_and_reusable() {
    check(
        r#"
        func divide(a,b) { return a / b }
        func choose(x) { return !x }
        mut i := 0
        while i < 200 {
            assert_equal(divide(7.5,2.5),3.0)
            assert_equal(choose(true),false)
            i += 1
        }
        mut caught := 0
        try { divide(1.5,-0.0) } except error { caught += 1 }
        try { divide(true,1.5) } except error { caught += 1 }
        try { divide(null,1) } except error { caught += 1 }
        try { choose(2.5) } except error { caught += 1 }
        assert_equal(caught,4)
        assert_equal(divide(7,2),3)
        assert_equal(choose(false),true)
        print("PASS")
    "#,
        &["divide", "choose"],
    );
}

#[test]
fn native_recursion_obeys_the_reference_depth_limit() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("depth.kujo");
    for argument in ["400", "400.5"] {
        let source = format!("func deep(x) {{ if x <= 0 {{ return 0 }} return deep(x - 1) }}\nprint(deep({argument}))\n");
        std::fs::write(&file, source).unwrap();
        for mode in [None, Some("--interpreter"), Some("--jit")] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.args(["run", "--scheduler-no-timeout"]).arg(&file);
            if let Some(flag) = mode {
                command.arg(flag);
            }
            let output = command.output().unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(4), "{argument} {mode:?}: {stderr}");
            assert!(stderr.contains("depth") && stderr.contains("256"), "{stderr}");
        }
    }
}

#[test]
fn native_depth_slots_are_released_after_recursive_errors_and_returns() {
    check(
        r#"
        func deep(x) { if x <= 0 { return 0 } return deep(x - 1) }
        func relay(x) { return deep(x) }
        mut i := 0
        while i < 200 { assert_equal(deep(1.5),0) assert_equal(relay(1.5),0) i += 1 }
        mut caught := 0
        i = 0
        while i < 300 {
            try { relay(400.5) } except error { caught += 1 }
            assert_equal(deep(1.5),0)
            i += 1
        }
        assert_equal(caught,300)
        print("PASS")
    "#,
        &["deep", "relay"],
    );
}

#[test]
fn native_callback_recursion_shares_the_depth_budget() {
    check(
        r#"
        mut recoveries := 0
        func recover(x) { try { return 1 / x } except error { recoveries += 1 } return 0 }
        func left(x) { if x <= 0 { return 0 } recover(0) return right(x - 1) }
        func right(x) { if x <= 0 { return 0 } return left(x - 1) }
        mut i := 0
        while i < 200 { assert_equal(left(1.5),0) assert_equal(right(1.5),0) i += 1 }
        mut caught := 0
        try { left(400.5) } except error { caught += 1 }
        assert_equal(caught,1)
        assert_equal(right(1.5),0)
        assert(recoveries > 0)
        print("PASS")
    "#,
        &["left", "right"],
    );
}

#[test]
fn compiled_argument_bridge_preserves_immediate_payload_bits() {
    use kujo::{compiler::Compiler, interpreter::Value, lexer, parser::Parser, vm::VM};
    let mut parser =
        Parser::new(lexer::tokenize("func identity(x) { return x } return identity").unwrap());
    let chunk = Compiler::new().compile(&parser.parse()).unwrap();
    let mut vm = VM::new();
    vm.set_jit_enabled(true);
    let function = vm.execute(chunk).unwrap();
    assert!(vm.jit_compile_bytecode_function(&function).unwrap());
    for value in [
        Value::Int(i64::MIN),
        Value::Int(i64::MAX),
        Value::Bool(true),
        Value::Bool(false),
        Value::Null,
        Value::Float(0.0),
        Value::Float(-0.0),
        Value::Float(f64::INFINITY),
        Value::Float(f64::NEG_INFINITY),
        Value::Float(f64::from_bits(0x7ff8_0000_0000_0042)),
    ] {
        let result = vm.call_function_from_jit(function.clone(), vec![value.clone()]).unwrap();
        match (&value, &result) {
            (Value::Float(expected), Value::Float(actual)) => {
                assert_eq!(actual.to_bits(), expected.to_bits())
            }
            _ => {
                assert_eq!(std::mem::discriminant(&result), std::mem::discriminant(&value));
                assert!(Value::equals(&result, &value));
            }
        }
    }
}

#[test]
fn copied_parameter_native_calls_preserve_caller_slots_and_snapshots() {
    check(
        r#"
        func copy_number(x) { mut result := x return result + 1 }
        func caller(values) {
            let snapshot := values
            let result := copy_number(4)
            assert_equal(values, [7,8])
            assert_equal(snapshot, [7,8])
            return result + values[0]
        }
        mut i := 0
        while i < 200 {
            assert_equal(copy_number(4), 5)
            assert_equal(caller([7,8]), 12)
            i += 1
        }
        print("PASS")
        "#,
        &["copy_number"],
    );
}

#[test]
fn direct_recursion_releases_each_copied_parameter_binding() {
    check(
        r#"
        func walk(x) {
            mut remaining := x
            if remaining < 1 { return remaining }
            return walk(remaining - 0.5)
        }
        mut i := 0
        while i < 200 {
            assert_equal(walk(3), 0.5)
            assert_equal(walk(2.5), 0.5)
            i += 1
        }
        print("PASS")
        "#,
        &["walk"],
    );
}
