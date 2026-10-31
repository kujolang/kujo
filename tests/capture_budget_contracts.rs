use std::process::Command;

fn verify(mode: &str, width: usize, scopes: usize, prefix_len: usize) {
    let directory = tempfile::tempdir().unwrap();
    let prefix = "x".repeat(prefix_len);
    let let_name = format!("{prefix}slot_000000000001");
    let const_name = format!("{prefix}slot_000000000002");
    let close_scopes = "}".repeat(scopes);
    let mut declarations = String::new();
    let mut updates = String::new();
    let mut checks = String::new();
    for index in 0..width * scopes {
        if index % width == 0 {
            declarations.push_str("if true {\n");
        }
        let name = format!("{prefix}slot_{index:012}");
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
        try {{ {let_name} = 99 }} except err {{ rejected_let = true }}
        mut rejected_const := false
        try {{ {const_name} = 99 }} except err {{ rejected_const = true }}
        assert(rejected_let)
        assert(rejected_const)
        if fail {{ throw("after writes") }}
        mut sum := 0
        {checks}
        return [calls, sum, data]
    }}
    return step
    {close_scopes}
}}
"#
    );
    assert!(owner.len() < 1_048_576);
    std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
    let width = width * scopes;
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
mut round := 5
while round <= 20 {{
    assert_equal(alias(false), [round, {original} + round * {mutable}, [round, 1]])
    assert_equal(first, [1, {first}, [1, 1]])
    round += 1
}}
assert_equal(separate(false), [3, {original} + 3 * {mutable}, [3, 1]])
print("COMPACT CAPTURE NAMES PASS")
"#,
        first = original + mutable,
        second = original + 2 * mutable,
        fourth = original + 4 * mutable
    );
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, main).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.arg(path).output().unwrap();
    assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"COMPACT CAPTURE NAMES PASS\n");
}
#[test]
fn vm_capture_budget_value_semantics() {
    for (width, scopes, prefix) in [(32, 1, 0), (8190, 1, 0), (1024, 1, 129), (2048, 3, 0)] {
        verify("vm", width, scopes, prefix);
    }
}
#[test]
fn interpreter_capture_budget_value_semantics() {
    for (width, scopes, prefix) in [(32, 1, 0), (8190, 1, 0), (1024, 1, 129), (2048, 3, 0)] {
        verify("interpreter", width, scopes, prefix);
    }
}
#[test]
fn jit_capture_budget_value_semantics() {
    for (width, scopes, prefix) in [(32, 1, 0), (8190, 1, 0), (1024, 1, 129), (2048, 3, 0)] {
        verify("jit", width, scopes, prefix);
    }
}
