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
"#,
        "k".repeat(1024)
    );
    std::fs::write(directory.path().join("state.kujo"), state).unwrap();
    let source = r#"
from state import lookup, key
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
