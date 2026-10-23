#[path = "support/native_api.rs"]
mod support;

#[test]
fn callers_keep_their_code_across_repeated_unwinds_and_generator_resumes() {
    support::success(
        r#"
        func leaf(x) { if x < 0 { throw("negative") } return x + 1 }
        func caller(x) {
            try { return leaf(x) * 3 } except error { return leaf(4) * 7 }
        }
        func* sequence() { yield caller(2) yield caller(-1) yield caller(5) }
        mut i := 0
        while i < 100 {
            assert_equal(caller(-1), 35)
            assert_equal(caller(3), 12)
            mut actual := []
            for value in sequence() { actual = push(actual, value) }
            assert_equal(actual, [9, 35, 18])
            i += 1
        }
        "#,
    );
}

#[test]
fn moved_call_frames_propagate_unhandled_recursive_exceptions() {
    support::failure(
        r#"
        func fail(n) {
            if n == 0 { throw("recursive leaf failure") }
            return fail(n - 1)
        }
        fail(4)
        "#,
        "recursive leaf failure",
    );
}

#[test]
fn moved_call_frames_restore_recursive_returns_and_exception_handlers() {
    support::success(
        r#"
        func recurse(n, input) {
            mut values := input
            if n == 0 { return values }
            values = push(values, n)
            return recurse(n - 1, values)
        }
        let original := [9]
        assert_equal(recurse(3, original), [9, 3, 2, 1])
        assert_equal(original, [9])
        func fail(n) { if n == 0 { throw("leaf") } return fail(n - 1) }
        mut caught := false
        try { fail(4) } except e { caught = true }
        assert(caught)
        assert_equal(recurse(1, [2]), [2, 1])
        func factory(base) {
            func step(n) { if n == 0 { return base } return step(n - 1) + 1 }
            return step
        }
        let step := factory(7)
        assert_equal(step(5), 12)
        func* sequence() { yield step(2) yield step(3) }
        mut seen := []
        for value in sequence() { seen = push(seen, value) }
        assert_equal(seen, [9, 10])
    "#,
    );
}
