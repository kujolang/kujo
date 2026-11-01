use std::process::{Command, Stdio};

fn check(source: &str) -> (usize, usize) {
    check_result(source, "PASS", false)
}

fn check_result(source: &str, expected_stdout: &str, non_callable_error: bool) -> (usize, usize) {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("condition.kujo");
    std::fs::write(&source_path, source).unwrap();
    let mut native_entries = 0;
    let mut completed_loops = 0;
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let stdout = directory.path().join("stdout");
        let stderr = directory.path().join("stderr");
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let mut child = command
            .env("DISABLE_FUNCTION_JIT", "1")
            .env("DEBUG_JIT", "1")
            .env("DEBUG_JIT_LOOP", "1")
            .arg(&source_path)
            .stdout(Stdio::from(std::fs::File::create(&stdout).unwrap()))
            .stderr(Stdio::from(std::fs::File::create(&stderr).unwrap()))
            .spawn()
            .unwrap();
        let started = std::time::Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > std::time::Duration::from_secs(20) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("{mode:?} timed out: {}", std::fs::read_to_string(&stderr).unwrap());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let diagnostics = std::fs::read_to_string(stderr).unwrap();
        // The accepted interpreter leaves this discarded call's error as a
        // value, whereas VM/JIT report a fatal diagnostic. Preserve both modes.
        if non_callable_error && mode != Some("--interpreter") {
            assert_eq!(status.code(), Some(4), "{mode:?}: {diagnostics}");
            assert!(diagnostics.contains("not callable"), "{mode:?}: {diagnostics}");
        } else {
            assert!(status.success(), "{mode:?}: {diagnostics}");
        }
        assert_eq!(std::fs::read_to_string(stdout).unwrap().trim(), expected_stdout);
        if mode == Some("--jit") {
            assert!(!diagnostics.contains("Inconsistent JIT stack depth"), "{diagnostics}");
            native_entries = diagnostics.matches("Executing compiled loop").count();
            completed_loops = diagnostics.matches("JIT: Execution completed successfully!").count();
            assert!(native_entries > 0, "{diagnostics}");
        }
    }
    (native_entries, completed_loops)
}

#[test]
fn condition_calls_and_matched_controls_enter_native_loops() {
    for source in [
        include_str!("../benchmarks/langarena/probes/loop_call_admission/simple-call.kujo"),
        include_str!("../benchmarks/langarena/probes/loop_call_admission/simple-bound.kujo"),
        include_str!("../benchmarks/langarena/probes/loop_call_admission/nested-call.kujo"),
        include_str!("../benchmarks/langarena/probes/loop_call_admission/nested-bound.kujo"),
        include_str!("../benchmarks/langarena/probes/loop_call_admission/body-call.kujo"),
        include_str!("../benchmarks/langarena/probes/loop_call_admission/effectful-condition.kujo"),
    ] {
        check(source);
    }
}

#[test]
fn builtin_len_does_not_resume_at_every_iteration() {
    // These sources contain exactly one loop. Require both its native entry and
    // the VM's completion diagnostic, which is skipped when a guard resumes VM
    // execution. Entry count alone also passes for a loop that immediately exits.
    for source in [
        include_str!("../benchmarks/langarena/probes/loop_call_admission/simple-call.kujo"),
        include_str!("../benchmarks/langarena/probes/loop_call_admission/body-call.kujo"),
    ] {
        assert_eq!(check(source), (1, 1));
    }
}

#[test]
fn builtin_alias_rebound_to_callback_preserves_each_condition_evaluation() {
    check(
        r#"
        mut selected := len
        mut calls := 0
        func replacement(value) { calls += 1 return 260 }
        func scan() {
            let text := repeat("é🙂", 150)
            mut i := 0
            while i < selected(text) {
                if i == 250 { selected = replacement }
                i += 1
            }
            return i
        }
        assert_equal(scan(), 260)
        assert_equal(calls, 10)
        print("PASS")
        "#,
    );
}

#[test]
fn shadowed_len_preserves_callback_effects() {
    check(
        r#"
        mut calls := 0
        func len(value) { calls += 1 return 300 }
        func scan() {
            mut i := 0
            while i < len("x") { i += 1 }
            return i
        }
        assert_equal(scan(), 300)
        assert_equal(calls, 301)
        print("PASS")
        "#,
    );
}

#[test]
fn callback_error_in_condition_preserves_prior_effects() {
    check(
        r#"
        mut calls := 0
        mut effects := 0
        func condition(value) {
            calls += 1
            assert(calls != 251, "condition stop")
            return 300
        }
        func scan() {
            mut i := 0
            while i < condition("x") { effects += 1 i += 1 }
        }
        mut caught := 0
        try { scan() } except error { caught += 1 }
        assert_equal(caught, 1)
        assert_equal(calls, 251)
        assert_equal(effects, 250)
        print("PASS")
        "#,
    );
}

#[test]
fn builtin_error_after_native_entry_preserves_prior_effects() {
    check(
        r#"
        mut effects := 0
        func scan() {
            mut value := repeat("x", 300)
            mut i := 0
            while i < len(value) {
                effects += 1
                if i == 250 { value = 0 }
                i += 1
            }
        }
        mut caught := 0
        try { scan() } except error { caught += 1 }
        assert_equal(caught, 1)
        assert_equal(effects, 251)
        print("PASS")
        "#,
    );
}

#[test]
fn non_callable_replacement_resumes_for_vm_error() {
    check_result(
        r#"
        mut selected := len
        mut effects := 0
        func scan() {
            let value := repeat("x", 300)
            mut i := 0
            while i < selected(value) {
                effects += 1
                if i == 250 {
                    selected = 7
                    assert_equal(effects, 251)
                    print("BEFORE ERROR")
                }
                i += 1
            }
        }
        scan()
        "#,
        "BEFORE ERROR",
        true,
    );
}
