use std::process::{Command, Stdio};

#[cfg(feature = "runtime-jit")]
#[test]
fn loop_call_lowering_registers_runtime_helper() {
    use kujo::bytecode::{Constant, OpCode};
    use kujo::jit::JitCompiler;
    use kujo::{compiler::Compiler, lexer, parser::Parser};

    // Compile without executing: the loop's guarded Call continuation must have
    // its runtime helper even when an observed callable would resume in the VM.
    // Function-local loops use local slots, matching the VM's admitted path.
    // Top-level loops contain scope opcodes and are deliberately not admitted.
    let mut parser = Parser::new(
        lexer::tokenize("func scan() { mut i := 0 while i < 200 { i += len(\"x\") } return i }")
            .unwrap(),
    );
    let script = Compiler::new().compile(&parser.parse()).unwrap();
    let chunk = script
        .constants
        .iter()
        .find_map(|constant| match constant {
            Constant::Function(chunk) => Some(chunk.as_ref()),
            _ => None,
        })
        .expect("fixture must contain a compiler-generated function");
    let (end, start) = chunk
        .instructions
        .iter()
        .enumerate()
        .find_map(|(pc, instruction)| match instruction {
            OpCode::JumpBack(start) => Some((pc, *start)),
            _ => None,
        })
        .expect("fixture must contain a compiler-generated loop");
    assert!(chunk.instructions[start..end].iter().any(|op| matches!(op, OpCode::Call(1))));
    let mut jit = JitCompiler::new().unwrap();
    assert!(jit.can_compile_loop(chunk, start, end), "fixture must meet VM loop admission");
    jit.compile_loop_with_int_dicts(chunk, start, end, Default::default())
        .expect("loop Call must lower with the standard runtime helper");
}

fn check(source: &str) -> (usize, usize) {
    check_result(source, "PASS", false)
}

#[test]
fn repeated_callback_fallback_has_bounded_native_reentry() {
    let (entries, completed) = check_result(
        r#"
        mut calls := 0
        func bound(text) { calls += 1 return len(text) }
        func scan(text) {
            mut i := 0
            while i < bound(text) { i += 1 }
            return i
        }
        let text := repeat("x", 300)
        assert_equal(scan(text), 300)
        assert_equal(scan(text), 300)
        assert_equal(scan(text), 300)
        assert_equal(calls, 903)
        print("PASS")
        "#,
        "PASS",
        false,
    );
    assert!(entries <= 12, "repeated fallback entered native code {entries} times");
    assert_eq!(completed, 0);
}

#[test]
fn callback_fallback_recovers_after_rebinding_to_builtin() {
    let (entries, completed) = check(
        r#"
        mut calls := 0
        func bound(text) { calls += 1 return len(text) }
        mut limit := bound
        func scan(text) {
            mut i := 0
            while i < limit(text) { i += 1 }
            return i
        }
        let text := repeat("x", 600)
        assert_equal(scan(text), 600)
        limit = len
        assert_equal(scan(text), 600)
        assert_equal(scan(text), 600)
        assert_equal(calls, 601)
        print("PASS")
        "#,
    );
    assert!(entries <= 14, "repeated fallback entered native code {entries} times");
    assert_eq!(completed, 2, "native execution must recover in later calls");
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
        // Calling a rebound non-function is a runtime failure in every engine.
        // Preserve the side effects preceding the invalid call during fallback.
        if non_callable_error {
            assert_eq!(status.code(), Some(4), "{mode:?}: {diagnostics}");
            assert!(diagnostics.contains("Cannot call non-function"), "{mode:?}: {diagnostics}");
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
fn loop_break_and_condition_exit_preserve_following_operands_and_values() {
    for limit in [150, 300] {
        let source = format!(
            r#"
            func scan(text) {{
                mut i := 0
                mut rows := [[7]]
                let snapshot := rows
                while i < len(text) {{
                    if i == {limit} {{ break }}
                    i += 1
                }}
                rows[0][0] = i
                assert_equal(snapshot[0][0], 7)
                return [i, rows]
            }}
            let result := scan(repeat("x", 300))
            assert_equal(result[0], {limit})
            assert_equal(result[1][0][0], {limit})
            print("PASS")
            "#
        );
        assert_eq!(check(&source), (1, 1));
    }
    // The first post-loop instruction produces the return operand directly.
    assert_eq!(
        check(
            r#"
        func scan(text) {
            mut i := 0
            while i < len(text) {
                if i == 150 { break }
                i += 1
            }
            return i
        }
        assert_equal(scan(repeat("x", 300)), 150)
        print("PASS")
    "#
        ),
        (1, 1)
    );
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

#[test]
fn repeated_calls_reuse_loops_without_crossing_chunk_identity() {
    let source = r#"
        func first(text) {
            mut i := 0
            mut total := 0
            while i < len(text) { total += 1 i += 1 }
            return total
        }
        func second(text) {
            mut i := 0
            mut total := 0
            while i < len(text) { total += 2 i += 1 }
            return total
        }
        mut round := 0
        while round < 3 {
            assert_equal(first(repeat("x", 300)), 300)
            assert_equal(second(repeat("x", 300)), 600)
            round += 1
        }
        print("PASS")
    "#;
    assert_eq!(check(source), (6, 6));
}

#[test]
fn reused_loop_keeps_nested_snapshots_across_collection_shapes() {
    let source = r#"
        func bump(rows) {
            mut current := rows
            mut i := 0
            while i < 300 { current[0][0] += 1 i += 1 }
            return current
        }
        let array := [[0]]
        mut dictionary := dict()
        dictionary[0] = [0]
        mut inner := dict()
        inner[0] = 0
        let nested_dictionary := [inner]
        let first := bump(array)
        let second := bump(dictionary)
        let third := bump(nested_dictionary)
        assert_equal(first[0][0], 300)
        assert_equal(second[0][0], 300)
        assert_equal(third[0][0], 300)
        assert_equal(array[0][0], 0)
        assert_equal(dictionary[0][0], 0)
        assert_equal(nested_dictionary[0][0], 0)
        print("PASS")
    "#;
    assert_eq!(check(source), (3, 3));
}

#[test]
fn loop_locals_keep_physical_identity_across_shadowing_and_iteration_scopes() {
    for source in [
        r#"
        func scan() {
            mut total := 0
            if false { mut i := 999 }
            mut i := 0
            while i < 300 { total += 1 i += 1 }
            return [total, i]
        }
        assert_equal(scan(), [300, 300])
        assert_equal(scan(), [300, 300])
        print("PASS")
        "#,
        r#"
        func scan(data) {
            mut total := 0
            for item in [5] { total += item }
            for item in data { total += item }
            return total
        }
        let data := range(0, 300)
        assert_equal(scan(data), 44855)
        assert_equal(scan(data), 44855)
        assert_equal(len(data), 300)
        print("PASS")
        "#,
        r#"
        func scan(data, first) {
            mut total := 0
            if first { for item in data { total += item } }
            else { for item in data { total += item } }
            return total
        }
        let data := range(0, 300)
        assert_equal(scan(data, false), 44850)
        assert_equal(scan(data, true), 44850)
        assert_equal(scan(data, false), 44850)
        print("PASS")
        "#,
        r#"
        func scan(data) {
            mut total := 0
            for item in data { total += item }
            for item in data { total += item }
            return total
        }
        let data := range(0, 300)
        assert_equal(scan(data), 89700)
        assert_equal(scan(data), 89700)
        print("PASS")
        "#,
    ] {
        let (entries, completed) = check(source);
        assert!(entries > 0 && completed > 0, "native loops must execute and complete");
    }
}

#[test]
fn loop_storage_keys_preserve_source_names_in_readonly_errors() {
    let (entries, _) = check_result(
        r#"
        func scan() {
            if false { let fixed := [0] }
            let fixed := [1]
            mut i := 0
            while i < 300 {
                if i == 250 { fixed[0] = 2 }
                i += 1
            }
        }
        try { scan() } except error { print(error.message) }
        "#,
        "Cannot mutate immutable let binding: fixed",
        false,
    );
    assert!(entries > 0, "readonly error must follow native loop entry");
}
