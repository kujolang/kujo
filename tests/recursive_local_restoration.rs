use std::process::Command;

fn verify(mode: &str) {
    let directory = tempfile::tempdir().unwrap();
    let declarations = (0..128).map(|i| format!("mut extra_{i} := {i}\n")).collect::<String>();
    let checks = (0..128).map(|i| format!("sum += extra_{i}\n")).collect::<String>();
    let module = format!(
        r#"
export func factory() {{
    mut counter := 0
    mut data := [0, 1]
    const fixed := [7]
    {declarations}
    func step(depth, should_fail) {{
        let entry := counter
        let original_depth := depth
        let saved := data
        counter += 1
        data[0] += 1
        if depth > 0 {{
            try {{ step(depth - 1, should_fail) }} except err {{}}
        }} else if should_fail {{ throw("leaf") }}
        assert_equal(depth, original_depth)
        assert_equal(saved[0], entry)
        assert_equal(data[1], 1)
        mut rejected := false
        try {{ fixed[0] = 99 }} except err {{ rejected = true }}
        assert(rejected)
        mut sum := 0
        {checks}
        return [counter, data[0], sum, fixed[0]]
    }}
    return step
}}
"#
    );
    let source = r#"
from owner import factory
let step := factory()
let alias := step
let separate := factory()
assert_equal(step(2, false), [3, 3, 8128, 7])
assert_equal(alias(2, true), [6, 6, 8128, 7])
assert_equal(separate(1, false), [2, 2, 8128, 7])
assert_equal(step(0, false), [7, 7, 8128, 7])
print("RECURSIVE LOCAL RESTORATION PASS")
"#;
    std::fs::write(directory.path().join("owner.kujo"), module).unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--untrusted"]);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.arg(path).output().unwrap();
    assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"RECURSIVE LOCAL RESTORATION PASS\n", "{mode}");
}

#[test]
fn vm_recursive_local_restoration() {
    verify("vm");
}
#[test]
fn interpreter_recursive_local_restoration() {
    verify("interpreter");
}
#[test]
fn jit_recursive_local_restoration() {
    verify("jit");
}
