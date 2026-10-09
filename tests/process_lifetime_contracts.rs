#[cfg(not(windows))]
#[test]
fn unsupported_lifetime_mode_rejects_before_script_execution() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("must-not-run.kujo");
    std::fs::write(&source, "print(\"EXECUTED\")\n").unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(["run", source.to_str().unwrap(), "--kill-children-on-exit"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("EXECUTED"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("supported only on Windows"));
}

#[cfg(windows)]
mod windows {
    use std::{
        fs,
        path::PathBuf,
        process::{Child, Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::Threading::{
            OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
            PROCESS_TERMINATE,
        },
    };

    // Helpers run only in explicitly marked child test processes.
    #[test]
    fn lifetime_grandchild_fixture() {
        let Ok(root) = std::env::var("KUJO_LIFETIME_TEST_DIR") else { return };
        fs::write(PathBuf::from(root).join("grandchild.pid"), std::process::id().to_string())
            .unwrap();
        thread::sleep(Duration::from_secs(30));
    }
    #[test]
    fn lifetime_child_fixture() {
        let Ok(root) = std::env::var("KUJO_LIFETIME_TEST_DIR") else { return };
        let _child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "windows::lifetime_grandchild_fixture", "--nocapture"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        fs::write(PathBuf::from(root).join("child.pid"), std::process::id().to_string()).unwrap();
        thread::sleep(Duration::from_secs(30));
    }
    #[test]
    fn lifetime_exited_parent_fixture() {
        let Ok(_) = std::env::var("KUJO_LIFETIME_TEST_DIR") else { return };
        let _child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "windows::lifetime_grandchild_fixture", "--nocapture"])
            .stdin(Stdio::null())
            // Inherit the native runner's capture pipes after this parent exits.
            .spawn()
            .unwrap();
    }

    #[test]
    fn native_deadline_and_cancel_terminate_descendants_after_parent_exit() {
        for interpreter in [false, true] {
            for cancel in [false, true] {
                let temporary = tempfile::tempdir().unwrap();
                let source = temporary.path().join("deadline.kujo");
                let cancel_file = temporary.path().join("cancel");
                let argv = serde_json::json!([
                    std::env::current_exe().unwrap().to_str().unwrap(),
                    "--exact",
                    "windows::lifetime_exited_parent_fixture",
                    "--nocapture"
                ]);
                let options = serde_json::json!({
                    "env":{"KUJO_LIFETIME_TEST_DIR":temporary.path().to_str().unwrap()},
                    "timeout_ms":5000, "max_output_bytes":4096,
                    "cancel_file":cancel_file.to_str().unwrap()
                });
                fs::write(&source, format!(
                    "let result := spawn_process(parse_json({}),parse_json({}))\nprint(result.{})\nprint(result.success)\n",
                    serde_json::to_string(&argv.to_string()).unwrap(),
                    serde_json::to_string(&options.to_string()).unwrap(),
                    if cancel { "cancelled" } else { "timed_out" },
                )).unwrap();
                let receipt = temporary.path().join("stdout");
                let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
                command.arg("run").arg(&source).args(["--untrusted", "--allow-process-exec"]);
                if interpreter {
                    command.arg("--interpreter");
                }
                let mut runtime = Runtime(
                    command
                        .stdin(Stdio::null())
                        .stdout(fs::File::create(&receipt).unwrap())
                        .stderr(Stdio::inherit())
                        .spawn()
                        .unwrap(),
                );
                let grandchild = wait_for_fixture(temporary.path(), "grandchild.pid");
                if cancel {
                    fs::write(&cancel_file, "cancel").unwrap();
                }
                let deadline = Instant::now() + Duration::from_secs(10);
                let status = loop {
                    if let Some(status) = runtime.0.try_wait().unwrap() {
                        break status;
                    }
                    assert!(
                        Instant::now() < deadline,
                        "native runner hung draining descendant pipes"
                    );
                    thread::sleep(Duration::from_millis(20));
                };
                assert!(status.success());
                assert_eq!(
                    fs::read_to_string(receipt).unwrap().replace("\r\n", "\n"),
                    "true\nfalse\n"
                );
                // SAFETY: handle is retained from our exact fixture process.
                assert_eq!(
                    unsafe { WaitForSingleObject(grandchild.0, 5000) },
                    WAIT_OBJECT_0,
                    "descendant survived native timeout/cancellation"
                );
            }
        }
    }

    struct Runtime(Child);
    impl Drop for Runtime {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    struct OwnedFixture(HANDLE);
    impl Drop for OwnedFixture {
        fn drop(&mut self) {
            // SAFETY: this handle refers to a process created by this fixture;
            // terminate on assertion failure too, then release our owned handle.
            unsafe {
                TerminateProcess(self.0, 1);
                CloseHandle(self.0);
            }
        }
    }
    fn wait_for_fixture(root: &std::path::Path, name: &str) -> OwnedFixture {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Ok(raw) = fs::read_to_string(root.join(name)) {
                if let Ok(pid) = raw.parse::<u32>() {
                    // SAFETY: PID comes from our isolated fixture directory;
                    // retain the handle to avoid PID reuse during later checks.
                    let handle =
                        unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, 0, pid) };
                    if !handle.is_null() {
                        return OwnedFixture(handle);
                    }
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("fixture did not start: {name}");
    }
    #[test]
    fn forced_runtime_exit_terminates_child_and_grandchild_in_both_modes() {
        for interpreter in [false, true] {
            let temporary = tempfile::tempdir().unwrap();
            let source = temporary.path().join("provider.kujo");
            let argv = serde_json::json!([
                std::env::current_exe().unwrap().to_str().unwrap(),
                "--exact",
                "windows::lifetime_child_fixture",
                "--nocapture"
            ]);
            let options = serde_json::json!({"env":{"KUJO_LIFETIME_TEST_DIR":temporary.path().to_str().unwrap()},"inherit_env":true,"timeout_ms":30000,"max_output_bytes":4096});
            // Decode JSON strings to avoid treating Windows backslashes as Kujo escapes.
            fs::write(
                &source,
                format!(
                    "spawn_process(parse_json({}),parse_json({}))\n",
                    serde_json::to_string(&argv.to_string()).unwrap(),
                    serde_json::to_string(&options.to_string()).unwrap()
                ),
            )
            .unwrap();
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(&source).args([
                "--kill-children-on-exit",
                "--untrusted",
                "--allow-process-exec",
            ]);
            if interpreter {
                command.arg("--interpreter");
            }
            let mut runtime = Runtime(
                command
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .unwrap(),
            );
            let child = wait_for_fixture(temporary.path(), "child.pid");
            let grandchild = wait_for_fixture(temporary.path(), "grandchild.pid");
            runtime.0.kill().unwrap();
            runtime.0.wait().unwrap();
            for process in [&child, &grandchild] {
                // SAFETY: owned process handle remains valid during the bounded wait.
                assert_eq!(
                    unsafe { WaitForSingleObject(process.0, 5000) },
                    WAIT_OBJECT_0,
                    "descendant survived provider exit"
                );
            }
        }
    }
}
