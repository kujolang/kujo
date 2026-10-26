use std::process::Command;

#[test]
fn reentrant_index_reads_current_capture_without_changing_saved_snapshots() {
    let cases = [
        ("array", "[1,2,3]", "[9,9,9]", "0", "[42,9,9]", false),
        (
            "dictionary",
            "{\"a\":1,\"b\":2}",
            "{\"a\":9,\"b\":9}",
            "\"a\"",
            "{\"a\":42,\"b\":9}",
            false,
        ),
        ("bytes", "bytes([1,2])", "bytes([9,9])", "0", "bytes([42,9])", false),
        ("throwing_index", "[1,2,3]", "[9]", "0", "[9]", true),
    ];
    let directory = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for (name, initial, replacement, index, expected, throws) in cases {
        let inner =
            if throws { "throw(\"index failed\")".to_owned() } else { format!("return {index}") };
        let write = if throws {
            "try { data[step(0)] = 42 } except error {}"
        } else {
            "data[step(0)] = 42"
        };
        let source = format!(
            r#"
func factory() {{
    mut data := {initial}
    func step(n) {{
        if n < 0 {{ return data }}
        if n == 0 {{ data = {replacement} {inner} }}
        {write}
        return data
    }}
    return step
}}
let step := factory()
let saved := step(-1)
assert_equal(step(1),{expected})
assert_equal(step(1),{expected})
assert_equal(saved,{initial})
print("CAPTURE ROOT PASS")
"#
        );
        let path = directory.path().join(format!("{name}.kujo"));
        std::fs::write(&path, source).unwrap();
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.args(["run", "--untrusted"]);
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command.arg(&path).output().unwrap();
            if !output.status.success()
                || !String::from_utf8_lossy(&output.stdout).contains("CAPTURE ROOT PASS")
            {
                failures
                    .push(format!("{name}/{mode}: {}", String::from_utf8_lossy(&output.stderr)));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
