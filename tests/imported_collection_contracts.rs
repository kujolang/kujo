#[path = "support/native_api.rs"]
mod support;

const HELPERS: &str = r#"
export func check(m, key, expected) {
    assert_equal(m[key], expected)
    assert_equal({...m}, m)
    let {...rest} := m
    assert_equal(rest, m)
    mut count := 0
    for k in m { assert_equal(m[k], get(m, k)) count += 1 }
    assert_equal(count, len(m))
    assert_equal(parse_json(to_json([m]))[0], m)
    mut caught := false
    try { let missing := m["01"] } except e { caught = true }
    assert(caught)
    return m
}
export func edit(m, key) { m[key] = 99 return m }
export func row_edit(a) { mut b := a b[0][0] = 99 return b }
export func state_edit(s) { s.rows[0][0] = 88 return s }
export func callback(f) { let a := f() return a[0][0] }
export func awaited(p) { let a := await p return a[0][0] }
export func optional(m) { return m?.present }
"#;

fn success(source: &str) {
    for interpreter in [false, true] {
        let out = support::run_modules(source, interpreter, &[("helpers.kujo", HELPERS)]);
        assert!(
            out.status.success(),
            "interpreter={interpreter}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn imported_helpers_read_and_update_every_dictionary_shape() {
    for (setup, key, expected) in [
        ("mut m := {\"0\": 7, \"1\": 8}", "0", "7"),
        ("mut m := {} m[0] = 7 m[1] = 8", "0", "7"),
        ("mut m := {} m[0] = null m[1] = 8", "0", "null"),
        ("mut m := {} m[0] = [7] m[1] = [8]", "0", "[7]"),
        ("mut m := {} m[-1] = 7 m[2] = 8", "-1", "7"),
    ] {
        success(&format!(
            r#"
            from helpers import check
            from helpers import edit
            {setup}
            let alias := m
            assert_equal(check(m, {key}, {expected}), m)
            mut changed := m
            changed = edit(changed, {key})
            assert_equal(changed[{key}], 99)
            assert_equal(m[{key}], {expected})
            assert_equal(alias, m)
        "#
        ));
    }
}

#[test]
fn imported_nested_writes_preserve_aliases_closures_and_struct_rows() {
    success(
        r#"
        from helpers import row_edit
        from helpers import state_edit
        from helpers import optional
        struct State { rows: array }
        func work() {
            mut m := {} m[0] = 7
            mut rows := [m]
            let alias := rows
            let snapshot := func() { return rows }
            rows = row_edit(rows)
            assert_equal(rows[0][0], 99)
            assert_equal(alias[0][0], 7)
            assert_equal(snapshot()[0][0], 7)
            mut state := State { rows: [m] }
            let old := state
            state = state_edit(state)
            assert_equal(state.rows[0][0], 88)
            assert_equal(old.rows[0][0], 7)
            assert_equal(optional({"present": 7}), 7)
            assert_equal(optional({"missing": 7}), null)
        }
        work()
    "#,
    );
}

#[test]
fn callback_and_await_results_keep_nested_dictionary_behavior() {
    success(
        r#"
        from helpers import callback
        from helpers import awaited
        func produce() { mut m := {} m[0] = 7 return [m] }
        async func later() { mut m := {} m[0] = 8 return [m] }
        assert_equal(callback(produce), 7)
        let p := later()
        assert_equal(awaited(p), 8)
        assert_equal(awaited(p), 8)
        mut first := await p
        first[0][0] = 99
        assert_equal(first[0][0], 99)
        assert_equal((await p)[0][0], 8)
    "#,
    );
}
