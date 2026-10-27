//! Imported nested writes preserve visibility, snapshots and ordered fallback.
use std::process::Command;

fn verify(mode: &str) {
    let directory = tempfile::tempdir().unwrap();
    let module = r#"
struct State { row: array, other: array }
mut state := State { row: range(0, 128), other: [0] }
mut data := {"1": range(0, 128), "01": [41]}
mut side := range(0, 2048)
export func multiple(n) {
    mut i := 0
    while i < n {
        state.row[i] = i + 100
        data[1][i] = i + 200
        i += 1
    }
    state.row[-128] += 1
    data["1"][-128] += 1
    let saved := state
    state.row[1] = true
    state.row[2] = null
    state.row[3] = 1.5
    assert_equal(saved.row[1], 101)
    assert_equal(state.row[1], true)
    assert_equal(state.row[2], null)
    assert_equal(state.row[3], 1.5)
    return [saved.row[0], saved.row[n - 1], data[1][0], data[1][n - 1]]
}
export func snapshot() { return state }
func replace_for_negative_read() {
    state := State { row: range(20, 148), other: [0] }
    return 128
}
export func negative_reads() {
    state := State { row: range(0, 128), other: [0] }
    state.row[0] = 71
    state.row[1] = 72
    let size := 128
    assert_equal(state.row[-128], 71)
    assert_equal(state.row[-size], 71)
    let minimum := -9223372036854775807 - 1
    mut caught := false
    try { let bad := state.row[-minimum] } except e { caught = true }
    assert(caught)
    assert_equal(state.row[0], 71)
    caught = false
    try { let bad := state.row[-129] } except e { caught = true }
    assert(caught)
    state.row[0] = 73
    // Calls still evaluate after the row snapshot and may replace its owner.
    let old := state.row[-replace_for_negative_read()]
    return [old, state.row[0], state.row[1]]
}
export func invalid_self_index() {
    state.row[0] += 1
    return state.row[-state]
}
export func invalid_self_leaf() {
    state.row[0] += 1
    return state.row[state]
}
export func invalid_self_selector() {
    state.row[0] += 1
    return state[state][-1]
}
export func dictionary() { return data }
export func side_snapshot() { return side }
export func row_before_write() {
    let saved := state.row
    state.row[0] = 51
    return [saved[0], state.row[0]]
}
export func mixed_writes() {
    state.row[0] = 52
    side[0] += 1
    return side[0]
}
export func mixed_dictionary_writes() {
    data[1][0] = 53
    side[0] += 1
    return side[0]
}
export func edit(n) {
    mut i := 0
    while i < n { state.row[0] += 1 i += 1 }
    return state.row[0]
}
export func edit_dict(n) {
    let key := 1
    mut i := 0
    while i < n { data[key][0] += 1 i += 1 }
    return data["1"][0]
}
export func escape_then_write() {
    state.row[0] = 7
    let saved := state
    let row := state.row
    state.row[0] = 9
    return [saved.row[0], row[0], state.row[0]]
}
export func fallback() {
    state.row[0] = 11
    state.row[1] = 12
    state.other[0] = 13
    data[1][0] = 14
    data[1][1] = [15]
    return [state.row[0], state.row[1], state.other[0], data[1][0], data[1][1][0]]
}
export func fail_after_write() { state.row[0] = 17 state.row[9999] = 1 }
func replace_root() {
    state := State { row: range(20, 148), other: [0] }
    return 0
}
export func replace_during_index() {
    state.row[0] = 18
    state.row[replace_root()] = 19
    return [state.row[0], state.row[1]]
}
export func read_before_index_replacement() {
    state.row[0] = 23
    let read := state.row[replace_root()]
    return [read, state.row[0]]
}
export func shadow() {
    state.row[0] = 25
    if true {
        mut state := State { row: [26], other: [0] }
        state.row[0] = 27
        assert_equal(state.row[0], 27)
    }
    return state.row[0]
}
func callback(v) { state.row[0] += v return state.row[0] }
export func callbacks() {
    state.row[0] = 30
    let values := map([1, 2], callback)
    return [values[0], values[1], state.row[0]]
}
export func scalar_values() {
    state.row[0] = true
    assert_equal(state.row[0], true)
    state.row[0] = null
    assert_equal(state.row[0], null)
    state.row[0] = 1.5
    return state.row[0]
}
"#;
    let source = r#"
from state import multiple, snapshot, dictionary, side_snapshot, row_before_write, mixed_writes, mixed_dictionary_writes, edit, edit_dict, escape_then_write, fallback, fail_after_write, replace_during_index, read_before_index_replacement, shadow, callbacks, scalar_values
let original := snapshot()
assert_equal(edit(8), 8)
assert_equal(edit(8), 16)
assert_equal(original.row[0], 0)
let original_dict := dictionary()
assert_equal(edit_dict(8), 8)
assert_equal(edit_dict(8), 16)
assert_equal(original_dict[1][0], 0)
assert_equal(dictionary()["01"][0], 41)
assert_equal(escape_then_write(), [7, 7, 9])
assert_equal(fallback(), [11, 12, 13, 14, 15])
mut caught := false
try { fail_after_write() } except e { caught = true }
assert(caught)
assert_equal(snapshot().row[0], 17)
let before_replace := snapshot()
assert_equal(replace_during_index(), [19, 21])
assert_equal(before_replace.row[0], 17)
assert_equal(read_before_index_replacement(), [23, 20])
assert_equal(shadow(), 25)
assert_equal(callbacks(), [31, 33, 33])
assert_equal(scalar_values(), 1.5)
let frozen := snapshot()
caught = false
try { frozen.row[0] = 99 } except e { caught = true }
assert(caught)
assert_equal(frozen.row[0], 1.5)
assert_equal(snapshot().row[0], 1.5)
assert_equal(row_before_write(), [1.5, 51])
let before_side := side_snapshot()
let before_mixed := snapshot()
let before_dictionary := dictionary()
mut i := 0
while i < 8 {
    assert_equal(mixed_writes(), i * 2 + 1)
    assert_equal(mixed_dictionary_writes(), i * 2 + 2)
    i += 1
}
assert_equal(side_snapshot()[0], 16)
assert_equal(snapshot().row[0], 52)
assert_equal(dictionary()[1][0], 53)
assert_equal(before_side[0], 0)
assert_equal(before_mixed.row[0], 51)
assert_equal(before_dictionary[1][0], 14)
assert_equal(frozen.row[0], 1.5)
for count in [2, 3, 4, 5, 32] {
    let before := snapshot()
    let before_data := dictionary()
    let old := before.row[0]
    let old_data := before_data[1][0]
    assert_equal(multiple(count), [101, count + 99, 201, count + 199])
    assert_equal(before.row[0], old)
    assert_equal(before_data[1][0], old_data)
    assert_equal(multiple(count), [101, count + 99, 201, count + 199])
}
from state import negative_reads, invalid_self_index, invalid_self_leaf, invalid_self_selector
assert_equal(negative_reads(), [73, 20, 21])
for expected in [21, 22] {
    caught = false
    try { invalid_self_index() } except e { caught = true }
    assert(caught)
    assert_equal(snapshot().row[0], expected)
}
let before_invalid := snapshot()
for expected in [23, 25] {
    caught = false
    try { invalid_self_leaf() } except e { caught = true }
    assert(caught)
    assert_equal(snapshot().row[0], expected)
    caught = false
    try { invalid_self_selector() } except e { caught = true }
    assert(caught)
    assert_equal(snapshot().row[0], expected + 1)
}
assert_equal(before_invalid.row[0], 22)
print("NESTED PUBLICATION PASS")
"#;
    std::fs::write(directory.path().join("state.kujo"), module).unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(path).arg("--untrusted");
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let result = command.output().unwrap();
    assert!(result.status.success(), "{mode}: {}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(result.stdout, b"NESTED PUBLICATION PASS\n", "{mode}");
}

#[test]
fn vm_nested_publication() {
    verify("vm");
}
#[test]
fn interpreter_nested_publication() {
    verify("interpreter");
}
#[test]
fn jit_nested_publication() {
    verify("jit");
}
