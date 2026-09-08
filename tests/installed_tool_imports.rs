use std::fs;
use std::process::Command;

#[test]
fn isolated_tools_preserve_cwd_without_loading_caller_modules() {
    let temp = tempfile::tempdir().unwrap();
    let caller = temp.path().join("caller");
    let tool = temp.path().join("tool");
    fs::create_dir_all(&caller).unwrap();
    fs::create_dir_all(&tool).unwrap();
    fs::write(caller.join("helper.kujo"), "export value := \"caller\"\n").unwrap();
    fs::write(tool.join("helper.kujo"), "export value := \"installed\"\n").unwrap();
    fs::write(
        tool.join("main.kujo"),
        "from helper import value\nprint(value)\nprint(os_getcwd())\nprint(to_json(args()))\n",
    )
    .unwrap();
    for interpreter in [false, true] {
        for isolated in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command
                .current_dir(&caller)
                .env_remove("KUJO_ISOLATED_IMPORTS")
                .env_remove("KUJO_SCRIPT_ARGS_JSON")
                .env_remove("KUJO_SCRIPT_ARGS")
                .env("KUJO_MODULE_PATH", &tool)
                .arg("run")
                .arg(tool.join("main.kujo"));
            if interpreter {
                command.arg("--interpreter");
            }
            if isolated {
                command.arg("--isolated-imports");
            }
            command.args(["--", "space argument", "a\u{1f}b"]);
            let output = command.output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let text = String::from_utf8(output.stdout).unwrap();
            let lines: Vec<_> = text.lines().collect();
            assert_eq!(lines[0], if isolated { "installed" } else { "caller" });
            assert_eq!(lines[1], caller.canonicalize().unwrap().to_str().unwrap());
            if isolated {
                let args: Vec<String> = serde_json::from_str(lines[2]).unwrap();
                assert_eq!(args, vec!["space argument", "a\u{1f}b"]);
            }
        }
    }
}

#[test]
fn isolated_tools_do_not_inherit_caller_lockfile_modules_or_script_arguments() {
    let temp = tempfile::tempdir().unwrap();
    let caller = temp.path().join("caller");
    let tool = temp.path().join("tool");
    fs::create_dir_all(caller.join("kennel_packages/poison")).unwrap();
    fs::create_dir_all(&tool).unwrap();
    fs::write(caller.join("kennel.toml"), "[package]\nname=\"caller\"\nversion=\"1.0.0\"\n")
        .unwrap();
    fs::write(
        caller.join("kennel.lock"),
        "version=1\n[[package]]\nname=\"poison\"\ninstall_path=\"kennel_packages/poison\"\n",
    )
    .unwrap();
    fs::write(caller.join("kennel_packages/poison/forbidden.kujo"), "export value := 1\n").unwrap();
    fs::write(tool.join("main.kujo"), "from forbidden import value\nprint(value)\n").unwrap();
    for interpreter in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command
            .current_dir(&caller)
            .env("KUJO_ISOLATED_IMPORTS", "1")
            .env_remove("KUJO_MODULE_PATH")
            .arg("run")
            .arg(tool.join("main.kujo"));
        if interpreter {
            command.arg("--interpreter");
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("forbidden"));
    }
    fs::write(tool.join("main.kujo"), "print(to_json(args()))\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
        .current_dir(&caller)
        .env("KUJO_SCRIPT_ARGS_JSON", "[\"inherited\"]")
        .args(["run", "--isolated-imports", "--interpreter"])
        .arg(tool.join("main.kujo"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "[]");
}
