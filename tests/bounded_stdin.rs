use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn bounded_stdin_has_matching_vm_and_interpreter_process_contracts() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("stdin.kujo");
    for (code, input, expected, succeeds) in [
        ("print(to_json(read_stdin(4)))", &b" a\n"[..], "\" a\\n\"", true),
        ("print(to_json(read_stdin(4)))", &b""[..], "\"\"", true),
        ("print(read_stdin(3))", &b"abcd"[..], "exceeds byte limit", false),
        ("print(read_stdin(3))", &[0xff][..], "not valid UTF-8", false),
        ("print(read_stdin(0))", &b""[..], "byte limit", false),
        ("print(read_stdin(8388609))", &b""[..], "byte limit", false),
        ("print(read_stdin(\"3\"))", &b""[..], "byte limit", false),
        ("print(read_stdin())", &b""[..], "expects", false),
    ] {
        std::fs::write(&script, code).unwrap();
        for interpreter in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(&script).arg("--untrusted");
            if interpreter {
                command.arg("--interpreter");
            }
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(input).unwrap();
            let result = child.wait_with_output().unwrap();
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.status.success(), succeeds, "interpreter={interpreter}: {combined}");
            assert!(combined.contains(expected), "interpreter={interpreter}: {combined}");
        }
    }
}
