use std::process::Command;

#[test]
fn debug_call_flag_changes_inside_a_callback_are_visible_on_return() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("debug_transition.kujo");
    std::fs::write(
        &source,
        r#"
func switch_debug() {
    env_set("DEBUG_JIT_CALL", "1")
    return 413
}
func call_switch() {
    mut i := 0
    while i < 2 { i += 1 }
    return switch_debug()
}
assert_equal(call_switch(),413)
print("DEBUG TRANSITION PASS")
"#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let measurements = directory.path().join(format!("{mode}.json"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").env_remove("DEBUG_JIT_CALL").env_remove("DISABLE_FUNCTION_JIT");
        match mode {
            "interpreter" => {
                command.arg("--interpreter");
            }
            "jit" => {
                command.arg("--jit").arg("--measurements").arg(&measurements);
            }
            _ => {}
        }
        let output = command.arg(&source).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode}: {stderr}");
        assert_eq!(output.stdout, b"DEBUG TRANSITION PASS\n", "{mode}");
        if mode == "jit" {
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&measurements).unwrap()).unwrap();
            assert_eq!(report["counters"]["jit_native_function_entries"], 1);
            // The flag was absent at entry and enabled by the callback. A cached
            // process-wide or call-entry flag would omit this return diagnostic.
            assert!(stderr.contains("jit_call_function: result=Int(413)"), "{stderr}");
            assert!(!stderr.contains("jit_call_function: stack before"), "{stderr}");
        }
    }
}
