//! Pending dictionary reads preserve selectors, errors and object snapshot order.
use std::process::Command;

fn run(mode: &str, owner: &str, source: &str) {
    let directory = tempfile::tempdir().unwrap();
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
        "{mode}: {}\n{owner}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"BOUNDARY PASS\n");
}

fn verify(mode: &str) {
    let mut cases = vec![
        ("return state[n - n]".to_owned(), false, "assert_equal(snapshot()[2], 2)"),
        ("return state[\"\" + \"0\"]".to_owned(), false, "assert_equal(snapshot()[2], 2)"),
        (
            "state[2] = payload return state[0]".to_owned(),
            false,
            "assert_equal(snapshot()[2], payload)",
        ),
        ("state[2] = None return state[0]".to_owned(), false, "assert_equal(snapshot()[2], None)"),
    ];
    for selector in ["null", "1.5", "true", "payload", "n / 0"] {
        cases.push((format!("return state[{selector}]"), true, "assert_equal(snapshot()[2], 2)"));
        cases.push((
            format!("state[{selector}] = n return state[0]"),
            true,
            "assert_equal(snapshot()[2], 2)",
        ));
    }
    for width in [3, 2048] {
        for (tail, error, check) in &cases {
            let owner = format!(
                r#"
mut state := {{"0": 0}}
mut k := 1
while k < {width} {{ state[k] = k k += 1 }}
export func snapshot() {{ return state }}
export func edit(n, payload) {{ state[0] = n state[1] = n {tail} }}
"#
            );
            let call = if *error {
                "mut caught := false try { edit(n, payload) } except e { caught = true } assert(caught)"
            } else {
                "assert_equal(edit(n, payload), n)"
            };
            let source = format!(
                r#"
from owner import snapshot, edit
let original := snapshot()
let payload := bytes([0,1,255])
mut n := 1
while n <= 8 {{
    {call}
    assert_equal(snapshot()[0], n)
    assert_equal(snapshot()[1], n)
    {check}
    n += 1
}}
assert_equal(original[0], 0)
assert_equal(original[1], 1)
assert_equal(original[2], 2)
assert_equal(payload[2], 255)
print("BOUNDARY PASS")
"#
            );
            run(mode, &owner, &source);
        }
    }
    run(
        mode,
        r#"
mut state := {"0": 0, "1": 1, "2": 2}
func replace() { state = {"0": 999, "1": 888, "2": 2} return 0 }
export func snapshot() { return state }
export func edit(n) { state[0] = n state[1] = n return state[replace()] }
"#,
        r#"
from owner import edit, snapshot
let original := snapshot()
mut n := 1
while n <= 8 {
    assert_equal(edit(n), n)
    assert_equal(snapshot()[0], 999)
    assert_equal(snapshot()[1], 888)
    n += 1
}
assert_equal(original[0], 0)
assert_equal(original[1], 1)
print("BOUNDARY PASS")
"#,
    );
}

#[test]
fn vm_pending_dictionary_boundaries() {
    verify("vm");
}
#[test]
fn interpreter_pending_dictionary_boundaries() {
    verify("interpreter");
}
#[test]
fn jit_pending_dictionary_boundaries() {
    verify("jit");
}
