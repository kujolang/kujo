//! Inline numeric dictionary pairs preserve compound updates and fallback behavior.
use std::process::Command;

fn verify(mode: &str) {
    for (first, second) in [
        ("0", "1"),
        ("-1", "2"),
        ("-268435456", "268435455"),
        ("-268435457", "268435456"),
        ("01", "+1"),
        ("-0", "0"),
    ] {
        for width in [2, 2048] {
            let directory = tempfile::tempdir().unwrap();
            let owner = r#"
mut state := {"0": 0}
mut key := 1
while key < WIDTH { state[key] = key key += 1 }
state["FIRST"] = 0
state["SECOND"] = 10
export func snapshot() { return state }
export func edit() {
  state["FIRST"] += 1
  state["SECOND"] += 1
  return state["FIRST"]
}
export func fail() {
  state["FIRST"] += 1
  state["SECOND"] += 1
  return state["absent"]
}
func observe() { let saved := state state["FIRST"] = 50 return saved }
export func with_callback() {
  state["FIRST"] += 1
  state["SECOND"] += 1
  return observe()
}
export func promote(payload) {
  state["FIRST"] = 100
  state["SECOND"] = false
  state["new"] = payload
  return state["FIRST"]
}
"#
            .replace("WIDTH", &width.to_string())
            .replace("FIRST", first)
            .replace("SECOND", second);
            let main = r#"
from owner import snapshot, edit, fail, with_callback, promote
let original := snapshot()
mut n := 0
while n < 4 {
  let saved := snapshot()
  assert_equal(edit(), n + 1)
  assert_equal(saved["FIRST"], n)
  assert_equal(saved["SECOND"], n + 10)
  n += 1
}
mut caught := false
try { fail() } except e { caught = true }
assert(caught)
assert_equal(snapshot()["FIRST"], 5)
assert_equal(snapshot()["SECOND"], 15)
let observed := with_callback()
assert_equal(observed["FIRST"], 6)
assert_equal(observed["SECOND"], 16)
assert_equal(snapshot()["FIRST"], 50)
let payload := bytes([0, 127, 255])
assert_equal(promote(payload), 100)
assert_equal(snapshot()["new"], payload)
assert_equal(snapshot()["SECOND"], false)
assert_equal(observed["FIRST"], 6)
assert_equal(original["FIRST"], 0)
assert_equal(original["SECOND"], 10)
print("DICTIONARY PAIR PASS")
"#
            .replace("FIRST", first)
            .replace("SECOND", second);
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
                "{mode}/{first}/{second}/{width}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"DICTIONARY PAIR PASS\n");
        }
    }
}

#[test]
fn vm_dictionary_pair_boundaries() {
    verify("vm");
}
#[test]
fn interpreter_dictionary_pair_boundaries() {
    verify("interpreter");
}
#[test]
fn jit_dictionary_pair_boundaries() {
    verify("jit");
}
