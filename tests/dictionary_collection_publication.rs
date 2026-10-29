//! Collection-valued captured entries preserve publication and snapshot boundaries.
use std::process::Command;

#[test]
fn collection_entries_preserve_type_changes_callbacks_errors_and_replacement_roots() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner.kujo"),
        r#"
mut state := {}
mut i := 0 while i < 128 { state[i] = [i] i += 1 }
struct Item { value: int }
export func snapshot() { return state }
export func edit(n) { state[0] = [n] return state[0] }
export func types() {
    state[0] = [3]
    let old := state[0]
    state[0] = {"value": 4}
    assert_equal(state[0]["value"], 4)
    state[0] = Item { value: 5 }
    assert_equal(state[0].value, 5)
    state[0] = false
    assert_equal(state[0], false)
    state[0] = null
    state[0] = -0.0
    state[0] = 6
    state[0] = [7]
    assert_equal(old[0], 3)
    return state[0]
}
func callback(n) {
    assert_equal(state[0][0], 8)
    state[0][0] = n
    return n
}
export func observe() {
    state[0] = [8]
    let saved := state
    let values := map([11], callback)
    assert_equal(values[0], 11)
    assert_equal(state[0][0], 11)
    assert_equal(saved[0][0], 8)
    state[1] = {"value": 12}
    return saved
}
export func fail_key() {
    state[0] = [9]
    state[true] = [99]
}
func literal_element() {
    assert_equal(state[0][0], 21)
    state[1] = [23]
    return 22
}
export func literal_callback() {
    state[0] = [21]
    state[0] = [literal_element()]
    assert_equal(state[0][0], 22)
    assert_equal(state[1][0], 23)
}
export func literal_alias() {
    state[0] = [30]
    let held := [state]
    state[0] = [31]
    assert_equal(held[0][0][0], 30)
    return held
}
export func literal_spread() {
    state[0] = [40]
    state[0] = [...state[0], 41]
    assert_equal(state[0][0], 40)
    assert_equal(state[0][1], 41)
    state[0] = {"value": 42}
    state[0] = {...state[0], "extra": 43}
    assert_equal(state[0]["value"], 42)
    assert_equal(state[0]["extra"], 43)
}
export func literal_error() {
    state[0] = [50]
    state[0] = [undefined_literal_element]
}
func key() { state = {"replacement": [40]} return "replacement" }
export func rebind() {
    state[0] = [10]
    state[key()] = [41]
    return state
}
"#,
    )
    .unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(
        &path,
        r#"
from owner import snapshot, edit, types, observe, fail_key, rebind, literal_callback, literal_alias, literal_spread, literal_error
let returned := edit(2)
assert_equal(returned[0], 2)
assert_equal(types()[0], 7)
assert_equal(returned[0], 2)
let original := snapshot()
let before_callback := observe()
assert_equal(original[0][0], 7)
assert_equal(before_callback[0][0], 8)
let after_callback := snapshot()
assert_equal(after_callback[0][0], 11)
assert_equal(after_callback[1]["value"], 12)
mut caught := false
try { fail_key() } except e { caught = true }
assert(caught)
let after_error := snapshot()
assert_equal(after_error[0][0], 9)
assert_equal(len(after_error), 128)
assert_equal(after_callback[0][0], 11)
literal_callback()
let literal_saved := literal_alias()
literal_spread()
mut literal_caught := false
try { literal_error() } except e { literal_caught = true }
assert(literal_caught)
assert_equal(snapshot()[0][0], 50)
assert_equal(literal_saved[0][0][0], 30)
let final := rebind()
assert_equal(len(final), 1)
assert_equal(final["replacement"][0], 41)
assert_equal(after_error[0][0], 9)
mut i := 2 while i < 128 {
    assert_equal(original[i][0], i)
    assert_equal(after_error[i][0], i)
    i += 1
}
print("COLLECTION ENTRY CONTRACT PASS")
"#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&path).arg("--untrusted");
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let result = command.output().unwrap();
        assert!(result.status.success(), "{mode}: {}", String::from_utf8_lossy(&result.stderr));
        assert_eq!(result.stdout, b"COLLECTION ENTRY CONTRACT PASS\n");
    }
}
