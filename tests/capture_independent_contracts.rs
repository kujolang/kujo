//! Capture setup elision must preserve lexical resolution, COW and error unwinding.
use std::process::Command;

fn verify(runtime: &str) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("helper.kujo"),
        include_str!("fixtures/capture_independent/helper.kujo"),
    )
    .unwrap();
    let main = dir.path().join("main.kujo");
    std::fs::write(&main, include_str!("fixtures/capture_independent/main.kujo")).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(main);
    if runtime != "vm" {
        command.arg(format!("--{runtime}"));
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{runtime}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "CAPTURE INDEPENDENCE PASS\n", "{runtime}");
}

#[test]
fn vm_capture_independence_preserves_semantics() {
    verify("vm");
}
#[test]
fn interpreter_capture_independence_preserves_semantics() {
    verify("interpreter");
}
#[test]
fn jit_capture_independence_preserves_semantics() {
    verify("jit");
}
