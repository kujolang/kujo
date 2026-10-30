use std::process::Command;

#[test]
fn string_first_batches_preserve_snapshots_and_error_publication() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner.kujo"),
        r#"
mut state := range(0, 128)
export func snapshot() { return state }
export func edit(n) { state[-128] = "entry" state[1] = n state[2] = n return n }
export func text(n) { state[0] = "first" state[1] = "second" state[2] = n return n }
export func binary(payload, n) { state[0] = payload state[1] = n state[2] = n return n }
export func fail(n) { state[0] = "failed" state[1] = n state[128] = n }
func read() { return state[0] }
export func callback(n) { state[0] = "visible" state[1] = read() state[2] = n }
"#,
    )
    .unwrap();
    let source = directory.path().join("main.kujo");
    std::fs::write(
        &source,
        r#"
from owner import snapshot, edit, text, binary, fail, callback
let original := snapshot()
mut i := 1 while i <= 4 {
 let before := snapshot()
 assert_equal(edit(i), i)
 let updated := snapshot()
 assert_equal(updated[0], "entry") assert_equal(updated[1], i) assert_equal(updated[2], i)
 assert_equal(text(i), i)
 assert_equal(snapshot()[1], "second") assert_equal(updated[1], i)
 assert_equal(before[127], 127)
 i += 1
}
let saved := snapshot()
mut caught := false
try { fail(71) } except err { caught = true }
assert(caught)
assert_equal(snapshot()[0], "failed") assert_equal(snapshot()[1], 71)
assert_equal(saved[0], "first") assert_equal(saved[1], "second")
callback(81) assert_equal(snapshot()[1], "visible")
let raw := bytes([0, 127, 128, 255])
assert_equal(binary(raw, 9), 9)
let stored := snapshot()
assert_equal(stored[0], raw) assert_equal(stored[1], 9) assert_equal(stored[2], 9)
text(10) assert_equal(stored[0], raw) assert_equal(snapshot()[0], "first")
i = 0 while i < 128 { assert_equal(original[i], i) i += 1 }
print("STRING BATCH CONTRACT PASS")
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
        assert_eq!(output.stdout, b"STRING BATCH CONTRACT PASS\n", "{mode}");
    }
}
