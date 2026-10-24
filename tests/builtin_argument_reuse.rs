use std::process::Command;

#[test]
fn builtin_callbacks_preserve_snapshots_and_recover_after_errors() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("builtin_reuse.kujo");
    std::fs::write(
        &file,
        r#"
        func convert(n) { return to_int(n / 2) }
        func forward(input) { return input }
        func nested_array(input) { return forward(forward(input)) }
        func bad_call(n) { return dict(n) }
        func nested(n) { return convert(n) + len([n, n]) }
        let snapshot := [4, 8, 12]
        mut failures := 0
        mut i := 0
        while i < 180 {
            assert_equal(map(snapshot, nested), [4, 6, 8])
            assert_equal(snapshot, [4, 8, 12])
            assert_equal(convert(14), 7)
            mut changed := nested_array(snapshot)
            changed[0] = i
            assert_equal(changed, [i, 8, 12])
            assert_equal(snapshot, [4, 8, 12])
            try { bad_call(i) } except error { failures += 1 }
            assert_equal(convert(14), 7)
            i += 1
        }
        assert_equal(failures, 180)
        mut caught := 0
        try { dict(1) } except error { caught += 1 }
        assert_equal(caught, 1)
        assert_equal(map(snapshot, nested), [4, 6, 8])
        assert_equal(snapshot, [4, 8, 12])
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
            for name in ["convert", "nested_array", "bad_call"] {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
        }
    }
}
