use std::process::Command;

#[test]
fn synchronous_callbacks_preserve_nested_state_errors_and_capabilities() {
    let fixture =
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/synchronous_callback_state.kujo");
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(fixture).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS", "{mode}");
    }
}

#[cfg(feature = "runtime-jit")]
#[test]
fn preloaded_callbacks_skip_script_compilation_but_keep_callee_jit() {
    let directory = tempfile::tempdir().unwrap();
    let metrics = directory.path().join("callback-metrics.json");
    let probe = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/benchmarks/langarena/probes/sort_callback_setup.kujo"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["run", "--untrusted", "--scheduler-no-timeout", "--jit", "--measurements"])
        .arg(&metrics)
        .arg(probe)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
    let counters = &report["counters"];
    assert!(
        counters["jit_native_function_entries"].as_u64().unwrap() > 0,
        "callee JIT disabled: {report}"
    );
    assert!(
        counters["jit_compile_entries"].as_u64().unwrap() < 16,
        "callback wrapper compilation repeated: {report}"
    );
}
