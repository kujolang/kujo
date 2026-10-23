use std::process::Command;

fn check(source: &str) {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("loops.kujo");
    std::fs::write(&file, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        command.env("DISABLE_FUNCTION_JIT", "1").env("DEBUG_JIT_LOOP", "1");
        let output = command.arg(&file).output().unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {error}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            assert!(error.contains("Executing compiled loop"), "{error}");
        }
    }
}

#[test]
fn loop_writes_and_reads_preserve_types_after_native_entry() {
    check(
        r#"
        let word := "done"
        func once() {
            mut mapping := {}
            mut i := 0
            mut item := 7
            mut mismatch := 0
            while i < 200 {
                if i > 150 { item = true }
                if i > 160 { item = 1.5 }
                if i > 170 { item = null }
                if i > 180 { item = word }
                mapping[i % 7] = item
                if mapping[i % 7] != item { mismatch += 1 }
                i += 1
            }
            assert_equal(mismatch, 0)
            assert_equal(mapping[3], word)
        }
        once()
        print("PASS")
    "#,
    );
}

#[test]
fn loop_growth_refreshes_dense_storage_before_later_reads() {
    check(
        r#"
        func once() {
            mut mapping := {}
            mut i := 0
            mut mismatch := 0
            while i < 2000 {
                mapping[i] = i * 2
                if mapping[i] != i * 2 { mismatch += 1 }
                if mapping[0] != 0 { mismatch += 1 }
                i += 1
            }
            assert_equal(mismatch, 0)
            assert_equal(len(mapping), 2000)
            assert_equal(mapping[1999], 3998)
            let snapshot := mapping
            mapping[0] = 99
            assert_equal(snapshot[0], 0)
            assert_equal(mapping[0], 99)
        }
        once()
        print("PASS")
    "#,
    );
}

#[test]
fn loop_sparse_and_string_keys_preserve_values() {
    check(
        r#"
        let key := "named"
        func once() {
            mut mapping := {}
            mut i := 0
            mut index := 0
            while i < 200 {
                index = i % 7
                if i > 150 { index = 1000000000 }
                if i > 170 { index = key }
                mapping[index] = i
                i += 1
            }
            assert_equal(mapping[1000000000], 170)
            assert_equal(mapping[key], 199)
            assert_equal(len(mapping), 9)
        }
        once()
        print("PASS")
    "#,
    );
}

#[test]
fn loop_missing_reads_are_catchable_without_replaying_effects() {
    check(
        r#"
        mut after := 0
        func once() {
            mut mapping := {}
            mut i := 0
            while i < 200 {
                mapping[i % 7] = i
                mut index := i % 7
                if i > 150 { index = 99 }
                let value := mapping[index]
                after += 1
                i += 1
            }
        }
        mut caught := 0
        try { once() } except error { caught += 1 }
        assert_equal(caught, 1)
        assert_equal(after, 151)
        print("PASS")
    "#,
    );
}

#[test]
fn loop_write_cannot_bypass_const_binding_after_warmup() {
    check(
        r#"
        func once() {
            const mapping := {}
            mut i := 0
            while i < 200 {
                if i > 150 { mapping[0] = 9 }
                i += 1
            }
        }
        mut caught := 0
        try { once() } except error { caught += 1 }
        assert_equal(caught, 1)
        print("PASS")
    "#,
    );
}

#[test]
fn loop_global_assignment_checks_readonly_bindings_and_keeps_prior_effects() {
    check(
        r#"
        let fixed := 7
        mut after := 0
        func once() {
            mut mapping := {}
            mut i := 0
            while i < 200 {
                mapping[i % 7] = i
                if i > 150 { fixed = 9 }
                after += 1
                i += 1
            }
        }
        mut caught := 0
        try { once() } except error { caught += 1 }
        assert_equal(caught, 1)
        assert_equal(after, 151)
        assert_equal(fixed, 7)
        print("PASS")
    "#,
    );
}
