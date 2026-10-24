use std::process::Command;

#[test]
fn discarded_function_locals_preserve_returns_effects_and_caller_state() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("local_returns.kujo");
    std::fs::write(
        &file,
        r#"
        mut effect := 0
        func scalar(a, b) { mut n := a + b; n += 3; return n }
        func update(input, n) {
            mut result := input
            result[0] = n
            mut scratch := n + 1
            scratch += 2
            return result
        }
        func recursive(n, acc) {
            if n == 0 { return acc }
            mut next := acc + n
            return recursive(n - 1, next)
        }
        func affect(n) { effect = n; mut scratch := n + 1; return scratch }
        func fail_after(n, values) {
            effect = n
            mut local := n + 1
            return values[local]
        }
        func nothing(n) { mut scratch := n + 1; effect = scratch }
        let snapshot := [7, 8]
        mut i := 0
        while i < 180 {
            assert_equal(scalar(i, 5), i + 8)
            assert_equal(update(snapshot, i), [i, 8])
            assert_equal(snapshot, [7, 8])
            assert_equal(recursive(8, i), i + 36)
            assert_equal(affect(i), i + 1)
            assert_equal(effect, i)
            nothing(i)
            assert_equal(effect, i + 1)
            i += 1
        }
        mut caught := 0
        mut destination := [19]
        try { destination = fail_after(37, []) } except error { caught += 1 }
        assert_equal(caught, 1)
        assert_equal(effect, 37)
        assert_equal(destination, [19])
        assert_equal(scalar(1, 2), 6)
        assert_equal(update(snapshot, 11), [11, 8])
        assert_equal(snapshot, [7, 8])
        print("PASS")
        "#,
    )
    .unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1");
        }
        let output = command.arg(&file).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            assert!(stderr.contains("Successfully compiled function 'scalar'"), "{stderr}");
            assert!(stderr.contains("Successfully compiled function 'update'"), "{stderr}");
        }
    }
}
