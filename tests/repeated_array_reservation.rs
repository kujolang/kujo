use std::process::Command;

fn verify(mode: &str) {
    for width in [8, 2048] {
        let directory = tempfile::tempdir().unwrap();
        let zeros = vec!["0"; width].join(",");
        let owner = format!(
            r#"
mut state := [{zeros}]
export func snapshot() {{ return state }}
export func update(n, limit) {{
    mut j := 0
    while j < limit {{
        state[j - j] = [n, j]
        state[1] = {{"n": n, "j": j}}
        if j % 2 == 0 {{ state[2] = "even" }} else {{ state[2] = "odd" }}
        j += 1
    }}
    return n
}}
export func aliases(n) {{
    mut j := 0
    while j < 64 {{
        state[0] = [n]
        state[-{width}] = {{"n": n}}
        j += 1
    }}
}}
export func fail_after_writes() {{
    mut j := 0
    while j < 64 {{ state[0] = [j] state[1] = {{"j": j}} j += 1 }}
    state[{width}] = 99
}}
export func dense() {{
    mut j := -{width}
    while j < 1 {{ state[j] = [j] j += 1 }}
}}
"#
        );
        std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
        let source = format!(
            r#"
from owner import snapshot, update, aliases, fail_after_writes, dense
let original := snapshot()
let invoke := update
assert_equal(invoke(1, 1024), 1)
let first := snapshot()
assert_equal(first[0], [1, 1023])
assert_equal(first[1], {{"n": 1, "j": 1023}})
assert_equal(first[2], "odd")
mut n := 2
while n < 12 {{
    let before := snapshot()
    assert_equal(update(n, 64), n)
    let after := snapshot()
    assert_equal(after[0], [n, 63])
    assert_equal(before[0][0], n - 1)
    n += 1
}}
aliases(99)
assert_equal(snapshot()[0], {{"n": 99}})
assert_equal(first[0], [1, 1023])
let before_error := snapshot()
mut caught := false
try {{ fail_after_writes() }} except error {{ caught = true }}
assert(caught)
assert_equal(before_error[0], {{"n": 99}})
assert_equal(snapshot()[0], [63])
assert_equal(snapshot()[1], {{"j": 63}})
let before_dense := snapshot()
dense()
assert_equal(snapshot()[0], [0])
assert_equal(snapshot()[1], [-{width} + 1])
assert_equal(before_dense[0], [63])
mut k := 0
while k < {width} {{ assert_equal(original[k], 0) k += 1 }}
print("REPEATED RESERVATION PASS")
"#
        );
        let path = directory.path().join("main.kujo");
        std::fs::write(&path, source).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(path).output().unwrap();
        assert!(
            output.status.success(),
            "{mode}/{width}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"REPEATED RESERVATION PASS\n");
    }
}
#[test]
fn vm_repeated_reservation_value_semantics() {
    verify("vm");
}
#[test]
fn interpreter_repeated_reservation_value_semantics() {
    verify("interpreter");
}
#[test]
fn jit_repeated_reservation_value_semantics() {
    verify("jit");
}
