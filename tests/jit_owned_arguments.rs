use std::process::Command;

#[test]
fn native_owned_arguments_survive_imported_helper_callbacks() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owned_helper.kujo"),
        r#"
        export func transform(value) {
            mut copy := value
            copy[0][0] = 9
            return copy
        }
        export func recurse(value, n) {
            if n == 0 { return transform(value) }
            return recurse(value, n - 1)
        }
        "#,
    )
    .unwrap();
    let mut source = String::from(
        r#"
        from owned_helper import recurse
        func relay(value) { return recurse(value, 3) }
        let original := [[1, 2], [3, 4]]
        "#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(relay(original), [[9,2],[3,4]])\n");
    }
    source.push_str(
        r#"
        mut caught := 0
        try { relay([]) } except error { caught += 1 }
        assert_equal(caught, 1)
        assert_equal(original, [[1,2],[3,4]])
        assert_equal(relay(original), [[9,2],[3,4]])
        print("PASS")
        "#,
    );
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let metrics = directory.path().join("import-metrics.json");
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1").arg("--measurements").arg(&metrics);
        }
        let output = command.arg(&path).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            assert!(stderr.contains("Successfully compiled function 'relay'"), "{stderr}");
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 100);
        }
    }
}

#[test]
fn native_owned_arguments_preserve_types_aliases_and_error_recovery() {
    let mut source = String::from(
        r#"
        let seed := [1,2]
        func identity(x) { return x }
        func four(a,b,c,d) { if c { return a } return d }
        func five(a,b,c,d,e) { return e }
        func looping(x) { mut result := x mut i := 0 while i < 3 { i += 1 } return result }
        func mutate(x) { mut local := x local[0] = 7 return local }
        func saved(x) { mut local := x let snapshot := local local[0] = 7 return snapshot }
        func recursive(x,n) { if n == 0 { return x } return recursive(x,n-1) }
        func direct(x) { if x == 0 { return direct(seed) } return x }
        func callback(x) { try { return x } except error { return null } }
        func relay(x) { return callback(x) }
        func indexed(x,i) { return x[i] }
        func apply(f,x) { return f(x) }
        func increment(x) { return x + 1 }
        func capture(offset) { return func(x) { return offset + x } }
        let captured := capture(5)
        "#,
    );
    // Exercise native admission from owned calls, not only a scalar-warmed cache.
    for _ in 0..200 {
        source.push_str(
            "assert_equal(apply(increment, 7), 8)\nassert_equal(apply(captured, 7), 12)\n",
        );
        source.push_str("assert_equal(identity(seed), seed)\nassert_equal(four(seed, null, true, 0), seed)\nassert_equal(five(0,1,2,3,seed),seed)\nassert_equal(looping(seed),seed)\nassert_equal(mutate(seed),[7,2])\nassert_equal(saved(seed),seed)\nassert_equal(recursive(seed,3),seed)\nassert_equal(direct(0),seed)\nassert_equal(relay(seed),seed)\nassert_equal(indexed(seed,1),2)\n");
    }
    source.push_str(
        r#"
        for value in [7, true, null, 2.5, "hello", [3,4], {"key":5}, bytes([0,255])] {
            assert_equal(identity(value), value)
            assert_equal(four(value, null, true, 0), value)
            assert_equal(five(0,1,2,3,value), value)
            assert_equal(looping(value), value)
            assert_equal(recursive(value,3), value)
            assert_equal(relay(value), value)
        }
        mut caught := 0
        try { indexed(seed,99) } except error { caught += 1 }
        assert_equal(caught,1)
        assert_equal(indexed(seed,0),1)
        assert_equal(seed,[1,2])
        print("PASS")
        "#,
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("owned_arguments.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let metrics = directory.path().join("metrics.json");
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1").arg("--measurements").arg(&metrics);
        }
        let output = command.arg(&path).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            for name in [
                "identity",
                "four",
                "five",
                "looping",
                "mutate",
                "saved",
                "recursive",
                "direct",
                "relay",
                "indexed",
                "apply",
            ] {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 900);
        }
    }
}
