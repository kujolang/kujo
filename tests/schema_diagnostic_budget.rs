#[path = "support/native_values.rs"]
mod native_values;
use kujo::interpreter::Value;
use native_values::{array, call, json};

#[test]
fn schema_error_paths_have_a_byte_budget() {
    let key = "p".repeat(65_536);
    let required: Vec<_> = (0..100).map(|i| format!("missing{i}")).collect();
    let value = json(&serde_json::json!({key: {}}).to_string());
    let schema =
        json(&serde_json::json!({"additionalProperties":{"required":required}}).to_string());
    assert!(
        matches!(call("json_schema_validate", &[value, schema]), Value::Error(message) if message.contains("diagnostic") && message.contains("limit"))
    );
}

#[test]
fn schema_missing_properties_have_an_error_count_budget() {
    let required: Vec<_> = (0..10_001).map(|i| format!("missing{i}")).collect();
    let schema = json(&serde_json::json!({"required":required}).to_string());
    assert!(
        matches!(call("json_schema_validate", &[json("{}"), schema]), Value::Error(message) if message.contains("diagnostic") && message.contains("limit"))
    );
}

#[test]
fn schema_array_items_share_the_diagnostic_budget() {
    let value = array(vec![json("{}"); 10_001]);
    let schema = json(r#"{"items":{"required":["missing"]}}"#);
    assert!(
        matches!(call("json_schema_validate", &[value, schema]), Value::Error(message) if message.contains("diagnostic") && message.contains("limit"))
    );
}

#[test]
fn schema_combinators_share_the_diagnostic_budget() {
    let required: Vec<_> = (0..6_000).map(|i| format!("missing{i}")).collect();
    for keyword in ["anyOf", "oneOf", "allOf"] {
        let schema = json(
            &serde_json::json!({keyword:[{"required":required},{"required":required}]}).to_string(),
        );
        assert!(
            matches!(call("json_schema_validate", &[json("{}"), schema]), Value::Error(message) if message.contains("diagnostic") && message.contains("limit")),
            "{keyword}"
        );
    }
}

#[test]
fn schema_valid_large_property_path_does_not_consume_diagnostic_budget() {
    let key = "p".repeat(65_536);
    let value = json(&serde_json::json!({key: {}}).to_string());
    let schema = json(r#"{"additionalProperties":{"type":"object"}}"#);
    let result = call("json_schema_validate", &[value, schema]);
    assert!(
        matches!(result, Value::Dict(ref fields) if matches!(fields.get("valid"), Some(Value::Bool(true)))),
        "{result:?}"
    );
}
