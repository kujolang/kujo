#[path = "support/native_api.rs"]
mod support;
use support::{failure, success};

#[test]
fn comparator_sort_is_stable_and_leaves_input_unchanged() {
    success(
        r#"
    func compare(a,b) { return a[0] - b[0] }
    let items := [[2,"a"],[1,"b"],[2,"c"],[1,"d"]]
    let sorted := sort_by(items, compare)
    assert_equal(sorted, [[1,"b"],[1,"d"],[2,"a"],[2,"c"]])
    assert_equal(items, [[2,"a"],[1,"b"],[2,"c"],[1,"d"]])
    func descending(a,b) { return b-a }
    assert_equal(sort_by([1,3,2], descending), [3,2,1])
    assert_equal(sort_by([], descending), [])
    assert_equal(sort_by([1], descending), [1])
    assert_equal(sort([3,1,2]), [1,2,3])
    "#,
    );
}

#[test]
fn comparator_errors_stop_and_do_not_publish_partial_sort() {
    success(
        r#"
    mut calls := 0
    func fail(a,b) { calls += 1 throw("comparison failed") }
    let items := [3,2,1]
    mut caught := false
    try { sort_by(items, fail) } except e { caught = true }
    assert(caught)
    assert_equal(calls, 1)
    assert_equal(items, [3,2,1])
    "#,
    );
    failure("func f(a,b) { return true } sort_by([2,1],f)", "integer");
    failure("sort_by([1],1)", "function");
    failure("sort_by([1])", "argument");
    failure("func f(a,b) { read_file(\"forbidden\") return 0 } sort_by([2,1],f)", "denied");
}

#[test]
fn imported_comparator_and_inconsistent_order_are_bounded() {
    for interpreter in [false, true] {
        let out = support::run_modules(
            r#"from comparators import compare
            assert_equal(sort_by([3,1,2], compare), [1,2,3])"#,
            interpreter,
            &[("comparators.kujo", "export func compare(a,b) { return a-b }")],
        );
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
    success(
        r#"
    func inconsistent(a,b) { return 1 }
    assert_equal(sort(sort_by([3,1,2], inconsistent)), [1,2,3])
    assert_equal(sort_by([3,1,2], wrapping_sub), [1,2,3])
    "#,
    );
}

#[test]
fn comparator_depth_arity_and_large_input_are_checked() {
    failure("func f(a,b) { sort_by([2,1],f) return 0 } sort_by([2,1],f)", "depth");
    let arity = kujo::interpreter::Interpreter::native_function_arity("sort_by").unwrap();
    assert!(arity.validate(2).is_ok());
    assert!(arity.validate(3).is_err());
    use kujo::interpreter::{Interpreter, Value};
    let mut interp = Interpreter::new();
    let value = interp.call_native_function_impl(
        "sort_by",
        &[
            Value::Array(std::sync::Arc::new(vec![Value::Null; 1_000_001])),
            Value::NativeFunction("wrapping_sub".into()),
        ],
    );
    assert!(matches!(value, Value::Error(error) if error.contains("integer")));
}

#[test]
fn comparator_sort_matches_independent_stable_order_across_merge_boundaries() {
    let mut source = String::from("func compare(a,b) { return a[0] - b[0] }\n");
    for size in [2, 5, 17, 65, 127] {
        let input: Vec<[i32; 2]> = (0..size).map(|i| [(i * 37) % 17, i]).collect();
        let mut expected = input.clone();
        expected.sort_by_key(|item| item[0]);
        source.push_str(&format!(
            "assert_equal(sort_by({},compare), {})\n",
            serde_json::to_string(&input).unwrap(),
            serde_json::to_string(&expected).unwrap()
        ));
    }
    success(&source);
}
