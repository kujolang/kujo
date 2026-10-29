//! Struct literal fields retain normal evaluation, publication and error boundaries.
use std::process::Command;

#[test]
fn struct_literal_fields_preserve_order_callbacks_snapshots_and_errors() {
    for (declaration, row) in [
        ("mut state := [Item { value: 0 }, Item { value: 1 }]", "state"),
        (r#"mut state := {"0": Item { value: 0 }, "1": Item { value: 1 }}"#, "state"),
        (
            r#"mut state := {"branch": {"0": Item { value: 0 }, "1": Item { value: 1 }}}"#,
            r#"state["branch"]"#,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let owner = r###"struct Item { value: int }
struct Envelope { before: array, value: int, after: array }
ROOT_DECLARATION
export func snapshot() { return ROW_ACCESS }
export func edit(n) { ROW_ACCESS[0] = Item { value: n } }
func change() { assert_equal(ROW_ACCESS[0].value, 20) ROW_ACCESS[1] = Item { value: 21 } return 22 }
export func ordered() {
  ROW_ACCESS[0] = Item { value: 20 }
  let result := Envelope { before: [ROW_ACCESS], value: change(), after: [ROW_ACCESS] }
  ROW_ACCESS[0] = Item { value: 23 }
  return result
}
export func failure() {
  ROW_ACCESS[0] = Item { value: 30 }
  ROW_ACCESS[0] = Item { value: [1][99] }
}
export func expose() {
  ROW_ACCESS[0] = Item { value: 40 }
  let result := Envelope { before: [ROW_ACCESS], value: 41, after: [ROW_ACCESS] }
  ROW_ACCESS[0] = Item { value: 42 }
  return result
}
"###
        .replace("ROOT_DECLARATION", declaration)
        .replace("ROW_ACCESS", row);
        std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
        let source = r###"from owner import snapshot, edit, ordered, failure, expose
let original := snapshot()
edit(2) edit(3)
assert_equal(original[0].value, 0)
assert_equal(original[1].value, 1)
let ordered_result := ordered()
assert_equal(ordered_result.before[0][0].value, 20)
assert_equal(ordered_result.before[0][1].value, 1)
assert_equal(ordered_result.value, 22)
assert_equal(ordered_result.after[0][0].value, 20)
assert_equal(ordered_result.after[0][1].value, 21)
assert_equal(snapshot()[0].value, 23)
mut caught := false
try { failure() } except e { caught = true }
assert(caught)
assert_equal(snapshot()[0].value, 30)
let exposed := expose()
assert_equal(exposed.before[0][0].value, 40)
assert_equal(exposed.after[0][0].value, 40)
assert_equal(snapshot()[0].value, 42)
assert_equal(original[0].value, 0)
print("STRUCT BOUNDARIES PASS")
"###;
        let path = directory.path().join("main.kujo");
        std::fs::write(&path, source).unwrap();
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(&path).arg("--untrusted");
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let result = command.output().unwrap();
            assert!(
                result.status.success(),
                "{row}, {mode}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, b"STRUCT BOUNDARIES PASS\n");
        }
    }
}
