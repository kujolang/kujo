use std::process::Command;

#[test]
fn native_loop_rebinding_persists_owned_values_without_changing_snapshots() {
    for (initial, replacement, expected) in [
        ("[-1,false]", "[i,true]", "[199,true]"),
        (
            "{\"number\":-1,\"flag\":false}",
            "{\"number\":i,\"flag\":true}",
            "{\"number\":199,\"flag\":true}",
        ),
    ] {
        for fail in [false, true] {
            let failure = if fail { "if i == 150 { assert_equal(10/(i-150),0) }" } else { "" };
            let expected = if fail { expected.replace("199", "150") } else { expected.to_owned() };
            let caught = i32::from(fail);
            let source = format!(
                r#"
func once() {{
    mut i := 0
    mut last := {initial}
    let snapshot := last
    mut caught := 0
    try {{
        while i < 200 {{ last = {replacement} {failure} i += 1 }}
    }} except error {{ caught += 1 }}
    return [last,snapshot,caught]
}}
let result := once()
assert_equal(result[0],{expected})
assert_equal(result[1],{initial})
assert_equal(result[2],{caught})
print("PASS")
"#
            );
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("rebind.kujo");
            std::fs::write(&path, source).unwrap();
            for mode in [Some("--interpreter"), None, Some("--jit")] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
                command.args(["run", "--scheduler-no-timeout"]);
                if let Some(mode) = mode {
                    command.arg(mode);
                }
                command.env("DISABLE_FUNCTION_JIT", "1").env("DEBUG_JIT_LOOP", "1");
                let output = command.arg(&path).output().unwrap();
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(output.status.success(), "{mode:?} {replacement}, fail={fail}: {stderr}");
                assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
                if mode == Some("--jit") {
                    assert!(stderr.contains("Executing compiled loop"), "{stderr}");
                }
            }
        }
    }
}
