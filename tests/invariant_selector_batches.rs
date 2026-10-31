use std::process::Command;

#[test]
fn invariant_arithmetic_preserves_snapshots_and_partial_error_publication() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner.kujo"),
        r#"
mut state := [0, 0, 0, 0, 0, 0, 0, 0]
export func snapshot() { return state }
export func product(n, a, b) {
 state[a * b] = n state[a * b + 1] = [n] state[a * b + 2] = {"v": n}
 return n
}
export func quotient(n, a, b) {
 state[a / b] = n state[a / b + 1] = [n] state[a / b + 2] = {"v": n}
 return n
}
export func loop_product(n, a, b) {
 mut j := 0 while j < 3 { state[a * b + j] = [n + j] j += 1 }
 return n
}
export func fail_product(n, a, b) {
 state[0] = n state[(a * b) / b] = [n] state[7] = n
 return n
}
export func fail_quotient(n, a, b) {
 state[0] = n state[a / b] = [n] state[7] = n
 return n
}
export func interior_zero(n) {
 mut j := 0 while j < 3 { state[1 / (j - 1)] = [n] j += 1 }
 return n
}
func observe() { return state[0] }
export func callback(n) {
 state[0] = n state[observe() / 2] = [n] state[2] = n
 return n
}
"#,
    )
    .unwrap();
    let source = directory.path().join("main.kujo");
    std::fs::write(&source, r#"
from owner import snapshot, product, quotient, loop_product, fail_product, fail_quotient, interior_zero, callback
let original := snapshot()
mut n := 1 while n <= 8 {
 assert_equal(product(n, 1, 2), n)
 let saved := snapshot()
 assert_equal(saved[2], n) assert_equal(saved[3], [n]) assert_equal(saved[4], {"v": n})
 assert_equal(quotient(n, -11, 2), n)
 assert_equal(snapshot()[3], n) assert_equal(snapshot()[4], [n]) assert_equal(snapshot()[5], {"v": n})
 assert_equal(quotient(n, 11, -2), n)
 assert_equal(loop_product(n, 1, 2), n)
 assert_equal(snapshot()[2], [n]) assert_equal(snapshot()[3], [n + 1]) assert_equal(snapshot()[4], [n + 2])
 assert_equal(saved[2], n) assert_equal(saved[3], [n]) assert_equal(saved[4], {"v": n})
 n += 1
}
let prior := snapshot()
mut caught := false
try { fail_product(71, 9223372036854775807, 2) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0], 71) assert_equal(snapshot()[7], prior[7])
caught = false
try { fail_quotient(72, 1, 0) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0], 72) assert_equal(snapshot()[7], prior[7])
caught = false
try { fail_quotient(73, -9223372036854775807 - 1, -1) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0], 73) assert_equal(snapshot()[7], prior[7])
caught = false
try { fail_quotient(74, 16, 2) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0], 74) assert_equal(snapshot()[7], prior[7])
caught = false
try { interior_zero(75) } except err { caught = true }
assert(caught) assert_equal(snapshot()[7], [75])
assert_equal(callback(2), 2)
assert_equal(snapshot()[0], 2) assert_equal(snapshot()[1], [2]) assert_equal(snapshot()[2], 2)
assert_equal(original, [0, 0, 0, 0, 0, 0, 0, 0])
print("INVARIANT SELECTOR CONTRACT PASS")
"#).unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&source).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"INVARIANT SELECTOR CONTRACT PASS\n", "{mode}");
    }
}
