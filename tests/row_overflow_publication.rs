//! Protect larger row batches before changing their publication storage.
use std::process::Command;

fn verify(mode: &str) {
    for owner in ["struct", "dictionary", "array"] {
        for count in [5, 16, 64] {
            for loops in [false, true] {
                let directory = tempfile::tempdir().unwrap();
                let (root, row) = match owner {
                    "struct" => (
                        "struct State { row: array } mut state := State { row: range(0, 128) }",
                        "state.row",
                    ),
                    "dictionary" => ("mut state := {\"row\": range(0, 128)}", "state[\"row\"]"),
                    _ => ("mut state := [range(0, 128)]", "state[0]"),
                };
                let edits = if loops {
                    format!("mut j := 0 while j < {count} {{ {row}[j] += 1 j += 1 }}")
                } else {
                    (0..count).map(|j| format!("{row}[{j}] += 1\n")).collect::<String>()
                };
                let module = format!(
                    r#"
{root}
let replacement := state
export func snapshot() {{ return state }}
export func leaf() {{ return {row} }}
export func edit() {{ {edits} return {row}[0] }}
func observe() {{ let saved := state {row}[0] += 9 return saved }}
export func callback() {{ {edits} return observe() }}
export func fail() {{ {edits} {row}[128] = 1 }}
export func fail_math(zero) {{ {edits} {row}[0] = 1 / zero }}
func replace_index() {{ state = replacement return 0 }}
export func replace() {{ {edits} {row}[replace_index()] = 42 }}
"#
                );
                let main = r#"
from owner import snapshot, leaf, edit, callback, fail, fail_math, replace
let original := snapshot()
let alias := leaf()
mut i := 0
while i < 3 {
 let before := snapshot()
 assert_equal(edit(), i + 1)
 let after := snapshot()
 mut j := 0
 while j < 128 {
   mut expected := j
   if j < COUNT { expected += i }
   assert_equal(BEFORE[j], expected)
   if j < COUNT { expected += 1 }
   assert_equal(AFTER[j], expected)
   assert_equal(ORIGINAL[j], j)
   assert_equal(alias[j], j)
   j += 1
 }
 i += 1
}
let observed := callback()
let after_callback := snapshot()
mut caught := false
try { fail() } except e { caught = true }
assert(caught)
let after_fail := snapshot()
caught = false
try { fail_math(0) } except e { caught = true }
assert(caught)
let after_math := snapshot()
replace()
let final := snapshot()
mut j := 0
while j < 128 {
 mut expected := j
 if j < COUNT { expected += 4 }
 assert_equal(OBSERVED[j], expected)
 if j == 0 { expected += 9 }
 assert_equal(AFTER_CALLBACK[j], expected)
 if j < COUNT { expected += 1 }
 assert_equal(AFTER_FAIL[j], expected)
 if j < COUNT { expected += 1 }
 assert_equal(AFTER_MATH[j], expected)
 assert_equal(ORIGINAL[j], j)
 assert_equal(alias[j], j)
 mut final_expected := j
 if j == 0 { final_expected = 42 }
 assert_equal(FINAL[j], final_expected)
 j += 1
}
print("ROW OVERFLOW PASS")
"#;
                let mut main = main.replace("COUNT", &count.to_string());
                for (token, name) in [
                    ("AFTER_CALLBACK", "after_callback"),
                    ("AFTER_FAIL", "after_fail"),
                    ("AFTER_MATH", "after_math"),
                    ("ORIGINAL", "original"),
                    ("OBSERVED", "observed"),
                    ("BEFORE", "before"),
                    ("AFTER", "after"),
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
                let result = command.output().unwrap();
                assert!(
                    result.status.success(),
                    "{mode}/{owner}/{count}/{loops}: {}{}",
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(result.stdout, b"ROW OVERFLOW PASS\n");
            }
        }
    }
}

#[test]
fn vm_row_overflow_publication() {
    verify("vm");
}
#[test]
fn interpreter_row_overflow_publication() {
    verify("interpreter");
}
#[test]
fn jit_row_overflow_publication() {
    verify("jit");
}
