//! Overflow batches preserve every snapshot key, callbacks and fallback behavior.
use std::process::Command;

fn verify(mode: &str) {
    for (init, row) in [
        ("mut state := {\"row\": seed}", "state[\"row\"]"),
        ("struct State { row: dict }\nmut state := State { row: seed }", "state.row"),
        ("mut state := [seed]", "state[0]"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let owner = r#"
mut seed := {}
mut k := 0
while k < 300 { seed[k] = k k += 1 }
INIT
seed = null
export func snapshot() { return state }
export func edit(count) {
  mut j := 0
  while j < count { ROW[j] += 1 j += 1 }
  ROW["0"] = ROW[0]
  return ROW[0]
}
func observe() { let saved := state ROW[0] = 999 return saved }
export func callback() { edit(257) return observe() }
export func fail() { edit(257) return ROW["missing"] }
export func fallback() { edit(257) ROW["payload"] = bytes([0, 255]) return ROW["payload"] }
export func insert() {
  edit(257)
  ROW["new"] = null ROW["01"] = false ROW["fraction"] = 1.25
  return ROW["01"]
}
"#
        .replace("INIT", init)
        .replace("ROW", row);
        let main = r#"
from owner import snapshot, edit, callback, fail, fallback, insert
let original := snapshot()
for count in [5, 16, 64, 257] {
  let before := snapshot()
  let first := BEFORE[0]
  mut repeat := 0
  while repeat < 3 { assert_equal(edit(count), first + repeat + 1) repeat += 1 }
  let after := snapshot()
  mut j := 0
  while j < 300 {
    if j < count { assert_equal(AFTER[j], BEFORE[j] + 3) }
    else { assert_equal(AFTER[j], BEFORE[j]) }
    assert_equal(ORIGINAL[j], j)
    j += 1
  }
}
for count in [4, 5, 16, 64, 257] {
  mut repeat := 0
  while repeat < 3 {
    let before := snapshot()
    let first := BEFORE[0]
    assert_equal(edit(count), first + 1)
    let after := snapshot()
    mut j := 0
    while j < 300 {
      if j < count { assert_equal(AFTER[j], BEFORE[j] + 1) }
      else { assert_equal(AFTER[j], BEFORE[j]) }
      assert_equal(ORIGINAL[j], j)
      j += 1
    }
    repeat += 1
  }
}
let before_callback := snapshot()
let observed := callback()
mut j := 0
while j < 257 { assert_equal(OBSERVED[j], BEFORE_CALLBACK[j] + 1) j += 1 }
let before_fail := snapshot()
mut caught := false
try { fail() } except e { caught = true }
assert(caught)
let after_fail := snapshot()
j = 0
while j < 257 { assert_equal(AFTER_FAIL[j], BEFORE_FAIL[j] + 1) j += 1 }
assert_equal(fallback(), bytes([0, 255]))
let before_insert := snapshot()
assert_equal(insert(), false)
let inserted := snapshot()
assert_equal(INSERTED["new"], null)
assert_equal(INSERTED["01"], false)
assert_equal(INSERTED["fraction"], 1.25)
assert_equal(has_key(BEFORE_INSERT, "new"), 0)
assert_equal(has_key(ORIGINAL, "payload"), 0)
print("NESTED SPILL PASS")
"#;
        let mut main = main.to_string();
        // Replace longer tokens first so BEFORE does not consume BEFORE_FAIL.
        for (token, name) in [
            ("BEFORE_CALLBACK", "before_callback"),
            ("BEFORE_INSERT", "before_insert"),
            ("BEFORE_FAIL", "before_fail"),
            ("AFTER_FAIL", "after_fail"),
            ("INSERTED", "inserted"),
            ("OBSERVED", "observed"),
            ("ORIGINAL", "original"),
            ("BEFORE", "before"),
            ("AFTER", "after"),
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
        assert_eq!(output.stdout, b"NESTED SPILL PASS\n");
    }
}
#[test]
fn vm_nested_dictionary_spill() {
    verify("vm");
}
#[test]
fn interpreter_nested_dictionary_spill() {
    verify("interpreter");
}
#[test]
fn jit_nested_dictionary_spill() {
    verify("jit");
}
