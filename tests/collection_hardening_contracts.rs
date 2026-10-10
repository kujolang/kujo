#[path = "support/native_values.rs"]
mod native_values;
use kujo::interpreter::{IntDictMap, Value};
use native_values::{array, call, json};
use std::sync::Arc;

#[test]
fn sum_overflow_is_a_runtime_error_instead_of_a_panic_or_wrap() {
    for values in
        [vec![Value::Int(i64::MAX), Value::Int(1)], vec![Value::Int(i64::MIN), Value::Int(-1)]]
    {
        assert!(
            matches!(call("sum", &[array(values)]), Value::Error(message) if message.contains("overflow"))
        );
    }
    assert!(matches!(call("sum", &[json("[1, 2, 3]")]), Value::Int(6)));
    assert!(matches!(call("sum", &[json("[1, 2.5, 3]")]), Value::Float(n) if n == 6.5));
}

#[test]
fn sort_does_not_round_large_integers_before_comparing_floats() {
    for (integer, float, int_first) in [
        (9_007_199_254_740_993, 9_007_199_254_740_992.0, false),
        (i64::MAX, 9_223_372_036_854_775_808.0, true),
        (-9_007_199_254_740_993, -9_007_199_254_740_992.0, true),
        (i64::MIN, -9_223_372_036_854_777_856.0, false),
    ] {
        let Value::Array(sorted) =
            call("sort", &[array(vec![Value::Float(float), Value::Int(integer)])])
        else {
            panic!("expected array")
        };
        assert_eq!(matches!(sorted[0], Value::Int(_)), int_first, "{sorted:?}");
        let Value::Array(sorted) =
            call("sort", &[array(vec![Value::Int(integer), Value::Float(float)])])
        else {
            panic!("expected array")
        };
        assert_eq!(matches!(sorted[0], Value::Int(_)), int_first, "{sorted:?}");
    }
}

#[test]
fn unique_does_not_treat_redacted_debug_output_as_value_identity() {
    let a = Value::Secret(Arc::new("first".into()));
    let b = Value::Secret(Arc::new("second".into()));
    for values in [
        vec![a.clone(), b.clone(), a.clone()],
        vec![array(vec![a.clone()]), array(vec![b]), array(vec![a])],
        vec![json("[1]"), json("[2]"), json("[1]")],
        vec![json(r#"{"a":1}"#), json(r#"{"a":2}"#), json(r#"{"a":1}"#)],
        vec![Value::Bytes(vec![1]), Value::Bytes(vec![2]), Value::Bytes(vec![1])],
    ] {
        let Value::Array(result) = call("unique", &[array(values)]) else {
            panic!("expected array")
        };
        assert_eq!(result.len(), 2);
    }
}

#[test]
fn invert_supports_every_integer_dictionary_representation() {
    let mut sparse = IntDictMap::default();
    sparse.insert(0, Value::Int(42));
    sparse.insert(1, Value::Int(7));
    for value in [
        Value::IntDict(Arc::new(sparse)),
        Value::DenseIntDict(Arc::new(vec![Value::Int(42), Value::Int(7)])),
        Value::DenseIntDictInt(Arc::new(vec![Some(42), Some(7)])),
        Value::DenseIntDictIntFull(Arc::new(vec![42, 7])),
        json(r#"{"0":42,"1":7}"#),
    ] {
        let actual = call("invert", &[value]);
        assert!(Value::equals(&actual, &json(r#"{"42":"0","7":"1"}"#)), "{actual:?}");
    }
}
