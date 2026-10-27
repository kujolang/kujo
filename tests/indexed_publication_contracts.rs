//! Contracts for deferred indexed publication candidates. These pass on the
//! ordinary-copy runtime and must remain true when allocation is optimized.
use std::process::Command;

const SCALARS: (&str, &str) = (
    r#"
mut data := range(0, 128)
export func dense() { data[0] = 0 data[1] = 1 data[2] = 2 data[3] = 3 data[4] = 4 data[5] = 5 return data[5] }
export func nested_write() { data[0] = 8 data[1] = [3, 4] return data[1][1] }
export func edit() {
    data[0] = 10
    data[1] = 2.5
    data[2] = true
    data[3] = "kept"
    let middle := data
    data[0] += 1
    data[4] = [9, 10]
    data[5] = null
    return [middle, data]
}
export func snapshot() { return data }
"#,
    r#"
from state import edit, dense, nested_write, snapshot
edit()
assert_equal(dense(), 5)
assert_equal(nested_write(), 4)
let result := edit()
assert_equal(result[0][0], 10)
assert_equal(result[1][0], 11)
assert_equal(result[0][1], 2.5)
assert_equal(result[0][2], true)
assert_equal(result[0][3], "kept")
assert_equal(result[1][4][1], 10)
assert_equal(result[1][5], null)
assert_equal(result[1][127], 127)
let saved := snapshot()
edit()
assert_equal(saved[0], 11)
assert_equal(result[0][0], 10)
"#,
);

const INDEX_EFFECTS: (&str, &str) = (
    r#"
mut data := range(0, 128)
export func warm() { data[127] += 1 }
func replace_index() { data = range(40, 168) return 0 }
export func index_forms() { data[-1] = 77 data[0] = 6 return [data[-1], data[0.9]] }
export func read_replaced() { data[0] = 7 return data[replace_index()] }
export func assign_replaced() { data[0] = 5 data[replace_index()] = 99 return data }
export func snapshot() { return data }
"#,
    r#"
from state import warm, index_forms, read_replaced, assign_replaced, snapshot
warm()
let forms := index_forms()
assert_equal(forms[0], 77)
assert_equal(forms[1], 6)
assert_equal(read_replaced(), 7)
assert_equal(snapshot()[0], 40)
assert_equal(assign_replaced()[0], 99)
assert_equal(snapshot()[1], 41)
assert_equal(read_replaced(), 7)
assert_equal(snapshot()[0], 40)
"#,
);

const ERRORS: (&str, &str) = (
    r#"
mut data := range(0, 128)
let fixed := [4, 5]
export func warm() { data[127] += 1 }
export func bounds() { data[0] = 11 data[999] = 3 }
export func bad_type() { data[0] = 12 data["bad"] = 3 }
export func immutable() { data[0] = 13 fixed[0] = 3 }
export func thrown() { data[0] = 14 throw("after write") }
export func snapshot() { return data }
export func fixed_snapshot() { return fixed }
"#,
    r#"
from state import warm, bounds, bad_type, immutable, thrown, snapshot, fixed_snapshot
warm()
mut caught := 0
try { bounds() } except e { caught += 1 }
assert_equal(snapshot()[0], 11)
try { bad_type() } except e { caught += 1 }
assert_equal(snapshot()[0], 12)
try { immutable() } except e { caught += 1 }
assert_equal(snapshot()[0], 13)
assert_equal(fixed_snapshot()[0], 4)
let old := snapshot()
try { thrown() } except e { caught += 1 }
assert_equal(snapshot()[0], 14)
assert_equal(old[0], 13)
assert_equal(caught, 4)
assert_equal(snapshot()[127], 128)
"#,
);

const NATIVE_AND_CAPTURE: (&str, &str) = (
    r#"
mut data := range(0, 128)
func plus_two(x) { return x + 2 }
export func warm() { data[127] += 1 }
export func native_round() {
    data[0] += 1
    let count := len(data)
    let old := data
    data = push(data, 900)
    let mapped := map(data, plus_two)
    data[0] += 1
    return [count, old, mapped, data]
}
export func capture_round() {
    data[0] = 40
    let saved := data
    func read_saved() { return saved }
    data[0] = 41
    return read_saved
}
export func snapshot() { return data }
"#,
    r#"
from state import warm, native_round, capture_round, snapshot
warm()
let first := native_round()
assert_equal(first[0], 128)
assert_equal(first[1][0], 1)
assert_equal(first[2][0], 3)
assert_equal(first[2][128], 902)
assert_equal(first[3][0], 2)
assert_equal(len(first[1]), 128)
let reader := capture_round()
assert_equal(reader()[0], 40)
assert_equal(snapshot()[0], 41)
native_round()
assert_equal(reader()[0], 40)
assert_equal(first[1][0], 1)
"#,
);

const SHADOW_AND_NESTED: (&str, &str) = (
    r#"
mut data := range(0, 128)
mut nested := {"rows": [range(0, 128), range(100, 228)]}
export func warm() { data[127] += 1 }
export func edit() {
    data[0] = 20
    if true {
        mut data := [3, 4]
        data[0] = 8
        assert_equal(data[0], 8)
    }
    nested["rows"][0][0] = 9
    let old := nested
    nested["rows"][0][1] = 10
    return [data, old, nested]
}
export func shadow_error() {
    data[0] = 30
    let data := [1, 2]
    data[0] = 7
}
export func snapshot() { return data }
"#,
    r#"
from state import warm, edit, shadow_error, snapshot
warm()
let result := edit()
assert_equal(result[0][0], 20)
assert_equal(result[1]["rows"][0][0], 9)
assert_equal(result[1]["rows"][0][1], 1)
assert_equal(result[2]["rows"][0][1], 10)
assert_equal(result[2]["rows"][1][0], 100)
mut caught := false
try { shadow_error() } except e { caught = true }
assert_equal(caught, true)
assert_equal(snapshot()[0], 30)
assert_equal(result[0][0], 20)
"#,
);

fn verify(mode: &str, case: (&str, &str)) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("state.kujo"), case.0).unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, format!("{}\nprint(\"INDEXED PUBLICATION PASS\")\n", case.1)).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(path).arg("--untrusted");
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"INDEXED PUBLICATION PASS\n", "{mode}");
}

macro_rules! contracts {
    ($vm:ident, $interpreter:ident, $jit:ident, $case:ident) => {
        #[test]
        fn $vm() {
            verify("vm", $case);
        }
        #[test]
        fn $interpreter() {
            verify("interpreter", $case);
        }
        #[test]
        fn $jit() {
            verify("jit", $case);
        }
    };
}
contracts!(vm_scalars, interpreter_scalars, jit_scalars, SCALARS);
contracts!(vm_index_effects, interpreter_index_effects, jit_index_effects, INDEX_EFFECTS);
contracts!(vm_partial_errors, interpreter_partial_errors, jit_partial_errors, ERRORS);
contracts!(
    vm_native_and_capture,
    interpreter_native_and_capture,
    jit_native_and_capture,
    NATIVE_AND_CAPTURE
);
contracts!(
    vm_shadow_and_nested,
    interpreter_shadow_and_nested,
    jit_shadow_and_nested,
    SHADOW_AND_NESTED
);
