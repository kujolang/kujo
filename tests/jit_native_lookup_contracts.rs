use std::process::Command;

#[test]
fn hot_builtin_calls_and_user_shadowing_agree_across_runtimes() {
    let dir = tempfile::tempdir().unwrap();
    for (name, source) in [
        (
            "native",
            r#"
func mask(value) { return bit_and(value, 4294967295) }
func shift(value) { return bit_shl(value, 2) }
mut i := 0
while i < 200 {
    assert_equal(mask(12345), 12345)
    assert_equal(shift(7), 28)
    i += 1
}
print("PASS")
"#,
        ),
        (
            "shadow",
            r#"
func bit_and(a, b) { return a + 7 }
func mask(value) { return bit_and(value, 4294967295) }
mut i := 0
while i < 200 { assert_equal(mask(12345), 12352) i += 1 }
print("PASS")
"#,
        ),
    ] {
        let path = dir.path().join(format!("{name}.kujo"));
        std::fs::write(&path, source).unwrap();
        for mode in [None, Some("--interpreter"), Some("--jit")] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(&path).arg("--scheduler-no-timeout");
            if let Some(flag) = mode {
                command.arg(flag);
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{name} {mode:?}: {:?}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        }
    }
}
