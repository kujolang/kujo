use std::process::Command;

#[test]
fn scalar_first_batches_preserve_snapshots_and_fallback_publication() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner.kujo"),
        r#"
mut state := range(0, 128)
export func snapshot() { return state }
export func mixed(n) { state[-128] = n state[1] = [n] state[2] = {"value": n} return n }
export func scalars(n) { state[0] = n state[1] = n state[2] = n return n }
export func typed(n) { state[0] = false state[1] = null state[2] = n return n }
export func repeated(n) { state[0] = n state[0] = n + 1 state[1] = n state[2] = n return n }
export func failing(n) { state[0] = n state[1] = [n] state[128] = n }
func observe() { return state[0] }
export func callback(n) { state[0] = n state[1] = [n] state[2] = observe() return n }
"#,
    )
    .unwrap();
    let source = directory.path().join("main.kujo");
    std::fs::write(
        &source,
        r#"
from owner import snapshot, mixed, scalars, typed, repeated, failing, callback
let original := snapshot()
mut i := 1
while i <= 4 {
 let before := snapshot()
 assert_equal(mixed(i), i)
 let values := snapshot()
 assert_equal(values[0], i) assert_equal(values[1], [i]) assert_equal(values[2], {"value": i})
 assert_equal(scalars(i + 10), i + 10)
 assert_equal(values[1], [i]) assert_equal(values[2], {"value": i})
 assert_equal(typed(i), i)
 let types := snapshot()
 assert_equal(types[0], false) assert_equal(types[1], null)
 assert_equal(repeated(i), i)
 assert_equal(snapshot()[0], i + 1)
 assert_equal(types[0], false) assert_equal(types[1], null)
 assert_equal(before[127], 127)
 i += 1
}
let saved := snapshot()
mut caught := false
try { failing(71) } except err { caught = true }
assert(caught)
assert_equal(snapshot()[0], 71) assert_equal(snapshot()[1], [71])
assert_equal(saved[0], 5) assert_equal(saved[1], 4)
assert_equal(callback(81), 81) assert_equal(snapshot()[2], 81)
i = 0 while i < 128 { assert_equal(original[i], i) i += 1 }
print("SCALAR BATCH CONTRACT PASS")
"#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&source).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"SCALAR BATCH CONTRACT PASS\n", "{mode}");
    }
}
