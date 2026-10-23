#[path = "support/native_api.rs"]
mod support;

#[test]
fn arithmetic_errors_unwind_nested_calls_and_preserve_caller_state() {
    for (operator, left, right) in [
        ("+", "9223372036854775807", "1"),
        ("-", "(-9223372036854775807 - 1)", "1"),
        ("*", "9223372036854775807", "2"),
        ("/", "8", "0"),
        ("%", "8", "0"),
        ("/", "(-9223372036854775807 - 1)", "-1"),
        ("%", "(-9223372036854775807 - 1)", "-1"),
        ("/", "8.0", "0.0"),
        ("%", "8.0", "0.0"),
    ] {
        support::success(&format!(
            r#"
            func calculate(a, b) {{ return a {operator} b }}
            func relay(a, b) {{ return 10 + calculate(a, b) }}
            mut completed := 0
            mut result := 42
            while completed < 3 {{
                mut caught := false
                try {{ result = relay({left}, {right}) }} except err {{ caught = true }}
                assert_equal(caught, true)
                assert_equal(result, 42)
                assert_equal(relay(6, 2), 10 + (6 {operator} 2))
                completed += 1
            }}
            assert_equal(completed, 3)
            "#
        ));
    }
}

#[test]
fn arithmetic_errors_reach_in_frame_and_outer_handlers() {
    support::success(
        r#"
        func negate(value) { return -value }
        func recover() {
            mut caught := false
            try { let value := 8 / 0 } except err { caught = true }
            assert_equal(caught, true)
            return 9
        }
        assert_equal(recover(), 9)
        mut caught := 0
        try {
            try { negate(-9223372036854775807 - 1) } except inner {
                caught += 1
                throw(inner)
            }
        } except outer { caught += 1 }
        assert_equal(caught, 2)
        assert_equal(negate(7), -7)
        func compound() {
            mut value := 9223372036854775807
            mut caught := false
            try { value += 1 } except err { caught = true }
            assert_equal(caught, true)
            assert_equal(value, 9223372036854775807)
        }
        compound()
        "#,
    );
}

#[test]
fn uncaught_arithmetic_errors_keep_diagnostics() {
    support::failure("func divide(a, b) { return a / b } divide(1, 0)", "Division by zero");
    support::failure(
        "func add(a, b) { return a + b } add(9223372036854775807, 1)",
        "Integer overflow",
    );
}
