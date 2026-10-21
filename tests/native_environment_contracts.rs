#[path = "support/native_api.rs"]
mod support;

#[test]
fn argument_only_calls_preserve_aliases_and_callback_global_freshness() {
    support::success(
        r#"
        mut source := [3, 1, 2]
        let old := source
        source = sort(source)
        assert_equal(old, [3, 1, 2])
        assert_equal(source, [1, 2, 3])
        assert_equal(wrapping_add(9223372036854775807, 1), -9223372036854775807 - 1)
        assert_equal(utf8_decode(decode_base64(encode_base64(utf8_encode("hello")))), "hello")
        assert_equal(parse_json(to_json({"a": 4}))["a"], 4)
        func late_dependency(x) { return "late" + str(x) }
        func replacement(c) { return late_dependency(len(c[0])) }
        assert_equal(regex_replace_with("aa", "a+", replacement), "late2")
        func compare(a, b) { return wrapping_sub(a, b) }
        assert_equal(sort_by([4, 2, 3], compare), [2, 3, 4])
        async func worker(x) { return late_dependency(x) }
        assert_equal(await parallel_map([1, 2], worker, 2), ["late1", "late2"])
        mut caught := false
        try { len([1], 2) } except e { caught = true }
        assert(caught)
        caught = false
        try { bit_shl(1, -1) } except e { caught = true }
        assert(caught)
    "#,
    );
}

#[test]
fn native_fast_path_still_enforces_restricted_capabilities() {
    support::failure("read_file(\"forbidden\")", "denied");
}

#[test]
fn bytecode_callbacks_keep_globals_and_imported_interpreter_helpers_fresh() {
    for interpreter in [false, true] {
        let out = support::run_modules(
            r#"
            from helpers import decorate
            mut count := 0
            func replacement(c) {
                count += 1
                return decorate(str(count))
            }
            assert_equal(regex_replace_with("aaa", "a", replacement), "[1][2][3]")
            assert_equal(count, 3)
            assert_equal(regex_replace_with("a", "a", replacement), "[4]")
            func nested(c) { return regex_replace_with(c[0], "a", replacement) }
            assert_equal(regex_replace_with("aa", "aa", nested), "[5][6]")
            assert_equal(count, 6)
            "#,
            interpreter,
            &[("helpers.kujo", "export func decorate(text) { return \"[\" + text + \"]\" }")],
        );
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
}
