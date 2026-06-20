// File: src/interpreter/native_functions/http.rs
//
// HTTP client native functions

use crate::interpreter::{DictMap, Value};
use crate::runtime_limits;
use crate::{builtins, network_policy};
use reqwest::Method;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const MAX_AI_TOOL_LOOP_STEPS: i64 = 16;

struct AiRequestConfig {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    timeout_seconds: f64,
    headers: Vec<(String, String)>,
}

fn ai_err_result(message: impl Into<String>) -> Value {
    Value::Result { is_ok: false, value: Box::new(Value::Str(Arc::new(message.into()))) }
}

fn ai_ok_result(value: Value) -> Value {
    Value::Result { is_ok: true, value: Box::new(value) }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Int(v) => Some(*v),
        Value::Float(v) => Some(*v as i64),
        _ => None,
    }
}

fn value_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Int(v) => Some(*v as f64),
        Value::Float(v) => Some(*v),
        _ => None,
    }
}

fn parse_ai_headers(options: &DictMap, surface: &str) -> Result<Vec<(String, String)>, Value> {
    let mut headers = Vec::new();
    if let Some(raw_headers) = options.get("headers") {
        let header_dict = dict_like_from_value(raw_headers).ok_or_else(|| {
            Value::Error(format!(
                "{}() requires options.headers to be a dictionary of string values",
                surface
            ))
        })?;

        for (key, value) in header_dict.iter() {
            match value {
                Value::Str(text) => headers.push((key.to_string(), text.to_string())),
                _ => {
                    return Err(Value::Error(format!(
                        "{}() requires options.headers['{}'] to be a string",
                        surface, key
                    )));
                }
            }
        }
    }

    Ok(headers)
}

fn dict_like_from_value(value: &Value) -> Option<DictMap> {
    match value {
        Value::Dict(dict) => Some((**dict).clone()),
        Value::FixedDict { keys, values } => {
            let mut result = DictMap::default();
            for (key, value) in keys.iter().zip(values.iter()) {
                result.insert(key.clone(), value.clone());
            }
            Some(result)
        }
        _ => None,
    }
}

fn header_pairs_from_value(value: &Value) -> Option<Vec<(String, String)>> {
    let dict = dict_like_from_value(value)?;
    Some(
        dict.iter()
            .filter_map(|(key, value)| {
                if let Value::Str(header_value) = value {
                    Some((key.to_string(), header_value.to_string()))
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn parse_ai_request_config(options: &DictMap, surface: &str) -> Result<AiRequestConfig, Value> {
    let endpoint = match options.get("endpoint") {
        Some(Value::Str(endpoint)) if !endpoint.trim().is_empty() => endpoint.as_ref().clone(),
        Some(Value::Str(_)) => {
            return Err(Value::Error(format!(
                "{}() requires options.endpoint to be a non-empty string",
                surface
            )));
        }
        _ => {
            return Err(Value::Error(format!("{}() requires options.endpoint (string)", surface)));
        }
    };

    let model = match options.get("model") {
        Some(Value::Str(model)) if !model.trim().is_empty() => model.as_ref().clone(),
        Some(Value::Str(_)) => {
            return Err(Value::Error(format!(
                "{}() requires options.model to be a non-empty string",
                surface
            )));
        }
        _ => {
            return Err(Value::Error(format!("{}() requires options.model (string)", surface)));
        }
    };

    let api_key = match options.get("api_key") {
        Some(Value::Str(key)) if !key.is_empty() => Some(key.as_ref().clone()),
        Some(Value::Str(_)) | None => None,
        Some(_) => {
            return Err(Value::Error(format!(
                "{}() requires options.api_key to be a string when provided",
                surface
            )));
        }
    };

    let timeout_seconds = match options.get("timeout") {
        Some(value) => {
            let timeout = value_to_f64(value).ok_or_else(|| {
                Value::Error(format!(
                    "{}() requires options.timeout to be a positive number",
                    surface
                ))
            })?;
            if timeout <= 0.0 {
                return Err(Value::Error(format!(
                    "{}() requires options.timeout to be a positive number",
                    surface
                )));
            }
            timeout
        }
        None => network_policy::default_http_timeout().as_secs_f64(),
    };

    let headers = parse_ai_headers(options, surface)?;

    Ok(AiRequestConfig { endpoint, model, api_key, timeout_seconds, headers })
}

fn ai_message(role: &str, content: impl Into<String>) -> Value {
    let mut message = DictMap::default();
    message.insert("role".into(), Value::Str(Arc::new(role.to_string())));
    message.insert("content".into(), Value::Str(Arc::new(content.into())));
    Value::Dict(Arc::new(message))
}

fn parse_ai_messages(input: &Value, surface: &str) -> Result<Vec<Value>, Value> {
    match input {
        Value::Str(prompt) => Ok(vec![ai_message("user", prompt.as_ref().clone())]),
        Value::Array(messages) => {
            let mut normalized = Vec::new();
            for (index, message) in messages.iter().enumerate() {
                let dict = match message {
                    Value::Dict(dict) => dict,
                    _ => {
                        return Err(Value::Error(format!(
                            "{}() requires messages[{}] to be a dictionary with role/content fields",
                            surface, index
                        )));
                    }
                };

                let role = match dict.get("role") {
                    Some(Value::Str(role)) if !role.is_empty() => role.as_ref().clone(),
                    _ => {
                        return Err(Value::Error(format!(
                            "{}() requires messages[{}].role to be a non-empty string",
                            surface, index
                        )));
                    }
                };

                let content = match dict.get("content") {
                    Some(Value::Str(content)) => content.as_ref().clone(),
                    _ => {
                        return Err(Value::Error(format!(
                            "{}() requires messages[{}].content to be a string",
                            surface, index
                        )));
                    }
                };

                normalized.push(ai_message(&role, content));
            }
            Ok(normalized)
        }
        _ => Err(Value::Error(format!(
            "{}() expects first argument to be a prompt string or messages array",
            surface
        ))),
    }
}

fn parse_ai_embedding_input(input: &Value, surface: &str) -> Result<Value, Value> {
    match input {
        Value::Str(text) => Ok(Value::Str(text.clone())),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                if !matches!(item, Value::Str(_)) {
                    return Err(Value::Error(format!(
                        "{}() requires input[{}] to be a string",
                        surface, index
                    )));
                }
            }
            Ok(Value::Array(items.clone()))
        }
        _ => Err(Value::Error(format!(
            "{}() expects first argument to be a string or array of strings",
            surface
        ))),
    }
}

fn merge_ai_extra_body(
    payload: &mut DictMap,
    options: &DictMap,
    reserved_keys: &[&str],
    surface: &str,
) -> Result<(), Value> {
    let Some(extra_body) = options.get("body") else {
        return Ok(());
    };
    let extra_body = dict_like_from_value(extra_body).ok_or_else(|| {
        Value::Error(format!(
            "{}() requires options.body to be a dictionary when provided",
            surface
        ))
    })?;

    for (key, value) in extra_body.iter() {
        if reserved_keys.iter().any(|reserved| *reserved == key.as_ref()) {
            return Err(Value::Error(format!(
                "{}() reserves options.body['{}']; pass it via top-level options instead",
                surface, key
            )));
        }
        payload.insert(key.clone(), value.clone());
    }

    Ok(())
}

fn truncate_for_error(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (count, ch) in text.chars().enumerate() {
        if count >= max_chars {
            out.push_str("...");
            return out;
        }
        out.push(ch);
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn is_ai_hash_excluded_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization"
            | "proxy-authorization"
            | "api-key"
            | "x-api-key"
            | "date"
            | "user-agent"
            | "x-request-id"
            | "request-id"
            | "idempotency-key"
    )
}

fn canonical_ai_hash_headers(headers: &[(String, String)]) -> Value {
    let mut entries: Vec<(String, String)> = headers
        .iter()
        .filter_map(|(name, value)| {
            if is_ai_hash_excluded_header(name) {
                None
            } else {
                Some((name.to_ascii_lowercase(), value.clone()))
            }
        })
        .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    Value::Array(Arc::new(
        entries
            .into_iter()
            .map(|(name, value)| {
                let mut entry = DictMap::default();
                entry.insert("name".into(), Value::Str(Arc::new(name)));
                entry.insert("value".into(), Value::Str(Arc::new(value)));
                Value::Dict(Arc::new(entry))
            })
            .collect(),
    ))
}

fn build_ai_request_hash_value(
    prompt_or_messages: &Value,
    options: &DictMap,
) -> Result<Value, Value> {
    let config = parse_ai_request_config(options, "ai_request_hash")?;
    let messages = parse_ai_messages(prompt_or_messages, "ai_request_hash")?;

    let mut payload = DictMap::default();
    payload.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
    payload.insert("messages".into(), Value::Array(Arc::new(messages)));
    merge_ai_extra_body(&mut payload, options, &["model", "messages"], "ai_request_hash")?;

    let mut normalized = DictMap::default();
    normalized.insert("_hash_version".into(), Value::Int(1));
    normalized.insert("endpoint".into(), Value::Str(Arc::new(config.endpoint.trim().to_string())));
    normalized.insert("model".into(), Value::Str(Arc::new(config.model)));
    normalized.insert("headers".into(), canonical_ai_hash_headers(&config.headers));
    normalized.insert("body".into(), Value::Dict(Arc::new(payload)));

    Ok(Value::Dict(Arc::new(normalized)))
}

fn ai_request_hash_value(prompt_or_messages: &Value, options: &DictMap) -> Result<String, Value> {
    let normalized = build_ai_request_hash_value(prompt_or_messages, options)?;
    let canonical_json = builtins::to_json(&normalized).map_err(|error| {
        Value::Error(format!("ai_request_hash() failed to serialize normalized request: {}", error))
    })?;
    Ok(sha256_hex(canonical_json.as_bytes()))
}

fn run_ai_request(
    surface: &str,
    config: &AiRequestConfig,
    payload: Value,
) -> Result<(i64, DictMap, String, Value), String> {
    network_policy::enforce_http_url_destination_policy(&config.endpoint, surface)?;
    let payload_json = builtins::to_json(&payload).map_err(|error| {
        format!("{} failed: request body serialization error: {}", surface, error)
    })?;

    let endpoint = config.endpoint.clone();
    let api_key = config.api_key.clone();
    let headers = config.headers.clone();
    let timeout_seconds = config.timeout_seconds;
    let surface_for_task = surface.to_string();

    let request_result = network_policy::run_blocking_http_task(surface, move || {
        let client = network_policy::build_http_client(Duration::from_secs_f64(timeout_seconds))?;
        let mut request = client.post(&endpoint);
        request = request.header("Content-Type", "application/json");
        request = request.header("Accept", "application/json");

        if let Some(api_key) = api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        for (name, value) in headers {
            request = request.header(&name, &value);
        }

        let response = request
            .body(payload_json)
            .send()
            .map_err(|error| format!("{} failed: {}", surface_for_task, error))?;
        network_policy::read_http_response_bytes(response, surface_for_task.as_str())
    })?;

    let (status, response_headers, body_bytes) = request_result;
    let body_text = String::from_utf8_lossy(&body_bytes).to_string();
    let parsed_body = builtins::parse_json(&body_text)
        .map_err(|error| format!("{} failed: response was not valid JSON ({})", surface, error))?;

    let mut headers_dict = DictMap::default();
    for (name, value) in response_headers.iter() {
        if let Ok(value_str) = value.to_str() {
            headers_dict.insert(
                name.as_str().to_string().into(),
                Value::Str(Arc::new(value_str.to_string())),
            );
        }
    }

    Ok((status as i64, headers_dict, body_text, parsed_body))
}

fn extract_chat_content(response_json: &Value) -> Option<String> {
    let root = match response_json {
        Value::Dict(root) => root,
        _ => return None,
    };
    let choices = match root.get("choices") {
        Some(Value::Array(choices)) => choices,
        _ => return None,
    };
    let first_choice = match choices.first() {
        Some(Value::Dict(choice)) => choice,
        _ => return None,
    };

    if let Some(Value::Dict(message)) = first_choice.get("message") {
        if let Some(Value::Str(content)) = message.get("content") {
            return Some(content.as_ref().clone());
        }
    }
    if let Some(Value::Str(text)) = first_choice.get("text") {
        return Some(text.as_ref().clone());
    }
    None
}

fn extract_chat_chunks(response_json: &Value) -> Vec<Value> {
    let mut chunks = Vec::new();
    let Some(root) = (match response_json {
        Value::Dict(root) => Some(root),
        _ => None,
    }) else {
        return chunks;
    };

    if let Some(Value::Array(choice_values)) = root.get("choices") {
        for choice in choice_values.iter() {
            if let Value::Dict(choice_dict) = choice {
                if let Some(Value::Dict(delta)) = choice_dict.get("delta") {
                    if let Some(Value::Str(content)) = delta.get("content") {
                        chunks.push(Value::Str(content.clone()));
                    }
                } else if let Some(Value::Dict(message)) = choice_dict.get("message") {
                    if let Some(Value::Str(content)) = message.get("content") {
                        chunks.push(Value::Str(content.clone()));
                    }
                } else if let Some(Value::Str(text)) = choice_dict.get("text") {
                    chunks.push(Value::Str(text.clone()));
                }
            }
        }
    }

    chunks
}

fn extract_embedding_vector(response_json: &Value) -> Option<Vec<Value>> {
    let root = match response_json {
        Value::Dict(root) => root,
        _ => return None,
    };
    let data = match root.get("data") {
        Some(Value::Array(data)) => data,
        _ => return None,
    };
    let first_item = match data.first() {
        Some(Value::Dict(item)) => item,
        _ => return None,
    };
    let embedding = match first_item.get("embedding") {
        Some(Value::Array(embedding)) => embedding,
        _ => return None,
    };

    let mut vector = Vec::new();
    for value in embedding.iter() {
        match value_to_f64(value) {
            Some(number) => vector.push(Value::Float(number)),
            None => return None,
        }
    }

    Some(vector)
}

fn extract_tool_call_names(response_json: &Value) -> Vec<String> {
    let mut names = Vec::new();
    let root = match response_json {
        Value::Dict(root) => root,
        _ => return names,
    };
    let choices = match root.get("choices") {
        Some(Value::Array(choices)) => choices,
        _ => return names,
    };
    let first_choice = match choices.first() {
        Some(Value::Dict(choice)) => choice,
        _ => return names,
    };
    let message = match first_choice.get("message") {
        Some(Value::Dict(message)) => message,
        _ => return names,
    };
    let tool_calls = match message.get("tool_calls") {
        Some(Value::Array(tool_calls)) => tool_calls,
        _ => return names,
    };

    for tool_call in tool_calls.iter() {
        let Value::Dict(call_dict) = tool_call else {
            continue;
        };
        let Some(Value::Dict(function_dict)) = call_dict.get("function") else {
            continue;
        };
        let Some(Value::Str(name)) = function_dict.get("name") else {
            continue;
        };
        names.push(name.as_ref().clone());
    }

    names
}

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "parallel_http" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "parallel_http() expects 1 argument (urls), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Array(urls)) = arg_values.first() {
                if urls.len() > runtime_limits::MAX_PARALLEL_HTTP_REQUESTS {
                    return Some(Value::Error(format!(
                        "parallel_http() request count exceeds maximum ({} > {})",
                        urls.len(),
                        runtime_limits::MAX_PARALLEL_HTTP_REQUESTS
                    )));
                }

                let mut url_strings = Vec::with_capacity(urls.len());
                for (index, value) in urls.iter().enumerate() {
                    match value {
                        Value::Str(s) => url_strings.push(s.as_ref().clone()),
                        _ => {
                            return Some(Value::Error(format!(
                                "parallel_http() URL at index {} must be a string",
                                index
                            )));
                        }
                    }
                }

                let mut handles = Vec::new();
                for url in url_strings {
                    let handle = std::thread::spawn(move || -> Result<(u16, String), String> {
                        network_policy::enforce_http_url_destination_policy(&url, "HTTP GET")?;
                        let client = network_policy::build_http_client(
                            network_policy::default_http_timeout(),
                        )?;
                        let response = client
                            .get(&url)
                            .send()
                            .map_err(|e| format!("HTTP GET failed: {}", e))?;
                        let (status, _, body_bytes) =
                            network_policy::read_http_response_bytes(response, "HTTP GET")?;
                        Ok((status, String::from_utf8_lossy(&body_bytes).to_string()))
                    });
                    handles.push(handle);
                }

                let mut results = Vec::new();
                for handle in handles {
                    match handle.join() {
                        Ok(Ok((status, body))) => {
                            let mut result_map = DictMap::default();
                            result_map.insert("status".into(), Value::Int(status as i64));
                            result_map.insert("body".into(), Value::Str(Arc::new(body)));
                            results.push(Value::Dict(Arc::new(result_map)));
                        }
                        Ok(Err(e)) => results.push(Value::Error(e)),
                        Err(_) => results.push(Value::Error("Thread panicked".to_string())),
                    }
                }

                Value::Array(Arc::new(results))
            } else {
                Value::Error("parallel_http requires an array of URL strings".to_string())
            }
        }

        "http_get" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "http_get() expects 1 argument (url), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(url)) = arg_values.first() {
                match builtins::http_get(url.as_ref()) {
                    Ok(result_map) => Value::Dict(Arc::new(result_map)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("http_get requires a URL string".to_string())
            }
        }

        "http_post" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "http_post() expects 2 arguments (url, body), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Str(url)), Some(Value::Str(body))) =
                (arg_values.first(), arg_values.get(1))
            {
                match builtins::http_post(url.as_ref(), body.as_ref()) {
                    Ok(result_map) => Value::Dict(Arc::new(result_map)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("http_post requires URL and JSON body strings".to_string())
            }
        }

        "http_request" => {
            if !(1..=2).contains(&arg_values.len()) {
                return Some(Value::Error(format!(
                    "http_request() expects 1-2 arguments ((request), or (url, options)), got {}",
                    arg_values.len()
                )));
            }

            let (url, options): (String, DictMap) = if arg_values.len() == 1 {
                let options = match arg_values.first().and_then(dict_like_from_value) {
                    Some(options) => options,
                    None => {
                        return Some(Value::Error(
                            "http_request() single-argument form requires a request dictionary"
                                .to_string(),
                        ));
                    }
                };

                let url = match options.get("url") {
                    Some(Value::Str(url)) if !url.trim().is_empty() => url.as_ref().clone(),
                    _ => {
                        return Some(Value::Error(
                            "http_request() request dictionary requires a non-empty 'url' string"
                                .to_string(),
                        ));
                    }
                };

                (url, options)
            } else {
                let url = match arg_values.first() {
                    Some(Value::Str(url)) => url.as_ref().clone(),
                    _ => {
                        return Some(Value::Error(
                            "http_request() requires a URL string as first argument".to_string(),
                        ));
                    }
                };

                let options = match arg_values.get(1).and_then(dict_like_from_value) {
                    Some(options) => options,
                    None => {
                        return Some(Value::Error(
                            "http_request() requires an options dictionary as second argument"
                                .to_string(),
                        ));
                    }
                };

                (url, options)
            };

            let method_name = options
                .get("method")
                .and_then(|value| {
                    if let Value::Str(text) = value {
                        Some(text.as_ref().to_uppercase())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "GET".to_string());

            if let Err(error) = Method::from_bytes(method_name.as_bytes()) {
                return Some(Value::Result {
                    is_ok: false,
                    value: Box::new(Value::Str(Arc::new(format!(
                        "Invalid HTTP method '{}': {}",
                        method_name, error
                    )))),
                });
            }

            let timeout_seconds = options
                .get("timeout")
                .and_then(|value| match value {
                    Value::Float(timeout) => Some(*timeout),
                    Value::Int(timeout) => Some(*timeout as f64),
                    _ => None,
                })
                .unwrap_or_else(|| network_policy::default_http_timeout().as_secs_f64())
                .max(0.001_f64);

            let headers: Vec<(String, String)> = options
                .get("_headers")
                .or_else(|| options.get("headers"))
                .and_then(header_pairs_from_value)
                .unwrap_or_default();

            let body = options.get("_body").or_else(|| options.get("body")).and_then(|value| {
                if let Value::Str(body) = value {
                    Some(body.to_string())
                } else {
                    None
                }
            });

            let request_result =
                network_policy::run_blocking_http_task("HTTP request", move || {
                    network_policy::enforce_http_url_destination_policy(&url, "HTTP request")?;
                    let method = Method::from_bytes(method_name.as_bytes()).map_err(|error| {
                        format!("Invalid HTTP method '{}': {}", method_name, error)
                    })?;
                    let client = network_policy::build_http_client(Duration::from_secs_f64(
                        timeout_seconds,
                    ))?;

                    let mut request = client.request(method, &url);
                    for (key, value) in headers {
                        request = request.header(&key, &value);
                    }
                    if let Some(body) = body {
                        request = request.body(body);
                    }

                    let response = request
                        .send()
                        .map_err(|error| format!("HTTP request failed: {}", error))?;
                    network_policy::read_http_response_bytes(response, "HTTP request")
                });

            match request_result {
                Ok((status, response_headers, body_bytes)) => {
                    let status = status as i64;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();

                    let mut result_dict = DictMap::default();
                    result_dict.insert("status".into(), Value::Int(status));
                    result_dict.insert("_status".into(), Value::Int(status));
                    result_dict.insert("body".into(), Value::Str(Arc::new(body.clone())));
                    result_dict.insert("_body".into(), Value::Str(Arc::new(body)));

                    let mut headers_dict = DictMap::default();
                    for (name, value) in response_headers.iter() {
                        if let Ok(value_str) = value.to_str() {
                            headers_dict.insert(
                                name.as_str().to_string().into(),
                                Value::Str(Arc::new(value_str.to_string())),
                            );
                        }
                    }
                    result_dict.insert("headers".into(), Value::Dict(Arc::new(headers_dict)));

                    Value::Result {
                        is_ok: true,
                        value: Box::new(Value::Dict(Arc::new(result_dict))),
                    }
                }
                Err(error) => {
                    Value::Result { is_ok: false, value: Box::new(Value::Str(Arc::new(error))) }
                }
            }
        }

        "ai_request_hash" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_request_hash() expects 2 arguments (prompt_or_messages, options), got {}",
                    arg_values.len()
                )));
            }

            let options = match arg_values.get(1).and_then(dict_like_from_value) {
                Some(options) => options,
                None => {
                    return Some(Value::Error(
                        "ai_request_hash() requires an options dictionary as second argument"
                            .to_string(),
                    ));
                }
            };

            match ai_request_hash_value(&arg_values[0], &options) {
                Ok(hash) => Value::Str(Arc::new(hash)),
                Err(error) => return Some(error),
            }
        }

        "ai_chat" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_chat() expects 2 arguments (prompt_or_messages, options), got {}",
                    arg_values.len()
                )));
            }

            let options = match arg_values.get(1) {
                Some(Value::Dict(options)) => options,
                _ => {
                    return Some(Value::Error(
                        "ai_chat() requires an options dictionary as second argument".to_string(),
                    ));
                }
            };

            let config = match parse_ai_request_config(options, "ai_chat") {
                Ok(config) => config,
                Err(error) => return Some(error),
            };
            let messages = match parse_ai_messages(&arg_values[0], "ai_chat") {
                Ok(messages) => messages,
                Err(error) => return Some(error),
            };

            let mut payload = DictMap::default();
            payload.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
            payload.insert("messages".into(), Value::Array(Arc::new(messages.clone())));
            if let Err(error) =
                merge_ai_extra_body(&mut payload, options, &["model", "messages"], "ai_chat")
            {
                return Some(error);
            }

            let request = run_ai_request("ai_chat", &config, Value::Dict(Arc::new(payload)));
            match request {
                Ok((status, headers, text, json)) => {
                    if !(200..300).contains(&status) {
                        return Some(ai_err_result(format!(
                            "ai_chat failed with HTTP status {}: {}",
                            status,
                            truncate_for_error(&text, 240)
                        )));
                    }

                    let message = extract_chat_content(&json).unwrap_or_default();
                    let mut result = DictMap::default();
                    result.insert("status".into(), Value::Int(status));
                    result.insert("model".into(), Value::Str(Arc::new(config.model)));
                    result.insert("message".into(), Value::Str(Arc::new(message)));
                    result.insert("text".into(), Value::Str(Arc::new(text)));
                    result.insert("json".into(), json);
                    result.insert("headers".into(), Value::Dict(Arc::new(headers)));
                    ai_ok_result(Value::Dict(Arc::new(result)))
                }
                Err(error) => ai_err_result(error),
            }
        }

        "ai_stream_chat" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_stream_chat() expects 2 arguments (prompt_or_messages, options), got {}",
                    arg_values.len()
                )));
            }

            let options = match arg_values.get(1) {
                Some(Value::Dict(options)) => options,
                _ => {
                    return Some(Value::Error(
                        "ai_stream_chat() requires an options dictionary as second argument"
                            .to_string(),
                    ));
                }
            };

            let config = match parse_ai_request_config(options, "ai_stream_chat") {
                Ok(config) => config,
                Err(error) => return Some(error),
            };
            let messages = match parse_ai_messages(&arg_values[0], "ai_stream_chat") {
                Ok(messages) => messages,
                Err(error) => return Some(error),
            };

            let mut payload = DictMap::default();
            payload.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
            payload.insert("messages".into(), Value::Array(Arc::new(messages)));
            payload.insert("stream".into(), Value::Bool(true));
            if let Err(error) = merge_ai_extra_body(
                &mut payload,
                options,
                &["model", "messages", "stream"],
                "ai_stream_chat",
            ) {
                return Some(error);
            }

            let request = run_ai_request("ai_stream_chat", &config, Value::Dict(Arc::new(payload)));
            match request {
                Ok((status, headers, text, json)) => {
                    if !(200..300).contains(&status) {
                        return Some(ai_err_result(format!(
                            "ai_stream_chat failed with HTTP status {}: {}",
                            status,
                            truncate_for_error(&text, 240)
                        )));
                    }

                    let mut chunks = extract_chat_chunks(&json);
                    if chunks.is_empty() && !text.is_empty() {
                        chunks.push(Value::Str(Arc::new(text.clone())));
                    }

                    let mut result = DictMap::default();
                    result.insert("status".into(), Value::Int(status));
                    result.insert("model".into(), Value::Str(Arc::new(config.model)));
                    result.insert("chunks".into(), Value::Array(Arc::new(chunks)));
                    result.insert("text".into(), Value::Str(Arc::new(text)));
                    result.insert("json".into(), json);
                    result.insert("headers".into(), Value::Dict(Arc::new(headers)));
                    ai_ok_result(Value::Dict(Arc::new(result)))
                }
                Err(error) => ai_err_result(error),
            }
        }

        "ai_embedding" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_embedding() expects 2 arguments (input, options), got {}",
                    arg_values.len()
                )));
            }

            let options = match arg_values.get(1) {
                Some(Value::Dict(options)) => options,
                _ => {
                    return Some(Value::Error(
                        "ai_embedding() requires an options dictionary as second argument"
                            .to_string(),
                    ));
                }
            };

            let config = match parse_ai_request_config(options, "ai_embedding") {
                Ok(config) => config,
                Err(error) => return Some(error),
            };
            let input = match parse_ai_embedding_input(&arg_values[0], "ai_embedding") {
                Ok(input) => input,
                Err(error) => return Some(error),
            };

            let mut payload = DictMap::default();
            payload.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
            payload.insert("input".into(), input);
            if let Err(error) =
                merge_ai_extra_body(&mut payload, options, &["model", "input"], "ai_embedding")
            {
                return Some(error);
            }

            let request = run_ai_request("ai_embedding", &config, Value::Dict(Arc::new(payload)));
            match request {
                Ok((status, headers, text, json)) => {
                    if !(200..300).contains(&status) {
                        return Some(ai_err_result(format!(
                            "ai_embedding failed with HTTP status {}: {}",
                            status,
                            truncate_for_error(&text, 240)
                        )));
                    }

                    let vector = match extract_embedding_vector(&json) {
                        Some(vector) => vector,
                        None => {
                            return Some(ai_err_result(
                                "ai_embedding failed: response JSON missing data[0].embedding numeric array"
                                    .to_string(),
                            ));
                        }
                    };

                    let mut result = DictMap::default();
                    result.insert("status".into(), Value::Int(status));
                    result.insert("model".into(), Value::Str(Arc::new(config.model)));
                    result.insert("vector".into(), Value::Array(Arc::new(vector)));
                    result.insert("text".into(), Value::Str(Arc::new(text)));
                    result.insert("json".into(), json);
                    result.insert("headers".into(), Value::Dict(Arc::new(headers)));
                    ai_ok_result(Value::Dict(Arc::new(result)))
                }
                Err(error) => ai_err_result(error),
            }
        }

        "ai_tool_loop" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_tool_loop() expects 2 arguments (prompt_or_messages, options), got {}",
                    arg_values.len()
                )));
            }

            let options = match arg_values.get(1) {
                Some(Value::Dict(options)) => options,
                _ => {
                    return Some(Value::Error(
                        "ai_tool_loop() requires an options dictionary as second argument"
                            .to_string(),
                    ));
                }
            };

            let config = match parse_ai_request_config(options, "ai_tool_loop") {
                Ok(config) => config,
                Err(error) => return Some(error),
            };

            let max_steps = match options.get("max_steps") {
                Some(value) => match value_to_i64(value) {
                    Some(v) if (1..=MAX_AI_TOOL_LOOP_STEPS).contains(&v) => v,
                    _ => {
                        return Some(Value::Error(format!(
                            "ai_tool_loop() requires options.max_steps to be an integer between 1 and {}",
                            MAX_AI_TOOL_LOOP_STEPS
                        )));
                    }
                },
                None => 4,
            };

            let tools = match options.get("tools") {
                Some(Value::Array(tools)) => Some(tools.clone()),
                Some(_) => {
                    return Some(Value::Error(
                        "ai_tool_loop() requires options.tools to be an array when provided"
                            .to_string(),
                    ));
                }
                None => None,
            };

            let tool_results = match options.get("tool_results") {
                Some(Value::Dict(results)) => Some(results.clone()),
                Some(_) => {
                    return Some(Value::Error(
                        "ai_tool_loop() requires options.tool_results to be a dictionary when provided"
                            .to_string(),
                    ));
                }
                None => None,
            };

            let mut messages = match parse_ai_messages(&arg_values[0], "ai_tool_loop") {
                Ok(messages) => messages,
                Err(error) => return Some(error),
            };

            let mut final_status = 0_i64;
            let mut final_text = String::new();
            let mut final_json = Value::Null;
            let mut last_message = String::new();
            let mut steps_taken = 0_i64;

            for _ in 0..max_steps {
                let mut payload = DictMap::default();
                payload.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
                payload.insert("messages".into(), Value::Array(Arc::new(messages.clone())));
                if let Some(tools) = &tools {
                    payload.insert("tools".into(), Value::Array(tools.clone()));
                }
                payload.insert("stream".into(), Value::Bool(false));
                if let Err(error) = merge_ai_extra_body(
                    &mut payload,
                    options,
                    &["model", "messages", "tools", "stream"],
                    "ai_tool_loop",
                ) {
                    return Some(error);
                }

                let request_result =
                    run_ai_request("ai_tool_loop", &config, Value::Dict(Arc::new(payload)));
                let (status, _headers, text, response_json) = match request_result {
                    Ok(result) => result,
                    Err(error) => return Some(ai_err_result(error)),
                };
                if !(200..300).contains(&status) {
                    return Some(ai_err_result(format!(
                        "ai_tool_loop failed with HTTP status {}: {}",
                        status,
                        truncate_for_error(&text, 240)
                    )));
                }

                steps_taken += 1;
                final_status = status;
                final_text = text;
                final_json = response_json.clone();
                last_message = extract_chat_content(&response_json).unwrap_or_default();
                messages.push(ai_message("assistant", last_message.clone()));

                let tool_call_names = extract_tool_call_names(&response_json);
                if tool_call_names.is_empty() {
                    break;
                }

                let Some(tool_results) = &tool_results else {
                    return Some(ai_err_result(
                        "ai_tool_loop requires options.tool_results to resolve tool_calls in model responses"
                            .to_string(),
                    ));
                };

                for tool_name in tool_call_names {
                    let tool_output = match tool_results.get(tool_name.as_str()) {
                        Some(Value::Str(output)) => output.as_ref().clone(),
                        Some(_) => {
                            return Some(ai_err_result(format!(
                                "ai_tool_loop requires options.tool_results['{}'] to be a string",
                                tool_name
                            )));
                        }
                        None => {
                            return Some(ai_err_result(format!(
                                "ai_tool_loop missing tool result for '{}'",
                                tool_name
                            )));
                        }
                    };

                    let mut tool_message = DictMap::default();
                    tool_message.insert("role".into(), Value::Str(Arc::new("tool".to_string())));
                    tool_message.insert("name".into(), Value::Str(Arc::new(tool_name.to_string())));
                    tool_message.insert("content".into(), Value::Str(Arc::new(tool_output)));
                    messages.push(Value::Dict(Arc::new(tool_message)));
                }
            }

            let mut result = DictMap::default();
            result.insert("status".into(), Value::Int(final_status));
            result.insert("model".into(), Value::Str(Arc::new(config.model)));
            result.insert("steps".into(), Value::Int(steps_taken));
            result.insert("message".into(), Value::Str(Arc::new(last_message)));
            result.insert("text".into(), Value::Str(Arc::new(final_text)));
            result.insert("json".into(), final_json);
            result.insert("messages".into(), Value::Array(Arc::new(messages)));
            ai_ok_result(Value::Dict(Arc::new(result)))
        }

        "http_put" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "http_put() expects 2 arguments (url, body), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Str(url)), Some(Value::Str(body))) =
                (arg_values.first(), arg_values.get(1))
            {
                match builtins::http_put(url.as_ref(), body.as_ref()) {
                    Ok(result_map) => Value::Dict(Arc::new(result_map)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("http_put requires URL and JSON body strings".to_string())
            }
        }

        "http_delete" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "http_delete() expects 1 argument (url), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(url)) = arg_values.first() {
                match builtins::http_delete(url.as_ref()) {
                    Ok(result_map) => Value::Dict(Arc::new(result_map)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("http_delete requires a URL string".to_string())
            }
        }

        "http_get_binary" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "http_get_binary() expects 1 argument (url), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(url)) = arg_values.first() {
                match builtins::http_get_binary(url.as_ref()) {
                    Ok(bytes) => Value::Bytes(bytes),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("http_get_binary requires a URL string".to_string())
            }
        }

        "http_get_stream" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "http_get_stream() expects 1 argument (url), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(url)) = arg_values.first() {
                match builtins::http_get_stream(url.as_ref()) {
                    Ok(bytes) => Value::Bytes(bytes),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("http_get_stream requires a URL string".to_string())
            }
        }

        "http_server" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "http_server() expects 1 argument (port), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Int(port)) = arg_values.first() {
                Value::HttpServer {
                    host: "0.0.0.0".to_string(),
                    port: *port as u16,
                    routes: Vec::new(),
                }
            } else {
                Value::Error("http_server requires a port number".to_string())
            }
        }

        "http_listen" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "http_listen() expects 2 arguments (host, port), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Str(host)), Some(Value::Int(port))) =
                (arg_values.first(), arg_values.get(1))
            {
                let host = host.trim();
                if host.is_empty() {
                    Value::Error("http_listen requires a non-empty host string".to_string())
                } else {
                    Value::HttpServer {
                        host: host.to_string(),
                        port: *port as u16,
                        routes: Vec::new(),
                    }
                }
            } else {
                Value::Error("http_listen requires host string and port number".to_string())
            }
        }

        "set_header" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(format!(
                    "set_header() expects 3 arguments (response, name, value), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(response), Some(Value::Str(key)), Some(Value::Str(value))) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                if let Value::HttpResponse { status, body, headers } = response {
                    let mut new_headers = headers.clone();
                    new_headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
                    Value::HttpResponse {
                        status: *status,
                        body: body.clone(),
                        headers: new_headers,
                    }
                } else {
                    Value::Error(
                        "set_header requires an HTTP response as first argument".to_string(),
                    )
                }
            } else {
                Value::Error(
                    "set_header requires response, header name, and header value".to_string(),
                )
            }
        }

        "set_headers" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "set_headers() expects 2 arguments (response, headers), got {}",
                    arg_values.len()
                )));
            }

            if let Some(response) = arg_values.first() {
                let header_pairs: Option<Vec<(String, Value)>> = match arg_values.get(1) {
                    Some(Value::Dict(headers_dict)) => Some(
                        headers_dict
                            .iter()
                            .map(|(key, value)| (key.as_ref().to_string(), value.clone()))
                            .collect(),
                    ),
                    Some(Value::FixedDict { keys, values }) => Some(
                        keys.iter()
                            .cloned()
                            .zip(values.iter().cloned())
                            .map(|(key, value)| (key.to_string(), value))
                            .collect(),
                    ),
                    _ => None,
                };

                if let Some(header_pairs) = header_pairs {
                    if let Value::HttpResponse { status, body, headers } = response {
                        let mut new_headers = headers.clone();
                        for (key, value) in header_pairs {
                            if let Value::Str(header_value) = value {
                                new_headers.insert(key, header_value.as_ref().to_string());
                            }
                        }
                        Value::HttpResponse {
                            status: *status,
                            body: body.clone(),
                            headers: new_headers,
                        }
                    } else {
                        Value::Error(
                            "set_headers requires an HTTP response as first argument".to_string(),
                        )
                    }
                } else {
                    Value::Error("set_headers requires response and headers dictionary".to_string())
                }
            } else {
                Value::Error("set_headers requires response and headers dictionary".to_string())
            }
        }

        "http_response" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "http_response() expects 2 arguments (status, body), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Int(status)), Some(Value::Str(body))) =
                (arg_values.first(), arg_values.get(1))
            {
                Value::HttpResponse {
                    status: *status as u16,
                    body: body.as_ref().to_string(),
                    headers: HashMap::new(),
                }
            } else {
                Value::Error("http_response requires status code and body string".to_string())
            }
        }

        "json_response" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "json_response() expects 2 arguments (status, data), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Int(status)), Some(data)) = (arg_values.first(), arg_values.get(1))
            {
                let body = match builtins::to_json(data) {
                    Ok(body) => body,
                    Err(error) => {
                        return Some(Value::Error(format!(
                            "json_response failed to serialize data: {}",
                            error
                        )));
                    }
                };
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                Value::HttpResponse { status: *status as u16, body, headers }
            } else {
                Value::Error("json_response requires status code and data".to_string())
            }
        }

        "html_response" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "html_response() expects 2 arguments (status, html), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Int(status)), Some(Value::Str(html))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "text/html; charset=utf-8".to_string());
                Value::HttpResponse {
                    status: *status as u16,
                    body: html.as_ref().to_string(),
                    headers,
                }
            } else {
                Value::Error("html_response requires status code and HTML string".to_string())
            }
        }

        "redirect_response" => {
            if !(1..=2).contains(&arg_values.len()) {
                return Some(Value::Error(format!(
                    "redirect_response() expects 1-2 arguments (url, headers?), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(url)) = arg_values.first() {
                let mut headers = HashMap::new();
                headers.insert("Location".to_string(), url.as_ref().to_string());

                if let Some(Value::Dict(extra_headers)) = arg_values.get(1) {
                    for (key, value) in extra_headers.iter() {
                        if let Value::Str(header_value) = value {
                            headers.insert(
                                key.as_ref().to_string(),
                                header_value.as_ref().to_string(),
                            );
                        }
                    }
                }

                Value::HttpResponse {
                    status: 302,
                    body: format!("Redirecting to {}", url.as_ref()),
                    headers,
                }
            } else {
                Value::Error("redirect_response requires a URL string".to_string())
            }
        }

        "jwt_encode" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "jwt_encode() expects 2 arguments (payload, secret), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Dict(payload)), Some(Value::Str(secret))) =
                (arg_values.first(), arg_values.get(1))
            {
                match builtins::jwt_encode(payload, secret) {
                    Ok(token) => Value::Str(Arc::new(token)),
                    Err(e) => Value::Error(e),
                }
            } else {
                Value::Error(
                    "jwt_encode requires a dictionary payload and secret key string".to_string(),
                )
            }
        }

        "jwt_decode" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "jwt_decode() expects 2 arguments (token, secret), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Str(token)), Some(Value::Str(secret))) =
                (arg_values.first(), arg_values.get(1))
            {
                match builtins::jwt_decode(token, secret) {
                    Ok(payload) => Value::Dict(Arc::new(payload)),
                    Err(e) => Value::Error(e),
                }
            } else {
                Value::Error("jwt_decode requires a token string and secret key string".to_string())
            }
        }

        "jwt_verify" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "jwt_verify() expects 2 arguments (token, secret), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Str(token)), Some(Value::Str(secret))) =
                (arg_values.first(), arg_values.get(1))
            {
                Value::Bool(builtins::jwt_verify(token, secret))
            } else {
                Value::Error("jwt_verify requires a token string and secret key string".to_string())
            }
        }

        "oauth2_auth_url" => {
            if arg_values.len() != 4 {
                return Some(Value::Error(format!(
                    "oauth2_auth_url() expects 4 arguments (client_id, redirect_uri, auth_url, scope), got {}",
                    arg_values.len()
                )));
            }

            if let (
                Some(Value::Str(client_id)),
                Some(Value::Str(redirect_uri)),
                Some(Value::Str(auth_url)),
                Some(Value::Str(scope)),
            ) = (arg_values.first(), arg_values.get(1), arg_values.get(2), arg_values.get(3))
            {
                Value::Str(Arc::new(builtins::oauth2_auth_url(
                    client_id.as_ref(),
                    redirect_uri.as_ref(),
                    auth_url.as_ref(),
                    scope.as_ref(),
                )))
            } else {
                Value::Error(
                    "oauth2_auth_url requires client_id, redirect_uri, auth_url, and scope strings"
                        .to_string(),
                )
            }
        }

        "oauth2_get_token" => {
            if arg_values.len() != 5 {
                return Some(Value::Error(format!(
                    "oauth2_get_token() expects 5 arguments (code, client_id, client_secret, token_url, redirect_uri), got {}",
                    arg_values.len()
                )));
            }

            if let (
                Some(Value::Str(code)),
                Some(Value::Str(client_id)),
                Some(Value::Str(client_secret)),
                Some(Value::Str(token_url)),
                Some(Value::Str(redirect_uri)),
            ) = (
                arg_values.first(),
                arg_values.get(1),
                arg_values.get(2),
                arg_values.get(3),
                arg_values.get(4),
            ) {
                match builtins::oauth2_get_token(
                    code,
                    client_id,
                    client_secret,
                    token_url,
                    redirect_uri,
                ) {
                    Ok(token_data) => Value::Dict(Arc::new(token_data)),
                    Err(e) => Value::Error(e),
                }
            } else {
                Value::Error(
                    "oauth2_get_token requires code, client_id, client_secret, token_url, and redirect_uri strings"
                        .to_string(),
                )
            }
        }

        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    fn str_value(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    fn one_shot_json_server(
        status_code: u16,
        response_body: &'static str,
    ) -> Option<(String, mpsc::Receiver<String>, std::thread::JoinHandle<()>)> {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return None,
            Err(error) => panic!("test listener should bind: {error}"),
        };
        let address = listener.local_addr().expect("test listener should have address");
        let endpoint = format!("http://127.0.0.1:{}/v1/mock", address.port());

        let (request_tx, request_rx) = mpsc::channel::<String>();
        let handle = std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };

            let mut buffer = Vec::new();
            let mut temp = [0_u8; 4096];
            let mut header_end = None;
            while header_end.is_none() {
                let read = stream.read(&mut temp).expect("request read should succeed");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&temp[..read]);
                header_end = buffer.windows(4).position(|window| window == b"\r\n\r\n");
            }

            let Some(header_end) = header_end else {
                return;
            };
            let body_start = header_end + 4;
            let header_text = String::from_utf8_lossy(&buffer[..header_end]).to_string();
            let content_length = header_text
                .lines()
                .find_map(|line| {
                    let lower = line.to_ascii_lowercase();
                    if let Some(length_str) = lower.strip_prefix("content-length:") {
                        return length_str.trim().parse::<usize>().ok();
                    }
                    None
                })
                .unwrap_or(0);

            while buffer.len().saturating_sub(body_start) < content_length {
                let read = stream.read(&mut temp).expect("request body read should succeed");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&temp[..read]);
            }

            let body_end = body_start.saturating_add(content_length).min(buffer.len());
            let body = String::from_utf8_lossy(&buffer[body_start..body_end]).to_string();
            let _ = request_tx.send(body);

            let response = format!(
                "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_code,
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });

        Some((endpoint, request_rx, handle))
    }

    fn ai_options(endpoint: &str, model: &str) -> Value {
        let mut options = DictMap::default();
        options.insert("endpoint".into(), str_value(endpoint));
        options.insert("model".into(), str_value(model));
        Value::Dict(Arc::new(options))
    }

    fn ai_hash_options(
        endpoint: &str,
        model: &str,
        api_key: &str,
        authorization: &str,
        trace_header: &str,
    ) -> Value {
        let mut headers = DictMap::default();
        headers.insert("Authorization".into(), str_value(authorization));
        headers.insert("X-Trace".into(), str_value(trace_header));

        let mut body = DictMap::default();
        body.insert("temperature".into(), Value::Float(0.2));
        body.insert("max_tokens".into(), Value::Int(64));

        let mut options = DictMap::default();
        options.insert("body".into(), Value::Dict(Arc::new(body)));
        options.insert("headers".into(), Value::Dict(Arc::new(headers)));
        options.insert("api_key".into(), str_value(api_key));
        options.insert("model".into(), str_value(model));
        options.insert("endpoint".into(), str_value(endpoint));
        Value::Dict(Arc::new(options))
    }

    fn ai_request_hash_for(prompt: &str, options: Value) -> String {
        let result = handle("ai_request_hash", &[str_value(prompt), options])
            .expect("ai_request_hash should return a value");
        match result {
            Value::Str(hash) => hash.as_ref().clone(),
            other => panic!("expected ai_request_hash to return string, got {:?}", other),
        }
    }

    #[test]
    fn test_ai_request_hash_is_stable_and_credential_independent() {
        let endpoint = "https://api.example.test/v1/chat/completions";
        let left = ai_request_hash_for(
            "Summarize Kujo",
            ai_hash_options(endpoint, "gpt-mock", "key-one", "Bearer key-one", "trace-1"),
        );
        let right = ai_request_hash_for(
            "Summarize Kujo",
            ai_hash_options(endpoint, "gpt-mock", "key-two", "Bearer key-two", "trace-1"),
        );

        assert_eq!(left, right);
        assert_eq!(left.len(), 64);
        assert!(left.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn test_ai_request_hash_changes_for_semantic_request_inputs() {
        let endpoint = "https://api.example.test/v1/chat/completions";
        let base = ai_request_hash_for(
            "Summarize Kujo",
            ai_hash_options(endpoint, "gpt-mock", "key", "Bearer key", "trace-1"),
        );
        let different_model = ai_request_hash_for(
            "Summarize Kujo",
            ai_hash_options(endpoint, "gpt-other", "key", "Bearer key", "trace-1"),
        );
        let different_prompt = ai_request_hash_for(
            "Explain Kujo",
            ai_hash_options(endpoint, "gpt-mock", "key", "Bearer key", "trace-1"),
        );
        let different_relevant_header = ai_request_hash_for(
            "Summarize Kujo",
            ai_hash_options(endpoint, "gpt-mock", "key", "Bearer key", "trace-2"),
        );

        assert_ne!(base, different_model);
        assert_ne!(base, different_prompt);
        assert_ne!(base, different_relevant_header);
    }

    #[test]
    fn test_ai_request_hash_validation_errors_match_ai_helpers() {
        let missing_options = handle("ai_request_hash", &[str_value("hello"), Value::Int(1)])
            .expect("ai_request_hash should return a value");
        assert!(
            matches!(missing_options, Value::Error(message) if message.contains("requires an options dictionary"))
        );

        let mut options = DictMap::default();
        options.insert("model".into(), str_value("gpt-mock"));
        let missing_endpoint =
            handle("ai_request_hash", &[str_value("hello"), Value::Dict(Arc::new(options))])
                .expect("ai_request_hash should return a value");
        assert!(
            matches!(missing_endpoint, Value::Error(message) if message.contains("requires options.endpoint"))
        );
    }

    #[test]
    fn test_http_response_helpers_and_header_mutation() {
        let response = handle("http_response", &[Value::Int(200), str_value("ok")]).unwrap();
        match response {
            Value::HttpResponse { status, body, headers } => {
                assert_eq!(status, 200);
                assert_eq!(body, "ok");
                assert!(headers.is_empty());
            }
            _ => panic!("Expected HttpResponse from http_response"),
        }

        let json_response = handle("json_response", &[Value::Int(201), Value::Int(7)]).unwrap();
        match json_response {
            Value::HttpResponse { status, headers, .. } => {
                assert_eq!(status, 201);
                assert_eq!(headers.get("Content-Type"), Some(&"application/json".to_string()));
            }
            _ => panic!("Expected HttpResponse from json_response"),
        }

        let html_response =
            handle("html_response", &[Value::Int(202), str_value("<h1>x</h1>")]).unwrap();
        match html_response {
            Value::HttpResponse { status, headers, .. } => {
                assert_eq!(status, 202);
                assert_eq!(
                    headers.get("Content-Type"),
                    Some(&"text/html; charset=utf-8".to_string())
                );
            }
            _ => panic!("Expected HttpResponse from html_response"),
        }

        let mut headers_dict = DictMap::default();
        headers_dict.insert("X-App".into(), str_value("kujo"));

        let set_header_result = handle(
            "set_header",
            &[
                Value::HttpResponse {
                    status: 200,
                    body: "ok".to_string(),
                    headers: HashMap::new(),
                },
                str_value("X-Test"),
                str_value("true"),
            ],
        )
        .unwrap();
        assert!(
            matches!(set_header_result, Value::HttpResponse { headers, .. } if headers.get("X-Test") == Some(&"true".to_string()))
        );

        let set_headers_result = handle(
            "set_headers",
            &[
                Value::HttpResponse {
                    status: 200,
                    body: "ok".to_string(),
                    headers: HashMap::new(),
                },
                Value::Dict(Arc::new(headers_dict)),
            ],
        )
        .unwrap();
        assert!(
            matches!(set_headers_result, Value::HttpResponse { headers, .. } if headers.get("X-App") == Some(&"kujo".to_string()))
        );
    }

    #[test]
    fn test_redirect_and_server_helpers() {
        let mut extra_headers = DictMap::default();
        extra_headers.insert("Cache-Control".into(), str_value("no-cache"));

        let redirect = handle(
            "redirect_response",
            &[str_value("https://example.com"), Value::Dict(Arc::new(extra_headers))],
        )
        .unwrap();
        assert!(
            matches!(redirect, Value::HttpResponse { status, headers, .. } if status == 302 && headers.get("Location") == Some(&"https://example.com".to_string()) && headers.get("Cache-Control") == Some(&"no-cache".to_string()))
        );

        let server = handle("http_server", &[Value::Int(8080)]).unwrap();
        assert!(
            matches!(server, Value::HttpServer { host, port, .. } if host == "0.0.0.0" && port == 8080)
        );

        let listen_server =
            handle("http_listen", &[str_value("127.0.0.1"), Value::Int(9191)]).unwrap();
        assert!(
            matches!(listen_server, Value::HttpServer { host, port, .. } if host == "127.0.0.1" && port == 9191)
        );
    }

    #[test]
    fn test_http_argument_shape_contract_errors() {
        let get_error = handle("http_get", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(get_error, Value::Error(message) if message.contains("http_get requires a URL string"))
        );

        let post_error = handle("http_post", &[str_value("https://example.com")]).unwrap();
        assert!(
            matches!(post_error, Value::Error(message) if message.contains("http_post() expects 2 arguments"))
        );

        let get_extra_error =
            handle("http_get", &[str_value("https://example.com"), str_value("extra")]).unwrap();
        assert!(
            matches!(get_extra_error, Value::Error(message) if message.contains("http_get() expects 1 argument"))
        );

        let set_header_error =
            handle("set_header", &[Value::Int(1), str_value("k"), str_value("v")]).unwrap();
        assert!(
            matches!(set_header_error, Value::Error(message) if message.contains("requires an HTTP response as first argument"))
        );

        let request_missing_url =
            handle("http_request", &[Value::Dict(Arc::new(DictMap::default()))]).unwrap();
        assert!(matches!(
            request_missing_url,
            Value::Error(message)
                if message.contains("request dictionary requires a non-empty 'url' string")
        ));
    }

    #[test]
    fn test_http_request_supports_single_request_dictionary_form() {
        let Some((endpoint, _request_rx, server_handle)) =
            one_shot_json_server(200, "{\"ok\":true}")
        else {
            eprintln!(
                "skipping test_http_request_supports_single_request_dictionary_form: local TCP bind not permitted in this environment"
            );
            return;
        };

        let mut headers = DictMap::default();
        headers.insert("X-Test".into(), str_value("1"));

        let request = Value::FixedDict {
            keys: Arc::new(vec![
                Arc::<str>::from("url"),
                Arc::<str>::from("method"),
                Arc::<str>::from("headers"),
            ]),
            values: vec![str_value(&endpoint), str_value("GET"), Value::Dict(Arc::new(headers))],
        };

        let result =
            handle("http_request", &[request]).expect("http_request should return a result value");
        server_handle.join().expect("server thread should finish");

        assert!(matches!(
            result,
            Value::Result { is_ok: true, value }
                if matches!(value.as_ref(), Value::Dict(dict)
                    if matches!(dict.get("status"), Some(Value::Int(200)))
                        && matches!(dict.get("body"), Some(Value::Str(body)) if body.contains("\"ok\":true")))
        ));
    }

    #[test]
    fn test_ai_chat_success_path_returns_normalized_result_and_request_payload() {
        let response_body = r#"{"choices":[{"message":{"content":"hello from model"}}]}"#;
        let Some((endpoint, request_rx, server_handle)) = one_shot_json_server(200, response_body)
        else {
            eprintln!(
                "skipping test_ai_chat_success_path_returns_normalized_result_and_request_payload: local TCP bind not permitted in this environment"
            );
            return;
        };

        let result =
            handle("ai_chat", &[str_value("Hello model"), ai_options(&endpoint, "gpt-mock")])
                .expect("ai_chat should return a value");

        let request_body = request_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("server should capture request body");
        server_handle.join().expect("server thread should finish");

        let parsed_request =
            builtins::parse_json(&request_body).expect("ai_chat request body should be JSON");
        match parsed_request {
            Value::Dict(payload) => {
                assert!(
                    matches!(payload.get("model"), Some(Value::Str(model)) if model.as_ref() == "gpt-mock")
                );
                assert!(
                    matches!(payload.get("messages"), Some(Value::Array(messages)) if messages.len() == 1)
                );
            }
            other => panic!("expected request body dict, got {:?}", other),
        }

        match result {
            Value::Result { is_ok: true, value } => match *value {
                Value::Dict(result_dict) => {
                    assert!(matches!(result_dict.get("status"), Some(Value::Int(200))));
                    assert!(
                        matches!(result_dict.get("message"), Some(Value::Str(message)) if message.as_ref() == "hello from model")
                    );
                }
                other => panic!("expected ai_chat success dict, got {:?}", other),
            },
            other => panic!("expected ai_chat ok result, got {:?}", other),
        }
    }

    #[test]
    fn test_ai_embedding_extracts_numeric_vector_regression() {
        let response_body = r#"{"data":[{"embedding":[0.5,1,2.25]}]}"#;
        let Some((endpoint, _request_rx, server_handle)) = one_shot_json_server(200, response_body)
        else {
            eprintln!(
                "skipping test_ai_embedding_extracts_numeric_vector_regression: local TCP bind not permitted in this environment"
            );
            return;
        };

        let result =
            handle("ai_embedding", &[str_value("seed text"), ai_options(&endpoint, "embed-mock")])
                .expect("ai_embedding should return a value");
        server_handle.join().expect("server thread should finish");

        match result {
            Value::Result { is_ok: true, value } => match *value {
                Value::Dict(result_dict) => match result_dict.get("vector") {
                    Some(Value::Array(vector)) => {
                        assert_eq!(vector.len(), 3);
                        assert!(matches!(vector[0], Value::Float(v) if (v - 0.5).abs() < 1e-9));
                        assert!(matches!(vector[1], Value::Float(v) if (v - 1.0).abs() < 1e-9));
                        assert!(matches!(vector[2], Value::Float(v) if (v - 2.25).abs() < 1e-9));
                    }
                    other => panic!("expected vector array, got {:?}", other),
                },
                other => panic!("expected ai_embedding success dict, got {:?}", other),
            },
            other => panic!("expected ai_embedding ok result, got {:?}", other),
        }
    }

    #[test]
    fn test_ai_helpers_surface_contract_failures_and_edge_validation() {
        let missing_options = handle("ai_chat", &[str_value("hello"), Value::Int(1)]).unwrap();
        assert!(
            matches!(missing_options, Value::Error(message) if message.contains("requires an options dictionary"))
        );

        let mut bad_options = DictMap::default();
        bad_options.insert("endpoint".into(), str_value("http://127.0.0.1:1"));
        bad_options.insert("model".into(), str_value("gpt-mock"));
        bad_options.insert("timeout".into(), Value::Int(0));
        let bad_timeout =
            handle("ai_chat", &[str_value("hello"), Value::Dict(Arc::new(bad_options))]).unwrap();
        assert!(
            matches!(bad_timeout, Value::Error(message) if message.contains("options.timeout to be a positive number"))
        );

        let mut tool_loop_options = DictMap::default();
        tool_loop_options.insert("endpoint".into(), str_value("http://127.0.0.1:1"));
        tool_loop_options.insert("model".into(), str_value("gpt-mock"));
        tool_loop_options.insert("max_steps".into(), Value::Int(0));
        let bad_steps =
            handle("ai_tool_loop", &[str_value("hello"), Value::Dict(Arc::new(tool_loop_options))])
                .unwrap();
        assert!(
            matches!(bad_steps, Value::Error(message) if message.contains("options.max_steps to be an integer between 1"))
        );
    }

    #[test]
    fn test_ai_chat_non_json_response_returns_deterministic_failure_result() {
        let Some((endpoint, _request_rx, server_handle)) = one_shot_json_server(200, "not-json")
        else {
            eprintln!(
                "skipping test_ai_chat_non_json_response_returns_deterministic_failure_result: local TCP bind not permitted in this environment"
            );
            return;
        };
        let result = handle("ai_chat", &[str_value("Hello"), ai_options(&endpoint, "gpt-mock")])
            .expect("ai_chat should return a result");
        server_handle.join().expect("server thread should finish");

        assert!(
            matches!(result, Value::Result { is_ok: false, value } if matches!(value.as_ref(), Value::Str(message) if message.contains("response was not valid JSON")))
        );
    }
}
