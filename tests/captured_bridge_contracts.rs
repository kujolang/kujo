use std::process::Command;

#[test]
fn imported_closure_reentry_through_bytecode_relay_keeps_current_capture() {
    let dir = tempfile::tempdir().unwrap();
    let module = r#"
export func factory(relay, kind) {
    mut data := [1,2]
    func step(n) {
        if n < 0 { return data }
        if n == 0 {
            if kind == "read" { return data[0] }
            data[0] = 9
            if kind == "error" { throw("inner failure") }
            return 0
        }
        if kind == "read" {
            data[0] = 9
            return [relay(step), data[0]]
        }
        try { relay(step) } except error {
            assert_equal(kind,"error")
        }
        return data
    }
    return step
}
"#;
    std::fs::write(dir.path().join("bridge_factory.kujo"), module).unwrap();
    let mut failures = Vec::new();
    for kind in ["write", "error", "read"] {
        let expected = if kind == "read" { "[9, 9]" } else { "[9, 2]" };
        let source = format!(
            r#"
from bridge_factory import factory
func relay(fn) {{ return fn(0) }}
let step := factory(relay,"{kind}")
let independent := factory(relay,"{kind}")
let saved := step(-1)
assert_equal(step(1),{expected})
assert_equal(step(1),{expected})
assert_equal(saved,[1,2])
assert_equal(independent(-1),[1,2])
print("BRIDGE PASS")
"#
        );
        let path = dir.path().join(format!("bridge_{kind}.kujo"));
        std::fs::write(&path, source).unwrap();
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run");
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command.arg(&path).output().unwrap();
            if !output.status.success()
                || !String::from_utf8_lossy(&output.stdout).contains("BRIDGE PASS")
            {
                failures.push(format!(
                    "{kind}/{mode}: stdout={}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
