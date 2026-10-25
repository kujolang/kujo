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
