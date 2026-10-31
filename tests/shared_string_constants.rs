use std::process::Command;

#[test]
fn shared_string_constants_preserve_recursive_imported_and_callback_values() {
    let directory = tempfile::tempdir().unwrap();
    let source =
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/shared_string_constants.kujo");
    for mode in ["vm", "interpreter", "jit"] {
        let metrics = directory.path().join("metrics.json");
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").env_remove("DISABLE_FUNCTION_JIT");
        match mode {
            "interpreter" => {
                command.arg("--interpreter");
            }
            "jit" => {
                command.arg("--jit").arg("--measurements").arg(&metrics);
            }
            _ => {}
        }
        let output = command.arg(source).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"PASS\n", "{mode}");
        if mode == "jit" {
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 1);
        }
    }
}
