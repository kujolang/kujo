//! Captured scalar changes retain types, snapshots, callbacks and error publication.
use std::process::Command;

fn verify(mode: &str) {
    let values = ["-9223372036854775807 - 1", "-0.0", "false", "null"];
    for first in values {
        for second in values {
            for scenario in ["ordinary", "snapshot", "callback", "error"] {
                let directory = tempfile::tempdir().unwrap();
                let owner = r#"
mut data := {"1": 99, "01": 77, "other": 88}
export func snapshot() { return data }
func change(x) { data[1] = SECOND_VALUE return 1 }
export func edit() {
    data[1] = FIRST_VALUE
    assert_equal(type(data[1]), type(FIRST_VALUE))
    OBSERVE_STEP
    CHANGE_STEP
    SNAPSHOT_CHECK
    assert_equal(data["01"], 77)
    assert_equal(data["other"], 88)
    RETURN_STEP
}
"#
                .replace("OBSERVE_STEP", if scenario == "snapshot" { "let saved := data" } else { "" })
                .replace("CHANGE_STEP", if scenario == "callback" {
                    "let mapped := map([0], change) assert_equal(mapped[0], 1)"
                } else { "data[1] = SECOND_VALUE" })
                .replace("SNAPSHOT_CHECK", if scenario == "snapshot" {
                    "assert_equal(saved[1], FIRST_VALUE) assert_equal(type(saved[1]), type(FIRST_VALUE))"
                } else { "" })
                .replace("RETURN_STEP", if scenario == "error" { "return data[\"missing\"]" } else { "return data[1]" })
                .replace("FIRST_VALUE", first)
                .replace("SECOND_VALUE", second);
                let main = r#"
from owner import snapshot, edit
let original := snapshot()
for count in [1, 2, 3] {
    let saved := snapshot()
    let old := saved[1]
    let old_type := type(saved[1])
    INVOCATION
    assert_equal(saved[1], old)
    assert_equal(type(saved[1]), old_type)
    let current := snapshot()
    assert_equal(current[1], SECOND_VALUE)
    assert_equal(type(current[1]), type(SECOND_VALUE))
}
assert_equal(original[1], 99)
print("SCALAR TRANSITIONS PASS")
"#
                .replace("INVOCATION", if scenario == "error" {
                    "mut caught := false try { edit() } except e { caught = true } assert(caught)"
                } else { "assert_equal(edit(), SECOND_VALUE)" })
                .replace("SECOND_VALUE", second);
                std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
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
                    "{mode}/{first}/{second}/{scenario}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(result.stdout, b"SCALAR TRANSITIONS PASS\n");
            }
        }
    }
}

#[test]
fn vm_dictionary_scalar_transitions() {
    verify("vm");
}
#[test]
fn interpreter_dictionary_scalar_transitions() {
    verify("interpreter");
}
#[test]
fn jit_dictionary_scalar_transitions() {
    verify("jit");
}

#[test]
fn initial_insertion_preserves_snapshots_callbacks_errors_and_root_replacement() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/dictionary_initial_insertion/main.kujo");
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&fixture).arg("--untrusted");
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let result = command.output().unwrap();
        assert!(result.status.success(), "{mode}: {}", String::from_utf8_lossy(&result.stderr));
        assert_eq!(result.stdout, b"INITIAL INSERTION CONTRACT PASS\n");
    }
}
