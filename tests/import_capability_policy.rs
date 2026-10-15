use std::fs;
use std::process::Command;

#[test]
fn pure_imports_remain_available_without_filesystem_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("payload.kujo"), "export answer := 42\n").unwrap();
    fs::write(dir.path().join("main.kujo"), "from payload import answer\nprint(answer)\n").unwrap();
    for interpreter in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.current_dir(dir.path()).args(["run", "main.kujo", "--untrusted"]);
        if interpreter {
            command.arg("--interpreter");
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
    }
}

#[test]
fn imported_initializers_preserve_caller_capabilities() {
    for interpreter in [false, true] {
        for import in ["import payload", "from payload import answer"] {
            for nested in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                let output_path = dir.path().join("effect.txt");
                let effect = format!(
                    "write_file({}, \"effect\")\nexport answer := 42\n",
                    serde_json::to_string(output_path.to_str().unwrap()).unwrap()
                );
                fs::write(dir.path().join("leaf.kujo"), &effect).unwrap();
                fs::write(
                    dir.path().join("payload.kujo"),
                    if nested { "from leaf import answer\nexport answer\n" } else { &effect },
                )
                .unwrap();
                fs::write(dir.path().join("main.kujo"), format!("{import}\nprint(answer)\n"))
                    .unwrap();
                for allow_write in [false, true] {
                    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
                    command.current_dir(dir.path()).args(["run", "main.kujo", "--untrusted"]);
                    if interpreter {
                        command.arg("--interpreter");
                    }
                    if allow_write {
                        command.arg("--allow-fs-write");
                    }
                    let output = command.output().unwrap();
                    if allow_write {
                        assert!(
                            output.status.success(),
                            "{}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        assert_eq!(fs::read_to_string(&output_path).unwrap(), "effect");
                        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
                    } else {
                        assert_eq!(
                            output.status.code(),
                            Some(4),
                            "{interpreter}/{import}/{nested}: {output:?}"
                        );
                        assert!(
                            String::from_utf8_lossy(&output.stderr).contains("Capability denied")
                        );
                        assert!(!output_path.exists(), "denied initializer produced a host effect");
                    }
                }
            }
        }
    }
}

#[test]
fn cached_initializers_are_not_reused_after_policy_restriction() {
    use kujo::{
        interpreter::{Interpreter, RuntimeCapabilityPolicy, Value},
        lexer::tokenize,
        parser::Parser,
    };
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data.txt");
    fs::write(&data, "private-data").unwrap();
    fs::write(
        dir.path().join("payload.kujo"),
        format!(
            "export data := read_file({})\n",
            serde_json::to_string(data.to_str().unwrap()).unwrap()
        ),
    )
    .unwrap();
    let program = Parser::new(tokenize("from payload import data\n").unwrap()).parse();
    let mut interpreter = Interpreter::new();
    interpreter.module_loader.add_search_path(dir.path());
    interpreter.eval_stmts(&program);
    assert!(interpreter.return_value.is_none());
    interpreter.set_capability_policy(RuntimeCapabilityPolicy::restricted());
    interpreter.eval_stmts(&program);
    assert!(
        matches!(interpreter.return_value, Some(Value::Error(ref message)) if message.contains("Capability denied")),
        "trusted cached exports bypassed restricted initialization"
    );
}

#[test]
fn test_run_preserves_capabilities_in_body_and_hooks() {
    for hook in ["body", "test_setup", "test_teardown"] {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("effect.txt");
        let effect = format!(
            "write_file({}, \"effect\")",
            serde_json::to_string(output_path.to_str().unwrap()).unwrap()
        );
        let source = if hook == "body" {
            format!("test \"policy\" {{ {effect} }}")
        } else {
            format!("{hook} {{ {effect} }}\ntest \"policy\" {{ assert(true) }}")
        };
        let path = dir.path().join("test.kujo");
        fs::write(&path, source).unwrap();
        for allow_write in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.args(["test-run", "--untrusted", "--verbose"]).arg(&path);
            if allow_write {
                command.arg("--allow-fs-write");
            }
            let output = command.output().unwrap();
            if allow_write {
                assert!(output.status.success(), "{hook}: {output:?}");
                assert_eq!(fs::read_to_string(&output_path).unwrap(), "effect");
            } else {
                assert_eq!(output.status.code(), Some(1), "{hook}: {output:?}");
                assert!(!output_path.exists(), "{hook}: denied host effect occurred");
                assert!(String::from_utf8_lossy(&output.stdout).contains("Capability denied"));
            }
        }
    }
}

#[test]
fn parallel_mapper_workers_preserve_capabilities() {
    for interpreter in [false, true] {
        for function in ["parallel_map", "par_map", "par_each"] {
            let dir = tempfile::tempdir().unwrap();
            let output_path = dir.path().join("effect.txt");
            let source = format!("func effect(n) {{ write_file({}, \"effect\"); return n }}\nprint(await {function}([1], effect, 1))\n", serde_json::to_string(output_path.to_str().unwrap()).unwrap());
            let path = dir.path().join("main.kujo");
            fs::write(&path, source).unwrap();
            for allow_write in [false, true] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
                command.arg("run").arg(&path).arg("--untrusted");
                if interpreter {
                    command.arg("--interpreter");
                }
                if allow_write {
                    command.arg("--allow-fs-write");
                }
                let output = command.output().unwrap();
                if allow_write {
                    assert!(output.status.success(), "{function}/{interpreter}: {output:?}");
                    assert_eq!(fs::read_to_string(&output_path).unwrap(), "effect");
                } else {
                    assert!(
                        !output_path.exists(),
                        "{function}/{interpreter}: denied worker wrote a file"
                    );
                    let text = format!(
                        "{}{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    );
                    assert!(
                        text.contains("Capability denied"),
                        "{function}/{interpreter}: {output:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn structured_hook_errors_fail_the_test_run() {
    for hook in ["test_setup", "test_teardown"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.kujo");
        fs::write(
            &path,
            format!("{hook} {{ throw(\"hook-failed\") }}\ntest \"policy\" {{ assert(true) }}"),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_kujo"))
            .args(["test-run", "--untrusted", "--verbose"])
            .arg(path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{hook}: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("hook-failed"),
            "{hook}: {output:?}"
        );
    }
}
