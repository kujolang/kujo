use std::process::Command;

#[test]
fn late_native_default_override_updates_its_global_owner_after_loop_warmup() {
    use kujo::{
        compiler::Compiler,
        interpreter::{Interpreter, Value},
        lexer::tokenize,
        module::ModuleLoader,
        parser::Parser,
        vm::VM,
    };
    use std::sync::{Arc, Mutex};

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("loop_defaults.kujo"), "export func sentinel() { return 1 }")
        .unwrap();
    let mut loader = ModuleLoader::new();
    loader.add_search_path(dir.path());
    let module = loader.load_module("loop_defaults").unwrap();
    let Value::Function(_, _, Some(module_environment)) = &module.exports["sentinel"] else {
        panic!("missing module capture");
    };
    // The try/except prevents whole-function compilation. The assignment occurs
    // after the loop becomes hot, while the default still has no explicit entry.
    let source = r#"
        func delayed_override() {
            try { let marker := 1 } except error { throw(error) }
            mut i := 0
            while i < 2048 {
                if i == 1800 { abs = 42 }
                i += 1
            }
        }
        delayed_override()
    "#;
    for compact in [false, true] {
        for mode in ["vm", "interpreter", "jit"] {
            let environment = if compact {
                module_environment.lock().unwrap().clone()
            } else {
                Interpreter::new().env
            };
            let snapshot = environment.clone();
            let parsed = Parser::new(tokenize(source).unwrap()).parse_with_diagnostics();
            assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
            let result = if mode == "interpreter" {
                let mut interpreter = Interpreter::new();
                interpreter.env = environment;
                interpreter.eval_stmts(&parsed.stmts);
                assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
                interpreter.env.get("abs")
            } else {
                let globals = Arc::new(Mutex::new(environment));
                let mut vm = VM::new();
                vm.set_globals(Arc::clone(&globals));
                vm.set_jit_enabled(mode == "jit");
                vm.execute(Compiler::new().compile(&parsed.stmts).unwrap()).unwrap();
                let result = globals.lock().unwrap().get("abs");
                result
            };
            assert!(matches!(result, Some(Value::Int(42))), "{mode}, compact={compact}");
            assert!(matches!(snapshot.get("abs"), Some(Value::NativeFunction(_))));
        }
    }
}

#[test]
fn imported_native_defaults_keep_aliases_overrides_captures_and_recursion() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("defaults.kujo"),
        r#"
        mut state := [7]
        const retained := sqrt
        func helper(value) { return abs(value) }
        export func read(value) { return helper(value) + floor(1.9) }
        export func recurse(n) { if n == 0 { return abs(-7) } return recurse(n - 1) }
        export func alias() { println("ALIAS PASS") return retained(16) }
        export func change() { abs = func(value) { return 90 } return helper(-1) }
        export func factory() { return func(value) { return floor(value) } }
        export func reject() { retained = abs }
        export func edit(items) { items[0] = 9 return items }
        export func callback(fn) { return fn(state) }
        export func saved() { return state }
    "#,
    )
    .unwrap();
    let path = dir.path().join("main.kujo");
    std::fs::write(
        &path,
        r#"
        from defaults import read
        from defaults import recurse
        from defaults import alias
        from defaults import change
        from defaults import factory
        from defaults import reject
        from defaults import edit
        from defaults import callback
        from defaults import saved
        assert_equal(read(-4), 5)
        assert_equal(recurse(5), 7)
        assert_equal(alias(), 4)
        let closure := factory()
        assert_equal(closure(2.9), 2)
        let snapshot := saved()
        assert_equal(callback(edit)[0], 9)
        assert_equal(snapshot[0], 7)
        assert_equal(saved()[0], 7)
        mut caught := false
        try { reject() } except error { caught = true }
        assert(caught)
        assert_equal(change(), 90)
        assert_equal(read(-4), 91)
        assert_equal(abs(-4), 4)
        print("NATIVE DEFAULTS PASS")
    "#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kujo"));
        cmd.args(["run", "--untrusted"]);
        if mode != "vm" {
            cmd.arg(format!("--{mode}"));
        }
        let output = cmd.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("NATIVE DEFAULTS PASS"));
    }
}
