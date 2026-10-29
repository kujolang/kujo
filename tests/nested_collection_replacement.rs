//! Nested collection replacement keeps observable owners and snapshots independent.
use std::process::Command;

#[test]
fn nested_collection_replacements_preserve_callbacks_aliases_errors_and_rebinding() {
    for (declaration, row) in [
        ("mut root := {\"a\": {\"b\": {\"row\": seed}}}", "root[\"a\"][\"b\"][\"row\"]"),
        (
            "struct State { rows: array } mut root := State { rows: [{\"row\": seed}] }",
            "root.rows[0][\"row\"]",
        ),
        ("mut root := [seed]", "root[0]"),
    ] {
        for array_leaf in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let seed = if array_leaf { "[[0], [1]]" } else { "{\"0\": [0], \"1\": [1]}" };
            let owner = format!(
                r#"
mut seed := {seed}
{declaration}
seed = null
export func snapshot() {{ return root }}
export func row_snapshot() {{ return {row} }}
export func edit(n) {{ {row}[0] = [n] return {row}[0] }}
func callback() {{
    assert_equal({row}[0][0], 20)
    {row}[1] = [21]
    return 22
}}
export func callbacks() {{
    {row}[0] = [20]
    {row}[0] = [callback()]
    assert_equal({row}[0][0], 22)
    assert_equal({row}[1][0], 21)
}}
export func types() {{
    {row}[0] = [30]
    let saved := {row}[0]
    {row}[0] = {{"value": 31}}
    assert_equal({row}[0]["value"], 31)
    {row}[0] = 32
    {row}[0] = [33]
    assert_equal(saved[0], 30)
    return saved
}}
export func fail() {{
    {row}[0] = [40]
    {row}[true] = [99]
}}
export func literal_alias() {{
    {row}[0] = [50]
    let saved := [root]
    {row}[0] = [51]
    return saved[0]
}}
func replace() {{
    root = null
    return 0
}}
export func rebind_error() {{
    {row}[0] = [60]
    {row}[replace()] = [61]
}}
export func is_replaced() {{ return root == null }}
"#
            );
            std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
            let old_row = row.replace("root", "original");
            let saved_row = row.replace("root", "saved");
            let before_row = row.replace("root", "before");
            let source = format!(
                r#"
from owner import snapshot, row_snapshot, edit, callbacks, types, fail, literal_alias, rebind_error, is_replaced
let original := snapshot()
let returned := edit(2)
assert_equal(edit(3)[0], 3)
assert_equal(returned[0], 2)
assert_equal({old_row}[0][0], 0)
let alias := row_snapshot()
assert_equal(edit(4)[0], 4)
assert_equal(alias[0][0], 3)
callbacks()
assert_equal(types()[0], 30)
mut caught := false
try {{ fail() }} except e {{ caught = true }}
assert(caught)
assert_equal(row_snapshot()[0][0], 40)
let saved := literal_alias()
assert_equal({saved_row}[0][0], 50)
let before := snapshot()
mut rebound := false
try {{ rebind_error() }} except e {{ rebound = true }}
assert(rebound)
assert(is_replaced())
assert_equal({before_row}[0][0], 51)
assert_equal({old_row}[0][0], 0)
assert_equal({old_row}[1][0], 1)
print("NESTED COLLECTION CONTRACT PASS")
"#
            );
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
                    "{row}, array={array_leaf}, {mode}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(result.stdout, b"NESTED COLLECTION CONTRACT PASS\n");
            }
        }
    }
}
