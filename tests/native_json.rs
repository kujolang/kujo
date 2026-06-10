use kujo::builtins;
use kujo::interpreter::{DictMap, Value};
use std::sync::Arc;

fn nested_json_array(depth: usize) -> String {
    let mut json = String::new();
    for _ in 0..depth {
        json.push('[');
    }
    json.push('0');
    for _ in 0..depth {
        json.push(']');
    }
    json
}

#[test]
fn parse_json_supports_all_json_root_value_kinds() {
    assert!(matches!(builtins::parse_json("null").expect("null root should parse"), Value::Null));
    assert!(matches!(
        builtins::parse_json("true").expect("bool root should parse"),
        Value::Bool(true)
    ));
    assert!(matches!(
        builtins::parse_json("42").expect("number root should parse"),
        Value::Int(42)
    ));
    assert!(matches!(
        builtins::parse_json("\"kujo\"").expect("string root should parse"),
        Value::Str(text) if text.as_ref() == "kujo"
    ));
    assert!(matches!(
        builtins::parse_json("[1,2,3]").expect("array root should parse"),
        Value::Array(values) if values.len() == 3
    ));

    let object =
        builtins::parse_json("{\"name\":\"kujo\",\"ok\":true}").expect("object root should parse");
    match object {
        Value::Dict(map) => {
            assert!(matches!(
                map.get("name"),
                Some(Value::Str(name)) if name.as_ref() == "kujo"
            ));
            assert!(matches!(map.get("ok"), Some(Value::Bool(true))));
        }
        other => panic!("expected parsed object to produce Value::Dict, got {:?}", other),
    }
}

#[test]
fn parse_json_invalid_input_reports_location() {
    let error = builtins::parse_json("{\"name\": }").expect_err("invalid json should fail");
    assert!(error.contains("line"), "expected line information, got: {}", error);
    assert!(error.contains("column"), "expected column information, got: {}", error);
}

#[test]
fn parse_json_rejects_excessive_input_size() {
    let oversized = format!("\"{}\"", "x".repeat(2 * 1024 * 1024));
    let error = builtins::parse_json(&oversized).expect_err("oversized json should fail");
    assert!(error.contains("maximum input size"), "expected size-limit error, got: {}", error);
}

#[test]
fn parse_json_rejects_excessive_nesting_depth() {
    let too_deep = nested_json_array(80);
    let error = builtins::parse_json(&too_deep).expect_err("deep json should fail");
    assert!(error.contains("maximum nesting depth"), "expected depth-limit error, got: {}", error);
}

#[test]
fn to_json_stringifies_primitive_and_nested_values() {
    assert_eq!(
        builtins::to_json(&Value::Bool(true)).expect("bool stringify should succeed"),
        "true"
    );

    let mut nested = DictMap::default();
    nested.insert(Arc::<str>::from("z"), Value::Int(26));
    nested
        .insert(Arc::<str>::from("a"), Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])));

    let encoded = builtins::to_json(&Value::Dict(Arc::new(nested)))
        .expect("nested dict stringify should succeed");
    assert_eq!(encoded, "{\"a\":[1,2],\"z\":26}");
}

#[test]
fn to_json_rejects_non_finite_numbers() {
    let nan_error = builtins::to_json(&Value::Float(f64::NAN)).expect_err("NaN should fail");
    assert!(
        nan_error.contains("non-finite"),
        "expected non-finite float error, got: {}",
        nan_error
    );

    let inf_error =
        builtins::to_json(&Value::Float(f64::INFINITY)).expect_err("infinity should fail");
    assert!(
        inf_error.contains("non-finite"),
        "expected non-finite float error, got: {}",
        inf_error
    );
}

#[test]
fn to_json_rejects_unsupported_value_types() {
    let error = builtins::to_json(&Value::NativeFunction("print".to_string()))
        .expect_err("native function should not serialize to json");
    assert!(error.contains("Cannot convert"), "expected unsupported-type error, got: {}", error);
}
