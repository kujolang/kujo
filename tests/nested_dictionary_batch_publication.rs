//! Bounded nested dictionary batches preserve aliases, snapshots and fallback order.
use std::process::Command;

fn verify(mode: &str) {
    for (init, row) in [
        ("mut state := {\"row\": seed}", "state[\"row\"]"),
        ("struct State { row: dict }\nmut state := State { row: seed }", "state.row"),
        ("mut state := [seed]", "state[0]"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let owner = r#"
mut seed := {"0": 0, "01": 10, "a": 20, "b": 30, "fifth": 40}
INIT
seed = null
export func snapshot() { return state }
export func edit() {
  ROW[0] += 1 ROW["01"] += 2 ROW["a"] = 1.25 ROW["b"] = false
  ROW["0"] = ROW[0] + 1
  return ROW[0]
}
export func insert() {
  ROW[0] = 10 ROW["new"] = null ROW["another"] = true ROW["last"] = -9223372036854775807
  return ROW["another"]
}
export func overflow() {
  ROW[0] = 20 ROW["01"] = 21 ROW["a"] = 22 ROW["b"] = 23 ROW["fifth"] = 24
  ROW[0] = 25 return ROW[0]
}
func observe() { let before := state ROW[0] = 99 return before }
export func callback() {
  ROW[0] = 30 ROW["01"] = 31 ROW["a"] = 32 ROW["b"] = 33 return observe()
}
export func fail() {
  ROW[0] = 40 ROW["01"] = 41 ROW["a"] = 42 ROW["b"] = 43 return ROW["missing"]
}
export func payload() { ROW[0] = 50 ROW["new"] = bytes([0, 255]) return ROW["new"] }
"#
        .replace("INIT", init)
        .replace("ROW", row);
        let main = r#"
from owner import snapshot, edit, insert, overflow, callback, fail, payload
let original := snapshot()
mut i := 0
while i < 4 {
  let saved := snapshot()
  assert_equal(edit(), (i + 1) * 2)
  assert_equal(SAVED[0], i * 2)
  assert_equal(SAVED["01"], 10 + i * 2)
  i += 1
}
assert_equal(insert(), true)
let inserted := snapshot()
assert_equal(INSERTED[0], 10)
assert_equal(INSERTED["new"], null)
assert_equal(INSERTED["another"], true)
assert_equal(INSERTED["last"], -9223372036854775807)
assert_equal(overflow(), 25)
let overflowed := snapshot()
assert_equal(OVERFLOWED["01"], 21)
assert_equal(OVERFLOWED["a"], 22)
assert_equal(OVERFLOWED["b"], 23)
assert_equal(OVERFLOWED["fifth"], 24)
let observed := callback()
assert_equal(OBSERVED[0], 30)
assert_equal(OBSERVED["01"], 31)
assert_equal(OBSERVED["a"], 32)
assert_equal(OBSERVED["b"], 33)
mut caught := false
try { fail() } except e { caught = true }
assert(caught)
let failed := snapshot()
assert_equal(FAILED[0], 40)
assert_equal(FAILED["01"], 41)
assert_equal(FAILED["a"], 42)
assert_equal(FAILED["b"], 43)
assert_equal(payload(), bytes([0, 255]))
assert_equal(INSERTED["new"], null)
assert_equal(ORIGINAL[0], 0)
assert_equal(ORIGINAL["01"], 10)
assert_equal(ORIGINAL["a"], 20)
assert_equal(ORIGINAL["b"], 30)
assert_equal(has_key(ORIGINAL, "new"), 0)
print("NESTED BATCH PASS")
"#;
        let mut main = main.to_string();
        for (token, name) in [
            ("SAVED", "saved"),
            ("INSERTED", "inserted"),
            ("OVERFLOWED", "overflowed"),
            ("OBSERVED", "observed"),
            ("FAILED", "failed"),
            ("ORIGINAL", "original"),
        ] {
            main = main.replace(token, &row.replace("state", name));
        }
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
            "{mode}/{row}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"NESTED BATCH PASS\n");
    }
}
#[test]
fn vm_nested_dictionary_batch() {
    verify("vm");
}
#[test]
fn interpreter_nested_dictionary_batch() {
    verify("interpreter");
}
#[test]
fn jit_nested_dictionary_batch() {
    verify("jit");
}
