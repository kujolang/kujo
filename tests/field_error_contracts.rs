use std::process::Command;

#[test]
fn field_errors_are_catchable_without_losing_targets_or_replaying_effects() {
    let mut source = String::from(
        r#"
        struct Box { value: int }
        struct Empty { other: int }
        mut before := 0
        mut after := 0
        func field(x) { before += 1 let result := x.value after += 1 return result }
        func relay(x) { return field(x) }
        let seed := Box { value: 7 }
    "#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(relay(seed),7)\n");
    }
    source.push_str(
        r#"
        mut target := 17
        mut caught := 0
        for bad in [Empty { other: 0 }, 7, null] {
            try { target = bad.value } except error { caught += 1 }
            assert_equal(target,17)
            try { target = field(bad) } except error { caught += 1 }
            assert_equal(target,17)
            try { target = relay(bad) } except error { caught += 1 }
            assert_equal(target,17)
        }
        assert_equal(caught,9)
        assert_equal(before,206)
        assert_equal(after,200)
        assert_equal(relay(seed),7)
        assert_equal(before,207)
        assert_equal(after,201)
        assert_equal(seed.value,7)
        print("PASS")
    "#,
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fields.kujo");
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
        let output = command.arg(&path).output().unwrap();
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
