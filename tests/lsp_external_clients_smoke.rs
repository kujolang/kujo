use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kujo_binary() -> String {
    env!("CARGO_BIN_EXE_kujo").to_string()
}

#[test]
fn python_external_client_can_launch_kujo_lsp() {
    let script = root().join("tools/lsp_smoke_clients/python_client.py");
    let output = Command::new("python3")
        .arg(script)
        .arg(kujo_binary())
        .output()
        .expect("failed to run python lsp client");

    assert!(
        output.status.success(),
        "python client failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn node_external_client_can_launch_kujo_lsp() {
    if Command::new("node").arg("--version").output().is_err() {
        return;
    }

    let script = root().join("tools/lsp_smoke_clients/node_client.mjs");
    let output = Command::new("node")
        .arg(script)
        .arg(kujo_binary())
        .output()
        .expect("failed to run node lsp client");

    assert!(
        output.status.success(),
        "node client failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
