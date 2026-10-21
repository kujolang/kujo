#[path = "support/native_api.rs"]
mod support;
use support::{failure, success};

#[test]
fn replacement_visits_captures_once_in_order_and_returns_literal_text() {
    success(
        r#"
    mut seen := []
    func replacement(c) { seen = push(seen, c) return "$literal" }
    assert_equal(regex_replace_with("a1 b", "([a-z])([0-9])?", replacement), "$literal $literal")
    assert_equal(seen, [["a1","a","1"],["b","b",null]])
    func empty(c) { return "|" }
    assert_equal(regex_replace_with("é雪", "", empty), "|é|雪|")
    assert_equal(regex_replace_with("abc", "z", empty), "abc")
    assert_equal(regex_replace("a1", "([0-9])", "$1$1"), "a11")
    func factory() {
        mut i := 0
        func next(c) { i += 1 return to_string(i) }
        return next
    }
    let next := factory()
    assert_equal(regex_replace_with("aa", "a", next), "12")
    "#,
    );
}

#[test]
fn replacement_errors_stop_and_callbacks_keep_capability_and_depth_checks() {
    success(
        r#"
    mut count := 0
    func fail(c) { count += 1 throw("replacement failed") }
    mut caught := false
    try { regex_replace_with("aaa", "a", fail) } except e { caught = true }
    assert(caught)
    assert_equal(count, 1)
    "#,
    );
    failure("func f(c) { return 3 } regex_replace_with(\"a\", \"a\", f)", "string");
    failure("func f(c) { return \"x\" } regex_replace_with(\"a\", \"[\", f)", "pattern");
    failure("regex_replace_with(\"a\", \"a\", 1)", "function");
    failure("regex_replace_with(\"a\", \"a\")", "argument");
    failure(
        "func f(c) { return read_file(\"forbidden\") } regex_replace_with(\"a\", \"a\", f)",
        "denied",
    );
    failure("func f(c) { return regex_replace_with(\"a\", \"a\", f) } regex_replace_with(\"a\", \"a\", f)", "depth");
}

#[test]
fn imported_callback_works_in_both_runtimes() {
    for interpreter in [false, true] {
        let out = support::run_modules(
            r#"from callbacks import replace_match
            assert_equal(regex_replace_with("x1 y2", "([a-z])([0-9])", replace_match), "1 2")"#,
            interpreter,
            &[("callbacks.kujo", "export func replace_match(c) { return c[2] }")],
        );
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
}

#[test]
fn replacement_resource_limits_are_enforced() {
    failure(
        r#"func f(c) { return "" } regex_replace_with(repeat("x", 8388609), "x", f)"#,
        "input exceeds",
    );
    failure(
        r#"func f(c) { return repeat("x", 4194305) } regex_replace_with("aa", "a", f)"#,
        "output exceeds",
    );
    let arity =
        kujo::interpreter::Interpreter::native_function_arity("regex_replace_with").unwrap();
    assert!(arity.validate(3).is_ok());
    assert!(arity.validate(2).is_err());
}
