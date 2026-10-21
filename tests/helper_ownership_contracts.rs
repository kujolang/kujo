#[path = "support/native_api.rs"]
mod support;

#[test]
fn helper_rebinding_preserves_aliases_and_recursive_updates() {
    support::success(
        r#"
        func edit(a, i) { mut b := a b[i] = 7 return b }
        func recursive(a, n) {
            if n == 0 { return a }
            a[n - 1] = n
            a = recursive(a, n - 1)
            return a
        }
        func work() {
            mut x := range(0, 100)
            let original := x
            let snapshot := func() { return x }
            mut i := 0
            while i < 100 { x = edit(x, i) i += 1 }
            assert_equal(x[99], 7)
            assert_equal(original[99], 99)
            assert_equal(snapshot()[99], 99)
            x = recursive(x, 20)
            assert_equal(x[19], 20)
            assert_equal(x[0], 1)
        }
        work()
    "#,
    );
}

#[test]
fn caught_failures_and_immutable_rebinding_keep_original_values() {
    support::success(
        r#"
        func fail(a, i) { a[0] = 99 if i == 100 { throw("failed") } return a }
        func work() {
            mut x := [1, 2]
            mut caught := false
            try { x = fail(x, 100) } except e { caught = true }
            assert_equal(caught, true)
            assert_equal(x, [1, 2])
            let y := [3, 4]
            try { y = fail(y, 0) } except e { assert_equal(y, [3, 4]) }
        }
        work()
        mut global := [5, 6]
        try { global = fail(global, 100) } except e { assert_equal(global, [5, 6]) }
    "#,
    );
}

#[test]
fn argument_effects_and_repeated_arguments_keep_snapshots() {
    support::success(
        r#"
        func edit(a, i) { a[i] = 7 return a }
        func pair(a, b) { a[0] = 8 assert_equal(b[0], 1) return a }
        func work() {
            mut x := [1, 2]
            x = pair(x, x)
            assert_equal(x, [8, 2])
            x = [1, 2]
            func change() { x = [9, 9] return 1 }
            x = edit(x, change())
            assert_equal(x, [1, 7])
        }
        work()
    "#,
    );
}

#[test]
fn parameter_aliases_remain_live_when_used_by_nested_expressions() {
    support::success(
        r#"
        func edit(a) {
            mut b := a
            let old := func() { return a }
            b[0] = 7
            return [a, b, old()]
        }
        func work() {
            mut x := [1, 2]
            x = edit(x)
            assert_equal(x, [[1, 2], [7, 2], [1, 2]])
        }
        work()
    "#,
    );
}

#[test]
fn imported_helpers_preserve_value_semantics_and_failure_recovery() {
    for interpreter in [false, true] {
        let out = support::run_modules(
            r#"
            from edits import edit
            func work() {
                mut x := [1, 2]
                let alias := x
                x = edit(x, 1)
                assert_equal(x, [1, 7])
                assert_equal(alias, [1, 2])
                try { x = edit(x, 9) } except e { assert_equal(x, [1, 7]) }
            }
            work()
        "#,
            interpreter,
            &[("edits.kujo", "export func edit(a, i) { mut b := a b[i] = 7 return b }")],
        );
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
}

#[test]
fn helpers_preserve_byte_dictionary_and_struct_snapshots() {
    support::success(
        r#"
        struct State { rows: array }
        func byte_edit(a) { mut b := a b[0] = 255 return b }
        func map_edit(a) { mut b := a b["rows"][0][0] = 7 return b }
        func state_edit(a) { mut b := a b.rows[0][0] = 8 return b }
        func work() {
            mut bytes := utf8_encode("AB")
            let old_bytes := bytes
            bytes = byte_edit(bytes)
            assert_equal(bytes[0], 255)
            assert_equal(old_bytes[0], 65)
            mut map := {"rows": [[1, 2]]}
            let old_map := map
            map = map_edit(map)
            assert_equal(map["rows"][0], [7, 2])
            assert_equal(old_map["rows"][0], [1, 2])
            mut state := State { rows: [[1, 2]] }
            let old_state := state
            state = state_edit(state)
            assert_equal(state.rows[0], [8, 2])
            assert_equal(old_state.rows[0], [1, 2])
        }
        work()
    "#,
    );
    support::failure(
        "func edit(a) { a[0] = 9 return a } func work() { let a := [1] a = edit(a) } work()",
        "immutable",
    );
}

#[test]
fn nested_blocks_cannot_consume_parameters_used_afterward() {
    support::success(
        r#"
        func edit(a) {
            try { mut b := a b[0] = 9 } except e {}
            assert_equal(a, [1])
            if true { mut c := a c[0] = 8 }
            return a
        }
        assert_equal(edit([1]), [1])
    "#,
    );
}

#[test]
fn helper_rebinding_preserves_named_arity_diagnostics() {
    support::failure(
        "func edit(a) { return a } func work() { mut x := [1] x = edit(x, 0) } work()",
        "edit expects 1 arguments, got 2",
    );
}

#[test]
fn caught_helper_index_errors_preserve_the_caller_snapshot() {
    support::success(
        r#"
        func fail(a, i) { a[0] = 99 a[i] = 1 return a }
        func work() {
            mut x := [1, 2]
            mut caught := false
            try { x = fail(x, 100) } except e { caught = true }
            assert(caught)
            assert_equal(x, [1, 2])
        }
        work()
        mut x := [3, 4]
        mut caught := false
        try { x[100] = 9 } except e { caught = true }
        assert(caught)
        assert_equal(x, [3, 4])
    "#,
    );
}
