use std::process::Command;

#[test]
fn arithmetic_selectors_preserve_snapshots_and_error_publication() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner.kujo"),
        r#"
mut state := [0, 0, 0, 0, 0, 0, 0, 0]
export func snapshot() { return state }
export func ascending(n, offset) {
 state[offset + 0] = n state[offset + 1] = [n] state[offset + 2] = {"value": n}
 return n
}
export func descending(n, offset) {
 state[offset - 0] = n state[offset - 1] = [n] state[offset - 2] = {"value": n}
 return n
}
export func loop_up(n, offset) {
 mut j := 0 while j < 3 { state[offset + j] = [n + j] j += 1 }
 return n
}
export func loop_down(n, offset) {
 mut j := 0 while j < 3 { state[offset - j] = [n + j] j += 1 }
 return n
}
export func overflow(n, limit) {
 state[0] = n state[limit + 1] = [n] state[2] = n
 return n
}
export func bounds(n, offset) {
 state[0] = n state[offset + 1] = [n] state[2] = n
 return n
}
func observe() { return state[0] }
export func callback(n, offset) {
 state[offset + 0] = [n] state[offset + 1] = observe() state[offset + 2] = n
 return n
}
"#,
    )
    .unwrap();
    let source = directory.path().join("main.kujo");
    std::fs::write(
        &source,
        r#"
from owner import snapshot, ascending, descending, loop_up, loop_down, overflow, bounds, callback
let original := snapshot()
mut n := 1 while n <= 8 {
 assert_equal(ascending(n, 1), n)
 let saved := snapshot()
 assert_equal(saved[1], n) assert_equal(saved[2], [n]) assert_equal(saved[3], {"value": n})
 assert_equal(descending(n, -1), n)
 assert_equal(snapshot()[7], n) assert_equal(snapshot()[6], [n]) assert_equal(snapshot()[5], {"value": n})
 assert_equal(loop_up(n, 3), n)
 assert_equal(snapshot()[3], [n]) assert_equal(snapshot()[4], [n + 1]) assert_equal(snapshot()[5], [n + 2])
 assert_equal(loop_down(n, 5), n)
 assert_equal(snapshot()[5], [n]) assert_equal(snapshot()[4], [n + 1]) assert_equal(snapshot()[3], [n + 2])
 assert_equal(saved[1], n) assert_equal(saved[2], [n]) assert_equal(saved[3], {"value": n})
 n += 1
}
let prior := snapshot()
mut caught := false
try { overflow(71, 9223372036854775807) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0], 71) assert_equal(snapshot()[2], prior[2])
caught = false
try { bounds(72, 7) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0], 72) assert_equal(snapshot()[2], prior[2])
assert_equal(callback(81, 0), 81)
assert_equal(snapshot()[0], [81]) assert_equal(snapshot()[1], [81]) assert_equal(snapshot()[2], 81)
assert_equal(original, [0, 0, 0, 0, 0, 0, 0, 0])
print("ARITHMETIC SELECTOR CONTRACT PASS")
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
        assert_eq!(output.stdout, b"ARITHMETIC SELECTOR CONTRACT PASS\n", "{mode}");
    }
}
