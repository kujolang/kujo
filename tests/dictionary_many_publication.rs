//! Multiple-key captured updates preserve snapshot and publication boundaries.
use std::process::Command;

fn verify(mode: &str) {
    for count in [2, 4, 16, 128] {
        for scenario in
            ["ordinary", "indexed", "snapshot", "callback", "error", "insert", "nonscalar"]
        {
            let directory = tempfile::tempdir().unwrap();
            let owner = r#"
mut state := {"0": 0, "01": 99}
mut i := 1
while i < COUNT { state[i] = i i += 1 }
export func snapshot() { return state }
func callback(x) { assert_equal(state[0], -1) state[1] = null return 1 }
export func edit() {
  mut i := 0
  while i < COUNT { state[i] = -1 i += 1 }
  assert_equal(state["01"], 99)
  ACTION
  return state
}
"#
            .replace("COUNT", &count.to_string())
            .replace(
                "ACTION",
                match scenario {
                    "snapshot" => "let saved := state state[0] = true assert_equal(saved[0], -1)",
                    "callback" => "let result := map([0], callback) assert_equal(result[0], 1)",
                    "error" => "return state[\"missing\"]",
                    "insert" => "state[\"new\"] = 42",
                    "nonscalar" => "state[0] = [42]",
                    _ => "state[0] = -0.0",
                },
            );
            let owner = if scenario == "indexed" {
                owner
                    .replace("  assert_equal(state[\"01\"], 99)", "")
                    .replace("  return state\n", "  return state[0]\n")
            } else {
                owner
            };

            let main = r#"
from owner import snapshot, edit
let original := snapshot()
mut turn := 0
while turn < 3 {
  INVOKE
  let current := snapshot()
  CHECK
  assert_equal(current[COUNT - 1], -1)
  assert_equal(current["01"], 99)
  assert_equal(original[0], 0)
  assert_equal(original[COUNT - 1], COUNT - 1)
  turn += 1
}
print("MULTI DICTIONARY PASS")
"#
            .replace("COUNT", &count.to_string())
            .replace(
                "INVOKE",
                if scenario == "error" {
                    "mut caught := false try { edit() } except e { caught = true } assert(caught)"
                } else {
                    "edit()"
                },
            )
            .replace(
                "CHECK",
                match scenario {
                    "snapshot" => "assert_equal(current[0], true)",
                    "callback" => "assert_equal(current[1], null)",
                    "insert" => "assert_equal(current[\"new\"], 42)",
                    "nonscalar" => "assert_equal(current[0][0], 42)",
                    "error" => "assert_equal(current[0], -1)",
                    _ => "assert_equal(type(current[0]), type(-0.0))",
                },
            );
            // The callback intentionally changes the last entry in a two-key map.
            let main = if count == 2 && scenario == "callback" {
                main.replace("assert_equal(current[2 - 1], -1)", "assert_equal(current[1], null)")
            } else {
                main
            };
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
                "{mode}/{count}/{scenario}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, b"MULTI DICTIONARY PASS\n");
        }
    }
}
#[test]
fn vm_multiple_dictionary_keys() {
    verify("vm");
}
#[test]
fn interpreter_multiple_dictionary_keys() {
    verify("interpreter");
}
#[test]
fn jit_multiple_dictionary_keys() {
    verify("jit");
}

fn verify_operator_snapshot(mode: &str) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner.kujo"),
        r#"struct Trigger {
    value: int,
    func op_add(other) {
        let saved := state
        state[0] = other + 1
        assert_equal(saved[0], other)
        assert_equal(saved[1], other)
        return saved[0]
    }
}
mut state := {"0": 0, "1": 0, "2": Trigger { value: 0 }}
export func snapshot() { return state }
export func edit(n) {
    state[0] = n
    state[1] = n
    return state[2] + n
}
"#,
    )
    .unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(
        &path,
        r#"from owner import snapshot, edit
let original := snapshot()
mut n := 1
while n <= 32 {
    assert_equal(edit(n), n)
    assert_equal(snapshot()[0], n + 1)
    assert_equal(snapshot()[1], n)
    n += 1
}
assert_equal(original[0], 0)
assert_equal(original[1], 0)
print("OPERATOR ESCAPE PASS")
"#,
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(path).arg("--untrusted");
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let result = command.output().unwrap();
    assert!(result.status.success(), "{mode}: {}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(result.stdout, b"OPERATOR ESCAPE PASS\n");
}
#[test]
fn vm_dictionary_operator_snapshot() {
    verify_operator_snapshot("vm");
}
#[test]
fn interpreter_dictionary_operator_snapshot() {
    verify_operator_snapshot("interpreter");
}
#[test]
fn jit_dictionary_operator_snapshot() {
    verify_operator_snapshot("jit");
}

fn verify_fallback_updates(mode: &str) {
    for insertion in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let update = if insertion { "state[n + 2048] = n" } else { "state[2] = \"changed\"" };
        let owner = format!(
            r#"
mut state := {{"0": 0}}
mut k := 1
while k < 2048 {{ state[k] = k k += 1 }}
export func snapshot() {{ return state }}
export func edit(n) {{ state[0] = n state[1] = n {update} return state[0] }}
"#
        );
        let check = if insertion {
            "assert_equal(snapshot()[n + 2048], n)"
        } else {
            "assert_equal(snapshot()[2], \"changed\")"
        };
        let source = format!(
            r#"
from owner import snapshot, edit
let original := snapshot()
mut n := 1
while n <= 32 {{
    assert_equal(edit(n), n)
    assert_equal(snapshot()[1], n)
    {check}
    n += 1
}}
assert_equal(original[0], 0)
assert_equal(original[1], 1)
assert_equal(original[2], 2)
print("FALLBACK PASS")
"#
        );
        std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
        let path = directory.path().join("main.kujo");
        std::fs::write(&path, source).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(path).arg("--untrusted");
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{mode}/{insertion}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, b"FALLBACK PASS\n");
    }
}
#[test]
fn vm_dictionary_string_and_insertion_updates() {
    verify_fallback_updates("vm");
}
#[test]
fn interpreter_dictionary_string_and_insertion_updates() {
    verify_fallback_updates("interpreter");
}
#[test]
fn jit_dictionary_string_and_insertion_updates() {
    verify_fallback_updates("jit");
}
