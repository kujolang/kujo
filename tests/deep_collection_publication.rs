//! Imported deeper collection updates preserve snapshots and evaluation order.
use std::process::Command;

fn verify(mode: &str) {
    for owner in ["dictionary", "struct", "array"] {
        for leaf in ["array", "dictionary"] {
            for depth in [3, 6] {
                let directory = tempfile::tempdir().unwrap();
                let seed = if leaf == "array" {
                    "mut seed := range(0, 32)"
                } else {
                    "mut seed := {} mut k := 0 while k < 32 { seed[k] = k k += 1 }"
                };
                let mut inner = "seed".to_string();
                for _ in 0..depth - 2 {
                    inner = format!("{{\"inner\": {inner}}}");
                }
                let (init, mut row) = match owner {
                    "dictionary" => {
                        (format!("mut state := {{\"row\": {inner}}}"), "state[\"row\"]".to_string())
                    }
                    "struct" => (
                        format!(
                            "struct State {{ row: any }}\nmut state := State {{ row: {inner} }}"
                        ),
                        "state.row".to_string(),
                    ),
                    _ => (format!("mut state := [{inner}]"), "state[0]".to_string()),
                };
                row.push_str(&"[\"inner\"]".repeat(depth - 2));
                let module = format!("{seed}\n{init}\nseed = null\n")
                    + &r#"
let replacement := state
export func snapshot() { return state }
export func leaf_snapshot() { return ROW }
export func edit() { ROW[0] += 1 ROW[1] += 2 return ROW[0] }
func observe() { let saved := state ROW[0] += 7 return saved }
export func callback() { ROW[0] += 1 return observe() }
export func fail() { ROW[0] += 2 return ROW["missing"] }
func index_effect() { ROW[0] += 5 return 0 }
export func read_effect() { return ROW[index_effect()] }
func replace_index() { state = replacement return 0 }
export func write_effect() { ROW[replace_index()] = 42 return ROW[0] }
"#
                    .replace("ROW", &row);
                let main = r#"
from owner import snapshot, leaf_snapshot, edit, callback, fail, read_effect, write_effect
let original := snapshot()
let alias := leaf_snapshot()
mut i := 0
while i < 4 {
  let before := snapshot()
  assert_equal(edit(), i + 1)
  assert_equal(BEFORE[0], i)
  assert_equal(BEFORE[1], 1 + i * 2)
  i += 1
}
let updated := snapshot()
assert_equal(UPDATED[0], 4)
assert_equal(UPDATED[1], 9)
assert_equal(alias[0], 0)
assert_equal(alias[1], 1)
let observed := callback()
assert_equal(OBSERVED[0], 5)
let after_callback := snapshot()
assert_equal(AFTER_CALLBACK[0], 12)
mut caught := false
try { fail() } except e { caught = true }
assert(caught)
let after_fail := snapshot()
assert_equal(AFTER_FAIL[0], 14)
assert_equal(read_effect(), 14)
let after_read := snapshot()
assert_equal(AFTER_READ[0], 19)
assert_equal(write_effect(), 42)
let final := snapshot()
assert_equal(FINAL[0], 42)
assert_equal(FINAL[1], 1)
assert_equal(AFTER_READ[0], 19)
mut k := 0
while k < 32 {
  assert_equal(ORIGINAL[k], k)
  assert_equal(alias[k], k)
  if k > 1 { assert_equal(UPDATED[k], k) assert_equal(FINAL[k], k) }
  k += 1
}
print("DEEP PUBLICATION PASS")
"#;
                let mut main = main.to_string();
                for (token, name) in [
                    ("AFTER_CALLBACK", "after_callback"),
                    ("AFTER_FAIL", "after_fail"),
                    ("AFTER_READ", "after_read"),
                    ("ORIGINAL", "original"),
                    ("UPDATED", "updated"),
                    ("OBSERVED", "observed"),
                    ("BEFORE", "before"),
                    ("FINAL", "final"),
                ] {
                    main = main.replace(token, &row.replace("state", name));
                }
                std::fs::write(directory.path().join("owner.kujo"), module).unwrap();
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
                    "{mode}/{owner}/{leaf}/{depth}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(output.stdout, b"DEEP PUBLICATION PASS\n");
            }
        }
    }
}

#[test]
fn vm_deep_collection_publication() {
    verify("vm");
}
#[test]
fn interpreter_deep_collection_publication() {
    verify("interpreter");
}
#[test]
fn jit_deep_collection_publication() {
    verify("jit");
}
