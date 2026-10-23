use std::process::Command;

#[test]
fn native_caller_can_invoke_full_vm_array_helper_and_recover_errors() {
    let mut source = String::from(
        r#"
        mut effects := 0
        func vector(n) {
            mut result := []
            mut i := 0
            effects += 1
            while i < n { result = push(result,i) i += 1 }
            if n < 0 { assert(false, "vector failure") }
            return result
        }
        func wrapper(n) { return vector(n) }
        "#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(wrapper(3),[0,1,2])\n");
    }
    source.push_str(
        r#"
        mut caught := 0
        mut saved := 17
        try { saved = wrapper(-1) } except error { caught += 1 }
        assert_equal(caught,1)
        assert_equal(saved,17)
        assert_equal(effects,201)
        assert_equal(wrapper(2),[0,1])
        assert_equal(effects,202)
        print("PASS")
        "#,
    );
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("cold_callback.kujo");
    std::fs::write(&file, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let metrics = directory.path().join("metrics.json");
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1").arg("--measurements").arg(&metrics);
        }
        let output = command.arg(&file).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            assert!(stderr.contains("Successfully compiled function 'wrapper'"), "{stderr}");
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 100);
        }
    }
}
