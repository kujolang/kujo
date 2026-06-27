// File: src/interpreter/native_functions/token.rs
//
// Deterministic token estimation and context fitting helpers.

use crate::interpreter::{DictMap, Value};
use std::sync::Arc;

const MAX_TOKEN_TEXT_CHARS: usize = 2_000_000;
const MAX_TOKEN_MESSAGES: usize = 100_000;

#[derive(Clone, Copy, Debug)]
struct TokenHeuristic {
    chars_per_token: usize,
    message_overhead: i64,
    name_overhead: i64,
}

impl TokenHeuristic {
    fn for_model(model: &str) -> Self {
        let lower = model.to_ascii_lowercase();
        if lower.starts_with("gpt") {
            return Self { chars_per_token: 4, message_overhead: 4, name_overhead: 1 };
        }
        if lower.starts_with("text-embedding") {
            return Self { chars_per_token: 4, message_overhead: 0, name_overhead: 0 };
        }
        Self { chars_per_token: 4, message_overhead: 3, name_overhead: 1 }
    }
}

#[derive(Clone, Debug)]
struct AiMessage {
    role: String,
    content: String,
    name: Option<String>,
    value: Value,
}

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "ai_count_tokens" => handle_ai_count_tokens(arg_values),
        "ai_fit_context" => handle_ai_fit_context(arg_values),
        _ => return None,
    };

    Some(result)
}

fn handle_ai_count_tokens(arg_values: &[Value]) -> Value {
    if !(1..=2).contains(&arg_values.len()) {
        return Value::Error(format!(
            "ai_count_tokens() expects 1 to 2 arguments, got {}",
            arg_values.len()
        ));
    }

    let options = match parse_options(arg_values.get(1), "ai_count_tokens") {
        Ok(options) => options,
        Err(error) => return error,
    };
    let model = match model_from_options(&options, "ai_count_tokens") {
        Ok(model) => model,
        Err(error) => return error,
    };
    let heuristic = TokenHeuristic::for_model(&model);

    match estimate_value_tokens(&arg_values[0], heuristic, "ai_count_tokens") {
        Ok(count) => Value::Int(count),
        Err(error) => error,
    }
}

fn handle_ai_fit_context(arg_values: &[Value]) -> Value {
    if !(2..=3).contains(&arg_values.len()) {
        return Value::Error(format!(
            "ai_fit_context() expects 2 to 3 arguments, got {}",
            arg_values.len()
        ));
    }

    let messages = match parse_messages(&arg_values[0], "ai_fit_context") {
        Ok(messages) => messages,
        Err(error) => return error,
    };
    let max_tokens = match parse_max_tokens(&arg_values[1]) {
        Ok(max_tokens) => max_tokens,
        Err(error) => return error,
    };
    let options = match parse_options(arg_values.get(2), "ai_fit_context") {
        Ok(options) => options,
        Err(error) => return error,
    };
    let model = match model_from_options(&options, "ai_fit_context") {
        Ok(model) => model,
        Err(error) => return error,
    };
    let heuristic = TokenHeuristic::for_model(&model);

    match fit_context(messages, max_tokens, heuristic) {
        Ok((messages, dropped, est_tokens, fits)) => {
            let mut result = DictMap::default();
            result.insert(Arc::<str>::from("messages"), Value::Array(Arc::new(messages)));
            result.insert(Arc::<str>::from("dropped"), Value::Int(dropped));
            result.insert(Arc::<str>::from("est_tokens"), Value::Int(est_tokens));
            result.insert(Arc::<str>::from("fits"), Value::Bool(fits));
            Value::Dict(Arc::new(result))
        }
        Err(error) => error,
    }
}

fn parse_options(value: Option<&Value>, surface: &str) -> Result<DictMap, Value> {
    match value {
        None => Ok(DictMap::default()),
        Some(Value::Dict(options)) => Ok((**options).clone()),
        Some(Value::FixedDict { keys, values }) => {
            let mut options = DictMap::default();
            for (key, value) in keys.iter().zip(values.iter()) {
                options.insert(key.clone(), value.clone());
            }
            Ok(options)
        }
        Some(_) => Err(Value::Error(format!("{surface}() requires options to be a dictionary"))),
    }
}

fn model_from_options(options: &DictMap, surface: &str) -> Result<String, Value> {
    match options.get("model") {
        Some(Value::Str(model)) => Ok(model.as_ref().clone()),
        Some(_) => Err(Value::Error(format!(
            "{surface}() requires options.model to be a string when provided"
        ))),
        None => Ok(String::new()),
    }
}

fn estimate_value_tokens(
    value: &Value,
    heuristic: TokenHeuristic,
    surface: &str,
) -> Result<i64, Value> {
    match value {
        Value::Str(text) => estimate_text_tokens(text, heuristic, surface),
        Value::Array(_) => {
            let messages = parse_messages(value, surface)?;
            estimate_messages_tokens(&messages, heuristic, surface)
        }
        _ => Err(Value::Error(format!(
            "{surface}() expects first argument to be a string or messages array"
        ))),
    }
}

fn parse_messages(value: &Value, surface: &str) -> Result<Vec<AiMessage>, Value> {
    let Value::Array(messages) = value else {
        return Err(Value::Error(format!("{surface}() requires messages to be an array")));
    };

    if messages.len() > MAX_TOKEN_MESSAGES {
        return Err(Value::Error(format!(
            "{surface}() messages exceeds message limit ({MAX_TOKEN_MESSAGES})"
        )));
    }

    messages
        .iter()
        .enumerate()
        .map(|(index, message)| parse_message(message, index, surface))
        .collect()
}

fn parse_message(value: &Value, index: usize, surface: &str) -> Result<AiMessage, Value> {
    let role = match get_key(value, "role") {
        Some(Value::Str(role)) if !role.is_empty() => role.as_ref().clone(),
        _ => {
            return Err(Value::Error(format!(
                "{surface}() requires messages[{index}].role to be a non-empty string"
            )))
        }
    };
    let content = match get_key(value, "content") {
        Some(Value::Str(content)) => content.as_ref().clone(),
        _ => {
            return Err(Value::Error(format!(
                "{surface}() requires messages[{index}].content to be a string"
            )))
        }
    };
    let name = match get_key(value, "name") {
        Some(Value::Str(name)) if !name.is_empty() => Some(name.as_ref().clone()),
        Some(Value::Str(_)) | None => None,
        Some(_) => {
            return Err(Value::Error(format!(
                "{surface}() requires messages[{index}].name to be a string when provided"
            )))
        }
    };

    Ok(AiMessage { role, content, name, value: normalized_message_value(value) })
}

fn normalized_message_value(value: &Value) -> Value {
    match value {
        Value::Dict(_) | Value::FixedDict { .. } => {
            let mut message = DictMap::default();
            if let Some(role) = get_key(value, "role") {
                message.insert(Arc::<str>::from("role"), role.clone());
            }
            if let Some(content) = get_key(value, "content") {
                message.insert(Arc::<str>::from("content"), content.clone());
            }
            if let Some(name) = get_key(value, "name") {
                message.insert(Arc::<str>::from("name"), name.clone());
            }
            Value::Dict(Arc::new(message))
        }
        other => other.clone(),
    }
}

fn get_key<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Dict(map) => map.get(key),
        Value::FixedDict { keys, values } => keys
            .iter()
            .position(|candidate| candidate.as_ref() == key)
            .and_then(|index| values.get(index)),
        _ => None,
    }
}

fn parse_max_tokens(value: &Value) -> Result<i64, Value> {
    match value {
        Value::Int(max_tokens) if *max_tokens >= 0 => Ok(*max_tokens),
        _ => Err(Value::Error(
            "ai_fit_context() requires max_tokens to be a non-negative integer".to_string(),
        )),
    }
}

fn fit_context(
    messages: Vec<AiMessage>,
    max_tokens: i64,
    heuristic: TokenHeuristic,
) -> Result<(Vec<Value>, i64, i64, bool), Value> {
    let original_len = messages.len();
    let last_user_index =
        messages.iter().rposition(|message| message.role.eq_ignore_ascii_case("user"));
    let mut kept: Vec<Option<AiMessage>> = messages.into_iter().map(Some).collect();

    loop {
        let current_messages: Vec<AiMessage> = kept.iter().filter_map(Clone::clone).collect();
        let est_tokens = estimate_messages_tokens(&current_messages, heuristic, "ai_fit_context")?;
        if est_tokens <= max_tokens {
            return Ok((
                current_messages.into_iter().map(|message| message.value).collect(),
                (original_len - kept.iter().filter(|message| message.is_some()).count()) as i64,
                est_tokens,
                true,
            ));
        }

        let Some(drop_index) = oldest_droppable_index(&kept, last_user_index) else {
            return Ok((
                current_messages.into_iter().map(|message| message.value).collect(),
                (original_len - kept.iter().filter(|message| message.is_some()).count()) as i64,
                est_tokens,
                false,
            ));
        };
        kept[drop_index] = None;
    }
}

fn oldest_droppable_index(
    messages: &[Option<AiMessage>],
    last_user_index: Option<usize>,
) -> Option<usize> {
    messages.iter().enumerate().find_map(|(index, message)| {
        let message = message.as_ref()?;
        if message.role.eq_ignore_ascii_case("system") || Some(index) == last_user_index {
            None
        } else {
            Some(index)
        }
    })
}

fn estimate_messages_tokens(
    messages: &[AiMessage],
    heuristic: TokenHeuristic,
    surface: &str,
) -> Result<i64, Value> {
    let mut total = 0_i64;
    for message in messages {
        total = checked_add(total, heuristic.message_overhead, surface)?;
        total =
            checked_add(total, estimate_text_tokens(&message.role, heuristic, surface)?, surface)?;
        total = checked_add(
            total,
            estimate_text_tokens(&message.content, heuristic, surface)?,
            surface,
        )?;
        if let Some(name) = &message.name {
            total = checked_add(total, heuristic.name_overhead, surface)?;
            total = checked_add(total, estimate_text_tokens(name, heuristic, surface)?, surface)?;
        }
    }
    Ok(total)
}

fn estimate_text_tokens(
    text: &str,
    heuristic: TokenHeuristic,
    surface: &str,
) -> Result<i64, Value> {
    let char_count = text.chars().count();
    if char_count > MAX_TOKEN_TEXT_CHARS {
        return Err(Value::Error(format!(
            "{surface}() text exceeds character limit ({MAX_TOKEN_TEXT_CHARS})"
        )));
    }
    if char_count == 0 {
        return Ok(0);
    }

    let weighted_chars: usize = text.chars().map(|ch| if ch.is_ascii() { 1 } else { 2 }).sum();
    Ok(weighted_chars.div_ceil(heuristic.chars_per_token) as i64)
}

fn checked_add(left: i64, right: i64, surface: &str) -> Result<i64, Value> {
    left.checked_add(right).ok_or_else(|| {
        Value::Error(format!("{surface}() estimated token count exceeds integer range"))
    })
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::interpreter::{DictMap, Value};
    use std::sync::Arc;

    fn s(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    fn d(entries: Vec<(&str, Value)>) -> Value {
        let mut map = DictMap::default();
        for (key, value) in entries {
            map.insert(Arc::<str>::from(key), value);
        }
        Value::Dict(Arc::new(map))
    }

    fn a(values: Vec<Value>) -> Value {
        Value::Array(Arc::new(values))
    }

    fn count(value: Value, options: Value) -> i64 {
        match handle("ai_count_tokens", &[value, options]).unwrap() {
            Value::Int(value) => value,
            other => panic!("expected int count, got {other:?}"),
        }
    }

    #[test]
    fn count_tokens_is_stable_for_text_and_model_families() {
        assert_eq!(count(s("Hello, Kujo!"), d(vec![("model", s("gpt-4o"))])), 3);
        assert_eq!(count(s("Hello, Kujo!"), d(vec![("model", s("text-embedding-3-small"))])), 3);
        assert_eq!(count(s("abcde"), d(vec![])), 2);
        assert_eq!(count(s(""), d(vec![])), 0);
    }

    #[test]
    fn count_tokens_counts_role_and_content_for_messages() {
        let messages = a(vec![
            d(vec![("role", s("system")), ("content", s("You are concise."))]),
            d(vec![("role", s("user")), ("content", s("Summarize the vector helper."))]),
        ]);

        assert_eq!(count(messages, d(vec![("model", s("gpt-4o"))])), 22);
    }

    #[test]
    fn fit_context_drops_oldest_non_system_messages_and_preserves_last_user() {
        let messages = a(vec![
            d(vec![("role", s("system")), ("content", s("Stay brief."))]),
            d(vec![("role", s("user")), ("content", s("Old question with lots of context."))]),
            d(vec![("role", s("assistant")), ("content", s("Old answer."))]),
            d(vec![("role", s("user")), ("content", s("Final question."))]),
        ]);

        let result =
            handle("ai_fit_context", &[messages, Value::Int(20), d(vec![("model", s("gpt-4o"))])])
                .unwrap();

        let Value::Dict(result) = result else {
            panic!("expected fit result dict");
        };
        assert!(matches!(result.get("dropped"), Some(Value::Int(2))));
        assert!(matches!(result.get("est_tokens"), Some(Value::Int(18))));
        assert!(matches!(result.get("fits"), Some(Value::Bool(true))));

        let Some(Value::Array(fitted)) = result.get("messages") else {
            panic!("expected fitted messages array");
        };
        assert_eq!(fitted.len(), 2);
        let first = match &fitted[0] {
            Value::Dict(map) => map,
            other => panic!("expected message dict, got {other:?}"),
        };
        let second = match &fitted[1] {
            Value::Dict(map) => map,
            other => panic!("expected message dict, got {other:?}"),
        };
        assert!(matches!(first.get("role"), Some(Value::Str(role)) if role.as_ref() == "system"));
        assert!(matches!(second.get("role"), Some(Value::Str(role)) if role.as_ref() == "user"));
        assert!(
            matches!(second.get("content"), Some(Value::Str(content)) if content.as_ref() == "Final question.")
        );
    }

    #[test]
    fn fit_context_reports_over_budget_when_minimum_context_is_too_large() {
        let messages = a(vec![
            d(vec![("role", s("system")), ("content", s("Do not drop this system message."))]),
            d(vec![("role", s("user")), ("content", s("Keep this final user message."))]),
        ]);

        let result =
            handle("ai_fit_context", &[messages, Value::Int(1), d(vec![("model", s("gpt-4o"))])])
                .unwrap();
        let Value::Dict(result) = result else {
            panic!("expected fit result dict");
        };
        assert!(matches!(result.get("dropped"), Some(Value::Int(0))));
        assert!(matches!(result.get("fits"), Some(Value::Bool(false))));
        assert!(matches!(result.get("est_tokens"), Some(Value::Int(value)) if *value > 1));
    }

    #[test]
    fn token_helpers_reject_invalid_inputs() {
        let invalid_options = handle("ai_count_tokens", &[s("x"), Value::Int(1)]).unwrap();
        assert!(matches!(invalid_options, Value::Error(message) if message.contains("options")));

        let invalid_messages =
            handle("ai_fit_context", &[a(vec![d(vec![("role", s("user"))])]), Value::Int(10)])
                .unwrap();
        assert!(matches!(invalid_messages, Value::Error(message) if message.contains("content")));

        let invalid_budget = handle("ai_fit_context", &[a(vec![]), Value::Int(-1)]).unwrap();
        assert!(matches!(invalid_budget, Value::Error(message) if message.contains("max_tokens")));
    }
}
