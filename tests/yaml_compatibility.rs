use kujo::{
    builtins::{parse_yaml, to_yaml},
    interpreter::Value,
};
use std::sync::Arc;

#[test]
fn yaml_preserves_scalars_tags_aliases_and_non_string_keys() {
    let value = parse_yaml("plain: yes\nboolean: true\ninteger: -42\nfloat: 1.25\nnil: null\ntagged: !custom [1, two]\nbase: &base [3, four]\nalias: *base\n7: seven\nfalse: no\n").unwrap();
    let Value::Dict(map) = value else { panic!("expected mapping") };
    assert!(matches!(map.get("plain"), Some(Value::Str(v)) if v.as_str() == "yes"));
    assert!(matches!(map.get("boolean"), Some(Value::Bool(true))));
    assert!(matches!(map.get("integer"), Some(Value::Int(-42))));
    assert!(matches!(map.get("float"), Some(Value::Float(v)) if *v == 1.25));
    assert!(matches!(map.get("nil"), Some(Value::Null)));
    assert!(matches!(map.get("Number(7)"), Some(Value::Str(v)) if v.as_str() == "seven"));
    assert!(matches!(map.get("Bool(false)"), Some(Value::Str(v)) if v.as_str() == "no"));
    for key in ["tagged", "base", "alias"] {
        assert!(matches!(map.get(key), Some(Value::Array(v)) if v.len() == 2));
    }
}

#[test]
fn yaml_serialization_keeps_existing_scalar_spelling() {
    for (value, expected) in [
        (Value::Null, "null\n"),
        (Value::Bool(true), "true\n"),
        (Value::Int(-42), "-42\n"),
        (Value::Float(1.25), "1.25\n"),
        (Value::Str(Arc::new("true".into())), "'true'\n"),
        (Value::Array(Arc::new(vec![Value::Int(1), Value::Bool(false)])), "- 1\n- false\n"),
    ] {
        assert_eq!(to_yaml(&value).unwrap(), expected);
    }
}

#[test]
fn yaml_rejects_duplicate_keys_malformed_input_and_multiple_documents() {
    for input in ["key: 1\nkey: 2\n", "[unterminated", "---\n1\n---\n2\n"] {
        assert!(parse_yaml(input).unwrap_err().starts_with("YAML parse error:"));
    }
}
