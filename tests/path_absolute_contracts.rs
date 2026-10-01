use std::{fs, path::Path, process::Command};

#[test]
fn native_absolute_path_syntax_is_pure_and_runtime_consistent() {
    let temporary = tempfile::tempdir().unwrap();
    let fixture = temporary.path().join("absolute.kujo");
    let paths = [
        "",
        "relative",
        "./relative",
        "../relative",
        "/missing/path",
        "C:relative",
        "C:/missing/path",
        r"C:\missing\path",
        r"\relative",
        r"\\server\share\file",
        r"\\?\C:\missing\path",
        "bad\0path",
    ];
    let mut source = String::new();
    for path in paths {
        let expected = !path.contains('\0') && Path::new(path).is_absolute();
        source.push_str(&format!(
            "assert(path_is_absolute(parse_json({}))=={},\"native syntax differs\")\n",
            serde_json::to_string(&serde_json::to_string(path).unwrap()).unwrap(),
            expected
        ));
    }
    source.push_str("print(\"absolute syntax passed\")\n");
    fs::write(&fixture, source).unwrap();
    for interpreter in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&fixture).arg("--untrusted");
        if interpreter {
            command.arg("--interpreter");
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("absolute syntax passed"));
    }
}
