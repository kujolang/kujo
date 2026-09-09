#![cfg(unix)]
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn run(source: &str, interpreter: bool) -> std::process::Output {
    let dir = tempdir().unwrap();
    let script = dir.path().join("test.kujo");
    fs::write(&script, source).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kujo"));
    cmd.arg("run").arg(script).arg("--isolated-imports");
    if interpreter {
        cmd.arg("--interpreter");
    }
    cmd.current_dir(dir.path()).output().unwrap()
}
#[test]
fn native_lock_and_symlink_contract_both_runtimes() {
    let script = r#"
write_file("target", "original")
assert_true(path_owned("target"))
symlink_atomic("target", "./current")
assert_equal(read_file("current"), "original")
a := file_lock("lock", 0)
failed := false
try { file_lock("lock", 0) } except err { failed = true }
assert_true(failed)
file_unlock(a)
b := file_lock("lock", 0)
file_unlock(b)
failed = false
try { file_lock("current", 0) } except err { failed = true }
assert_true(failed)
failed = false
try { symlink_atomic("missing", "./target") } except err { failed = true }
assert_true(failed)
assert_equal(read_file("target"), "original")
write_file("other", "updated")
symlink_atomic("other", "./current")
assert_equal(read_file("current"), "updated")
print("PASS")
"#;
    for interpreter in [false, true] {
        let result = run(script, interpreter);
        assert!(
            result.status.success(),
            "{} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "PASS");
    }
}
#[test]
fn replacement_preserves_args_env_stdio_and_exit_status() {
    let script = r#"exec_process(["/bin/sh", "-c", "test \"$1\" = 'space value' && test \"$2\" = '' && test \"$NATIVE_TEST\" = value && printf PASS; exit 7", "test", "space value", ""], {"env": {"NATIVE_TEST": "value"}, "env_deny": ["KUJO_SCRIPT_ARGS", "KUJO_SCRIPT_ARGS_JSON"]})"#;
    for interpreter in [false, true] {
        let result = run(script, interpreter);
        assert_eq!(
            result.status.code(),
            Some(7),
            "{} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, b"PASS");
    }
}

#[test]
fn replacement_rejects_invalid_options_and_arguments() {
    for interpreter in [false, true] {
        for expression in [
            "exec_process([], {})",
            "exec_process([\"/bin/sh\"], {\"unknown\": true})",
            "exec_process([\"/bin/sh\"], {\"env\": {\"X\": 3}})",
            "exec_process([\"/bin/sh\"], {\"env_deny\": [3]})",
            "exec_process([\"/missing-native-command\"], {})",
            "file_lock(\"lock\", -1)",
            "file_unlock(999999)",
        ] {
            let result = run(&format!("mut failed := false\ntry {{ {expression} }} except err {{ failed = true }}\nassert_true(failed)\n"), interpreter);
            assert!(
                result.status.success(),
                "{expression}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}

#[test]
fn restricted_runtime_denies_host_effects() {
    let dir = tempdir().unwrap();
    for expression in [
        "file_lock(\"lock\", 0)",
        "symlink_atomic(\"x\", \"./current\")",
        "exec_process([\"/bin/sh\", \"-c\", \"exit 0\"], {})",
        "path_owned(\".\")",
    ] {
        let script = dir.path().join("restricted.kujo");
        fs::write(&script, expression).unwrap();
        for mode in [None, Some("--interpreter")] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_kujo"));
            cmd.arg("run").arg(&script).arg("--untrusted").current_dir(dir.path());
            if let Some(mode) = mode {
                cmd.arg(mode);
            }
            let output = cmd.output().unwrap();
            assert!(!output.status.success(), "restricted call succeeded: {expression}");
            assert!(!dir.path().join("lock").exists());
            assert!(!dir.path().join("current").exists());
        }
    }
}
