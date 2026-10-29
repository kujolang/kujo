//! New nested keys retain immediate reads and publication at observable boundaries.
use std::process::Command;

#[test]
fn nested_initial_insertion_preserves_owners_snapshots_callbacks_errors_and_aliases() {
    for (root, row) in [
        ("mut state := {\"row\": seed}", "state[\"row\"]"),
        ("mut state := [seed]", "state[0]"),
        ("struct State { row: dict } mut state := State { row: seed }", "state.row"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let owner = r#"
mut seed := {"01": 77}
mut i := 0 while i < 128 { seed[i] = i i += 1 }
ROOT
seed = null
export func snapshot() { return state }
export func cold(k) { ROW[k] = k return ROW[k] }
func callback(x) {
    assert_equal(ROW[200], 200)
    ROW[201] = false
    return x
}
export func observe() {
    ROW[200] = 200
    assert_equal(ROW["200"], 200)
    let saved := state
    let result := map([7], callback)
    assert_equal(result[0], 7)
    assert_equal(has_key(SAVED_ROW, 201), 0)
    assert_equal(SAVED_ROW[200], 200)
    ROW["01"] = 88
    assert_equal(ROW[1], 1)
    return saved
}
export func fail_after_write() {
    ROW[202] = null
    return ROW["missing"]
}
export func replace() {
    ROW[203] = 203
    let saved := state
    ROW = {"replacement": true}
    return saved
}
"#
        .replace("ROOT", root)
        .replace("SAVED_ROW", &row.replace("state", "saved"))
        .replace("ROW", row);
        let source = r#"
from owner import snapshot, cold, observe, fail_after_write, replace
// No user snapshot exists for the first new-key write.
assert_equal(cold(128), 128)
let original := snapshot()
let seen := observe()
assert_equal(ORIGINAL_ROW[128], 128)
assert_equal(has_key(ORIGINAL_ROW, 200), 0)
assert_equal(SEEN_ROW[200], 200)
assert_equal(has_key(SEEN_ROW, 201), 0)
mut caught := false
try { fail_after_write() } except e { caught = true }
assert(caught)
let after_error := snapshot()
assert_equal(ERROR_ROW[201], false)
assert_equal(ERROR_ROW[202], null)
assert_equal(ERROR_ROW["01"], 88)
assert_equal(ERROR_ROW[1], 1)
let before_replace := replace()
assert_equal(REPLACED_ROW[203], 203)
let final := snapshot()
assert_equal(len(FINAL_ROW), 1)
assert_equal(FINAL_ROW["replacement"], true)
assert_equal(has_key(ERROR_ROW, 203), 0)
print("NESTED INITIAL INSERTION CONTRACT PASS")
"#
        .replace("ORIGINAL_ROW", &row.replace("state", "original"))
        .replace("SEEN_ROW", &row.replace("state", "seen"))
        .replace("ERROR_ROW", &row.replace("state", "after_error"))
        .replace("REPLACED_ROW", &row.replace("state", "before_replace"))
        .replace("FINAL_ROW", &row.replace("state", "final"));
        std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
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
                "{row}/{mode}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, b"NESTED INITIAL INSERTION CONTRACT PASS\n");
        }
    }
}
