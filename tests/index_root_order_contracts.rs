// User-approved root-after-index semantics. The imported-native-default candidate
// is verified separately; these regressions accompany the subsequent compiler fix.
use std::process::Command;

#[test]
fn indexed_assignment_reads_replacement_after_index_in_all_runtimes() {
    let cases: &[(&str, &str)] = &[
        (
            "replacement",
            r#"mut data := [1,2,3]
let saved := data
func replace_index() { data = [9,9,9] return 0 }
data[replace_index()] = 42
assert_equal(data,[42,9,9])
assert_equal(saved,[1,2,3])
print("ROOT ORDER PASS")
"#,
        ),
        (
            "replacement_grows",
            r#"mut data := [1]
func replace_index() { data = [9,9,9] return 2 }
data[replace_index()] = 42
assert_equal(data,[9,9,42])
print("ROOT ORDER PASS")
"#,
        ),
        (
            "replacement_shrinks",
            r#"mut data := [1,2,3]
func replace_index() { data = [9] return 2 }
mut caught := false
try { data[replace_index()] = 42 } except error { caught = true }
assert(caught)
assert_equal(data,[9])
print("ROOT ORDER PASS")
"#,
        ),
        (
            "rhs_then_index",
            r#"mut data := [1,2,3]
mut trace := []
func rhs() { trace = push(trace,1) return 42 }
func replace_index() { trace = push(trace,2) data = [9,9,9] return -1 }
data[replace_index()] = rhs()
assert_equal(trace,[1,2])
assert_equal(data,[9,9,42])
print("ROOT ORDER PASS")
"#,
        ),
        (
            "dictionary_replacement",
            r#"mut data := {"a":1,"b":2}
let saved := data
func replace_index() { data = {"a":9,"c":9} return "a" }
data[replace_index()] = 42
assert_equal(data,{"a":42,"c":9})
assert_equal(saved,{"a":1,"b":2})
print("ROOT ORDER PASS")
"#,
        ),
        (
            "bytes_replacement",
            r#"mut data := bytes([1,2,3])
let saved := data
func replace_index() { data = bytes([9,9,9]) return -1 }
data[replace_index()] = 42
assert_equal(data,bytes([9,9,42]))
assert_equal(saved,bytes([1,2,3]))
print("ROOT ORDER PASS")
"#,
        ),
        (
            "immutable_index_effect",
            r#"let data := [1]
mut calls := 0
func index() { calls += 1 return 0 }
mut caught := false
try { data[index()] = 42 } except err { caught = true }
assert(caught)
assert_equal(calls,1)
assert_equal(data,[1])
print("ROOT ORDER PASS")
"#,
        ),
        (
            "throwing_index_keeps_replacement",
            r#"mut data := [1,2,3]
let saved := data
func index() { data = [9] throw("index failed") }
mut caught := false
try { data[index()] = 42 } except err { caught = true }
assert(caught)
assert_equal(data,[9])
assert_equal(saved,[1,2,3])
print("ROOT ORDER PASS")
"#,
        ),
        (
            "compound_read_and_write_order",
            r#"mut data := [1,2]
        mut trace := []
        mut calls := 0
        func index() { calls += 1 trace = push(trace,1) data = [calls * 10,calls] return 0 }
        func rhs() { trace = push(trace,2) return 2 }
        data[index()] += rhs()
        assert_equal(calls,2)
        assert_equal(trace,[1,2,1])
        assert_equal(data,[3,2])
        print("ROOT ORDER PASS")
        "#,
        ),
        (
            "scoped_global_replacement",
            r#"mut data := [1,2,3]
func index() { data = [9,9,9] return 0 }
func write() { data[index()] = 42 }
mut count := 0
while count < 256 { write() assert_equal(data,[42,9,9]) count += 1 }
print("ROOT ORDER PASS")
"#,
        ),
    ];
    let dir = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for (name, source) in cases {
        let path = dir.path().join(format!("{name}.kujo"));
        std::fs::write(&path, source).unwrap();
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.args(["run", "--untrusted"]);
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command.arg(&path).output().unwrap();
            if !output.status.success()
                || !String::from_utf8_lossy(&output.stdout).contains("ROOT ORDER PASS")
            {
                failures
                    .push(format!("{name}/{mode}: {}", String::from_utf8_lossy(&output.stderr)));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
