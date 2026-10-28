//! Captured dictionary leaves preserve snapshots, callbacks and fallback ordering.
use std::process::Command;

fn verify(mode: &str) {
    for (root, row) in [
        ("mut state := {\"row\": seed}", "state[\"row\"]"),
        ("struct State { row: dict }\nmut state := State { row: seed }", "state.row"),
        ("mut state := [seed]", "state[0]"),
    ] {
        for key in ["0", "\"0\"", "\"01\"", "\"item\""] {
            let directory = tempfile::tempdir().unwrap();
            let owner = r#"
mut seed := {"0": 0, "01": 0, "item": 0, "other": 1}
ROOT
seed = null
export func snapshot() { return state }
export func edit() { ROW[KEY] += 1 return ROW[KEY] }
export func typed(value) { ROW[KEY] = value return ROW[KEY] }
export func indexed(key) { ROW[KEY] = 7 return ROW[key] }
export func fail() { ROW[KEY] = 8 return ROW["missing"] }
func observe() { let before := state ROW[KEY] = 20 return before }
export func callback() { ROW[KEY] = 9 return observe() }
export func siblings() {
    let before := ROW
    ROW[KEY] = 30
    ROW["other"] = 40
    return before
}
export func payload(value) { ROW[KEY] = 31 ROW["new"] = value return ROW[KEY] }
"#
            .replace("ROOT", root)
            .replace("ROW", row)
            .replace("KEY", key);
            let saved_row = row.replace("state", "saved");
            let original_row = row.replace("state", "original");
            let final_row = row.replace("state", "final");
            let main = r#"
from owner import snapshot, edit, typed, indexed, fail, callback, siblings, payload
let original := snapshot()
assert_equal(edit(), 1)
assert_equal(edit(), 2)
let saved := snapshot()
assert_equal(edit(), 3)
assert_equal(SAVED[KEY], 2)
assert_equal(typed(-9223372036854775807), -9223372036854775807)
assert_equal(typed(1.25), 1.25)
assert_equal(typed(false), false)
assert_equal(typed(null), null)
assert_equal(indexed(KEY), 7)
mut caught := false
try { fail() } except e { caught = true }
assert(caught)
let after_error := snapshot()
assert_equal(ERROR[KEY], 8)
let observed := callback()
assert_equal(OBSERVED[KEY], 9)
let sibling := siblings()
assert_equal(sibling[KEY], 20)
assert_equal(sibling["other"], 1)
let bytes_value := bytes([0, 128, 255])
assert_equal(payload(bytes_value), 31)
let final := snapshot()
assert_equal(FINAL[KEY], 31)
assert_equal(FINAL["other"], 40)
assert_equal(FINAL["new"], bytes_value)
assert_equal(ORIGINAL[KEY], 0)
assert_equal(ORIGINAL["other"], 1)
assert_equal(SAVED[KEY], 2)
print("NESTED DICTIONARY PASS")
"#
            .replace("SAVED", &saved_row)
            .replace("ORIGINAL", &original_row)
            .replace("FINAL", &final_row)
            .replace("ERROR", &row.replace("state", "after_error"))
            .replace("OBSERVED", &row.replace("state", "observed"))
            .replace("KEY", key);
            std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
            let path = directory.path().join("main.kujo");
            std::fs::write(&path, main).unwrap();
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(path).arg("--untrusted");
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{mode}/{row}/{key}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"NESTED DICTIONARY PASS\n");
        }
    }
}

#[test]
fn vm_nested_dictionary_publication() {
    verify("vm");
}
#[test]
fn interpreter_nested_dictionary_publication() {
    verify("interpreter");
}
#[test]
fn jit_nested_dictionary_publication() {
    verify("jit");
}
