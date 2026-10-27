//! Dictionary publication must retain ordinary recursion, callback and closure semantics.
use std::process::Command;

const STATE: &str = r#"mut data := {"k": 0, "other": 99}
export func recursive(n) {
    data["k"] += 1
    if n > 0 { return recursive(n - 1) }
    return data["k"]
}
func step(x) { data["k"] += x return data["k"] }
export func callback() {
    data["k"] = 10
    let results := map([1, 2], step)
    return [results, data]
}
export func capture() {
    data["k"] = 40
    let saved := data
    func read() { return saved }
    data["k"] = 41
    return read
}
export func snapshot() { return data }
"#;
const MAIN: &str = r#"from state import recursive, callback, capture, snapshot
let original := snapshot()
assert_equal(recursive(4), 5)
assert_equal(snapshot()["k"], 5)
assert_equal(recursive(4), 10)
assert_equal(snapshot()["k"], 10)
let result := callback()
assert_equal(result[0], [11, 13])
assert_equal(result[1]["k"], 13)
let reader := capture()
assert_equal(reader()["k"], 40)
assert_equal(snapshot()["k"], 41)
assert_equal(recursive(3), 45)
assert_equal(reader()["k"], 40)
assert_equal(original["k"], 0)
assert_equal(result[1]["k"], 13)
assert_equal(snapshot()["other"], 99)
print("DICTIONARY REENTRY PASS")
"#;

fn verify(mode: &str) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("state.kujo"), STATE).unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, MAIN).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(path).arg("--untrusted");
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"DICTIONARY REENTRY PASS\n", "{mode}");
}

#[test]
fn vm_dictionary_reentry() {
    verify("vm");
}
#[test]
fn interpreter_dictionary_reentry() {
    verify("interpreter");
}
#[test]
fn jit_dictionary_reentry() {
    verify("jit");
}
