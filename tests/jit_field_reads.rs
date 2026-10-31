use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn local_field_reads_preserve_integer_dictionary_loop_admission() {
    // VM field lookup supports integer dictionaries. The interpreter does not
    // promise dictionary dot access; its portable dictionary syntax is indexing.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("mixed_fields.kujo");
    std::fs::write(
        &path,
        r#"
func once() {
    mut data := {}
    data[0] = 7
    data[-1] = 1
    mut i := 0
    mut sum := 0
    while i < 200 {
        sum += data[0]
        assert_equal(data.missing,null)
        data[0] = 7
        i += 1
    }
    return sum
}
assert_equal(once(),1400)
print("PASS")
"#,
    )
    .unwrap();
    for mode in ["vm", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command
            .args(["run", "--scheduler-no-timeout"])
            .env("DISABLE_FUNCTION_JIT", "1")
            .env("DEBUG_JIT_LOOP", "1");
        if mode == "jit" {
            command.arg("--jit");
        }
        let output = command.arg(&path).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode}: {stderr}");
        assert_eq!(output.stdout, b"PASS\n");
        if mode == "jit" {
            assert!(stderr.contains("Executing compiled loop"), "{stderr}");
        }
    }
}

#[test]
fn local_field_reads_survive_receiver_replacement_and_return() {
    for name_length in [8, 256] {
        let name = format!("T{}", "x".repeat(name_length - 1));
        let mut source = format!(
            r#"
struct {name} {{ value: any }}
func replace_local(item) {{
    let before := item.value
    let original := item
    item = {name} {{ value: [9,10] }}
    mut after := item.value
    after[0] = 99
    assert_equal(original.value,before)
    assert_equal(item.value,[9,10])
    return [before,after,item.value]
}}
let seed := [7,8]
let holder := {name} {{ value: seed }}
"#
        );
        for _ in 0..200 {
            source.push_str("assert_equal(replace_local(holder),[[7,8],[99,10],[9,10]])\n");
        }
        source.push_str(
            r#"
mut returned := replace_local(holder)
returned[0][0] = 123
returned[2][0] = 456
assert_equal(returned,[[123,8],[99,10],[456,10]])
assert_equal(holder.value,[7,8])
assert_equal(seed,[7,8])
print("PASS")
"#,
        );
        check(source, &["replace_local"]);
    }
}

#[test]
fn native_field_reads_preserve_types_snapshots_and_errors() {
    let mut source = String::from(
        r#"
struct Holder { value: any }
mut effects := 0
func read(item) { effects += 1 return item.value }
let seed := [7,8]
let holder := Holder { value: seed }
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(read(holder),seed)\n");
    }
    source.push_str(
        r#"
mut result := read(holder)
result[0] = 99
assert_equal(seed,[7,8])
assert_equal(holder.value,[7,8])
assert_equal(read(Holder { value: true }),true)
assert_equal(read(Holder { value: null }),null)
assert_equal(read(Holder { value: 2.5 }),2.5)
assert_equal(read(Holder { value: bytes([0,255]) }),bytes([0,255]))
mut target := 17
mut caught := 0
try { target = read(3) } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(target,17)
assert_equal(effects,206)
assert_equal(read(holder),seed)
print("PASS")
"#,
    );
    check(source, &["read"]);
}

#[test]
fn native_field_methods_preserve_receiver_and_recover() {
    let mut source = String::from(
        r#"
func roundtrip(ch,x) { ch.send(x) return ch.receive() }
func describe(parser) { return parser.help() }
func broken(ch,x) { if x < 0 { return ch.missing() } return ch.receive() }
let ch := channel()
let parser := arg_parser()
let expected := parser.help()
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(roundtrip(ch,7),7)\nassert_equal(describe(parser),expected)\nch.send(9)\nassert_equal(broken(ch,0),9)\n");
    }
    source.push_str(
        r#"
mut caught := 0
mut target := 17
try { target = broken(ch,-1) } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(target,17)
assert_equal(roundtrip(ch,8),8)
assert_equal(describe(parser),expected)
print("PASS")
"#,
    );
    check(source, &["roundtrip", "describe", "broken"]);
}

fn check(source: String, names: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("field_methods.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [Some("--interpreter"), None, Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let metrics = directory.path().join("metrics.json");
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1").arg("--measurements").arg(&metrics);
        }
        let mut child =
            command.arg(&path).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(30) {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!("{mode:?} timed out: {}", String::from_utf8_lossy(&output.stderr));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            for name in names {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 100);
        }
    }
}

#[test]
fn native_loop_field_reads_persist_owned_results() {
    for (initial, replacement, expected) in [("[-1,false]", "holder.value", "[199,true]")] {
        for fail in [false, true] {
            let failure = if fail { "if i == 150 { assert_equal(10/(i-150),0) }" } else { "" };
            let expected = if fail { expected.replace("199", "150") } else { expected.to_owned() };
            let caught = i32::from(fail);
            let source = format!(
                r#"
struct Holder {{ value: any }}
func once() {{
    mut i := 0
    mut last := {initial}
    let snapshot := last
    mut caught := 0
    try {{
        while i < 200 {{ let holder := Holder {{ value: [i,true] }} last = {replacement} {failure} i += 1 }}
    }} except error {{ caught += 1 }}
    return [last,snapshot,caught]
}}
let result := once()
assert_equal(result[0],{expected})
assert_equal(result[1],{initial})
assert_equal(result[2],{caught})
print("PASS")
"#
            );
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("rebind.kujo");
            std::fs::write(&path, source).unwrap();
            for mode in [Some("--interpreter"), None, Some("--jit")] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
                command.args(["run", "--scheduler-no-timeout"]);
                if let Some(mode) = mode {
                    command.arg(mode);
                }
                command.env("DISABLE_FUNCTION_JIT", "1").env("DEBUG_JIT_LOOP", "1");
                let output = command.arg(&path).output().unwrap();
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(output.status.success(), "{mode:?} {replacement}, fail={fail}: {stderr}");
                assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
                if mode == Some("--jit") {
                    assert!(stderr.contains("Executing compiled loop"), "{stderr}");
                }
            }
        }
    }
}
