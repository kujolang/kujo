use std::{fs, process::Command};

#[test]
fn directory_pages_match_vm_interpreter_and_enforce_capabilities() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("entries")).unwrap();
    for name in ["claim-aaa.json", "claim-aaa-child.json", "claim-bbb.json", "ignore.txt"] {
        fs::write(root.path().join("entries").join(name), "").unwrap();
    }
    let script = root.path().join("page.kujo");
    fs::write(&script, "print(to_json(list_dir_page(\"entries\", \"\", 2, \".json\")))\n").unwrap();
    for interpreter in [false, true] {
        let run = |flags: &[&str]| {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.current_dir(root.path()).arg("run");
            if interpreter {
                command.arg("--interpreter");
            }
            command.args(flags).arg(&script).output().unwrap()
        };
        let denied = run(&["--untrusted"]);
        assert!(!denied.status.success());
        assert!(String::from_utf8_lossy(&denied.stderr)
            .contains("filesystem-read required for list_dir_page"));
        let output = run(&["--allow-fs-read"]);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let page: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(page["entries"], serde_json::json!(["claim-aaa-child.json", "claim-aaa.json"]));
        assert_eq!(page["truncated"], true);
        assert_eq!(page["buffered_entries"], 3);
        assert_eq!(page["examined_entries"], 4);
        for bad in ["0", "10001", "1.5", "true", "null"] {
            fs::write(
                &script,
                format!("print(list_dir_page(\"entries\", \"\", {bad}, \".json\"))\n"),
            )
            .unwrap();
            assert!(!run(&["--allow-fs-read"]).status.success());
        }
        fs::write(&script, "print(to_json(list_dir_page(\"entries\", \"\", 2, \".json\")))\n")
            .unwrap();
    }
}
