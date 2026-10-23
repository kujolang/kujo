use std::process::Command;

#[test]
fn native_callback_owned_result_survives_return_and_repeated_calls() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("owned_result.kujo");
    // Unroll calls so this probes whole-function compilation, independently of
    // loop compilation. The final call must run after the hot-call threshold.
    let mut source =
        String::from("func render(x) { return str(x) }\nfunc width(x) { return len(str(x)) }\n");
    for _ in 0..200 {
        source.push_str("assert_equal(render(17), \"17\")\n");
        source.push_str("assert_equal(width(17), 2)\n");
    }
    source.push_str("assert_equal(render(29), \"29\")\nprint(\"PASS\")\n");
    std::fs::write(&file, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let metrics = directory.path().join("metrics.json");
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(flag) = mode {
            command.arg(flag);
        }
        if mode == Some("--jit") {
            command.arg("--measurements").arg(&metrics);
        }
        let output = command.arg(&file).output().unwrap();
        assert!(output.status.success(), "{mode:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS", "{mode:?}");
        if mode == Some("--jit") {
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() > 0);
        }
    }
}
