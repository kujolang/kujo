//! Captured scalar batching must preserve type changes and failed writes.
use std::process::Command;

#[test]
fn captured_scalar_updates_preserve_dynamic_operators_and_partial_writes_on_error() {
    for (case, owner, source) in [
        (
            "type-change",
            r###"struct Trigger {
    value: int,
    func op_add(other) {
        let saved := state
        assert_equal(saved[0], other)
        assert_equal(saved[7], other)
        state[9] = 999
        assert_equal(saved[9], 0)
        return other + value
    }
}
mut state := {}
mut j := 0
while j < 16 { state[j] = 0 j += 1 }
mut counter := 0
export func snapshot() { return state }
export func set_number(n) { counter = n }
export func set_trigger() { counter = Trigger { value: 100 } }
export func edit(n) {
    mut j := 0
    while j < 8 { state[j] = n j += 1 }
    return counter + n
}
"###,
            r###"from owner import snapshot, set_number, set_trigger, edit
let original := snapshot()
assert_equal(edit(1), 1)
let before := snapshot()
set_trigger()
assert_equal(edit(2), 102)
assert_equal(snapshot()[9], 999)
assert_equal(before[9], 0)
set_number(7)
assert_equal(edit(3), 10)
let final := snapshot()
mut j := 0
while j < 16 {
    assert_equal(original[j], 0)
    if j < 8 {
        assert_equal(before[j], 1)
        assert_equal(final[j], 3)
    } else {
        assert_equal(before[j], 0)
        if j == 9 { assert_equal(final[j], 999) } else { assert_equal(final[j], 0) }
    }
    j += 1
}
print("CAPTURE TYPE CHANGE PASS")
"###,
        ),
        (
            "immutable-write",
            r###"mut state := {}
mut j := 0
while j < 16 { state[j] = 0 j += 1 }
const counter := 0
export func snapshot() { return state }
export func counter_value() { return counter }
export func edit(n) {
    mut j := 0
    while j < 8 { state[j] = n j += 1 }
    counter = n
    return n
}
"###,
            r###"from owner import snapshot, counter_value, edit
let original := snapshot()
mut caught := false
try { edit(7) } except e { caught = true }
assert(caught)
assert_equal(counter_value(), 0)
let after := snapshot()
mut j := 0
while j < 16 {
    assert_equal(original[j], 0)
    if j < 8 { assert_equal(after[j], 7) } else { assert_equal(after[j], 0) }
    j += 1
}
print("CAPTURE IMMUTABLE WRITE PASS")
"###,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
        let path = directory.path().join("main.kujo");
        std::fs::write(&path, source).unwrap();
        let expected = if case == "type-change" {
            "CAPTURE TYPE CHANGE PASS\n"
        } else {
            "CAPTURE IMMUTABLE WRITE PASS\n"
        };
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(&path).arg("--untrusted");
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let result = command.output().unwrap();
            assert!(
                result.status.success(),
                "{case}, {mode}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, expected.as_bytes(), "{case}, {mode}");
        }
    }
}
