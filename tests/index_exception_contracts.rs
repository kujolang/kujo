#[path = "support/native_api.rs"]
mod support;

#[test]
fn indexed_read_errors_unwind_to_handlers_and_resume() {
    for (setup, expression) in [
        ("let x := []", "x[0]"),
        ("let x := [1]", "x[-2]"),
        ("let x := \"é\"", "x[1]"),
        ("let x := utf8_encode(\"a\")", "x[2]"),
        ("let x := {}", "x[\"missing\"]"),
        ("let x := [1]", "x[\"bad\"]"),
        ("let x := 1", "x[0]"),
        ("", "[][0]"),
    ] {
        support::success(&format!(
            r#"
            func fail() {{ {setup} return {expression} }}
            func caller() {{ return 5 + fail() }}
            mut caught := 0
            try {{ caller() }} except err {{ caught += 1 }}
            try {{ {setup} let result := {expression} }} except err {{ caught += 1 }}
            assert_equal(caught, 2)
            assert_equal(2 + 3, 5)
        "#
        ));
    }
}

#[test]
fn indexed_write_errors_are_catchable_and_preserve_snapshots() {
    for (binding, initial, index) in [
        ("let", "[[1, 2]]", "0"),
        ("const", "[1, 2]", "0"),
        ("mut", "[1, 2]", "\"bad\""),
        ("mut", "[1, 2]", "true"),
        ("mut", "[1, 2]", "9"),
        ("mut", "7", "0"),
    ] {
        support::success(&format!(
            r#"
            func work() {{
                {binding} x := {initial}
                let before := x
                let snapshot := func() {{ return x }}
                mut caught := false
                try {{ x[{index}] = 99 }} except err {{ caught = true }}
                assert(caught)
                assert_equal(x, before)
                assert_equal(snapshot(), before)
            }}
            work()
            {binding} x := {initial}
            let before := x
            mut caught := false
            try {{ x[{index}] = 99 }} except err {{ caught = true }}
            assert(caught)
            assert_equal(x, before)
        "#
        ));
    }
}

#[test]
fn uncaught_index_errors_still_fail() {
    support::failure("func fail() { let x := [] return x[0] } fail()", "Index out of bounds");
    support::failure("func fail() { let x := [1] x[0] = 2 } fail()", "immutable");
}

#[test]
fn rejected_consuming_pop_writeback_is_catchable() {
    support::success(
        r#"
        func work() {
            let frozen := [[1, 2]]
            mut caught := false
            try {
                let pair := pop(frozen[0])
                frozen[0] = pair[0]
            } except err { caught = true }
            assert(caught)
            assert_equal(frozen, [[1, 2]])
        }
        work()
    "#,
    );
}
