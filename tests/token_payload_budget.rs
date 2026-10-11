#[path = "support/native_values.rs"]
mod native_values;

use kujo::builtins;
use kujo::interpreter::Value;
use native_values::{array, call, json};
use std::sync::Arc;

#[test]
fn context_fixture_keeps_required_evidence_with_stable_byte_and_estimate_budgets() {
    let messages = json(include_str!("fixtures/context_budget/messages.json"));
    let options = json(r#"{"model":"gpt-fixture"}"#);
    let original = builtins::to_json(&messages).unwrap();
    let total = call("ai_count_tokens", &[messages.clone(), options.clone()]);
    let fitted = call("ai_fit_context", &[messages, Value::Int(40), options]);
    let encoded = builtins::to_json(&fitted).unwrap();
    let expected = json(include_str!("fixtures/context_budget/fitted.json"));
    assert!(Value::equals(&fitted, &expected), "{encoded}");
    assert_eq!(original.len(), 345);
    assert_eq!(encoded.len(), 185);
    assert!(matches!(total, Value::Int(80)), "{total:?}");
    let Value::Dict(fields) = fitted else { panic!("fit dictionary") };
    let retained = fields.get("messages").unwrap().clone();
    let recount = call("ai_count_tokens", &[retained, json(r#"{"model":"gpt-fixture"}"#)]);
    assert!(Value::equals(&recount, fields.get("est_tokens").unwrap()));

    // Errors are also an output contract. Never trade validation for a budget.
    let invalid = array(vec![json(r#"{"role":"assistant","content":1}"#)]);
    let error = call("ai_fit_context", &[invalid, Value::Int(0)]);
    assert!(
        matches!(error, Value::Error(message) if message == "ai_fit_context() requires messages[0].content to be a string")
    );
    // Keep non-ASCII counting distinct from UTF-8 payload bytes.
    let unicode = Value::Str(Arc::new("世界🦀".into()));
    assert!(matches!(call("ai_count_tokens", &[unicode]), Value::Int(2)));
}
