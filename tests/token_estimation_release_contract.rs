#[path = "support/native_values.rs"]
mod native_values;
use kujo::interpreter::Value;
use native_values::{array, call, json};
use std::sync::Arc;

fn text(value: String) -> Value {
    Value::Str(Arc::new(value))
}

#[test]
fn token_estimates_match_weighted_scalar_oracle_for_ascii_and_unicode() {
    // Independent scalar oracle, including byte/scalar boundaries, combining
    // marks, control characters and non-ASCII at either end of long text.
    for unit in ["", "abcde", "\0\t\n\u{7f}", "é", "界", "🦀", "e\u{301}", "a界z"] {
        for length in [0, 1, 3, 4, 15, 16, 31, 32, 127, 1024] {
            let source = unit.repeat(length);
            let weighted: usize =
                source.chars().map(|c| if (c as u32) < 128 { 1 } else { 2 }).sum();
            let expected = Value::Int(weighted.div_ceil(4) as i64);
            for model in ["", "GPT-4o", "text-embedding-3-small"] {
                let options = json(&format!(r#"{{"model":"{model}"}}"#));
                assert!(Value::equals(
                    &call("ai_count_tokens", &[text(source.clone()), options]),
                    &expected
                ));
            }
        }
    }
    for source in [format!("é{}", "a".repeat(65536)), format!("{}é", "a".repeat(65536))] {
        assert!(Value::equals(&call("ai_count_tokens", &[text(source)]), &Value::Int(16385)));
    }
}

#[test]
fn token_text_limits_remain_character_based_and_keep_exact_errors() {
    for (unit, expected) in [("a", 500000), ("界", 1000000), ("🦀", 1000000)] {
        let allowed = unit.repeat(2_000_000);
        assert!(Value::equals(&call("ai_count_tokens", &[text(allowed)]), &Value::Int(expected)));
        let oversized = text(unit.repeat(2_000_001));
        let result = call("ai_count_tokens", &[oversized.clone()]);
        assert!(
            matches!(result, Value::Error(e) if e == "ai_count_tokens() text exceeds character limit (2000000)")
        );
        let message = Value::FixedDict {
            keys: Arc::new(vec![Arc::from("role"), Arc::from("content")]),
            values: vec![text("assistant".into()), oversized],
        };
        let result = call("ai_fit_context", &[array(vec![message]), Value::Int(0)]);
        assert!(
            matches!(result, Value::Error(e) if e == "ai_fit_context() text exceeds character limit (2000000)")
        );
    }
}
