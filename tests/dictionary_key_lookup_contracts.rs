//! Imported dictionary reads must preserve key identity, nulls and failures.
use std::process::Command;

fn verify(mode: &str) {
    let directory = tempfile::tempdir().unwrap();
    let state = format!(
        r#"
const long_key := "{}"
mut data := {{long_key: 7, "": 2, "null": null, "01": 3, "+1": 4, "-9223372036854775808": 5, "9223372036854775807": 6}}
export func lookup(key) {{ return data[key] }}
export func key() {{ return long_key }}
mut rows := {{long_key: [0, 1], "": [2], "01": [3], "+1": [4], "-9223372036854775808": [5], "9223372036854775807": [6]}}
export func snapshot() {{ return rows }}
export func edit(key, index, value) {{ rows[key][index] = value }}
export func replace(key, value) {{ rows[key] = value }}
export func edit_then_fail(key) {{ rows[key][0] = 99 rows["missing"][0] = 1 }}
"#,
        "k".repeat(1024)
    );
    std::fs::write(directory.path().join("state.kujo"), state).unwrap();
    let source = r#"
from state import lookup, key, snapshot, edit, replace, edit_then_fail
let long_key := key()
mut i := 0
while i < 64 { assert_equal(lookup(long_key), 7) i += 1 }
assert_equal(lookup(""), 2)
assert_equal(lookup("null"), null)
assert_equal(lookup("01"), 3)
assert_equal(lookup("+1"), 4)
assert_equal(lookup(parse_int("-9223372036854775808")), 5)
assert_equal(lookup(9223372036854775807), 6)
mut caught := 0
try { lookup(1) } except e { caught += 1 }
try { lookup("missing\n\"key") } except e { caught += 1 }
try { lookup(1.5) } except e { caught += 1 }
assert_equal(caught, 3)
assert_equal(lookup(long_key), 7)
let old := snapshot()
edit(long_key, 0, 8)
edit("", 0, 9)
edit(parse_int("-9223372036854775808"), 0, 10)
edit(9223372036854775807, 0, 11)
assert_equal(old[long_key][0], 0)
assert_equal(old[""][0], 2)
assert_equal(snapshot()[long_key][0], 8)
assert_equal(snapshot()["-9223372036854775808"][0], 10)
assert_equal(snapshot()[9223372036854775807][0], 11)
let before_failure := snapshot()
mut write_errors := 0
try { edit("missing", 0, 0) } except e { write_errors += 1 }
try { edit(1, 0, 0) } except e { write_errors += 1 }
try { edit(1.5, 0, 0) } except e { write_errors += 1 }
try { edit(long_key, 9, 0) } except e { write_errors += 1 }
assert_equal(write_errors, 4)
assert_equal(snapshot()[long_key], before_failure[long_key])
replace("new", [12])
replace(1, [13])
replace(long_key, [14])
assert_equal(before_failure[long_key][0], 8)
assert_equal(snapshot()[long_key][0], 14)
assert_equal(snapshot()["new"][0], 12)
assert_equal(snapshot()["1"][0], 13)
assert_equal(snapshot()["01"][0], 3)
assert_equal(snapshot()["+1"][0], 4)
try { edit_then_fail(long_key) } except e { write_errors += 1 }
assert_equal(write_errors, 5)
assert_equal(snapshot()[long_key][0], 99)
print("DICTIONARY LOOKUP PASS")
"#;
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(path).arg("--untrusted");
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"DICTIONARY LOOKUP PASS\n", "{mode}");
}

#[test]
fn vm_dictionary_keys() {
    verify("vm");
}
#[test]
fn interpreter_dictionary_keys() {
    verify("interpreter");
}
#[test]
fn jit_dictionary_keys() {
    verify("jit");
}
