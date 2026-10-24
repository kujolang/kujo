use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn native_caller_preserves_channel_field_markers_and_error_recovery() {
    let mut source = String::from(
        r#"
mut before := 0
mut after := 0
func roundtrip(ch,x) {
    before += 1
    if x < 0 { return ch.missing() }
    ch.send(x)
    let result := ch.receive()
    after += 1
    return result
}
func plain(ch,x) { return x }
func relay(operation,ch,x) { return operation(ch,x) }
let ch := channel()
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(relay(plain,ch,7),7)\n");
    }
    source.push_str(
        r#"
assert_equal(before,0)
assert_equal(after,0)
assert_equal(relay(roundtrip,ch,7),7)
assert_equal(before,1)
assert_equal(after,1)
mut target := 17
mut caught := 0
try { target = relay(roundtrip,ch,-1) } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(target,17)
assert_equal(before,2)
assert_equal(after,1)
assert_equal(relay(roundtrip,ch,9),9)
assert_equal(before,3)
assert_equal(after,2)
print("PASS")
"#,
    );
    check(source);
}

#[test]
fn native_caller_preserves_arg_parser_field_markers() {
    let mut source = String::from(
        r#"
func describe(parser) { return parser.help() }
func plain(parser) { return "warm" }
func relay(operation,parser) { return operation(parser) }
let parser := arg_parser()
let expected := parser.help()
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(relay(plain,parser),\"warm\")\n");
    }
    source.push_str("assert_equal(relay(describe,parser),expected)\nassert_equal(parser.help(),expected)\nprint(\"PASS\")\n");
    check(source);
}

fn check(source: String) {
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
            assert!(stderr.contains("Successfully compiled function 'relay'"), "{stderr}");
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 100);
        }
    }
}
