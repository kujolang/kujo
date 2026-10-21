#[path = "support/native_api.rs"]
mod support;

#[test]
fn large_callable_aliases_preserve_captures_recursion_and_collection_snapshots() {
    let padding = "value += 1\n".repeat(1000);
    let source = format!(
        r#"
        func factory(seed) {{
            mut state := [seed]
            func step(value) {{
                if value < 0 {{ {padding} }}
                state[0] += value
                return state
            }}
            return step
        }}
        let first := factory(10)
        let alias := first
        let other := factory(100)
        let snapshot := first(2)
        assert_equal(alias(3), [15])
        assert_equal(first(4), [19])
        assert_equal(snapshot, [12])
        assert_equal(other(1), [101])
        func factorial(n) {{
            if n <= 1 {{ return 1 }}
            return n * factorial(n - 1)
        }}
        let recursive_alias := factorial
        assert_equal(recursive_alias(8), 40320)
    "#
    );
    support::success(&source);
}
