#[path = "support/native_api.rs"]
mod support;

#[test]
fn scalar_append_and_nested_snapshots_preserve_values_in_both_runtimes() {
    support::success(
        r#"
let leaves := [0, 1.5, true, null, "🙂", bytes([0, 255])]
mut result := []
let empty := result
for item in leaves { result = push(result, item) }
assert_equal(result, leaves)
assert_equal(empty, [])
let saved := result
mut nested := [result]
nested[0][5][0] = 65
assert_equal(saved, leaves)
assert_equal(result, leaves)
assert_equal(nested[0][5], bytes([65, 255]))
result = push(result, [leaves])
assert_equal(result[6][0], leaves)
assert_equal(saved, leaves)
"#,
    );
}
