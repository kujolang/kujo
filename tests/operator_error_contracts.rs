use std::process::Command;

#[test]
fn assigned_operator_errors_preserve_target_effects_and_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operators.kujo");
    std::fs::write(
        &path,
        r#"
        mut calls := 0
        mut after := 0
        struct Operand {
            value: int,
            func op_add(other) {
                calls += 1
                assert(value > 0, "binary operator failed")
                after += 1
                return value + other.value
            },
            func op_neg() {
                calls += 1
                assert(value > 0, "unary operator failed")
                after += 1
                return -value
            }
        }
        let bad := Operand { value: 0 }
        let good := Operand { value: 3 }
        mut caught := 0
        mut result := 17
        mut i := 0
        while i < 200 {
            try { result = bad + good } except error { caught += 1 }
            assert_equal(result, 17)
            try { result = -bad } except error { caught += 1 }
            assert_equal(result, 17)
            assert_equal(good + good, 6)
            assert_equal(-good, -3)
            i += 1
        }
        assert_equal(caught, 400)
        assert_equal(calls, 800)
        assert_equal(after, 400)
        assert_equal(bad.value, 0)
        assert_equal(good.value, 3)
        print("PASS")
        "#,
    )
    .unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let output = command.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    }
}
