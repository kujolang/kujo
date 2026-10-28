use std::process::Command;

fn verify(mode: &str) {
    let directory = tempfile::tempdir().unwrap();
    let width = 8192;
    let mut declarations = String::new();
    let mut updates = String::new();
    let mut checks = String::new();
    for index in 0..width {
        let name = format!("slot_{index:012}");
        let kind = match index % 3 {
            0 => "mut",
            1 => "let",
            _ => "const",
        };
        declarations.push_str(&format!("{kind} {name} := {index}\n"));
        if index % 3 == 0 {
            updates.push_str(&format!("{name} += 1\n"));
        }
        checks.push_str(&format!("sum += {name}\n"));
    }
    let owner = format!(
        r#"
export func factory() {{
    mut calls := 0
    mut data := [0, 1]
    {declarations}
    func step(fail) {{
        let before := data
        calls += 1
        data[0] += 1
        {updates}
        assert_equal(before[0], calls - 1)
        mut rejected_let := false
        try {{ slot_000000000001 = 99 }} except err {{ rejected_let = true }}
        mut rejected_const := false
        try {{ slot_000000000002 = 99 }} except err {{ rejected_const = true }}
        assert(rejected_let)
        assert(rejected_const)
        if fail {{ throw("after writes") }}
        mut sum := 0
        {checks}
        return [calls, sum, data]
    }}
    return step
}}
"#
    );
    assert!(owner.len() < 1_048_576);
    std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
    let original = width * (width - 1) / 2;
    let mutable = (width + 2) / 3;
    let main = format!(
        r#"
from owner import factory
let step := factory()
let alias := step
let separate := factory()
let first := step(false)
assert_equal(first, [1, {first}, [1, 1]])
assert_equal(alias(false), [2, {second}, [2, 1]])
assert_equal(separate(false), [1, {first}, [1, 1]])
mut caught := false
try {{ step(true) }} except err {{ caught = true }}
assert(caught)
assert_equal(alias(false), [4, {fourth}, [4, 1]])
assert_equal(first, [1, {first}, [1, 1]])
assert_equal(separate(false), [2, {second}, [2, 1]])
print("COMPACT CAPTURE NAMES PASS")
"#,
        first = original + mutable,
        second = original + 2 * mutable,
        fourth = original + 4 * mutable
    );
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, main).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--untrusted"]);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.arg(path).output().unwrap();
    assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"COMPACT CAPTURE NAMES PASS\n");
}
#[test]
fn vm_compact_capture_names() {
    verify("vm");
}
#[test]
fn interpreter_compact_capture_names() {
    verify("interpreter");
}
#[test]
fn jit_compact_capture_names() {
    verify("jit");
}
