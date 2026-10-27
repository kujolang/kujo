//! Alternating captured namespaces retain live state through returns and errors.
use std::process::Command;

const STATE: &str = r#"
from bridge import invoke
mut data := [1, 2]
mut fail := false
export func read() { return data }
export func set_fail(value) { fail = value }
export func step(n) {
    let stamp := n * 13 + 7
    let old := data
    let previous := data[0]
    data[0] += 1
    if n == 0 {
        if fail { throw("leaf failure") }
        return data
    }
    let result := invoke(step, n - 1)
    assert_equal(stamp, n * 13 + 7)
    assert_equal(old[0], previous)
    assert_equal(data[0], previous + n + 1)
    return result
}
"#;

const BRIDGE: &str = r#"
mut data := [100]
mut native := false
export func set_native(value) { native = value }
export func read_bridge() { return data }
export func invoke(f, n) {
    let old := data
    let previous := data[0]
    data[0] += 1
    mut result := []
    if native { let values := map([n], f) result = values[0] }
    else { result = f(n) }
    assert_equal(old[0], previous)
    data[0] += 10
    return result
}
"#;

const MAIN: &str = r#"
from state import step, read, set_fail
from bridge import read_bridge, set_native
for native in [false, true] {
    set_native(native)
    let old := read()
    let previous := old[0]
    let bridge_old := read_bridge()
    let saved := step(4)
    assert_equal(saved[0], previous + 5)
    assert_equal(read()[0], previous + 5)
    assert_equal(old[0], previous)
    assert_equal(read_bridge()[0], bridge_old[0] + 44)
    set_fail(true)
    mut caught := false
    try { step(4) } except e { caught = true }
    assert_equal(caught, true)
    assert_equal(read()[0], previous + 10)
    assert_equal(read_bridge()[0], bridge_old[0] + 48)
    assert_equal(saved[0], previous + 5)
    set_fail(false)
    assert_equal(step(0)[0], previous + 11)
}
print("FOREIGN CAPTURE PASS")
"#;

fn verify(runtime: &str) {
    let directory = tempfile::tempdir().unwrap();
    for (name, source) in [("state.kujo", STATE), ("bridge.kujo", BRIDGE), ("main.kujo", MAIN)] {
        std::fs::write(directory.path().join(name), source).unwrap();
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(directory.path().join("main.kujo")).arg("--untrusted");
    if runtime != "vm" {
        command.arg(format!("--{runtime}"));
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{runtime}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"FOREIGN CAPTURE PASS\n");
}

#[test]
fn vm_foreign_capture_reentry_preserves_live_state_and_errors() {
    verify("vm");
}

#[test]
fn interpreter_foreign_capture_reentry_preserves_live_state_and_errors() {
    verify("interpreter");
}

#[test]
fn jit_foreign_capture_reentry_preserves_live_state_and_errors() {
    verify("jit");
}
