//! Direct collection replacements preserve array indexing and publication boundaries.
use std::process::Command;

#[test]
fn direct_array_collection_updates_preserve_indices_snapshots_errors_and_shadowing() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner.kujo"),
        r#"
struct Item { value: int }
mut state := [[0], [1], [2]]
export func snapshot() { return state }
export func edit(n) { state[-1] = [n] return state[2] }
export func types() {
    state[-1] = [10]
    state[2] = {"value": 11}
    state[-1] = Item { value: 12 }
    state[2] = Item { value: 13 }
    let saved := state[-1]
    state[2] = [14]
    return saved
}
export func two_indices() { state[0] = [20] state[-1] = [21] }
export func scalar_transition() { state[2] = [30] state[-1] = 31 state[2] = [32] }
export func invalid_index(index) { state[2] = [40] state[index] = [99] }
export func shadow() {
    state[2] = [50]
    if true { mut state := [[100]] state[0] = [101] assert_equal(state[0][0], 101) }
    return state[2]
}
export func immutable() {
    const other := [[0]]
    state[2] = [60]
    other[0] = [99]
}
func replace() { state = [[70], [71], [72]] return -1 }
export func replace_during_index() { state[0] = [69] state[replace()] = [73] }
"#,
    )
    .unwrap();
    let source = r#"
from owner import snapshot, edit, types, two_indices, scalar_transition, invalid_index, shadow, immutable, replace_during_index
let original := snapshot()
let returned := edit(3)
edit(4)
assert_equal(returned[0], 3)
assert_equal(original[2][0], 2)
let alias := snapshot()
edit(5)
assert_equal(alias[2][0], 4)
assert_equal(types().value, 13)
assert_equal(snapshot()[2][0], 14)
two_indices()
assert_equal(snapshot()[0][0], 20)
assert_equal(snapshot()[2][0], 21)
scalar_transition()
assert_equal(snapshot()[2][0], 32)
for index in [3, -4, true, "0"] {
    mut caught := false
    try { invalid_index(index) } except error { caught = true }
    assert(caught)
    assert_equal(snapshot()[2][0], 40)
}
assert_equal(shadow()[0], 50)
mut caught := false
try { immutable() } except error { caught = true }
assert(caught)
assert_equal(snapshot()[2][0], 60)
let before := snapshot()
replace_during_index()
assert_equal(snapshot()[0][0], 70)
assert_equal(snapshot()[2][0], 73)
assert_equal(before[0][0], 20)
assert_equal(before[2][0], 60)
assert_equal(original[0][0], 0)
assert_equal(original[2][0], 2)
print("ARRAY COLLECTION CONTRACT PASS")
"#;
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&path).arg("--untrusted");
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let result = command.output().unwrap();
        assert!(result.status.success(), "{mode}: {}", String::from_utf8_lossy(&result.stderr));
        assert_eq!(result.stdout, b"ARRAY COLLECTION CONTRACT PASS\n");
    }
}
