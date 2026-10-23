use std::process::Command;

#[test]
fn duplicate_literal_keys_keep_the_last_value_and_evaluate_every_expression() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("duplicate_keys.kujo");
    std::fs::write(
        &file,
        r#"
        mut calls := 0
        func next_value() { calls += 1 return calls }
        let result := {"a":next_value(), "b":next_value(), "a":next_value()}
        assert_equal(calls, 3)
        assert_equal(len(result), 2)
        assert_equal(result["a"], 3)
        assert_equal(result["b"], 2)
        assert_equal(result, {"a":3,"b":2})
        func build(x) { return {"a":1,"a":x} }
        mut n := 0
        while n < 200 { assert_equal(build(n), {"a":n}) n += 1 }
        print("PASS")
    "#,
    )
    .unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(flag) = mode {
            command.arg(flag);
        }
        let output = command.arg(&file).output().unwrap();
        assert!(output.status.success(), "{mode:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS", "{mode:?}");
    }
}
