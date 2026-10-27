use std::process::Command;

fn verify(mode: &str, shared_helper: bool) {
    let directory = tempfile::tempdir().unwrap();
    let declarations = (0..128).map(|i| format!("mut extra_{i} := {i}\n")).collect::<String>();
    let checks = (0..128).map(|i| format!("sum += extra_{i}\n")).collect::<String>();
    let recurse = if shared_helper { "bounce" } else { "invoke" };
    let owner = format!(
        r#"
from bridge import invoke
mut owner_calls := 0
mut helper_calls := 0
mut helper_validations := 0
export func owner_state() {{ return [owner_calls, helper_calls, helper_validations] }}
func relay(f, n, fail) {{ return invoke(f, n, fail) }}
func bounce(f, n, fail) {{
    helper_calls += 1
    let saved_n := n
    mut owner_calls := [n]
    let snapshot := owner_calls
    try {{
        let result := relay(f, n, fail)
        assert_equal(n, saved_n)
        owner_calls[0] = 999
        assert_equal(snapshot[0], saved_n)
        helper_validations += 1
        return result
    }} except err {{
        assert_equal(n, saved_n)
        assert_equal(snapshot[0], saved_n)
        helper_validations += 1
        throw(err)
    }}
}}
export func factory() {{
    mut counter := 0
    mut validated := 0
    mut data := [0, 1]
    const fixed := [7]
    {declarations}
    func step(depth, should_fail) {{
        owner_calls += 1
        let entry := counter
        let original_depth := depth
        let saved := data
        counter += 1
        data[0] += 1
        if depth > 0 {{
            try {{ {recurse}(step, depth - 1, should_fail) }} except err {{}}
        }} else if should_fail {{ throw("leaf") }}
        assert_equal(depth, original_depth)
        assert_equal(saved[0], entry)
        assert_equal(data[1], 1)
        mut rejected := false
        try {{ fixed[0] = 99 }} except err {{ rejected = true }}
        assert(rejected)
        mut sum := 0
        {checks}
        assert_equal(owner_calls, counter)
        validated += 1
        return [counter, data[0], sum, fixed[0], validated, helper_calls, helper_validations]
    }}
    return step
}}
"#
    );
    let bridge = r#"
mut invocations := 0
mut bridge_validated := 0
export func bridge_validations() { return bridge_validated }
export func bridge_count() { return invocations }
export func invoke(f, n, fail) {
    invocations += 1
    let saved_n := n
    let before := invocations
    mut row := [n]
    let saved := row
    try {
        let result := f(n, fail)
        assert_equal(n, saved_n)
        assert_equal(row[0], saved_n)
        assert(invocations >= before)
        row[0] = 999
        assert_equal(saved[0], saved_n)
        bridge_validated += 1
        return result
    } except err {
        assert_equal(n, saved_n)
        assert_equal(row[0], saved_n)
        assert(invocations >= before)
        bridge_validated += 1
        throw(err)
    }
}
"#;
    let source = r#"
from owner import factory, owner_state
from bridge import bridge_count, bridge_validations
let step := factory()
let alias := step
let separate := factory()
assert_equal(step(2, false), [3, 3, 8128, 7, 3, HELPER_2, HELPER_2])
assert_equal(alias(2, true), [6, 6, 8128, 7, 5, HELPER_4, HELPER_4])
assert_equal(separate(1, false), [2, 2, 8128, 7, 2, HELPER_1, HELPER_1])
assert_equal(step(0, false), [7, 7, 8128, 7, 6, HELPER_4, HELPER_4])
assert_equal(step(16, false), [24, 24, 8128, 7, 23, HELPER_20, HELPER_20])
assert_equal(alias(16, true), [41, 41, 8128, 7, 39, HELPER_36, HELPER_36])
assert_equal(separate(0, false), [3, 3, 8128, 7, 3, HELPER_1, HELPER_1])
assert_equal(bridge_count(), 37)
assert_equal(bridge_validations(), 37)
// Independently created function captures retain their original global snapshot.
assert_equal(owner_state(), [0, 0, 0])
print("FOREIGN LOCAL PREFIX PASS")
"#;
    let mut source = source.to_string();
    for count in [36, 20, 4, 2, 1] {
        source = source.replace(
            &format!("HELPER_{count}"),
            &if shared_helper { count } else { 0 }.to_string(),
        );
    }
    std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
    std::fs::write(directory.path().join("bridge.kujo"), bridge).unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--untrusted"]);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.arg(path).output().unwrap();
    assert!(
        output.status.success(),
        "{mode} shared={shared_helper}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"FOREIGN LOCAL PREFIX PASS\n");
}

#[test]
fn vm_foreign_local_prefix() {
    verify("vm", false);
}
#[test]
fn interpreter_foreign_local_prefix() {
    verify("interpreter", false);
}
#[test]
fn jit_foreign_local_prefix() {
    verify("jit", false);
}
#[test]
fn vm_shared_helper_foreign_local_prefix() {
    verify("vm", true);
}
#[test]
fn interpreter_shared_helper_foreign_local_prefix() {
    verify("interpreter", true);
}
#[test]
fn jit_shared_helper_foreign_local_prefix() {
    verify("jit", true);
}
