// File: src/interpreter/native_functions/http.rs
//
// HTTP client native functions

use crate::interpreter::{DictMap, HttpResponseStreamParts, Interpreter, Value};
use crate::runtime_limits;
use crate::{builtins, network_policy};
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_AI_TOOL_LOOP_STEPS: i64 = 16;
const AI_CASSETTE_VERSION: i64 = 1;
const HTTP_RESPONSE_STREAM_CHUNK_BYTES: usize = 16 * 1024;
const HTTP_RESPONSE_STREAM_EVENT_CAPACITY: usize = 16;

fn http_stream_observed_at_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn http_stream_event(kind: &str) -> DictMap {
    let mut event = DictMap::default();
    event.insert("type".into(), Value::Str(Arc::new(kind.to_string())));
    event.insert("observed_at_ms".into(), Value::Int(http_stream_observed_at_ms()));
    event
}

struct IncrementalHttpResponseBody {
    response: reqwest::blocking::Response,
    events: SyncSender<Value>,
    cancelled: Arc<AtomicBool>,
    max_response_bytes: usize,
    total_bytes: usize,
    chunk_index: i64,
    terminal: bool,
}

enum HttpRequestResult {
    Buffered(u16, reqwest::header::HeaderMap, Vec<u8>),
    Incremental(reqwest::blocking::Response),
}

impl IncrementalHttpResponseBody {
    fn emit(&self, event: DictMap) {
        let _ = self.events.send(Value::Dict(Arc::new(event)));
    }

    fn emit_terminal(&mut self, kind: &str, reason: Option<&str>) {
        if self.terminal {
            return;
        }
        self.terminal = true;
        let mut event = http_stream_event(kind);
        event.insert("total_bytes".into(), Value::Int(self.total_bytes as i64));
        event.insert("chunks".into(), Value::Int(self.chunk_index));
        if let Some(reason) = reason {
            event.insert("reason".into(), Value::Str(Arc::new(reason.to_string())));
        }
        self.emit(event);
    }
}

impl Read for IncrementalHttpResponseBody {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.cancelled.load(Ordering::Acquire) {
            self.emit_terminal("cancelled", Some("callback_cancelled"));
            return Ok(0);
        }

        if self.total_bytes >= self.max_response_bytes {
            let mut probe = [0_u8; 1];
            return match self.response.read(&mut probe) {
                Ok(0) => {
                    self.emit_terminal("complete", None);
                    Ok(0)
                }
                Ok(_) => {
                    self.emit_terminal("error", Some("max_response_bytes_exceeded"));
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "incremental HTTP response exceeded max_response_bytes",
                    ))
                }
                Err(error) => {
                    self.emit_terminal("error", Some("upstream_read_error"));
                    Err(error)
                }
            };
        }

        let allowed = output
            .len()
            .min(HTTP_RESPONSE_STREAM_CHUNK_BYTES)
            .min(self.max_response_bytes - self.total_bytes);
        if allowed == 0 {
            return Ok(0);
        }

        match self.response.read(&mut output[..allowed]) {
            Ok(0) => {
                self.emit_terminal("complete", None);
                Ok(0)
            }
            Ok(read) => {
                self.total_bytes += read;
                self.chunk_index += 1;
                let mut event = http_stream_event("chunk");
                event.insert("chunk_index".into(), Value::Int(self.chunk_index));
                event.insert("total_bytes".into(), Value::Int(self.total_bytes as i64));
                event.insert("bytes".into(), Value::Bytes(output[..read].to_vec()));
                self.emit(event);
                Ok(read)
            }
            Err(error) => {
                self.emit_terminal("error", Some("upstream_read_error"));
                Err(error)
            }
        }
    }
}

impl Drop for IncrementalHttpResponseBody {
    fn drop(&mut self) {
        if !self.terminal {
            let reason = if self.cancelled.load(Ordering::Acquire) {
                "callback_cancelled"
            } else {
                "downstream_disconnected"
            };
            self.emit_terminal("cancelled", Some(reason));
        }
    }
}

fn incremental_http_response_value(
    response: reqwest::blocking::Response,
    max_response_bytes: usize,
    callback: Option<Value>,
) -> Result<Value, String> {
    let status = response.status().as_u16();
    let content_length = response.content_length().and_then(|value| usize::try_from(value).ok());
    if content_length.is_some_and(|value| value > max_response_bytes) {
        return Err(format!(
            "HTTP request failed: response body exceeds maximum network body size ({} bytes > {} bytes)",
            content_length.unwrap_or_default(),
            max_response_bytes
        ));
    }

    let mut headers = HashMap::new();
    let mut event_headers = DictMap::default();
    for (name, value) in response.headers() {
        if let Ok(value) = value.to_str() {
            headers.insert(name.as_str().to_string(), value.to_string());
            event_headers
                .insert(name.as_str().to_string().into(), Value::Str(Arc::new(value.to_string())));
        }
    }

    let (events_tx, events_rx) = sync_channel(HTTP_RESPONSE_STREAM_EVENT_CAPACITY);
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut headers_event = http_stream_event("headers");
    headers_event.insert("status".into(), Value::Int(i64::from(status)));
    headers_event.insert("headers".into(), Value::Dict(Arc::new(event_headers)));
    if let Some(length) = content_length {
        headers_event.insert("content_length".into(), Value::Int(length as i64));
    } else {
        headers_event.insert("content_length".into(), Value::Null);
    }
    let _ = events_tx.send(Value::Dict(Arc::new(headers_event)));

    let body = IncrementalHttpResponseBody {
        response,
        events: events_tx,
        cancelled: Arc::clone(&cancelled),
        max_response_bytes,
        total_bytes: 0,
        chunk_index: 0,
        terminal: false,
    };
    let stream = HttpResponseStreamParts {
        body: Box::new(body),
        events: events_rx,
        cancelled,
        content_length,
    };

    Ok(Value::HttpStreamingResponse {
        status,
        headers,
        stream: Arc::new(Mutex::new(Some(stream))),
        callback: callback.map(Box::new),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AiCassetteMode {
    Off,
    Record { dir: PathBuf },
    ReplayStrict { dir: PathBuf },
    ReplayFallthrough { dir: PathBuf, record_dir: PathBuf },
}

struct AiRequestConfig {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    timeout_seconds: f64,
    headers: Vec<(String, String)>,
    cassette: AiCassetteMode,
    structured_errors: bool,
    provider: String,
}

struct AiHttpResponse {
    status: i64,
    headers: DictMap,
    text: String,
    json: Option<Value>,
    decode_error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredAiCassette {
    _cassette_version: i64,
    request_meta: StoredAiCassetteRequest,
    response: StoredAiCassetteResponse,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredAiCassetteRequest {
    hash: String,
    surface: String,
    endpoint: String,
    model: String,
    normalized: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredAiCassetteResponse {
    status: i64,
    headers: HashMap<String, String>,
    body: String,
}

fn ai_err_result(message: impl Into<String>) -> Value {
    Value::Result { is_ok: false, value: Box::new(Value::Str(Arc::new(message.into()))) }
}

fn ai_err_structured(error: DictMap) -> Value {
    Value::Result { is_ok: false, value: Box::new(Value::Dict(Arc::new(error))) }
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

fn ai_endpoint_denied_config_error(message: String, structured_errors: bool) -> Value {
    if !structured_errors {
        return Value::Error(message);
    }

    let mut error = DictMap::default();
    error.insert("kind".into(), Value::Str(Arc::new("endpoint_denied".to_string())));
    error.insert("message".into(), Value::Str(Arc::new(message)));
    error.insert("http_status".into(), Value::Null);
    error.insert("retry_after_ms".into(), Value::Null);
    error.insert("provider_code".into(), Value::Null);
    error.insert("body_excerpt".into(), Value::Null);
    ai_err_structured(error)
}

fn path_from_non_empty_string(value: &Value, field: &str, surface: &str) -> Result<PathBuf, Value> {
    match value {
        Value::Str(path) if !path.trim().is_empty() => Ok(PathBuf::from(path.as_ref())),
        Value::Str(_) => Err(Value::Error(format!(
            "{}() requires options.cassette.{} to be a non-empty string",
            surface, field
        ))),
        _ => Err(Value::Error(format!(
            "{}() requires options.cassette.{} to be a string",
            surface, field
        ))),
    }
}

fn parse_ai_cassette_mode(options: &DictMap, surface: &str) -> Result<AiCassetteMode, Value> {
    if let Some(raw_cassette) = options.get("cassette") {
        let cassette = dict_like_from_value(raw_cassette).ok_or_else(|| {
            Value::Error(format!(
                "{}() requires options.cassette to be a dictionary when provided",
                surface
            ))
        })?;
        let mode = match cassette.get("mode") {
            Some(Value::Str(mode)) => mode.trim().to_ascii_lowercase(),
            Some(_) => {
                return Err(Value::Error(format!(
                    "{}() requires options.cassette.mode to be a string",
                    surface
                )));
            }
            None => "replay".to_string(),
        };
        let dir = match cassette.get("dir") {
            Some(value) => Some(path_from_non_empty_string(value, "dir", surface)?),
            None => None,
        };
        let record_dir = match cassette.get("record_dir") {
            Some(value) => Some(path_from_non_empty_string(value, "record_dir", surface)?),
            None => None,
        };

        return match mode.as_str() {
            "off" | "none" | "disabled" => Ok(AiCassetteMode::Off),
            "record" => match dir.or_else(ai_record_dir_from_env) {
                Some(dir) => Ok(AiCassetteMode::Record { dir }),
                None => Err(Value::Error(format!(
                    "{}() requires options.cassette.dir or KUJO_AI_RECORD for record mode",
                    surface
                ))),
            },
            "replay" | "strict" => match dir.or_else(ai_replay_dir_from_env) {
                Some(dir) => Ok(AiCassetteMode::ReplayStrict { dir }),
                None => Err(Value::Error(format!(
                    "{}() requires options.cassette.dir or KUJO_AI_REPLAY for replay mode",
                    surface
                ))),
            },
            "fallthrough" => {
                let Some(dir) = dir.or_else(ai_replay_dir_from_env).or_else(ai_record_dir_from_env)
                else {
                    return Err(Value::Error(format!(
                        "{}() requires options.cassette.dir, KUJO_AI_REPLAY, or KUJO_AI_RECORD for fallthrough mode",
                        surface
                    )));
                };
                let record_dir = record_dir
                    .or_else(ai_record_dir_from_env)
                    .unwrap_or_else(|| dir.clone());
                Ok(AiCassetteMode::ReplayFallthrough { dir, record_dir })
            }
            _ => Err(Value::Error(format!(
                "{}() requires options.cassette.mode to be one of off, record, replay, strict, or fallthrough",
                surface
            ))),
        };
    }

    Ok(ai_cassette_mode_from_env())
}

fn non_empty_env_path(name: &str) -> Option<PathBuf> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn ai_record_dir_from_env() -> Option<PathBuf> {
    non_empty_env_path("KUJO_AI_RECORD")
}

fn ai_replay_dir_from_env() -> Option<PathBuf> {
    non_empty_env_path("KUJO_AI_REPLAY")
}

fn ai_replay_mode_from_env() -> String {
    env::var("KUJO_AI_REPLAY_MODE")
        .unwrap_or_else(|_| "strict".to_string())
        .trim()
        .to_ascii_lowercase()
}

fn ai_cassette_mode_from_env() -> AiCassetteMode {
    if let Some(replay_dir) = ai_replay_dir_from_env() {
        return match ai_replay_mode_from_env().as_str() {
            "fallthrough" => {
                let record_dir = ai_record_dir_from_env().unwrap_or_else(|| replay_dir.clone());
                AiCassetteMode::ReplayFallthrough { dir: replay_dir, record_dir }
            }
            _ => AiCassetteMode::ReplayStrict { dir: replay_dir },
        };
    }

    if let Some(record_dir) = ai_record_dir_from_env() {
        return AiCassetteMode::Record { dir: record_dir };
    }

    AiCassetteMode::Off
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
    parse_ai_request_config_inner(options, surface, true, true)
}

fn parse_ai_request_hash_config(
    options: &DictMap,
    surface: &str,
) -> Result<AiRequestConfig, Value> {
    parse_ai_request_config_inner(options, surface, false, false)
}

fn parse_ai_request_config_inner(
    options: &DictMap,
    surface: &str,
    parse_cassette: bool,
    enforce_endpoint_allowlist: bool,
) -> Result<AiRequestConfig, Value> {
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
        Some(Value::Secret(key)) if !key.is_empty() => Some(key.as_ref().clone()),
        Some(Value::Str(_)) | None => None,
        Some(Value::Secret(_)) => None,
        Some(_) => {
            return Err(Value::Error(format!(
                "{}() requires options.api_key to be a string or secret when provided",
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
    let structured_errors = match options.get("structured_errors") {
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            return Err(Value::Error(format!(
                "{}() requires options.structured_errors to be a boolean when provided",
                surface
            )));
        }
        None => false,
    };
    if enforce_endpoint_allowlist {
        network_policy::ai_endpoint_allowed(&endpoint, surface)
            .map_err(|message| ai_endpoint_denied_config_error(message, structured_errors))?;
    }
    let provider = match options.get("provider") {
        Some(Value::Str(provider)) => provider.as_ref().clone(),
        Some(_) => {
            return Err(Value::Error(format!(
                "{}() requires options.provider to be a string when provided",
                surface
            )));
        }
        None => String::new(),
    };
    let cassette = if parse_cassette {
        parse_ai_cassette_mode(options, surface)?
    } else {
        AiCassetteMode::Off
    };

    Ok(AiRequestConfig {
        endpoint,
        model,
        api_key,
        timeout_seconds,
        headers,
        cassette,
        structured_errors,
        provider,
    })
}

fn ai_message_value(role: &str, content: Value) -> Value {
    let mut message = DictMap::default();
    message.insert("role".into(), Value::Str(Arc::new(role.to_string())));
    message.insert("content".into(), content);
    Value::Dict(Arc::new(message))
}

fn ai_text_message_value(role: &str, content: impl Into<String>) -> Value {
    ai_message_value(role, Value::Str(Arc::new(content.into())))
}

fn ai_text_block(content: impl Into<String>) -> Value {
    let mut block = DictMap::default();
    block.insert("type".into(), Value::Str(Arc::new("text".to_string())));
    block.insert("text".into(), Value::Str(Arc::new(content.into())));
    Value::Dict(Arc::new(block))
}

fn ai_image_url_block(url: impl Into<String>, detail: Option<String>) -> Value {
    let mut image_url = DictMap::default();
    image_url.insert("url".into(), Value::Str(Arc::new(url.into())));
    if let Some(detail) = detail {
        image_url.insert("detail".into(), Value::Str(Arc::new(detail)));
    }

    let mut block = DictMap::default();
    block.insert("type".into(), Value::Str(Arc::new("image_url".to_string())));
    block.insert("image_url".into(), Value::Dict(Arc::new(image_url)));
    Value::Dict(Arc::new(block))
}

fn validate_ai_content_block(block: &Value, surface: &str, path: &str) -> Result<Value, Value> {
    let dict = match block {
        Value::Dict(dict) => dict,
        _ => {
            return Err(Value::Error(format!(
                "{}() requires {} to be a content block dictionary",
                surface, path
            )));
        }
    };
    let block_type = match dict.get("type") {
        Some(Value::Str(block_type)) if !block_type.is_empty() => block_type.as_ref().clone(),
        _ => {
            return Err(Value::Error(format!(
                "{}() requires {}.type to be a non-empty string",
                surface, path
            )));
        }
    };

    match block_type.as_str() {
        "text" => match dict.get("text") {
            Some(Value::Str(_)) => Ok(block.clone()),
            _ => Err(Value::Error(format!(
                "{}() requires {}.text to be a string for text content blocks",
                surface, path
            ))),
        },
        "image_url" => {
            let image_url = match dict.get("image_url") {
                Some(Value::Dict(image_url)) => image_url,
                _ => {
                    return Err(Value::Error(format!(
                        "{}() requires {}.image_url to be a dictionary",
                        surface, path
                    )));
                }
            };
            match image_url.get("url") {
                Some(Value::Str(url)) if !url.is_empty() => {}
                _ => {
                    return Err(Value::Error(format!(
                        "{}() requires {}.image_url.url to be a non-empty string",
                        surface, path
                    )));
                }
            }
            if !matches!(image_url.get("detail"), None | Some(Value::Str(_))) {
                return Err(Value::Error(format!(
                    "{}() requires {}.image_url.detail to be a string when provided",
                    surface, path
                )));
            }
            Ok(block.clone())
        }
        _ => Err(Value::Error(format!(
            "{}() does not support {} content block type '{}'",
            surface, path, block_type
        ))),
    }
}

fn normalize_ai_message_content(
    content: &Value,
    surface: &str,
    path: &str,
) -> Result<Value, Value> {
    match content {
        Value::Str(_) => Ok(content.clone()),
        Value::Array(blocks) => {
            let mut normalized = Vec::with_capacity(blocks.len());
            for (index, block) in blocks.iter().enumerate() {
                normalized.push(validate_ai_content_block(
                    block,
                    surface,
                    &format!("{}[{}]", path, index),
                )?);
            }
            Ok(Value::Array(Arc::new(normalized)))
        }
        _ => Err(Value::Error(format!(
            "{}() requires {} to be a string or array of content blocks",
            surface, path
        ))),
    }
}

fn parse_ai_messages(input: &Value, surface: &str) -> Result<Vec<Value>, Value> {
    match input {
        Value::Str(prompt) => Ok(vec![ai_text_message_value("user", prompt.as_ref().clone())]),
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
                    Some(content) => normalize_ai_message_content(
                        content,
                        surface,
                        &format!("messages[{}].content", index),
                    )?,
                    None => {
                        return Err(Value::Error(format!(
                            "{}() requires messages[{}].content to be a string or content block array",
                            surface, index
                        )));
                    }
                };

                normalized.push(ai_message_value(&role, content));
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

fn redact_ai_error_text(text: &str, config: &AiRequestConfig) -> String {
    let mut redacted = text.to_string();
    if let Some(api_key) = &config.api_key {
        if !api_key.is_empty() {
            redacted = redacted.replace(api_key, "[redacted]");
            redacted = redacted.replace(&format!("Bearer {}", api_key), "Bearer [redacted]");
        }
    }
    for (name, value) in &config.headers {
        if !value.is_empty() && is_ai_hash_excluded_header(name) {
            redacted = redacted.replace(value, "[redacted]");
        }
    }
    redacted
}

fn ai_error_string(config: &AiRequestConfig, message: String) -> Value {
    ai_err_result(redact_ai_error_text(&message, config))
}

fn ai_structured_error(
    kind: &str,
    message: String,
    http_status: Option<i64>,
    retry_after_ms: Option<i64>,
    provider_code: Option<String>,
    body_excerpt: Option<String>,
    config: &AiRequestConfig,
) -> Value {
    let mut error = DictMap::default();
    error.insert("kind".into(), Value::Str(Arc::new(kind.to_string())));
    error.insert("message".into(), Value::Str(Arc::new(redact_ai_error_text(&message, config))));
    error.insert("http_status".into(), http_status.map(Value::Int).unwrap_or(Value::Null));
    error.insert("retry_after_ms".into(), retry_after_ms.map(Value::Int).unwrap_or(Value::Null));
    error.insert(
        "provider_code".into(),
        provider_code
            .map(|code| Value::Str(Arc::new(redact_ai_error_text(&code, config))))
            .unwrap_or(Value::Null),
    );
    error.insert(
        "body_excerpt".into(),
        body_excerpt
            .map(|excerpt| Value::Str(Arc::new(redact_ai_error_text(&excerpt, config))))
            .unwrap_or(Value::Null),
    );
    ai_err_structured(error)
}

fn retry_after_header(headers: &DictMap) -> Option<String> {
    headers.iter().find_map(|(name, value)| {
        if name.eq_ignore_ascii_case("retry-after") {
            if let Value::Str(text) = value {
                return Some(text.as_ref().clone());
            }
        }
        None
    })
}

fn parse_retry_after(headers: &DictMap) -> Option<i64> {
    let raw = retry_after_header(headers)?;
    let trimmed = raw.trim();
    if let Ok(seconds) = trimmed.parse::<i64>() {
        return Some(seconds.max(0) * 1000);
    }
    let parsed = DateTime::parse_from_rfc2822(trimmed).ok()?;
    Some((parsed.with_timezone(&Utc) - Utc::now()).num_milliseconds().max(0))
}

fn provider_code_from_body(text: &str) -> Option<String> {
    let json = builtins::parse_json(text).ok()?;
    let root = match json {
        Value::Dict(root) => root,
        _ => return None,
    };
    if let Some(Value::Dict(error)) = root.get("error") {
        if let Some(Value::Str(code)) = error.get("code") {
            return Some(code.as_ref().clone());
        }
        if let Some(Value::Str(code)) = error.get("type") {
            return Some(code.as_ref().clone());
        }
    }
    if let Some(Value::Str(code)) = root.get("code") {
        return Some(code.as_ref().clone());
    }
    None
}

fn classify_transport_error(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("timeout") || lower.contains("timed out") {
        "timeout"
    } else {
        "network"
    }
}

fn ai_transport_error(_surface: &str, config: &AiRequestConfig, error: String) -> Value {
    if config.structured_errors {
        let kind = classify_transport_error(&error);
        ai_structured_error(kind, error, None, None, None, None, config)
    } else {
        ai_error_string(config, error)
    }
}

fn ai_http_status_error(
    surface: &str,
    config: &AiRequestConfig,
    status: i64,
    headers: &DictMap,
    text: &str,
) -> Value {
    let excerpt = truncate_for_error(text, 240);
    let message = format!("{} failed with HTTP status {}: {}", surface, status, excerpt);
    if config.structured_errors {
        let kind = if status == 429 { "rate_limited" } else { "http_error" };
        ai_structured_error(
            kind,
            message,
            Some(status),
            parse_retry_after(headers),
            provider_code_from_body(text),
            Some(excerpt),
            config,
        )
    } else {
        ai_error_string(config, message)
    }
}

fn ai_decode_error(surface: &str, config: &AiRequestConfig, error: String, text: &str) -> Value {
    let message = format!("{} failed: response was not valid JSON ({})", surface, error);
    if config.structured_errors {
        ai_structured_error(
            "decode_error",
            message,
            None,
            None,
            None,
            Some(truncate_for_error(text, 240)),
            config,
        )
    } else {
        ai_error_string(config, message)
    }
}

fn ai_invalid_response_error(_surface: &str, config: &AiRequestConfig, message: String) -> Value {
    if config.structured_errors {
        ai_structured_error("invalid_response", message, None, None, None, None, config)
    } else {
        ai_error_string(config, message)
    }
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
    let config = parse_ai_request_hash_config(options, "ai_request_hash")?;
    let messages = parse_ai_messages(prompt_or_messages, "ai_request_hash")?;

    let mut payload = DictMap::default();
    payload.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
    payload.insert("messages".into(), Value::Array(Arc::new(messages)));
    merge_ai_extra_body(&mut payload, options, &["model", "messages"], "ai_request_hash")?;

    Ok(build_ai_request_key_value(&config, &Value::Dict(Arc::new(payload))))
}

fn build_ai_request_key_value(config: &AiRequestConfig, payload: &Value) -> Value {
    let mut normalized = DictMap::default();
    normalized.insert("_hash_version".into(), Value::Int(1));
    normalized.insert("endpoint".into(), Value::Str(Arc::new(config.endpoint.trim().to_string())));
    normalized.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
    normalized.insert("headers".into(), canonical_ai_hash_headers(&config.headers));
    normalized.insert("body".into(), payload.clone());
    Value::Dict(Arc::new(normalized))
}

fn ai_request_hash_value(prompt_or_messages: &Value, options: &DictMap) -> Result<String, Value> {
    let normalized = build_ai_request_hash_value(prompt_or_messages, options)?;
    let canonical_json = builtins::to_json(&normalized).map_err(|error| {
        Value::Error(format!("ai_request_hash() failed to serialize normalized request: {}", error))
    })?;
    Ok(sha256_hex(canonical_json.as_bytes()))
}

fn ai_request_key(config: &AiRequestConfig, payload: &Value) -> Result<(String, Value), String> {
    let normalized = build_ai_request_key_value(config, payload);
    let canonical_json = builtins::to_json(&normalized)
        .map_err(|error| format!("AI cassette request serialization error: {}", error))?;
    Ok((sha256_hex(canonical_json.as_bytes()), normalized))
}

fn cassette_path(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{}.json", key))
}

fn redact_ai_headers(headers: &DictMap) -> HashMap<String, String> {
    let mut redacted = HashMap::new();
    for (name, value) in headers.iter() {
        let Value::Str(text) = value else {
            continue;
        };
        let output = if is_ai_hash_excluded_header(name) {
            "[redacted]".to_string()
        } else {
            text.as_ref().clone()
        };
        redacted.insert(name.to_string(), output);
    }
    redacted
}

fn cassette_headers_to_dict(headers: &HashMap<String, String>) -> DictMap {
    let mut dict = DictMap::default();
    for (name, value) in headers {
        dict.insert(name.as_str().into(), Value::Str(Arc::new(value.clone())));
    }
    dict
}

fn response_is_event_stream(headers: &DictMap, body: &str) -> bool {
    let content_type = headers
        .get("content-type")
        .or_else(|| headers.get("Content-Type"))
        .and_then(|value| match value {
            Value::Str(text) => Some(text.as_ref().to_ascii_lowercase()),
            _ => None,
        })
        .unwrap_or_default();
    content_type.contains("text/event-stream")
        || body.starts_with("data:")
        || body.contains("\ndata:")
}

fn parse_ai_event_stream_body(body: &str) -> Result<Value, String> {
    let mut choices = Vec::new();
    let mut usage = None;
    let mut data_lines = Vec::new();

    fn process_event(
        data_lines: &mut Vec<String>,
        choices: &mut Vec<Value>,
        usage: &mut Option<Value>,
    ) -> Result<bool, String> {
        if data_lines.is_empty() {
            return Ok(true);
        }
        let data = data_lines.join("\n");
        data_lines.clear();
        if data.trim() == "[DONE]" {
            return Ok(false);
        }
        let event = builtins::parse_json(data.trim())
            .map_err(|error| format!("event-stream JSON decode error: {}", error))?;
        let Value::Dict(event_dict) = event else {
            return Ok(true);
        };
        if let Some(Value::Array(event_choices)) = event_dict.get("choices") {
            choices.extend(event_choices.iter().cloned());
        }
        if let Some(event_usage) = event_dict.get("usage") {
            usage.replace(event_usage.clone());
        }
        Ok(true)
    }

    for line in body.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            if !process_event(&mut data_lines, &mut choices, &mut usage)? {
                break;
            }
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            data_lines.push(data.trim_start().to_string());
        }
    }
    let _ = process_event(&mut data_lines, &mut choices, &mut usage)?;

    let mut root = DictMap::default();
    root.insert("choices".into(), Value::Array(Arc::new(choices)));
    if let Some(usage) = usage {
        root.insert("usage".into(), usage);
    }
    Ok(Value::Dict(Arc::new(root)))
}

fn parse_ai_response_body(body: &str, headers: &DictMap) -> (Option<Value>, Option<String>) {
    match builtins::parse_json(body) {
        Ok(json) => (Some(json), None),
        Err(_) if response_is_event_stream(headers, body) => {
            match parse_ai_event_stream_body(body) {
                Ok(json) => (Some(json), None),
                Err(stream_error) => (None, Some(stream_error)),
            }
        }
        Err(error) => (None, Some(error)),
    }
}

fn replay_ai_cassette(
    surface: &str,
    dir: &Path,
    key: &str,
) -> Result<Option<AiHttpResponse>, String> {
    let path = cassette_path(dir, key);
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&path).map_err(|error| {
        format!(
            "{} failed: kind:\"replay_miss\" cassette '{}' could not be read: {}",
            surface,
            path.display(),
            error
        )
    })?;
    let cassette: StoredAiCassette = serde_json::from_str(&contents).map_err(|error| {
        format!("{} failed: cassette '{}' was not valid JSON ({})", surface, path.display(), error)
    })?;
    if cassette._cassette_version != AI_CASSETTE_VERSION {
        return Err(format!(
            "{} failed: cassette '{}' has unsupported version {}",
            surface,
            path.display(),
            cassette._cassette_version
        ));
    }
    if cassette.request_meta.hash != key {
        return Err(format!(
            "{} failed: cassette '{}' hash mismatch (expected {}, found {})",
            surface,
            path.display(),
            key,
            cassette.request_meta.hash
        ));
    }

    let headers = cassette_headers_to_dict(&cassette.response.headers);
    let (json, decode_error) = parse_ai_response_body(&cassette.response.body, &headers);
    Ok(Some(AiHttpResponse {
        status: cassette.response.status,
        headers,
        text: cassette.response.body,
        json,
        decode_error,
    }))
}

fn store_ai_cassette(
    surface: &str,
    dir: &Path,
    key: &str,
    normalized_request: &Value,
    config: &AiRequestConfig,
    status: i64,
    headers: &DictMap,
    body: &str,
) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|error| {
        format!(
            "{} failed: could not create AI cassette directory '{}': {}",
            surface,
            dir.display(),
            error
        )
    })?;
    let normalized_json = builtins::to_json(normalized_request)
        .and_then(|json| {
            serde_json::from_str::<serde_json::Value>(&json).map_err(|e| e.to_string())
        })
        .map_err(|error| {
            format!("{} failed: cassette request serialization error: {}", surface, error)
        })?;
    let cassette = StoredAiCassette {
        _cassette_version: AI_CASSETTE_VERSION,
        request_meta: StoredAiCassetteRequest {
            hash: key.to_string(),
            surface: surface.to_string(),
            endpoint: config.endpoint.trim().to_string(),
            model: config.model.clone(),
            normalized: normalized_json,
        },
        response: StoredAiCassetteResponse {
            status,
            headers: redact_ai_headers(headers),
            body: redact_ai_error_text(body, config),
        },
    };
    let serialized = serde_json::to_string_pretty(&cassette)
        .map_err(|error| format!("{} failed: cassette serialization error: {}", surface, error))?;
    let path = cassette_path(dir, key);
    let temp_path = dir.join(format!(".{}.{}.tmp", key, std::process::id()));
    fs::write(&temp_path, serialized).map_err(|error| {
        format!(
            "{} failed: could not write AI cassette '{}': {}",
            surface,
            temp_path.display(),
            error
        )
    })?;
    fs::rename(&temp_path, &path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        format!(
            "{} failed: could not finalize AI cassette '{}': {}",
            surface,
            path.display(),
            error
        )
    })
}

fn run_ai_request(
    surface: &str,
    config: &AiRequestConfig,
    payload: Value,
) -> Result<AiHttpResponse, String> {
    let (cassette_key, normalized_request) = ai_request_key(config, &payload)?;
    match &config.cassette {
        AiCassetteMode::ReplayStrict { dir } => {
            return match replay_ai_cassette(surface, dir, &cassette_key)? {
                Some(response) => Ok(response),
                None => Err(format!(
                    "{} failed: kind:\"replay_miss\" cassette '{}' was not found; strict replay does not use the network",
                    surface,
                    cassette_path(dir, &cassette_key).display()
                )),
            };
        }
        AiCassetteMode::ReplayFallthrough { dir, .. } => {
            if let Some(response) = replay_ai_cassette(surface, dir, &cassette_key)? {
                return Ok(response);
            }
        }
        AiCassetteMode::Off | AiCassetteMode::Record { .. } => {}
    }

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
        if surface_for_task == "ai_stream_chat" {
            request = request.header("Accept", "text/event-stream, application/json");
        } else {
            request = request.header("Accept", "application/json");
        }

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
    let mut headers_dict = DictMap::default();
    for (name, value) in response_headers.iter() {
        if let Ok(value_str) = value.to_str() {
            headers_dict.insert(
                name.as_str().to_string().into(),
                Value::Str(Arc::new(value_str.to_string())),
            );
        }
    }
    let body_text = String::from_utf8_lossy(&body_bytes).to_string();
    let (parsed_body, decode_error) = parse_ai_response_body(&body_text, &headers_dict);

    match &config.cassette {
        AiCassetteMode::Record { dir } => {
            store_ai_cassette(
                surface,
                dir,
                &cassette_key,
                &normalized_request,
                config,
                status as i64,
                &headers_dict,
                &body_text,
            )?;
        }
        AiCassetteMode::ReplayFallthrough { record_dir, .. } => {
            store_ai_cassette(
                surface,
                record_dir,
                &cassette_key,
                &normalized_request,
                config,
                status as i64,
                &headers_dict,
                &body_text,
            )?;
        }
        AiCassetteMode::Off | AiCassetteMode::ReplayStrict { .. } => {}
    }

    Ok(AiHttpResponse {
        status: status as i64,
        headers: headers_dict,
        text: body_text,
        json: parsed_body,
        decode_error,
    })
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
                    };
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

fn extract_chat_chunk_text(choice_dict: &DictMap) -> Option<String> {
    if let Some(Value::Dict(delta)) = choice_dict.get("delta") {
        if let Some(Value::Str(content)) = delta.get("content") {
            return Some(content.as_ref().clone());
        }
    } else if let Some(Value::Dict(message)) = choice_dict.get("message") {
        if let Some(Value::Str(content)) = message.get("content") {
            return Some(content.as_ref().clone());
        }
    } else if let Some(Value::Str(text)) = choice_dict.get("text") {
        return Some(text.as_ref().clone());
    }
    None
}

fn make_stream_callback_chunk(root: &DictMap, choice: &Value, is_last: bool) -> Value {
    let mut raw = DictMap::default();
    raw.insert("choices".into(), Value::Array(Arc::new(vec![choice.clone()])));
    if is_last {
        if let Some(usage) = root.get("usage") {
            raw.insert("usage".into(), usage.clone());
        }
    }
    Value::Dict(Arc::new(raw))
}

fn extract_stream_callback_chunks(response_json: &Value) -> Vec<(String, Value)> {
    let mut chunks = Vec::new();
    let Some(root) = (match response_json {
        Value::Dict(root) => Some(root),
        _ => None,
    }) else {
        return chunks;
    };

    let Some(Value::Array(choice_values)) = root.get("choices") else {
        return chunks;
    };
    for (index, choice) in choice_values.iter().enumerate() {
        if let Value::Dict(choice_dict) = choice {
            if let Some(delta) = extract_chat_chunk_text(choice_dict) {
                let raw =
                    make_stream_callback_chunk(root, choice, index + 1 == choice_values.len());
                chunks.push((delta, raw));
            }
        }
    }

    chunks
}

fn deliver_stream_callback_chunks<F>(
    response_json: &Value,
    response_text: &str,
    mut invoke: F,
) -> Result<Vec<Value>, Value>
where
    F: FnMut(&str, &Value) -> Result<bool, Value>,
{
    let callback_chunks = extract_stream_callback_chunks(response_json);
    if callback_chunks.is_empty() && !response_text.is_empty() {
        let raw = response_json.clone();
        let should_continue = invoke(response_text, &raw)?;
        let delivered = vec![Value::Str(Arc::new(response_text.to_string()))];
        if !should_continue {
            return Ok(delivered);
        }
        return Ok(delivered);
    }

    let mut delivered = Vec::new();
    for (delta, raw) in callback_chunks {
        let should_continue = invoke(&delta, &raw)?;
        delivered.push(Value::Str(Arc::new(delta)));
        if !should_continue {
            break;
        }
    }
    Ok(delivered)
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

fn extract_usage(response_json: &Value) -> Option<Value> {
    let root = match response_json {
        Value::Dict(root) => root,
        _ => return None,
    };
    let usage = match root.get("usage") {
        Some(Value::Dict(usage)) => usage,
        _ => return None,
    };

    let mut normalized = DictMap::default();
    for key in ["prompt_tokens", "completion_tokens", "total_tokens"] {
        if let Some(value) = usage.get(key).and_then(value_to_i64) {
            normalized.insert(key.into(), Value::Int(value));
        }
    }
    if normalized.is_empty() {
        None
    } else {
        Some(Value::Dict(Arc::new(normalized)))
    }
}

fn extract_finish_reason(response_json: &Value) -> Value {
    let root = match response_json {
        Value::Dict(root) => root,
        _ => return Value::Null,
    };
    let choices = match root.get("choices") {
        Some(Value::Array(choices)) => choices,
        _ => return Value::Null,
    };
    for choice in choices.iter().rev() {
        let Value::Dict(choice) = choice else {
            continue;
        };
        match choice.get("finish_reason") {
            Some(Value::Str(reason)) => return Value::Str(reason.clone()),
            Some(Value::Null) => continue,
            _ => continue,
        }
    }
    Value::Null
}

fn first_ai_choice(response_json: &Value) -> Option<&DictMap> {
    let root = match response_json {
        Value::Dict(root) => root,
        _ => return None,
    };
    let choices = match root.get("choices") {
        Some(Value::Array(choices)) => choices,
        _ => return None,
    };
    match choices.first() {
        Some(Value::Dict(choice)) => Some(choice),
        _ => None,
    }
}

fn extract_tool_calls(response_json: &Value) -> Value {
    let Some(choice) = first_ai_choice(response_json) else {
        return Value::Array(Arc::new(Vec::new()));
    };
    let message = match choice.get("message") {
        Some(Value::Dict(message)) => message,
        _ => return Value::Array(Arc::new(Vec::new())),
    };
    let tool_calls = match message.get("tool_calls") {
        Some(Value::Array(tool_calls)) => tool_calls,
        _ => return Value::Array(Arc::new(Vec::new())),
    };

    let mut calls = Vec::new();
    for tool_call in tool_calls.iter() {
        let Value::Dict(call_dict) = tool_call else {
            continue;
        };
        let function_dict = match call_dict.get("function") {
            Some(Value::Dict(function)) => function,
            _ => continue,
        };

        let id = match call_dict.get("id") {
            Some(Value::Str(id)) => id.as_ref().clone(),
            _ => String::new(),
        };
        let name = match function_dict.get("name") {
            Some(Value::Str(name)) => name.as_ref().clone(),
            _ => String::new(),
        };
        let arguments_json = match function_dict.get("arguments") {
            Some(Value::Str(arguments)) => arguments.as_ref().clone(),
            Some(arguments) => builtins::to_json(arguments).unwrap_or_default(),
            None => String::new(),
        };

        let mut normalized = DictMap::default();
        normalized.insert("id".into(), Value::Str(Arc::new(id)));
        normalized.insert("name".into(), Value::Str(Arc::new(name)));
        normalized.insert("arguments_json".into(), Value::Str(Arc::new(arguments_json)));
        calls.push(Value::Dict(Arc::new(normalized)));
    }

    Value::Array(Arc::new(calls))
}

fn add_ai_envelope_fields(
    result: &mut DictMap,
    response_json: &Value,
    config: &AiRequestConfig,
    include_tool_calls: bool,
) {
    if let Some(usage) = extract_usage(response_json) {
        result.insert("usage".into(), usage);
    }
    result.insert("finish_reason".into(), extract_finish_reason(response_json));
    if include_tool_calls {
        result.insert("tool_calls".into(), extract_tool_calls(response_json));
    }
    result.insert("provider".into(), Value::Str(Arc::new(config.provider.clone())));
}

fn build_ai_stream_chat_result(
    config: &AiRequestConfig,
    response: AiHttpResponse,
    chunks_override: Option<Vec<Value>>,
) -> Value {
    if !(200..300).contains(&response.status) {
        return ai_http_status_error(
            "ai_stream_chat",
            config,
            response.status,
            &response.headers,
            &response.text,
        );
    }
    let Some(json) = response.json else {
        return ai_decode_error(
            "ai_stream_chat",
            config,
            response.decode_error.unwrap_or_else(|| "unknown error".to_string()),
            &response.text,
        );
    };

    let mut chunks = chunks_override.unwrap_or_else(|| extract_chat_chunks(&json));
    if chunks.is_empty() && !response.text.is_empty() {
        chunks.push(Value::Str(Arc::new(response.text.clone())));
    }

    let mut result = DictMap::default();
    result.insert("status".into(), Value::Int(response.status));
    result.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
    result.insert("chunks".into(), Value::Array(Arc::new(chunks)));
    result.insert("text".into(), Value::Str(Arc::new(response.text)));
    result.insert("headers".into(), Value::Dict(Arc::new(response.headers)));
    add_ai_envelope_fields(&mut result, &json, config, false);
    result.insert("json".into(), json);
    ai_ok_result(Value::Dict(Arc::new(result)))
}

fn invoke_interpreter_stream_callback(
    interp: &mut Interpreter,
    callback: &Value,
    delta: &str,
    raw: &Value,
) -> Result<bool, Value> {
    let result = interp
        .call_user_function(callback, &[Value::Str(Arc::new(delta.to_string())), raw.clone()]);
    match result {
        Value::Error(_) | Value::ErrorObject { .. } => Err(result),
        Value::Bool(false) => Ok(false),
        _ => Ok(true),
    }
}

pub(crate) fn handle_ai_stream_chat_with_callback_invoker<F>(
    arg_values: &[Value],
    mut invoke: F,
) -> Option<Value>
where
    F: FnMut(&str, &Value) -> Result<bool, Value>,
{
    if arg_values.len() != 3 {
        return Some(Value::Error(format!(
            "ai_stream_chat() expects 2 or 3 arguments (prompt_or_messages, options, on_chunk), got {}",
            arg_values.len()
        )));
    }

    let options = match arg_values.get(1).and_then(dict_like_from_value) {
        Some(options) => options,
        None => {
            return Some(Value::Error(
                "ai_stream_chat() requires an options dictionary as second argument".to_string(),
            ));
        }
    };

    let config = match parse_ai_request_config(&options, "ai_stream_chat") {
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
        &options,
        &["model", "messages", "stream"],
        "ai_stream_chat",
    ) {
        return Some(error);
    }

    let request = run_ai_request("ai_stream_chat", &config, Value::Dict(Arc::new(payload)));
    match request {
        Ok(response) => {
            if !(200..300).contains(&response.status) {
                return Some(ai_http_status_error(
                    "ai_stream_chat",
                    &config,
                    response.status,
                    &response.headers,
                    &response.text,
                ));
            }
            let Some(json) = response.json.clone() else {
                return Some(ai_decode_error(
                    "ai_stream_chat",
                    &config,
                    response.decode_error.unwrap_or_else(|| "unknown error".to_string()),
                    &response.text,
                ));
            };
            let chunks =
                match deliver_stream_callback_chunks(&json, &response.text, |delta, raw| {
                    invoke(delta, raw)
                }) {
                    Ok(chunks) => chunks,
                    Err(error) => return Some(error),
                };
            Some(build_ai_stream_chat_result(&config, response, Some(chunks)))
        }
        Err(error) => Some(ai_transport_error("ai_stream_chat", &config, error)),
    }
}

#[cfg(test)]
pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let mut interp = Interpreter::new();
    handle_with_interpreter(&mut interp, name, arg_values)
}

pub fn handle_with_interpreter(
    interp: &mut Interpreter,
    name: &str,
    arg_values: &[Value],
) -> Option<Value> {
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
            if !(1..=3).contains(&arg_values.len()) {
                return Some(Value::Error(format!(
                    "http_request() expects 1-3 arguments ((request), (url, options), or (url, options, on_stream_event)), got {}",
                    arg_values.len()
                )));
            }

            let callback = match arg_values.get(2) {
                None => None,
                Some(
                    value @ (Value::Function(_, _, _)
                    | Value::NativeFunction(_)
                    | Value::BytecodeFunction { .. }),
                ) => Some(value.clone()),
                Some(_) => {
                    return Some(Value::Error(
                        "http_request() third argument must be a function".to_string(),
                    ));
                }
            };

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

            let incremental = match options.get("response_stream") {
                None => false,
                Some(Value::Bool(value)) => *value,
                Some(_) => {
                    return Some(Value::Result {
                        is_ok: false,
                        value: Box::new(Value::Str(Arc::new(
                            "HTTP response_stream must be a boolean".to_string(),
                        ))),
                    });
                }
            };
            if callback.is_some() && !incremental {
                return Some(Value::Error(
                    "http_request() stream callback requires options.response_stream=true"
                        .to_string(),
                ));
            }

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

            let max_response_bytes = match options.get("max_response_bytes") {
                None => network_policy::MAX_NETWORK_BODY_BYTES,
                Some(Value::Int(value))
                    if *value > 0 && *value <= network_policy::MAX_NETWORK_BODY_BYTES as i64 =>
                {
                    *value as usize
                }
                Some(Value::Int(_)) => {
                    return Some(Value::Result {
                        is_ok: false,
                        value: Box::new(Value::Str(Arc::new(format!(
                            "HTTP max_response_bytes must be 1-{}",
                            network_policy::MAX_NETWORK_BODY_BYTES
                        )))),
                    });
                }
                Some(_) => {
                    return Some(Value::Result {
                        is_ok: false,
                        value: Box::new(Value::Str(Arc::new(
                            "HTTP max_response_bytes must be an integer".to_string(),
                        ))),
                    });
                }
            };

            let force_deny_private = match options.get("destination_policy") {
                None => false,
                Some(Value::Str(policy)) if policy.as_ref() == "inherit" => false,
                Some(Value::Str(policy)) if policy.as_ref() == "deny_private" => true,
                Some(Value::Str(policy)) => {
                    return Some(Value::Result {
                        is_ok: false,
                        value: Box::new(Value::Str(Arc::new(format!(
                            "Invalid HTTP destination_policy '{}'; expected inherit or deny_private",
                            policy
                        )))),
                    });
                }
                Some(_) => {
                    return Some(Value::Result {
                        is_ok: false,
                        value: Box::new(Value::Str(Arc::new(
                            "HTTP destination_policy must be a string".to_string(),
                        ))),
                    });
                }
            };

            let pin_dns = match options.get("pin_dns") {
                None => false,
                Some(Value::Bool(value)) => *value,
                Some(_) => {
                    return Some(Value::Result {
                        is_ok: false,
                        value: Box::new(Value::Str(Arc::new(
                            "HTTP pin_dns must be a boolean".to_string(),
                        ))),
                    });
                }
            };

            let follow_redirects = match options.get("redirects") {
                None => true,
                Some(Value::Str(policy)) if policy.as_ref() == "follow" => true,
                Some(Value::Str(policy)) if policy.as_ref() == "none" => false,
                Some(Value::Str(policy)) => {
                    return Some(Value::Result {
                        is_ok: false,
                        value: Box::new(Value::Str(Arc::new(format!(
                            "Invalid HTTP redirects policy '{}'; expected follow or none",
                            policy
                        )))),
                    });
                }
                Some(_) => {
                    return Some(Value::Result {
                        is_ok: false,
                        value: Box::new(Value::Str(Arc::new(
                            "HTTP redirects policy must be a string".to_string(),
                        ))),
                    });
                }
            };

            let headers: Vec<(String, String)> = options
                .get("_headers")
                .or_else(|| options.get("headers"))
                .and_then(header_pairs_from_value)
                .unwrap_or_default();

            let body =
                options.get("_body").or_else(|| options.get("body")).and_then(
                    |value| match value {
                        Value::Str(body) => Some(body.as_bytes().to_vec()),
                        Value::Bytes(body) => Some(body.clone()),
                        _ => None,
                    },
                );

            let request_result =
                network_policy::run_blocking_http_task("HTTP request", move || {
                    let method = Method::from_bytes(method_name.as_bytes()).map_err(|error| {
                        format!("Invalid HTTP method '{}': {}", method_name, error)
                    })?;
                    let client = network_policy::build_policy_http_client(
                        &url,
                        Duration::from_secs_f64(timeout_seconds),
                        force_deny_private,
                        pin_dns,
                        follow_redirects,
                        "HTTP request",
                    )?;

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
                    if incremental {
                        Ok(HttpRequestResult::Incremental(response))
                    } else {
                        network_policy::read_http_response_bytes_bounded(
                            response,
                            "HTTP request",
                            max_response_bytes,
                        )
                        .map(|(status, headers, body)| {
                            HttpRequestResult::Buffered(status, headers, body)
                        })
                    }
                });

            match request_result {
                Ok(HttpRequestResult::Buffered(status, response_headers, body_bytes)) => {
                    let status = status as i64;
                    let body = String::from_utf8_lossy(&body_bytes).to_string();

                    let mut result_dict = DictMap::default();
                    result_dict.insert("status".into(), Value::Int(status));
                    result_dict.insert("_status".into(), Value::Int(status));
                    result_dict.insert("body".into(), Value::Str(Arc::new(body.clone())));
                    result_dict.insert("_body".into(), Value::Str(Arc::new(body)));
                    result_dict.insert("_body_bytes".into(), Value::Bytes(body_bytes));

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
                Ok(HttpRequestResult::Incremental(response)) => {
                    match incremental_http_response_value(response, max_response_bytes, callback) {
                        Ok(response) => Value::Result { is_ok: true, value: Box::new(response) },
                        Err(error) => Value::Result {
                            is_ok: false,
                            value: Box::new(Value::Str(Arc::new(error))),
                        },
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

        "ai_text" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "ai_text() expects 1 argument (content), got {}",
                    arg_values.len()
                )));
            }
            match &arg_values[0] {
                Value::Str(content) => ai_text_block(content.as_ref().clone()),
                _ => Value::Error("ai_text() requires content to be a string".to_string()),
            }
        }

        "ai_image_url" => {
            if !(1..=2).contains(&arg_values.len()) {
                return Some(Value::Error(format!(
                    "ai_image_url() expects 1 or 2 arguments (url, detail?), got {}",
                    arg_values.len()
                )));
            }
            let url = match &arg_values[0] {
                Value::Str(url) if !url.is_empty() => url.as_ref().clone(),
                _ => {
                    return Some(Value::Error(
                        "ai_image_url() requires url to be a non-empty string".to_string(),
                    ));
                }
            };
            let detail = match arg_values.get(1) {
                Some(Value::Str(detail)) => Some(detail.as_ref().clone()),
                Some(_) => {
                    return Some(Value::Error(
                        "ai_image_url() requires detail to be a string when provided".to_string(),
                    ));
                }
                None => None,
            };
            ai_image_url_block(url, detail)
        }

        "ai_message" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_message() expects 2 arguments (role, content_or_blocks), got {}",
                    arg_values.len()
                )));
            }
            let role = match &arg_values[0] {
                Value::Str(role) if !role.is_empty() => role.as_ref().clone(),
                _ => {
                    return Some(Value::Error(
                        "ai_message() requires role to be a non-empty string".to_string(),
                    ));
                }
            };
            let content = match normalize_ai_message_content(
                &arg_values[1],
                "ai_message",
                "content_or_blocks",
            ) {
                Ok(content) => content,
                Err(error) => return Some(error),
            };
            ai_message_value(&role, content)
        }

        "ai_chat" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_chat() expects 2 arguments (prompt_or_messages, options), got {}",
                    arg_values.len()
                )));
            }

            let options = match arg_values.get(1).and_then(dict_like_from_value) {
                Some(options) => options,
                None => {
                    return Some(Value::Error(
                        "ai_chat() requires an options dictionary as second argument".to_string(),
                    ));
                }
            };

            let config = match parse_ai_request_config(&options, "ai_chat") {
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
                merge_ai_extra_body(&mut payload, &options, &["model", "messages"], "ai_chat")
            {
                return Some(error);
            }

            let request = run_ai_request("ai_chat", &config, Value::Dict(Arc::new(payload)));
            match request {
                Ok(response) => {
                    if !(200..300).contains(&response.status) {
                        return Some(ai_http_status_error(
                            "ai_chat",
                            &config,
                            response.status,
                            &response.headers,
                            &response.text,
                        ));
                    }
                    let Some(json) = response.json else {
                        return Some(ai_decode_error(
                            "ai_chat",
                            &config,
                            response.decode_error.unwrap_or_else(|| "unknown error".to_string()),
                            &response.text,
                        ));
                    };

                    let message = match extract_chat_content(&json) {
                        Some(message) => message,
                        None if config.structured_errors => {
                            return Some(ai_invalid_response_error(
                                "ai_chat",
                                &config,
                                "ai_chat failed: response JSON missing assistant content"
                                    .to_string(),
                            ));
                        }
                        None => String::new(),
                    };
                    let mut result = DictMap::default();
                    result.insert("status".into(), Value::Int(response.status));
                    result.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
                    result.insert("message".into(), Value::Str(Arc::new(message)));
                    result.insert("text".into(), Value::Str(Arc::new(response.text)));
                    result.insert("headers".into(), Value::Dict(Arc::new(response.headers)));
                    add_ai_envelope_fields(&mut result, &json, &config, true);
                    result.insert("json".into(), json);
                    ai_ok_result(Value::Dict(Arc::new(result)))
                }
                Err(error) => ai_transport_error("ai_chat", &config, error),
            }
        }

        "ai_stream_chat" => {
            if !(2..=3).contains(&arg_values.len()) {
                return Some(Value::Error(format!(
                    "ai_stream_chat() expects 2 or 3 arguments (prompt_or_messages, options, on_chunk), got {}",
                    arg_values.len()
                )));
            }
            let callback = if arg_values.len() == 3 {
                match &arg_values[2] {
                    value @ Value::Function(_, _, _) => Some(value.clone()),
                    Value::BytecodeFunction { .. } => {
                        return Some(Value::Error(
                            "ai_stream_chat() bytecode callbacks are only supported by the VM"
                                .to_string(),
                        ));
                    }
                    _ => {
                        return Some(Value::Error(
                            "ai_stream_chat() third argument must be a function".to_string(),
                        ));
                    }
                }
            } else {
                None
            };

            let options = match arg_values.get(1).and_then(dict_like_from_value) {
                Some(options) => options,
                None => {
                    return Some(Value::Error(
                        "ai_stream_chat() requires an options dictionary as second argument"
                            .to_string(),
                    ));
                }
            };

            let config = match parse_ai_request_config(&options, "ai_stream_chat") {
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
                &options,
                &["model", "messages", "stream"],
                "ai_stream_chat",
            ) {
                return Some(error);
            }

            if let Some(callback) = callback {
                return handle_ai_stream_chat_with_callback_invoker(arg_values, |delta, raw| {
                    invoke_interpreter_stream_callback(interp, &callback, delta, raw)
                });
            }

            let request = run_ai_request("ai_stream_chat", &config, Value::Dict(Arc::new(payload)));
            match request {
                Ok(response) => build_ai_stream_chat_result(&config, response, None),
                Err(error) => ai_transport_error("ai_stream_chat", &config, error),
            }
        }

        "ai_embedding" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_embedding() expects 2 arguments (input, options), got {}",
                    arg_values.len()
                )));
            }

            let options = match arg_values.get(1).and_then(dict_like_from_value) {
                Some(options) => options,
                None => {
                    return Some(Value::Error(
                        "ai_embedding() requires an options dictionary as second argument"
                            .to_string(),
                    ));
                }
            };

            let config = match parse_ai_request_config(&options, "ai_embedding") {
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
                merge_ai_extra_body(&mut payload, &options, &["model", "input"], "ai_embedding")
            {
                return Some(error);
            }

            let request = run_ai_request("ai_embedding", &config, Value::Dict(Arc::new(payload)));
            match request {
                Ok(response) => {
                    if !(200..300).contains(&response.status) {
                        return Some(ai_http_status_error(
                            "ai_embedding",
                            &config,
                            response.status,
                            &response.headers,
                            &response.text,
                        ));
                    }
                    let Some(json) = response.json else {
                        return Some(ai_decode_error(
                            "ai_embedding",
                            &config,
                            response.decode_error.unwrap_or_else(|| "unknown error".to_string()),
                            &response.text,
                        ));
                    };

                    let vector = match extract_embedding_vector(&json) {
                        Some(vector) => vector,
                        None => {
                            return Some(ai_invalid_response_error(
                                "ai_embedding",
                                &config,
                                "ai_embedding failed: response JSON missing data[0].embedding numeric array"
                                    .to_string(),
                            ));
                        }
                    };

                    let mut result = DictMap::default();
                    result.insert("status".into(), Value::Int(response.status));
                    result.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
                    result.insert("vector".into(), Value::Array(Arc::new(vector)));
                    result.insert("text".into(), Value::Str(Arc::new(response.text)));
                    result.insert("headers".into(), Value::Dict(Arc::new(response.headers)));
                    add_ai_envelope_fields(&mut result, &json, &config, false);
                    result.insert("json".into(), json);
                    ai_ok_result(Value::Dict(Arc::new(result)))
                }
                Err(error) => ai_transport_error("ai_embedding", &config, error),
            }
        }

        "ai_tool_loop" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "ai_tool_loop() expects 2 arguments (prompt_or_messages, options), got {}",
                    arg_values.len()
                )));
            }

            let options = match arg_values.get(1).and_then(dict_like_from_value) {
                Some(options) => options,
                None => {
                    return Some(Value::Error(
                        "ai_tool_loop() requires an options dictionary as second argument"
                            .to_string(),
                    ));
                }
            };

            let config = match parse_ai_request_config(&options, "ai_tool_loop") {
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
                Some(results) => match dict_like_from_value(results) {
                    Some(results) => Some(Arc::new(results)),
                    None => {
                        return Some(Value::Error(
                            "ai_tool_loop() requires options.tool_results to be a dictionary when provided"
                                .to_string(),
                        ));
                    }
                },
                None => None,
            };

            let mut messages = match parse_ai_messages(&arg_values[0], "ai_tool_loop") {
                Ok(messages) => messages,
                Err(error) => return Some(error),
            };

            let mut final_status = 0_i64;
            let mut final_text = String::new();
            let mut final_json = Value::Null;
            let mut final_headers = DictMap::default();
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
                    &options,
                    &["model", "messages", "tools", "stream"],
                    "ai_tool_loop",
                ) {
                    return Some(error);
                }

                let request_result =
                    run_ai_request("ai_tool_loop", &config, Value::Dict(Arc::new(payload)));
                let response = match request_result {
                    Ok(response) => response,
                    Err(error) => return Some(ai_transport_error("ai_tool_loop", &config, error)),
                };
                if !(200..300).contains(&response.status) {
                    return Some(ai_http_status_error(
                        "ai_tool_loop",
                        &config,
                        response.status,
                        &response.headers,
                        &response.text,
                    ));
                }
                let Some(response_json) = response.json else {
                    return Some(ai_decode_error(
                        "ai_tool_loop",
                        &config,
                        response.decode_error.unwrap_or_else(|| "unknown error".to_string()),
                        &response.text,
                    ));
                };

                steps_taken += 1;
                final_status = response.status;
                final_text = response.text;
                final_headers = response.headers;
                final_json = response_json.clone();
                last_message = extract_chat_content(&response_json).unwrap_or_default();
                messages.push(ai_text_message_value("assistant", last_message.clone()));

                let tool_call_names = extract_tool_call_names(&response_json);
                if tool_call_names.is_empty() {
                    break;
                }

                let Some(tool_results) = &tool_results else {
                    return Some(ai_invalid_response_error(
                        "ai_tool_loop",
                        &config,
                        "ai_tool_loop requires options.tool_results to resolve tool_calls in model responses"
                            .to_string(),
                    ));
                };

                for tool_name in tool_call_names {
                    let tool_output = match tool_results.get(tool_name.as_str()) {
                        Some(Value::Str(output)) => output.as_ref().clone(),
                        Some(_) => {
                            return Some(ai_invalid_response_error(
                                "ai_tool_loop",
                                &config,
                                format!(
                                    "ai_tool_loop requires options.tool_results['{}'] to be a string",
                                    tool_name
                                ),
                            ));
                        }
                        None => {
                            return Some(ai_invalid_response_error(
                                "ai_tool_loop",
                                &config,
                                format!("ai_tool_loop missing tool result for '{}'", tool_name),
                            ));
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
            result.insert("model".into(), Value::Str(Arc::new(config.model.clone())));
            result.insert("steps".into(), Value::Int(steps_taken));
            result.insert("message".into(), Value::Str(Arc::new(last_message)));
            result.insert("text".into(), Value::Str(Arc::new(final_text)));
            result.insert("headers".into(), Value::Dict(Arc::new(final_headers)));
            add_ai_envelope_fields(&mut result, &final_json, &config, true);
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

        "http_download_file" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(format!(
                    "http_download_file() expects 3 arguments (url, output_path, options), got {}",
                    arg_values.len()
                )));
            }
            let (Value::Str(url), Value::Str(output_path), Some(options)) =
                (&arg_values[0], &arg_values[1], dict_like_from_value(&arg_values[2]))
            else {
                return Some(Value::Error(
                    "http_download_file requires URL and output path strings plus an options dictionary"
                        .to_string(),
                ));
            };
            let max_bytes = match options.get("max_bytes") {
                Some(Value::Int(value)) if *value > 0 => *value as u64,
                Some(_) => {
                    return Some(Value::Error(
                        "http_download_file options.max_bytes must be a positive integer"
                            .to_string(),
                    ));
                }
                None => runtime_limits::MAX_NETWORK_BODY_BYTES as u64,
            };
            let overwrite = match options.get("overwrite") {
                Some(Value::Bool(value)) => *value,
                Some(_) => {
                    return Some(Value::Error(
                        "http_download_file options.overwrite must be a boolean".to_string(),
                    ));
                }
                None => false,
            };
            let headers = match options.get("headers") {
                Some(value) => match dict_like_from_value(value) {
                    Some(headers) => {
                        let mut pairs = Vec::with_capacity(headers.len());
                        for (name, value) in headers.iter() {
                            let Value::Str(value) = value else {
                                return Some(Value::Error(format!(
                                    "http_download_file options.headers['{}'] must be a string",
                                    name
                                )));
                            };
                            pairs.push((name.to_string(), value.to_string()));
                        }
                        pairs
                    }
                    None => {
                        return Some(Value::Error(
                            "http_download_file options.headers must be a dictionary".to_string(),
                        ));
                    }
                },
                None => Vec::new(),
            };
            match builtins::http_download_file(
                url.as_ref(),
                output_path.as_ref(),
                headers,
                max_bytes,
                overwrite,
            ) {
                Ok(result) => Value::Dict(Arc::new(result)),
                Err(error) => Value::Error(error),
            }
        }

        "http_upload_file" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(format!(
                    "http_upload_file() expects 3 arguments (url, input_path, options), got {}",
                    arg_values.len()
                )));
            }
            let (Value::Str(url), Value::Str(input_path), Some(options)) =
                (&arg_values[0], &arg_values[1], dict_like_from_value(&arg_values[2]))
            else {
                return Some(Value::Error(
                    "http_upload_file requires URL and input path strings plus an options dictionary"
                        .to_string(),
                ));
            };
            let method = match options.get("method") {
                Some(Value::Str(value)) => value.to_string(),
                Some(_) => {
                    return Some(Value::Error(
                        "http_upload_file options.method must be a string".to_string(),
                    ));
                }
                None => "PUT".to_string(),
            };
            let max_response_bytes = match options.get("max_response_bytes") {
                Some(Value::Int(value)) if *value > 0 => match usize::try_from(*value) {
                    Ok(value) => value,
                    Err(_) => {
                        return Some(Value::Error(
                            "http_upload_file options.max_response_bytes is too large".to_string(),
                        ));
                    }
                },
                Some(_) => {
                    return Some(Value::Error(
                        "http_upload_file options.max_response_bytes must be a positive integer"
                            .to_string(),
                    ));
                }
                None => runtime_limits::MAX_NETWORK_BODY_BYTES,
            };
            let headers = match options.get("headers") {
                Some(value) => match dict_like_from_value(value) {
                    Some(headers) => {
                        let mut pairs = Vec::with_capacity(headers.len());
                        for (name, value) in headers.iter() {
                            let Value::Str(value) = value else {
                                return Some(Value::Error(format!(
                                    "http_upload_file options.headers['{}'] must be a string",
                                    name
                                )));
                            };
                            pairs.push((name.to_string(), value.to_string()));
                        }
                        pairs
                    }
                    None => {
                        return Some(Value::Error(
                            "http_upload_file options.headers must be a dictionary".to_string(),
                        ));
                    }
                },
                None => Vec::new(),
            };
            match builtins::http_upload_file(
                url.as_ref(),
                input_path.as_ref(),
                &method,
                headers,
                max_response_bytes,
            ) {
                Ok(result) => Value::Dict(Arc::new(result)),
                Err(error) => Value::Error(error),
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
                } else if let Value::HttpStreamingResponse { status, headers, stream, callback } =
                    response
                {
                    let mut new_headers = headers.clone();
                    new_headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
                    Value::HttpStreamingResponse {
                        status: *status,
                        headers: new_headers,
                        stream: stream.clone(),
                        callback: callback.clone(),
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
                    } else if let Value::HttpStreamingResponse {
                        status,
                        headers,
                        stream,
                        callback,
                    } = response
                    {
                        let mut new_headers = headers.clone();
                        for (key, value) in header_pairs {
                            if let Value::Str(header_value) = value {
                                new_headers.insert(key, header_value.as_ref().to_string());
                            }
                        }
                        Value::HttpStreamingResponse {
                            status: *status,
                            headers: new_headers,
                            stream: stream.clone(),
                            callback: callback.clone(),
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

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Int(status)), Some(Value::Str(body))) => Value::HttpResponse {
                    status: *status as u16,
                    body: body.as_bytes().to_vec(),
                    headers: HashMap::new(),
                },
                (Some(Value::Int(status)), Some(Value::Bytes(body))) => Value::HttpResponse {
                    status: *status as u16,
                    body: body.clone(),
                    headers: HashMap::new(),
                },
                _ => Value::Error(
                    "http_response requires status code and string or bytes body".to_string(),
                ),
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
                Value::HttpResponse { status: *status as u16, body: body.into_bytes(), headers }
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
                    body: html.as_bytes().to_vec(),
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
                    body: format!("Redirecting to {}", url.as_ref()).into_bytes(),
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Mutex};

    static AI_ENV_LOCK: Mutex<()> = Mutex::new(());
    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(1);

    fn str_value(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    fn one_shot_json_server(
        status_code: u16,
        response_body: &'static str,
    ) -> Option<(String, mpsc::Receiver<Vec<u8>>, std::thread::JoinHandle<()>)> {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return None,
            Err(error) => panic!("test listener should bind: {error}"),
        };
        let address = listener.local_addr().expect("test listener should have address");
        let endpoint = format!("http://127.0.0.1:{}/v1/mock", address.port());

        let (request_tx, request_rx) = mpsc::channel::<Vec<u8>>();
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
            let _ = request_tx.send(buffer[body_start..body_end].to_vec());

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

    fn gated_stream_server(
    ) -> Option<(String, mpsc::Receiver<()>, mpsc::Sender<()>, std::thread::JoinHandle<()>)> {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return None,
            Err(error) => panic!("test listener should bind: {error}"),
        };
        let address = listener.local_addr().expect("test listener should have address");
        let endpoint = format!("http://127.0.0.1:{}/stream", address.port());
        let (first_chunk_tx, first_chunk_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let first = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"delta\":\"first\"}\n\n";
            let _ = stream.write_all(first);
            let _ = stream.flush();
            let _ = first_chunk_tx.send(());
            let _ = release_rx.recv();
            let _ = stream.write_all(b"data: [DONE]\n\n");
            let _ = stream.flush();
        });
        Some((endpoint, first_chunk_rx, release_tx, handle))
    }

    #[test]
    fn http_request_incremental_response_exposes_headers_chunks_and_completion() {
        let _guard = AI_ENV_LOCK.lock().expect("HTTP env lock should not be poisoned");
        clear_ai_cassette_env();
        let Some((endpoint, first_chunk_rx, release_tx, server)) = gated_stream_server() else {
            eprintln!("skipping incremental HTTP response test: local TCP bind unavailable");
            return;
        };

        let mut options = DictMap::default();
        options.insert("method".into(), str_value("GET"));
        options.insert("response_stream".into(), Value::Bool(true));
        options.insert("max_response_bytes".into(), Value::Int(4096));
        let result =
            handle("http_request", &[str_value(&endpoint), Value::Dict(Arc::new(options))])
                .expect("http_request should be handled");
        let Value::Result { is_ok: true, value } = result else {
            panic!("expected successful incremental HTTP result, got {result:?}");
        };
        first_chunk_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("upstream should flush the first chunk within the test deadline");
        let Value::HttpStreamingResponse { status, stream, .. } = *value else {
            panic!("expected a streaming HTTP response");
        };
        assert_eq!(status, 200);
        let mut parts = stream
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("stream should be single-use and available");
        let headers = parts.events.recv().expect("headers event should arrive");
        assert_eq!(Interpreter::http_stream_event_type(&headers), Some("headers"));

        let mut first = [0_u8; 128];
        let first_len = parts.body.read(&mut first).expect("first chunk should read");
        assert!(String::from_utf8_lossy(&first[..first_len]).contains("first"));
        let first_event = parts.events.recv().expect("chunk event should arrive");
        assert_eq!(Interpreter::http_stream_event_type(&first_event), Some("chunk"));

        release_tx.send(()).expect("second upstream chunk should be released");
        let mut remainder = Vec::new();
        parts.body.read_to_end(&mut remainder).expect("stream should finish");
        assert!(String::from_utf8_lossy(&remainder).contains("[DONE]"));
        let mut saw_complete = false;
        while let Ok(event) = parts.events.recv_timeout(Duration::from_millis(100)) {
            if Interpreter::http_stream_event_type(&event) == Some("complete") {
                saw_complete = true;
                break;
            }
        }
        assert!(saw_complete, "stream should emit a completion event");
        server.join().expect("stream server should finish");
    }

    #[test]
    fn http_request_incremental_response_reports_downstream_disconnect() {
        let _guard = AI_ENV_LOCK.lock().expect("HTTP env lock should not be poisoned");
        clear_ai_cassette_env();
        let Some((endpoint, first_chunk_rx, release_tx, server)) = gated_stream_server() else {
            eprintln!("skipping incremental HTTP disconnect test: local TCP bind unavailable");
            return;
        };

        let mut options = DictMap::default();
        options.insert("method".into(), str_value("GET"));
        options.insert("response_stream".into(), Value::Bool(true));
        let result =
            handle("http_request", &[str_value(&endpoint), Value::Dict(Arc::new(options))])
                .expect("http_request should be handled");
        let Value::Result { is_ok: true, value } = result else {
            panic!("expected successful incremental HTTP result, got {result:?}");
        };
        first_chunk_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("upstream should flush the first chunk within the test deadline");
        let Value::HttpStreamingResponse { stream, .. } = *value else {
            panic!("expected a streaming HTTP response");
        };
        let parts = stream
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("stream should be single-use and available");
        let headers = parts.events.recv().expect("headers event should arrive");
        assert_eq!(Interpreter::http_stream_event_type(&headers), Some("headers"));

        drop(parts.body);
        let cancelled = parts.events.recv().expect("cancellation event should arrive");
        assert_eq!(Interpreter::http_stream_event_type(&cancelled), Some("cancelled"));
        let Value::Dict(fields) = cancelled else {
            panic!("expected cancellation event fields");
        };
        assert!(matches!(
            fields.get("reason"),
            Some(Value::Str(reason)) if reason.as_str() == "downstream_disconnected"
        ));

        release_tx.send(()).expect("upstream should be released");
        server.join().expect("stream server should finish");
    }

    fn ai_options(endpoint: &str, model: &str) -> Value {
        let mut options = DictMap::default();
        options.insert("endpoint".into(), str_value(endpoint));
        options.insert("model".into(), str_value(model));
        Value::Dict(Arc::new(options))
    }

    fn ai_replay_fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ai_cassettes")
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let id = TEMP_DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), id))
    }

    fn ai_options_with_cassette(endpoint: &str, model: &str, mode: &str, dir: &Path) -> Value {
        let mut cassette = DictMap::default();
        cassette.insert("mode".into(), str_value(mode));
        cassette.insert("dir".into(), str_value(&dir.to_string_lossy()));

        let mut options = DictMap::default();
        options.insert("endpoint".into(), str_value(endpoint));
        options.insert("model".into(), str_value(model));
        options.insert("cassette".into(), Value::Dict(Arc::new(cassette)));
        Value::Dict(Arc::new(options))
    }

    fn ai_options_with_cassette_flags(
        endpoint: &str,
        model: &str,
        mode: &str,
        dir: &Path,
        structured_errors: bool,
        provider: &str,
    ) -> Value {
        let mut options = match ai_options_with_cassette(endpoint, model, mode, dir) {
            Value::Dict(options) => (*options).clone(),
            _ => unreachable!("helper returns dict"),
        };
        options.insert("structured_errors".into(), Value::Bool(structured_errors));
        options.insert("provider".into(), str_value(provider));
        Value::Dict(Arc::new(options))
    }

    fn result_ok_dict(value: Value) -> DictMap {
        match value {
            Value::Result { is_ok: true, value } => match *value {
                Value::Dict(dict) => (*dict).clone(),
                other => panic!("expected ok dict, got {:?}", other),
            },
            other => panic!("expected ok result, got {:?}", other),
        }
    }

    fn result_err_dict(value: Value) -> DictMap {
        match value {
            Value::Result { is_ok: false, value } => match *value {
                Value::Dict(dict) => (*dict).clone(),
                other => panic!("expected err dict, got {:?}", other),
            },
            other => panic!("expected err result, got {:?}", other),
        }
    }

    fn result_err_string(value: Value) -> String {
        match value {
            Value::Result { is_ok: false, value } => match *value {
                Value::Str(message) => message.as_ref().clone(),
                other => panic!("expected err string, got {:?}", other),
            },
            other => panic!("expected err result, got {:?}", other),
        }
    }

    fn clear_ai_cassette_env() {
        std::env::remove_var("KUJO_AI_RECORD");
        std::env::remove_var("KUJO_AI_REPLAY");
        std::env::remove_var("KUJO_AI_REPLAY_MODE");
        std::env::remove_var(network_policy::AI_ALLOWED_ENDPOINTS_ENV);
        std::env::remove_var(network_policy::OUTBOUND_DESTINATION_POLICY_ENV);
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
    fn test_ai_options_api_key_accepts_secret_value() {
        let mut options = DictMap::default();
        options.insert("endpoint".into(), str_value("https://api.example.test/v1/chat"));
        options.insert("model".into(), str_value("gpt-secret"));
        options.insert(
            "api_key".into(),
            Value::Secret(Arc::new("sk-revealed-for-request".to_string())),
        );

        let config =
            parse_ai_request_config(&options, "ai_chat").expect("secret api_key should parse");

        assert_eq!(config.api_key.as_deref(), Some("sk-revealed-for-request"));
    }

    #[test]
    fn test_ai_message_builders_create_multimodal_shape() {
        let text = handle("ai_text", &[str_value("Describe this")])
            .expect("ai_text should return a value");
        let image =
            handle("ai_image_url", &[str_value("https://example.test/cat.png"), str_value("low")])
                .expect("ai_image_url should return a value");
        let message = handle(
            "ai_message",
            &[str_value("user"), Value::Array(Arc::new(vec![text.clone(), image.clone()]))],
        )
        .expect("ai_message should return a value");

        let Value::Dict(message) = message else {
            panic!("ai_message should return a dictionary");
        };
        assert!(matches!(message.get("role"), Some(Value::Str(role)) if role.as_ref() == "user"));
        let Some(Value::Array(blocks)) = message.get("content") else {
            panic!("ai_message content should be an array");
        };
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], Value::Dict(block)
            if matches!(block.get("type"), Some(Value::Str(kind)) if kind.as_ref() == "text")
                && matches!(block.get("text"), Some(Value::Str(content)) if content.as_ref() == "Describe this")));
        assert!(matches!(&blocks[1], Value::Dict(block)
            if matches!(block.get("type"), Some(Value::Str(kind)) if kind.as_ref() == "image_url")
                && matches!(block.get("image_url"), Some(Value::Dict(image_url))
                    if matches!(image_url.get("url"), Some(Value::Str(url)) if url.as_ref() == "https://example.test/cat.png")
                        && matches!(image_url.get("detail"), Some(Value::Str(detail)) if detail.as_ref() == "low"))));
    }

    #[test]
    fn test_parse_ai_messages_accepts_multimodal_blocks_unchanged() {
        let text = ai_text_block("Summarize the image");
        let image = ai_image_url_block("https://example.test/image.png", Some("high".to_string()));
        let message =
            ai_message_value("user", Value::Array(Arc::new(vec![text.clone(), image.clone()])));

        let messages = parse_ai_messages(&Value::Array(Arc::new(vec![message])), "ai_chat")
            .expect("multimodal message should parse");

        assert_eq!(messages.len(), 1);
        let Value::Dict(parsed) = &messages[0] else {
            panic!("parsed message should be a dictionary");
        };
        assert!(matches!(parsed.get("role"), Some(Value::Str(role)) if role.as_ref() == "user"));
        let Some(Value::Array(blocks)) = parsed.get("content") else {
            panic!("parsed content should be an array");
        };
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], Value::Dict(block)
            if matches!(block.get("type"), Some(Value::Str(kind)) if kind.as_ref() == "text")
                && matches!(block.get("text"), Some(Value::Str(content)) if content.as_ref() == "Summarize the image")));
        assert!(matches!(&blocks[1], Value::Dict(block)
            if matches!(block.get("type"), Some(Value::Str(kind)) if kind.as_ref() == "image_url")
                && matches!(block.get("image_url"), Some(Value::Dict(image_url))
                    if matches!(image_url.get("url"), Some(Value::Str(url)) if url.as_ref() == "https://example.test/image.png")
                        && matches!(image_url.get("detail"), Some(Value::Str(detail)) if detail.as_ref() == "high"))));
    }

    #[test]
    fn test_ai_message_builders_are_accepted_by_ai_helpers_before_replay() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        std::env::set_var(network_policy::OUTBOUND_DESTINATION_POLICY_ENV, "deny_private");
        let replay_dir = unique_temp_dir("kujo_ai_multimodal_replay");
        fs::create_dir_all(&replay_dir).expect("temp replay dir should be created");
        let endpoint = "http://127.0.0.1:1/v1/chat/completions";

        let message = ai_message_value(
            "user",
            Value::Array(Arc::new(vec![
                ai_text_block("Describe this"),
                ai_image_url_block("https://example.test/cat.png", None),
            ])),
        );
        let options = ai_options_with_cassette(endpoint, "gpt-replay", "replay", &replay_dir);

        let chat =
            handle("ai_chat", &[Value::Array(Arc::new(vec![message.clone()])), options.clone()])
                .expect("ai_chat should return a value");
        assert!(result_err_string(chat).contains("kind:\"replay_miss\""));

        let tool_loop = handle("ai_tool_loop", &[Value::Array(Arc::new(vec![message])), options])
            .expect("ai_tool_loop should return a value");
        assert!(result_err_string(tool_loop).contains("kind:\"replay_miss\""));
        clear_ai_cassette_env();
    }

    #[test]
    fn test_ai_cassette_strict_replay_runs_all_ai_helpers_without_network() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        std::env::set_var(network_policy::OUTBOUND_DESTINATION_POLICY_ENV, "deny_private");
        let fixture_dir = ai_replay_fixture_dir();
        let chat_endpoint = "http://127.0.0.1:1/v1/chat/completions";
        let embedding_endpoint = "http://127.0.0.1:1/v1/embeddings";

        let chat = handle(
            "ai_chat",
            &[
                str_value("Hello model"),
                ai_options_with_cassette(chat_endpoint, "gpt-replay", "replay", &fixture_dir),
            ],
        )
        .expect("ai_chat should return a value");
        assert!(matches!(chat, Value::Result { is_ok: true, value }
                if matches!(value.as_ref(), Value::Dict(result)
                    if matches!(result.get("message"), Some(Value::Str(message)) if message.as_ref() == "hello from cassette"))));

        let stream = handle(
            "ai_stream_chat",
            &[
                str_value("Stream please"),
                ai_options_with_cassette(chat_endpoint, "gpt-replay", "replay", &fixture_dir),
            ],
        )
        .expect("ai_stream_chat should return a value");
        assert!(matches!(stream, Value::Result { is_ok: true, value }
                if matches!(value.as_ref(), Value::Dict(result)
                    if matches!(result.get("chunks"), Some(Value::Array(chunks)) if chunks.len() == 2))));

        let embedding = handle(
            "ai_embedding",
            &[
                str_value("seed text"),
                ai_options_with_cassette(
                    embedding_endpoint,
                    "embed-replay",
                    "replay",
                    &fixture_dir,
                ),
            ],
        )
        .expect("ai_embedding should return a value");
        assert!(matches!(embedding, Value::Result { is_ok: true, value }
                if matches!(value.as_ref(), Value::Dict(result)
                    if matches!(result.get("vector"), Some(Value::Array(vector)) if vector.len() == 3))));

        let tool_loop = handle(
            "ai_tool_loop",
            &[
                str_value("Plan this"),
                ai_options_with_cassette(chat_endpoint, "gpt-replay", "replay", &fixture_dir),
            ],
        )
        .expect("ai_tool_loop should return a value");
        assert!(matches!(tool_loop, Value::Result { is_ok: true, value }
                if matches!(value.as_ref(), Value::Dict(result)
                    if matches!(result.get("message"), Some(Value::Str(message)) if message.as_ref() == "tool loop done"))));
        clear_ai_cassette_env();
    }

    #[test]
    fn test_ai_stream_chat_callback_replay_invokes_chunks_in_order() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        std::env::set_var(network_policy::OUTBOUND_DESTINATION_POLICY_ENV, "deny_private");
        let fixture_dir = ai_replay_fixture_dir();
        let endpoint = "http://127.0.0.1:1/v1/chat/completions";
        let mut seen = Vec::new();

        let result = handle_ai_stream_chat_with_callback_invoker(
            &[
                str_value("Stream please"),
                ai_options_with_cassette(endpoint, "gpt-replay", "replay", &fixture_dir),
                str_value("test_callback_placeholder"),
            ],
            |delta, raw| {
                if let Value::Dict(raw) = raw {
                    assert!(matches!(raw.get("choices"), Some(Value::Array(choices)) if choices.len() == 1));
                } else {
                    panic!("raw stream chunk should be a dictionary");
                }
                seen.push(delta.to_string());
                Ok(true)
            },
        )
        .expect("ai_stream_chat should return a value");

        assert_eq!(seen, vec!["chunk one".to_string(), "chunk two".to_string()]);
        let result = result_ok_dict(result);
        assert!(matches!(result.get("chunks"), Some(Value::Array(chunks)) if chunks.len() == 2));
        assert!(
            matches!(result.get("finish_reason"), Some(Value::Str(reason)) if reason.as_ref() == "stop")
        );
        assert!(matches!(result.get("usage"), Some(Value::Dict(usage))
                if matches!(usage.get("total_tokens"), Some(Value::Int(6)))));
        clear_ai_cassette_env();
    }

    #[test]
    fn test_ai_stream_chat_callback_false_cancels_replay() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        std::env::set_var(network_policy::OUTBOUND_DESTINATION_POLICY_ENV, "deny_private");
        let fixture_dir = ai_replay_fixture_dir();
        let endpoint = "http://127.0.0.1:1/v1/chat/completions";
        let mut seen = Vec::new();

        let result = handle_ai_stream_chat_with_callback_invoker(
            &[
                str_value("Stream please"),
                ai_options_with_cassette(endpoint, "gpt-replay", "replay", &fixture_dir),
                str_value("test_callback_placeholder"),
            ],
            |delta, _raw| {
                seen.push(delta.to_string());
                Ok(false)
            },
        )
        .expect("ai_stream_chat should return a value");

        assert_eq!(seen, vec!["chunk one".to_string()]);
        let result = result_ok_dict(result);
        assert!(matches!(result.get("chunks"), Some(Value::Array(chunks))
            if chunks.len() == 1
                && matches!(chunks.first(), Some(Value::Str(chunk)) if chunk.as_ref() == "chunk one")));
        clear_ai_cassette_env();
    }

    #[test]
    fn test_ai_event_stream_body_parses_chunks_done_and_usage() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}],",
            "\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\n",
            "data: [DONE]\n\n",
        );
        let parsed = parse_ai_event_stream_body(body).expect("event stream should parse");
        let chunks = extract_chat_chunks(&parsed);

        assert_eq!(chunks.len(), 2);
        assert!(matches!(chunks.first(), Some(Value::Str(chunk)) if chunk.as_ref() == "hello "));
        assert!(
            matches!(extract_finish_reason(&parsed), Value::Str(reason) if reason.as_ref() == "stop")
        );
        assert!(matches!(extract_usage(&parsed), Some(Value::Dict(usage))
            if matches!(usage.get("total_tokens"), Some(Value::Int(3)))));
    }

    #[test]
    fn test_ai_cassette_env_replay_miss_is_deterministic_and_hermetic() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        let replay_dir = unique_temp_dir("kujo_ai_empty_replay");
        fs::create_dir_all(&replay_dir).expect("temp replay dir should be created");
        std::env::set_var("KUJO_AI_REPLAY", replay_dir.to_string_lossy().to_string());
        std::env::set_var("KUJO_AI_REPLAY_MODE", "strict");
        std::env::set_var(network_policy::OUTBOUND_DESTINATION_POLICY_ENV, "deny_private");

        let result = handle(
            "ai_chat",
            &[
                str_value("missing cassette"),
                ai_options("http://127.0.0.1:1/v1/chat/completions", "gpt-replay"),
            ],
        )
        .expect("ai_chat should return a value");
        assert!(matches!(result, Value::Result { is_ok: false, value }
                if matches!(value.as_ref(), Value::Str(message)
                    if message.contains("kind:\"replay_miss\"")
                        && message.contains("strict replay does not use the network"))));

        clear_ai_cassette_env();
        let _ = fs::remove_dir_all(replay_dir);
    }

    #[test]
    fn test_ai_endpoint_allowlist_allows_matching_replay_endpoint() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        std::env::set_var(network_policy::AI_ALLOWED_ENDPOINTS_ENV, "http://127.0.0.1:1/v1");

        let result = handle(
            "ai_chat",
            &[
                str_value("Hello model"),
                ai_options_with_cassette(
                    "http://127.0.0.1:1/v1/chat/completions",
                    "gpt-replay",
                    "replay",
                    &ai_replay_fixture_dir(),
                ),
            ],
        )
        .expect("ai_chat should return a value");
        let result = result_ok_dict(result);
        assert!(
            matches!(result.get("message"), Some(Value::Str(message)) if message.as_ref() == "hello from cassette")
        );

        clear_ai_cassette_env();
    }

    #[test]
    fn test_ai_endpoint_allowlist_denies_structured_error() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        std::env::set_var(network_policy::AI_ALLOWED_ENDPOINTS_ENV, "https://api.example.test/v1");

        let result = handle(
            "ai_chat",
            &[
                str_value("Hello model"),
                ai_options_with_cassette_flags(
                    "http://127.0.0.1:1/v1/chat/completions",
                    "gpt-replay",
                    "replay",
                    &ai_replay_fixture_dir(),
                    true,
                    "",
                ),
            ],
        )
        .expect("ai_chat should return a value");
        let error = result_err_dict(result);
        assert!(
            matches!(error.get("kind"), Some(Value::Str(kind)) if kind.as_ref() == "endpoint_denied")
        );
        assert!(matches!(error.get("message"), Some(Value::Str(message))
                if message.contains(network_policy::AI_ALLOWED_ENDPOINTS_ENV)));

        clear_ai_cassette_env();
    }

    #[test]
    fn test_ai_request_hash_ignores_endpoint_allowlist() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        std::env::set_var(network_policy::AI_ALLOWED_ENDPOINTS_ENV, "https://api.example.test/v1");

        let result = handle(
            "ai_request_hash",
            &[
                str_value("Hello model"),
                ai_options("http://127.0.0.1:1/v1/chat/completions", "gpt-replay"),
            ],
        )
        .expect("ai_request_hash should return a value");
        assert!(matches!(result, Value::Str(hash) if hash.len() == 64));

        clear_ai_cassette_env();
    }

    #[test]
    fn test_ai_request_honors_private_destination_policy_when_networking() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        std::env::set_var(network_policy::OUTBOUND_DESTINATION_POLICY_ENV, "deny_private");

        let mut options = DictMap::default();
        options.insert("endpoint".into(), str_value("http://127.0.0.1:1/v1/chat/completions"));
        options.insert("model".into(), str_value("gpt-replay"));
        let config =
            parse_ai_request_config(&options, "ai_chat").expect("network config should parse");
        let mut payload = DictMap::default();
        payload.insert("model".into(), str_value("gpt-replay"));
        payload.insert("messages".into(), Value::Array(Arc::new(Vec::new())));

        let error = match run_ai_request("ai_chat", &config, Value::Dict(Arc::new(payload))) {
            Ok(_) => panic!("private destination policy should block before opening a socket"),
            Err(error) => error,
        };
        assert!(error.contains("blocked by outbound destination policy"));

        clear_ai_cassette_env();
    }

    #[test]
    fn test_ai_cassette_store_round_trips_and_redacts_sensitive_headers() {
        let temp_dir = unique_temp_dir("kujo_ai_cassette_store");
        let mut options = DictMap::default();
        options.insert("endpoint".into(), str_value("https://api.example.test/v1/chat"));
        options.insert("model".into(), str_value("gpt-redact"));
        options.insert("api_key".into(), str_value("secret-token"));
        let mut headers = DictMap::default();
        headers.insert("Authorization".into(), str_value("Bearer secret-token"));
        headers.insert("X-Trace".into(), str_value("trace-1"));
        options.insert("headers".into(), Value::Dict(Arc::new(headers)));

        let config =
            parse_ai_request_config(&options, "ai_chat").expect("config should parse for test");
        let payload = {
            let mut payload = DictMap::default();
            payload.insert("model".into(), str_value("gpt-redact"));
            payload.insert(
                "messages".into(),
                Value::Array(Arc::new(vec![ai_text_message_value("user", "Hello")])),
            );
            Value::Dict(Arc::new(payload))
        };
        let (key, normalized) = ai_request_key(&config, &payload).expect("key should build");
        let mut response_headers = DictMap::default();
        response_headers.insert("Authorization".into(), str_value("Bearer response-secret"));
        response_headers.insert("X-Usage".into(), str_value("1"));

        store_ai_cassette(
            "ai_chat",
            &temp_dir,
            &key,
            &normalized,
            &config,
            200,
            &response_headers,
            "{\"ok\":true,\"echo\":\"secret-token\"}",
        )
        .expect("cassette should store");
        let cassette_text =
            fs::read_to_string(cassette_path(&temp_dir, &key)).expect("cassette should exist");
        assert!(!cassette_text.contains("secret-token"));
        assert!(!cassette_text.contains("response-secret"));
        assert!(!cassette_text.contains("Bearer"));
        assert!(cassette_text.contains("[redacted]"));

        let replayed = replay_ai_cassette("ai_chat", &temp_dir, &key)
            .expect("replay lookup should parse")
            .expect("cassette should be found");
        assert_eq!(replayed.status, 200);
        assert!(
            matches!(replayed.json, Some(Value::Dict(body)) if matches!(body.get("ok"), Some(Value::Bool(true))))
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_ai_success_envelope_extracts_usage_finish_reason_tool_calls_and_provider() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        let endpoint = "http://127.0.0.1:1/v1/chat/completions";
        let result = handle(
            "ai_chat",
            &[
                str_value("Hello model"),
                ai_options_with_cassette_flags(
                    endpoint,
                    "gpt-replay",
                    "replay",
                    &ai_replay_fixture_dir(),
                    false,
                    "mock-provider",
                ),
            ],
        )
        .expect("ai_chat should return a value");
        let result = result_ok_dict(result);

        assert!(
            matches!(result.get("message"), Some(Value::Str(message)) if message.as_ref() == "hello from cassette")
        );
        assert!(
            matches!(result.get("provider"), Some(Value::Str(provider)) if provider.as_ref() == "mock-provider")
        );
        assert!(
            matches!(result.get("finish_reason"), Some(Value::Str(reason)) if reason.as_ref() == "tool_calls")
        );
        assert!(matches!(result.get("usage"), Some(Value::Dict(usage))
                if matches!(usage.get("prompt_tokens"), Some(Value::Int(7)))
                    && matches!(usage.get("completion_tokens"), Some(Value::Int(5)))
                    && matches!(usage.get("total_tokens"), Some(Value::Int(12)))));
        assert!(matches!(result.get("tool_calls"), Some(Value::Array(calls))
                if calls.len() == 1
                    && matches!(&calls[0], Value::Dict(call)
                        if matches!(call.get("id"), Some(Value::Str(id)) if id.as_ref() == "call_1")
                            && matches!(call.get("name"), Some(Value::Str(name)) if name.as_ref() == "lookup")
                            && matches!(call.get("arguments_json"), Some(Value::Str(args)) if args.contains("\"query\":\"kujo\"")))));
    }

    #[test]
    fn test_ai_structured_http_error_rate_limit_and_backward_compat_string_error() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        let endpoint = "http://127.0.0.1:1/v1/chat/completions";
        let plain = handle(
            "ai_chat",
            &[
                str_value("Rate limit"),
                ai_options_with_cassette(
                    endpoint,
                    "gpt-replay",
                    "replay",
                    &ai_replay_fixture_dir(),
                ),
            ],
        )
        .expect("ai_chat should return a value");
        assert!(matches!(plain, Value::Result { is_ok: false, value }
                if matches!(value.as_ref(), Value::Str(message)
                    if message.contains("ai_chat failed with HTTP status 429"))));

        let structured = handle(
            "ai_chat",
            &[
                str_value("Rate limit"),
                ai_options_with_cassette_flags(
                    endpoint,
                    "gpt-replay",
                    "replay",
                    &ai_replay_fixture_dir(),
                    true,
                    "mock-provider",
                ),
            ],
        )
        .expect("ai_chat should return a value");
        let error = result_err_dict(structured);
        assert!(
            matches!(error.get("kind"), Some(Value::Str(kind)) if kind.as_ref() == "rate_limited")
        );
        assert!(matches!(error.get("http_status"), Some(Value::Int(429))));
        assert!(matches!(error.get("retry_after_ms"), Some(Value::Int(3000))));
        assert!(
            matches!(error.get("provider_code"), Some(Value::Str(code)) if code.as_ref() == "rate_limit_exceeded")
        );
        assert!(
            matches!(error.get("body_excerpt"), Some(Value::Str(excerpt)) if excerpt.contains("slow down"))
        );
    }

    #[test]
    fn test_ai_success_envelope_omits_usage_when_provider_omits_usage() {
        let json = builtins::parse_json(r#"{"choices":[{"message":{"content":"ok"}}]}"#)
            .expect("test JSON should parse");
        assert!(extract_usage(&json).is_none());
        assert!(matches!(extract_finish_reason(&json), Value::Null));
    }

    #[test]
    fn test_ai_structured_decode_and_invalid_response_errors() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        let endpoint = "http://127.0.0.1:1/v1/chat/completions";

        let decode = handle(
            "ai_chat",
            &[
                str_value("Hello"),
                ai_options_with_cassette_flags(
                    endpoint,
                    "gpt-mock",
                    "replay",
                    &ai_replay_fixture_dir(),
                    true,
                    "",
                ),
            ],
        )
        .expect("ai_chat should return a value");
        let decode = result_err_dict(decode);
        assert!(
            matches!(decode.get("kind"), Some(Value::Str(kind)) if kind.as_ref() == "decode_error")
        );

        let invalid = handle(
            "ai_chat",
            &[
                str_value("Missing content"),
                ai_options_with_cassette_flags(
                    endpoint,
                    "gpt-replay",
                    "replay",
                    &ai_replay_fixture_dir(),
                    true,
                    "",
                ),
            ],
        )
        .expect("ai_chat should return a value");
        let invalid = result_err_dict(invalid);
        assert!(
            matches!(invalid.get("kind"), Some(Value::Str(kind)) if kind.as_ref() == "invalid_response")
        );
    }

    #[test]
    fn test_ai_structured_errors_redact_key_material() {
        let config = AiRequestConfig {
            endpoint: "https://api.example.test/v1/chat".to_string(),
            model: "gpt-redact".to_string(),
            api_key: Some("secret-token".to_string()),
            timeout_seconds: 1.0,
            headers: vec![("Authorization".to_string(), "Bearer secret-token".to_string())],
            cassette: AiCassetteMode::Off,
            structured_errors: true,
            provider: String::new(),
        };
        let headers = DictMap::default();
        let result = ai_http_status_error(
            "ai_chat",
            &config,
            500,
            &headers,
            "{\"error\":{\"message\":\"secret-token leaked\",\"code\":\"secret-token\"}}",
        );
        let error = result_err_dict(result);
        assert!(
            matches!(error.get("kind"), Some(Value::Str(kind)) if kind.as_ref() == "http_error")
        );
        assert!(matches!(error.get("http_status"), Some(Value::Int(500))));
        for key in ["message", "provider_code", "body_excerpt"] {
            match error.get(key) {
                Some(Value::Str(text)) => assert!(!text.contains("secret-token")),
                Some(Value::Null) => {}
                other => panic!("expected string/null for {}, got {:?}", key, other),
            }
        }
    }

    #[test]
    fn test_http_response_helpers_and_header_mutation() {
        let response = handle("http_response", &[Value::Int(200), str_value("ok")]).unwrap();
        match response {
            Value::HttpResponse { status, body, headers } => {
                assert_eq!(status, 200);
                assert_eq!(body, b"ok");
                assert!(headers.is_empty());
            }
            _ => panic!("Expected HttpResponse from http_response"),
        }

        let binary_response =
            handle("http_response", &[Value::Int(200), Value::Bytes(vec![0, 0x80, 0xff])]).unwrap();
        assert!(
            matches!(binary_response, Value::HttpResponse { body, .. } if body == vec![0, 0x80, 0xff])
        );

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
                Value::HttpResponse { status: 200, body: b"ok".to_vec(), headers: HashMap::new() },
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
                Value::HttpResponse { status: 200, body: b"ok".to_vec(), headers: HashMap::new() },
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
    fn test_http_request_preserves_binary_body_bytes() {
        let Some((endpoint, request_rx, server_handle)) = one_shot_json_server(200, "{}") else {
            eprintln!("skipping test_http_request_preserves_binary_body_bytes: local TCP bind not permitted in this environment");
            return;
        };
        let mut options = DictMap::default();
        options.insert("method".into(), str_value("POST"));
        options.insert("_body".into(), Value::Bytes(vec![0, 0x7f, 0x80, 0xff]));
        let result =
            handle("http_request", &[str_value(&endpoint), Value::Dict(Arc::new(options))])
                .expect("http_request should return a result value");
        server_handle.join().expect("server thread should finish");
        assert!(matches!(result, Value::Result { is_ok: true, ref value }
            if matches!(value.as_ref(), Value::Dict(dict)
                if matches!(dict.get("_body_bytes"), Some(Value::Bytes(bytes)) if bytes == b"{}"))));
        assert_eq!(
            request_rx.recv().expect("request body should be captured"),
            vec![0, 0x7f, 0x80, 0xff]
        );
    }

    #[test]
    fn test_ai_chat_success_path_returns_normalized_result_and_request_payload() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        let endpoint = "http://127.0.0.1:1/v1/chat/completions";

        let result = handle(
            "ai_chat",
            &[
                str_value("Hello model"),
                ai_options_with_cassette(
                    endpoint,
                    "gpt-replay",
                    "replay",
                    &ai_replay_fixture_dir(),
                ),
            ],
        )
        .expect("ai_chat should return a value");

        assert!(matches!(result, Value::Result { is_ok: true, value }
                if matches!(value.as_ref(), Value::Dict(result_dict)
                    if matches!(result_dict.get("status"), Some(Value::Int(200)))
                        && matches!(result_dict.get("message"), Some(Value::Str(message)) if message.as_ref() == "hello from cassette"))));
    }

    #[test]
    fn test_ai_embedding_extracts_numeric_vector_regression() {
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        let endpoint = "http://127.0.0.1:1/v1/embeddings";

        let result = handle(
            "ai_embedding",
            &[
                str_value("seed text"),
                ai_options_with_cassette(
                    endpoint,
                    "embed-replay",
                    "replay",
                    &ai_replay_fixture_dir(),
                ),
            ],
        )
        .expect("ai_embedding should return a value");

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
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();

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
        let _guard = AI_ENV_LOCK.lock().expect("AI env lock should not be poisoned");
        clear_ai_cassette_env();
        let endpoint = "http://127.0.0.1:1/v1/chat/completions";
        let result = handle(
            "ai_chat",
            &[
                str_value("Hello"),
                ai_options_with_cassette(endpoint, "gpt-mock", "replay", &ai_replay_fixture_dir()),
            ],
        )
        .expect("ai_chat should return a result");

        assert!(
            matches!(result, Value::Result { is_ok: false, value } if matches!(value.as_ref(), Value::Str(message) if message.contains("response was not valid JSON")))
        );
    }
}
