use std::fs;
use std::process::Command;

#[test]
fn directory_page_is_available_in_both_runtimes_and_capability_gated() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("records")).unwrap();
    fs::write(root.path().join("records/a.json"), "{}").unwrap();
    fs::write(root.path().join("records/a-b.json"), "{}").unwrap();
    let script = root.path().join("page.kujo");
    fs::write(&script, format!(
        "page := list_dir_beneath({}, \"records\", \"\", \".json\", 1, 100)\nprint(to_json(page))\n",
        serde_json::to_string(root.path().to_str().unwrap()).unwrap()
    )).unwrap();
    for interpreter in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&script);
        if interpreter {
            command.arg("--interpreter");
        }
        let result = command.output().unwrap();
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        let page: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(page["names"], serde_json::json!(["a.json"]));
        assert_eq!(page["next_cursor"], "a");
        assert_eq!(page["truncated"], true);
    }
    let denied = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .arg("run")
        .arg(&script)
        .arg("--untrusted")
        .output()
        .unwrap();
    assert!(!denied.status.success());
    let permitted = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .arg("run")
        .arg(&script)
        .arg("--allow-fs-read")
        .output()
        .unwrap();
    assert!(permitted.status.success(), "{}", String::from_utf8_lossy(&permitted.stderr));
}
