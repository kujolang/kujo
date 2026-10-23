use std::process::Command;

#[test]
fn hot_non_integer_constants_preserve_values_across_runtimes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("constants.kujo");
    std::fs::write(
        &path,
        r#"
func half(value) { return value / 2.0 }
func greeting() { return "hello" }
mut rng := 42
func random_int(maximum) {
    rng = (rng * 3877 + 29573) % 139968
    return to_int(to_float(rng) / 139968.0 * maximum)
}
mut total := 0
mut i := 0
while i < 1000 {
    assert_equal(half(7), 3.5)
    assert_equal(greeting(), "hello")
    total += random_int(1000000)
    i += 1
}
assert_equal(total, 516069931)
print("PASS")
"#,
    )
    .unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&path).arg("--scheduler-no-timeout");
        if let Some(flag) = mode {
            command.arg(flag);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "mode={mode:?}, status={:?}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    }
}
