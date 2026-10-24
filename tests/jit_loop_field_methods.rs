use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn native_loop_resumes_before_creating_method_receivers() {
    check(
        r#"
func once() {
    let ch := channel()
    let parser := arg_parser()
    let expected := parser.help()
    mut i := 0
    mut total := 0
    while i < 200 {
        ch.send(7)
        total += ch.receive()
        assert_equal(parser.help(),expected)
        i += 1
    }
    return total
}
assert_equal(once(),1400)
print("PASS")
"#
        .into(),
    );
}

fn check(source: String) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("field_methods.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [Some("--interpreter"), None, Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command
            .args(["run", "--scheduler-no-timeout"])
            .env("DISABLE_FUNCTION_JIT", "1")
            .env("DEBUG_JIT_LOOP", "1");
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
            assert!(stderr.contains("Executing compiled loop"), "{stderr}");
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_boundary_entries"].as_u64().unwrap() > 0);
        }
    }
}
