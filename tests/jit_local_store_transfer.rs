use std::process::Command;

#[test]
fn owned_local_stores_preserve_recursive_snapshots_branches_and_errors() {
    let source = include_str!("fixtures/jit_local_store_transfer.kujo");
    let path = std::env::temp_dir().join(format!("kujo-local-store-{}.kujo", std::process::id()));
    std::fs::write(&path, source).unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "local-store-transfer: OK");
    }
    std::fs::remove_file(path).unwrap();
}
