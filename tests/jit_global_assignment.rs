use std::process::Command;

#[test]
fn hot_named_assignments_preserve_globals_and_local_shadowing() {
    // Keep the shadow function's argument live past local initialization so this
    // binding test compiles without the separately unsupported ReleaseDeadLocal.
    run(
        r#"
        mut count := 0
        func bump() { count += 1 return count }
        func set_count(x) { count = x return count }
        func shadow(x) { mut count := x count = count + 2 return count + x - x }
        mut i := 0
        while i < 200 {
            assert_equal(bump(), i + 1)
            assert_equal(shadow(i), i + 2)
            i += 1
        }
        assert_equal(count, 200)
        i = 0
        while i < 200 { assert_equal(set_count(i), i) i += 1 }
        assert_equal(count, 199)
        count = 2.5
        assert_equal(set_count(17), 17)
        assert_equal(count, 17)
        print("PASS")
    "#,
        "bump",
    );
}

#[test]
fn compiled_readonly_global_write_is_atomic_and_catchable() {
    run(
        r#"
        const fixed := 10
        mut before := 0
        mut after := 0
        func attempt(x, y) {
            if x > 0 {
                before += 1
                fixed = y
                after += 1
            }
            return 7
        }
        mut i := 0
        while i < 200 { assert_equal(attempt(0, 2), 7) i += 1 }
        mut caught := 0
        i = 0
        while i < 3 {
            try { attempt(1, 2) } except error { caught += 1 }
            assert_equal(attempt(0, 2), 7)
            i += 1
        }
        assert_equal(caught, 3)
        assert_equal(fixed, 10)
        assert_equal(before, 3)
        assert_equal(after, 0)
        print("PASS")
    "#,
        "attempt",
    );
}

fn run(source: &str, compiled_name: &str) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("globals.kujo");
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
        assert!(output.status.success(), "mode={mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            if compiled_name == "bump" {
                assert!(stderr.contains("Successfully compiled function 'shadow'"), "{stderr}");
                assert!(stderr.contains("Successfully compiled function 'set_count'"), "{stderr}");
            }
            if compiled_name == "recurse" {
                assert!(stderr.contains("with direct-arg optimization for recursion"), "{stderr}");
                assert!(stderr.contains("direct-arg call to 'recurse'"), "{stderr}");
            }

            assert!(
                stderr.contains(&format!("Successfully compiled function '{compiled_name}'")),
                "{stderr}"
            );
        }
    }
}

#[test]
fn direct_native_assignment_reads_the_argument_and_updates_the_global() {
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
    let mut parser = Parser::new(
        lexer::tokenize("func set_count(x) { count = x return count } return set_count").unwrap(),
    );
    let chunk = Compiler::new().compile(&parser.parse()).unwrap();
    let mut vm = VM::new();
    let Value::BytecodeFunction { chunk, .. } = vm.execute(chunk).unwrap() else {
        panic!("function expected")
    };
    let mut jit = JitCompiler::new().unwrap();
    let compiled = jit.compile_function_with_direct_arg(&chunk, "set_count").unwrap();
    let mut globals = HashMap::from([("count".to_string(), Value::Int(1))]);
    let mut locals = HashMap::new();
    let mut stack = Vec::new();
    let mut names = HashMap::new();
    for name in ["x", "count"] {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hash);
        names.insert(hash.finish(), name.to_string());
    }
    let mut ctx = VMContext::new(&mut stack, &mut locals, &mut globals);
    ctx.var_names_ptr = &mut names;
    for value in [17, 42, -5] {
        // SAFETY: the compiler and all context-owned structures remain live, and
        // this call has exclusive access to the context and its pointed-to maps.
        let result = unsafe { compiled(&mut ctx, value) };
        assert!(ctx.runtime_error.is_none());
        assert_eq!(result, value);
        assert!(matches!(globals.get("count"), Some(Value::Int(actual)) if *actual == value));
        assert!(!locals.contains_key("count"));
    }
}

#[test]
fn recursive_native_assignment_failure_stops_outer_side_effects() {
    run(
        r#"
const fixed := 10
mut after := 0
func recurse(x) {
    if x < 0 { return 7 }
    if x > 0 { recurse(x - 1) after += 1 return 7 }
    fixed = 20
    return 7
}
mut i := 0
while i < 200 { assert_equal(recurse(-1), 7) i += 1 }
mut caught := 0
i = 0
while i < 3 {
    try { recurse(3) } except error { caught += 1 }
    i += 1
}
assert_equal(caught, 3)
assert_equal(after, 0)
assert_equal(fixed, 10)
print("PASS")

"#,
        "recurse",
    );
}

#[test]
fn native_constructed_global_assignment_updates_environment_and_enforces_readonly() {
    run(
        r#"
        mut state := {"rows": [[1,2]]}
        const fixed := {"rows": [[1,2]]}
        mut effects := 0
        func replace(value) { state = {"rows": [[value,8]]} return 9 }
        func locked(value) {
            if value > 0 { effects += 1 fixed = {"rows": [[value,8]]} effects += 100 }
            return 9
        }
        mut i := 0
        while i < 200 {
            assert_equal(replace(i),9)
            assert_equal(state,{"rows": [[i,8]]})
            assert_equal(locked(0),9)
            i += 1
        }
        mut caught := 0
        try { locked(7) } except error { caught += 1 }
        assert_equal(caught,1)
        assert_equal(effects,1)
        assert_equal(fixed,{"rows": [[1,2]]})
        assert_equal(replace(42),9)
        assert_equal(state,{"rows": [[42,8]]})
        print("PASS")
        "#,
        "replace",
    );
}

#[test]
fn native_constructed_readonly_global_assignment_is_atomic_and_catchable() {
    run(
        r#"
        const fixed := {"rows": [[1,2]]}
        mut effects := 0
        func locked(value) {
            if value > 0 { effects += 1 fixed = {"rows": [[value,8]]} effects += 100 }
            return 9
        }
        mut i := 0
        while i < 200 { assert_equal(locked(0),9) i += 1 }
        mut caught := 0
        try { locked(7) } except error { caught += 1 }
        assert_equal(caught,1)
        assert_equal(effects,1)
        assert_equal(fixed,{"rows": [[1,2]]})
        assert_equal(locked(0),9)
        print("PASS")
        "#,
        "locked",
    );
}
