//! Array-owned rows preserve value semantics across deferred publication and fallback.
use std::process::Command;

fn verify(mode: &str) {
    let directory = tempfile::tempdir().unwrap();
    let module = r#"
mut state := [range(0, 128), [99]]
export func snapshot() { return state }
export func edit(n) {
    mut i := 0
    while i < n { state[0][i] = i + 100 i += 1 }
    let rows := 2
    state[-rows][-128] += 1
    assert_equal(state[-2][0], 101)
    assert_equal(state[-rows][-128], 101)
    return [state[0][0], state[0][n - 1], state[1][0]]
}
export func escapes() {
    state[0][0] = 7
    let saved := state
    let row := state[0]
    state[-2][0] = 8
    return [saved[0][0], row[0], state[0][0]]
}
export func types() {
    state[0][0] = -0.0
    assert_equal(type(state[-2][0]), type(-0.0))
    state[0][0] = true
    assert_equal(type(state[0][0]), type(true))
    state[0][0] = null
    assert_equal(state[-2][0], null)
    state[0][0] = [11]
    return state[0][0][0]
}
func callback(v) { state[0][0] += v return state[0][0] }
export func callbacks() {
    state[0][0] = 30
    let values := map([1, 2], callback)
    return [values[0], values[1], state[0][0]]
}
func replace() { state = [range(20, 148), [99]] return 0 }
export func replace_write() { state[0][0] = 18 state[0][replace()] = 19 return [state[0][0], state[0][1]] }
export func replace_read() { state[0][0] = 23 let old := state[0][replace()] return [old, state[0][0]] }
export func invalid(which) {
    state[0][0] = 42
    if which == 0 { return state["0"][0] }
    if which == 1 { return state[-3][0] }
    if which == 2 { return state[0][128] }
    if which == 3 { return state[-state][0] }
    if which == 4 { return state[state][0] }
    let minimum := -9223372036854775807 - 1
    return state[-minimum][0]
}
export func shared_rows() {
    let row := [1, 2]
    state = [row, row]
    state[0][0] = 9
    assert_equal(row[0], 1)
    return [state[0][0], state[1][0]]
}
export func other_row() { state[0][0] = 10 state[1][0] = 20 return [state[0][0], state[1][0]] }
"#;
    let source = r#"
from owner import snapshot, edit, escapes, types, callbacks, replace_write, replace_read, invalid, shared_rows, other_row
let original := snapshot()
for count in [1, 2, 3, 4, 5, 16] {
    let saved := snapshot()
    let old := saved[0][0]
    mut last := 100 + count - 1
    if count == 1 { last = 101 }
    assert_equal(edit(count), [101, last, 99])
    assert_equal(saved[0][0], old)
    assert_equal(edit(count), [101, last, 99])
}
assert_equal(original[0][0], 0)
assert_equal(escapes(), [7, 7, 8])
assert_equal(types(), 11)
assert_equal(callbacks(), [31, 33, 33])
assert_equal(replace_write(), [19, 21])
assert_equal(replace_read(), [23, 20])
for which in range(0, 6) {
    let saved := snapshot()
    let old := saved[0][0]
    mut caught := false
    try { invalid(which) } except e { caught = true }
    assert(caught)
    assert_equal(snapshot()[0][0], 42)
    assert_equal(saved[0][0], old)
}
assert_equal(shared_rows(), [9, 1])
assert_equal(other_row(), [10, 20])
assert_equal(original[0][0], 0)
print("ARRAY ROW PASS")
"#;
    std::fs::write(directory.path().join("owner.kujo"), module).unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(path).arg("--untrusted");
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let result = command.output().unwrap();
    assert!(result.status.success(), "{mode}: {}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(result.stdout, b"ARRAY ROW PASS\n");
}

#[test]
fn vm_array_row_publication() {
    verify("vm");
}
#[test]
fn interpreter_array_row_publication() {
    verify("interpreter");
}
#[test]
fn jit_array_row_publication() {
    verify("jit");
}
