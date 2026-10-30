use std::process::Command;

#[test]
fn mixed_pairs_preserve_two_element_snapshots_and_publication_order() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner.kujo"),
        r#"
struct Item { value: int }
mut state := [0, 1]
export func snapshot() { return state }
export func pair(n) { state[0] = n state[1] = [n] return n }
export func reverse(n) { state[0] = [n] state[1] = n return n }
export func collections(n) { state[0] = [n] state[1] = {"value": n} return n }
export func structures(n) { state[0] = Item { value: n } state[1] = n return n }
export func raw(blob, n) { state[0] = blob state[1] = [n] return n }
export func text(n) { state[0] = "entry" state[1] = [n] return n }
export func arrays(n) { state[0] = [n] state[1] = [n + 1] return n }
export func dictionaries(n) { state[0] = {"value": n} state[1] = {"value": n + 1} return n }
export func repeated(n) { state[0] = n state[0] = [n] return n }
export func fail(n) { state[0] = n state[2] = [n] }
func read() { return state[0] }
export func callback(n) { state[0] = [n] state[1] = read() return n }
"#,
    )
    .unwrap();
    let source = directory.path().join("main.kujo");
    std::fs::write(
        &source,
        r#"
from owner import snapshot, pair, reverse, collections, structures, raw, text, arrays, dictionaries, repeated, fail, callback
let original := snapshot()
let blob := bytes([0, 127, 128, 255])
mut i := 1 while i <= 8 {
 assert_equal(pair(i), i)
 let first := snapshot()
 assert_equal(first[0], i) assert_equal(first[1], [i])
 assert_equal(reverse(i), i)
 assert_equal(snapshot()[0], [i]) assert_equal(snapshot()[1], i)
 assert_equal(first[0], i) assert_equal(first[1], [i])
 assert_equal(collections(i), i)
 let mixed := snapshot()
 assert_equal(mixed[0], [i]) assert_equal(mixed[1], {"value": i})
 assert_equal(structures(i), i)
 assert_equal(snapshot()[0].value, i) assert_equal(snapshot()[1], i)
 assert_equal(raw(blob, i), i)
 let saved_raw := snapshot()
 assert_equal(text(i), i)
 assert_equal(snapshot()[0], "entry") assert_equal(snapshot()[1], [i])
 assert_equal(saved_raw[0], blob) assert_equal(saved_raw[1], [i])
 assert_equal(arrays(i), i)
 assert_equal(snapshot(), [[i], [i + 1]])
 assert_equal(dictionaries(i), i)
 assert_equal(snapshot(), [{"value": i}, {"value": i + 1}])
 assert_equal(repeated(i), i)
 assert_equal(snapshot(), [[i], {"value": i + 1}])
 assert_equal(mixed[0], [i]) assert_equal(mixed[1], {"value": i})
 i += 1
}
let before_error := snapshot()
mut caught := false
try { fail(71) } except err { caught = true }
assert(caught)
assert_equal(snapshot()[0], 71) assert_equal(snapshot()[1], {"value": 9})
assert_equal(before_error, [[8], {"value": 9}])
assert_equal(callback(81), 81) assert_equal(snapshot(), [[81], [81]])
assert_equal(original, [0, 1])
print("MIXED PAIR CONTRACT PASS")
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
        assert_eq!(output.stdout, b"MIXED PAIR CONTRACT PASS\n", "{mode}");
    }
}
