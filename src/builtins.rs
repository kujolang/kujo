// File: src/builtins.rs
//
// Built-in native functions for the Kujo standard library.
// These are implemented in Rust for performance and provide
// core functionality for math, strings, arrays, I/O operations, and JSON.

use crate::interpreter::{DictMap, Value};
use crate::network_policy;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Global seeded RNG for deterministic testing
/// When set, all random functions use this instead of thread_rng()
static SEEDED_RNG: Mutex<Option<StdRng>> = Mutex::new(None);

fn lock_seeded_rng() -> std::sync::MutexGuard<'static, Option<StdRng>> {
    SEEDED_RNG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Returns a HashMap of all built-in functions
pub fn get_builtins() -> HashMap<String, Value> {
    let mut builtins = HashMap::new();

    // Math constants
    builtins.insert("PI".to_string(), Value::Float(std::f64::consts::PI));
    builtins.insert("E".to_string(), Value::Float(std::f64::consts::E));

    builtins
}

/// Math functions
pub fn abs(x: f64) -> f64 {
    x.abs()
}

pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

pub fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

pub fn floor(x: f64) -> f64 {
    x.floor()
}

pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

pub fn round(x: f64) -> f64 {
    x.round()
}

pub fn min(a: f64, b: f64) -> f64 {
    a.min(b)
}

pub fn max(a: f64, b: f64) -> f64 {
    a.max(b)
}

pub fn sin(x: f64) -> f64 {
    x.sin()
}

pub fn cos(x: f64) -> f64 {
    x.cos()
}

pub fn tan(x: f64) -> f64 {
    x.tan()
}

pub fn log(x: f64) -> f64 {
    x.ln()
}

pub fn exp(x: f64) -> f64 {
    x.exp()
}

/// Random number functions
/// Set a seed for deterministic random number generation (for testing)
pub fn set_random_seed(seed: u64) {
    let rng = StdRng::seed_from_u64(seed);
    *lock_seeded_rng() = Some(rng);
}

/// Clear the random seed, returning to true randomness
pub fn clear_random_seed() {
    *lock_seeded_rng() = None;
}

/// Generate a random float between 0.0 and 1.0
pub fn random() -> f64 {
    let mut seeded = lock_seeded_rng();
    if let Some(ref mut rng) = *seeded {
        rng.gen::<f64>()
    } else {
        drop(seeded); // Release lock before using thread_rng
        let mut rng = rand::thread_rng();
        rng.gen::<f64>()
    }
}

/// Generate a random integer between min and max (inclusive)
pub fn random_int(min: f64, max: f64) -> f64 {
    let min_i = min as i64;
    let max_i = max as i64;

    let mut seeded = lock_seeded_rng();
    if let Some(ref mut rng) = *seeded {
        rng.gen_range(min_i..=max_i) as f64
    } else {
        drop(seeded); // Release lock before using thread_rng
        let mut rng = rand::thread_rng();
        rng.gen_range(min_i..=max_i) as f64
    }
}

/// Select a random element from an array
pub fn random_choice(arr: &[Value]) -> Value {
    if arr.is_empty() {
        return Value::Int(0);
    }

    let mut seeded = lock_seeded_rng();
    let idx = if let Some(ref mut rng) = *seeded {
        rng.gen_range(0..arr.len())
    } else {
        drop(seeded); // Release lock before using thread_rng
        let mut rng = rand::thread_rng();
        rng.gen_range(0..arr.len())
    };

    arr[idx].clone()
}

/// Generate a RFC 4122 version 4 UUID string.
pub fn uuid_v4() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a random lowercase alphanumeric identifier.
pub fn random_id(length: usize) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

    let make_char = |idx: usize| ALPHABET[idx] as char;
    let mut seeded = lock_seeded_rng();

    if let Some(ref mut rng) = *seeded {
        (0..length).map(|_| make_char(rng.gen_range(0..ALPHABET.len()))).collect()
    } else {
        drop(seeded);
        let mut rng = rand::thread_rng();
        (0..length).map(|_| make_char(rng.gen_range(0..ALPHABET.len()))).collect()
    }
}

/// Array generation functions
/// Generate a range of numbers
/// range(stop) - generate [0, 1, 2, ..., stop-1]
/// range(start, stop) - generate [start, start+1, ..., stop-1]
/// range(start, stop, step) - generate [start, start+step, start+2*step, ..., <stop]
pub fn range(args: &[Value]) -> Result<Vec<Value>, String> {
    match args.len() {
        1 => {
            // range(stop)
            let stop = match &args[0] {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => return Err("range() requires numeric arguments".to_string()),
            };

            if stop < 0 {
                return Ok(vec![]);
            }

            Ok((0..stop).map(Value::Int).collect())
        }
        2 => {
            // range(start, stop)
            let start = match &args[0] {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => return Err("range() requires numeric arguments".to_string()),
            };
            let stop = match &args[1] {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => return Err("range() requires numeric arguments".to_string()),
            };

            if start >= stop {
                return Ok(vec![]);
            }

            Ok((start..stop).map(Value::Int).collect())
        }
        3 => {
            // range(start, stop, step)
            let start = match &args[0] {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => return Err("range() requires numeric arguments".to_string()),
            };
            let stop = match &args[1] {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => return Err("range() requires numeric arguments".to_string()),
            };
            let step = match &args[2] {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => return Err("range() requires numeric arguments".to_string()),
            };

            if step == 0 {
                return Err("range() step cannot be zero".to_string());
            }

            let mut result = Vec::new();
            if step > 0 {
                let mut current = start;
                while current < stop {
                    result.push(Value::Int(current));
                    current += step;
                }
            } else {
                let mut current = start;
                while current > stop {
                    result.push(Value::Int(current));
                    current += step;
                }
            }

            Ok(result)
        }
        _ => Err("range() requires 1, 2, or 3 arguments".to_string()),
    }
}

/// String functions
pub fn str_len(s: &str) -> f64 {
    s.chars().count() as f64
}

pub fn substring(s: &str, start: f64, end: f64) -> String {
    let start_idx = start as usize;
    let end_idx = end as usize;
    let chars: Vec<char> = s.chars().collect();

    if start_idx >= chars.len() {
        return String::new();
    }

    let end_idx = end_idx.min(chars.len());
    chars[start_idx..end_idx].iter().collect()
}

pub fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

pub fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

pub fn trim_start(s: &str) -> String {
    s.trim_start().to_string()
}

pub fn trim_end(s: &str) -> String {
    s.trim_end().to_string()
}

pub fn contains(s: &str, substr: &str) -> bool {
    s.contains(substr)
}

pub fn replace(s: &str, old: &str, new: &str) -> String {
    s.replace(old, new)
}

pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

pub fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

pub fn index_of(s: &str, substr: &str) -> f64 {
    match s.find(substr) {
        Some(idx) => s[..idx].chars().count() as f64,
        None => -1.0,
    }
}

pub fn repeat(s: &str, count: f64) -> String {
    let count = count as usize;
    s.repeat(count)
}

pub fn char_at(s: &str, index: f64) -> String {
    let idx = index as usize;
    s.chars().nth(idx).map(|c| c.to_string()).unwrap_or_default()
}

pub fn is_empty(s: &str) -> bool {
    s.is_empty()
}

pub fn count_chars(s: &str) -> i64 {
    s.chars().count() as i64
}

pub fn split(s: &str, delimiter: &str) -> Vec<String> {
    s.split(delimiter).map(|s| s.to_string()).collect()
}

pub fn join(arr: &[String], separator: &str) -> String {
    arr.join(separator)
}

/// Advanced string methods
/// Pad string on the left with specified character to reach target width
pub fn str_pad_left(s: &str, width: i64, pad_char: &str) -> String {
    let pad_char = pad_char.chars().next().unwrap_or(' ');
    let current_len = s.chars().count();
    let target_width = width as usize;

    if current_len >= target_width {
        return s.to_string();
    }

    let pad_count = target_width - current_len;
    format!("{}{}", pad_char.to_string().repeat(pad_count), s)
}

/// Pad string on the right with specified character to reach target width
pub fn str_pad_right(s: &str, width: i64, pad_char: &str) -> String {
    let pad_char = pad_char.chars().next().unwrap_or(' ');
    let current_len = s.chars().count();
    let target_width = width as usize;

    if current_len >= target_width {
        return s.to_string();
    }

    let pad_count = target_width - current_len;
    format!("{}{}", s, pad_char.to_string().repeat(pad_count))
}

/// Split string into lines (handles \n, \r\n, and \r)
pub fn str_lines(s: &str) -> Vec<String> {
    s.lines().map(|line| line.to_string()).collect()
}

/// Split string into words (splits on whitespace)
pub fn str_words(s: &str) -> Vec<String> {
    s.split_whitespace().map(|word| word.to_string()).collect()
}

/// Reverse a string
pub fn str_reverse(s: &str) -> String {
    s.chars().rev().collect()
}

/// Convert string to URL-friendly slug
pub fn str_slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '_' {
                '-'
            } else {
                ' ' // Will be filtered out
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
        .replace("--", "-")
        .trim_matches('-')
        .to_string()
}

/// Truncate string to specified length with optional suffix
pub fn str_truncate(s: &str, max_len: i64, suffix: &str) -> String {
    let len = s.chars().count();
    let max_len = max_len as usize;

    if len <= max_len {
        return s.to_string();
    }

    let suffix_len = suffix.chars().count();
    let truncate_at = max_len.saturating_sub(suffix_len);

    let truncated: String = s.chars().take(truncate_at).collect();
    format!("{}{}", truncated, suffix)
}

/// Convert string to camelCase
pub fn str_to_camel_case(s: &str) -> String {
    let words: Vec<&str> =
        s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();

    if words.is_empty() {
        return String::new();
    }

    let mut result = words[0].to_lowercase();
    for word in &words[1..] {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.push_str(&first.to_uppercase().to_string());
            result.push_str(&chars.as_str().to_lowercase());
        }
    }

    result
}

/// Convert string to snake_case
pub fn str_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_upper = false;

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_was_upper {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_was_upper = true;
        } else if c.is_alphanumeric() {
            result.push(c);
            prev_was_upper = false;
        } else if !c.is_whitespace() && c != '_' && c != '-' {
            // Skip non-alphanumeric characters except underscore and hyphen
            prev_was_upper = false;
        } else {
            if i > 0 && !result.ends_with('_') {
                result.push('_');
            }
            prev_was_upper = false;
        }
    }

    result.trim_matches('_').to_string()
}

/// Convert string to kebab-case
pub fn str_to_kebab_case(s: &str) -> String {
    str_to_snake_case(s).replace('_', "-")
}

/// String formatting function
/// Format a string with sprintf-style placeholders
/// Supports: %s (string), %d (integer), %f (float)
pub fn format_string(template: &str, args: &[Value]) -> Result<String, String> {
    let mut result = String::new();
    let mut arg_index = 0;
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '%' {
                    // Escaped %%
                    result.push('%');
                    chars.next();
                } else if next_ch == 's' || next_ch == 'd' || next_ch == 'f' {
                    chars.next();

                    if arg_index >= args.len() {
                        return Err(format!(
                            "format() missing argument for placeholder %{}",
                            next_ch
                        ));
                    }

                    let formatted = match next_ch {
                        's' => {
                            // %s - string
                            match &args[arg_index] {
                                Value::Str(s) => s.as_ref().clone(),
                                Value::Int(n) => n.to_string(),
                                Value::Float(f) => f.to_string(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "null".to_string(),
                                Value::Array(_) => "[Array]".to_string(),
                                Value::Dict(_) => "{Dict}".to_string(),
                                _ => format!("{:?}", args[arg_index]),
                            }
                        }
                        'd' => {
                            // %d - integer
                            match &args[arg_index] {
                                Value::Int(n) => n.to_string(),
                                Value::Float(f) => (*f as i64).to_string(),
                                Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                                _ => {
                                    return Err(format!(
                                        "format() %d requires numeric argument, got {:?}",
                                        args[arg_index]
                                    ))
                                }
                            }
                        }
                        'f' => {
                            // %f - float
                            match &args[arg_index] {
                                Value::Float(f) => f.to_string(),
                                Value::Int(n) => (*n as f64).to_string(),
                                _ => {
                                    return Err(format!(
                                        "format() %f requires numeric argument, got {:?}",
                                        args[arg_index]
                                    ))
                                }
                            }
                        }
                        _ => unreachable!(),
                    };

                    result.push_str(&formatted);
                    arg_index += 1;
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// JSON functions
const MAX_JSON_INPUT_BYTES: usize = 1_048_576;
const MAX_JSON_NESTING_DEPTH: usize = 64;

/// Parse a JSON string into a Kujo value
/// Infrastructure for json.parse() builtin
#[allow(dead_code)]
pub fn parse_json(json_str: &str) -> Result<Value, String> {
    if json_str.len() > MAX_JSON_INPUT_BYTES {
        return Err(format!(
            "JSON input exceeds maximum input size of {} bytes",
            MAX_JSON_INPUT_BYTES
        ));
    }

    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(json_value) => {
            validate_json_nesting_depth(&json_value, MAX_JSON_NESTING_DEPTH)?;
            Ok(json_to_kujo_value(json_value))
        }
        Err(e) => Err(format!("JSON parse error: {}", e)),
    }
}

/// Convert a Kujo value to a JSON string
/// Infrastructure for json.stringify() builtin
#[allow(dead_code)]
pub fn to_json(value: &Value) -> Result<String, String> {
    let json_value = kujo_value_to_json(value)?;
    match serde_json::to_string(&json_value) {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("JSON serialization error: {}", e)),
    }
}

/// Convert a Kujo value to a pretty-printed JSON string
#[allow(dead_code)]
pub fn to_json_pretty(value: &Value) -> Result<String, String> {
    let json_value = kujo_value_to_json(value)?;
    match serde_json::to_string_pretty(&json_value) {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("JSON serialization error: {}", e)),
    }
}

/// Convert serde_json::Value to Kujo Value
fn json_to_kujo_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null, // null -> Null
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            // Preserve integer vs float distinction
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                if let Ok(i) = i64::try_from(u) {
                    Value::Int(i)
                } else {
                    Value::Float(u as f64)
                }
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::Str(Arc::new(s)),
        serde_json::Value::Array(arr) => {
            let kujo_arr: Vec<Value> = arr.into_iter().map(json_to_kujo_value).collect();
            Value::Array(Arc::new(kujo_arr))
        }
        serde_json::Value::Object(obj) => {
            let mut kujo_dict = DictMap::default();
            for (key, val) in obj {
                kujo_dict.insert(key.into(), json_to_kujo_value(val));
            }
            Value::Dict(Arc::new(kujo_dict))
        }
    }
}

/// Convert Kujo Value to serde_json::Value
fn kujo_value_to_json(value: &Value) -> Result<serde_json::Value, String> {
    kujo_value_to_json_with_depth(value, 0)
}

fn kujo_value_to_json_with_depth(value: &Value, depth: usize) -> Result<serde_json::Value, String> {
    if depth > MAX_JSON_NESTING_DEPTH {
        return Err(format!(
            "JSON value exceeds maximum nesting depth of {}",
            MAX_JSON_NESTING_DEPTH
        ));
    }

    match value {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Int(n) => Ok(serde_json::Value::Number(serde_json::Number::from(*n))),
        Value::Float(n) => {
            if !n.is_finite() {
                return Err(format!("Cannot convert non-finite float {} to JSON", n));
            }

            let number = serde_json::Number::from_f64(*n)
                .ok_or_else(|| format!("Cannot convert non-finite float {} to JSON", n))?;
            Ok(serde_json::Value::Number(number))
        }
        Value::Str(s) => Ok(serde_json::Value::String(s.as_ref().clone())),
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Array(arr) => {
            let mut json_arr = Vec::new();
            for item in arr.iter() {
                json_arr.push(kujo_value_to_json_with_depth(item, depth + 1)?);
            }
            Ok(serde_json::Value::Array(json_arr))
        }
        Value::Dict(dict) => {
            let mut json_obj = serde_json::Map::new();
            let mut entries: Vec<(&Arc<str>, &Value)> = dict.iter().collect();
            entries.sort_by(|(left, _), (right, _)| left.as_ref().cmp(right.as_ref()));
            for (key, val) in entries {
                json_obj.insert(key.to_string(), kujo_value_to_json_with_depth(val, depth + 1)?);
            }
            Ok(serde_json::Value::Object(json_obj))
        }
        Value::FixedDict { keys, values } => {
            let mut json_obj = serde_json::Map::new();
            for (key, val) in keys.iter().zip(values.iter()) {
                json_obj.insert(key.to_string(), kujo_value_to_json_with_depth(val, depth + 1)?);
            }
            Ok(serde_json::Value::Object(json_obj))
        }
        Value::IntDict(dict) => {
            let mut json_obj = serde_json::Map::new();
            let mut entries: Vec<(&i64, &Value)> = dict.iter().collect();
            entries.sort_by_key(|(key, _)| **key);
            for (key, val) in entries {
                json_obj.insert(key.to_string(), kujo_value_to_json_with_depth(val, depth + 1)?);
            }
            Ok(serde_json::Value::Object(json_obj))
        }
        Value::DenseIntDict(values) => {
            let mut json_obj = serde_json::Map::new();
            for (index, val) in values.iter().enumerate() {
                json_obj.insert(index.to_string(), kujo_value_to_json_with_depth(val, depth + 1)?);
            }
            Ok(serde_json::Value::Object(json_obj))
        }
        _ => Err(format!("Cannot convert {:?} to JSON", value)),
    }
}

fn validate_json_nesting_depth(root: &serde_json::Value, max_depth: usize) -> Result<(), String> {
    let mut stack: Vec<(&serde_json::Value, usize)> = vec![(root, 0)];

    while let Some((value, depth)) = stack.pop() {
        if depth > max_depth {
            return Err(format!("JSON input exceeds maximum nesting depth of {}", max_depth));
        }

        match value {
            serde_json::Value::Array(items) => {
                for item in items {
                    stack.push((item, depth + 1));
                }
            }
            serde_json::Value::Object(entries) => {
                for nested in entries.values() {
                    stack.push((nested, depth + 1));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// TOML functions
/// Parse a TOML string into a Kujo value
/// Infrastructure for toml.parse() builtin
#[allow(dead_code)]
pub fn parse_toml(toml_str: &str) -> Result<Value, String> {
    match toml::from_str::<toml::Value>(toml_str) {
        Ok(toml_value) => Ok(toml_to_kujo_value(toml_value)),
        Err(e) => Err(format!("TOML parse error: {}", e)),
    }
}

/// Convert a Kujo value to a TOML string
/// Infrastructure for toml.stringify() builtin
#[allow(dead_code)]
pub fn to_toml(value: &Value) -> Result<String, String> {
    let toml_value = kujo_value_to_toml(value)?;
    match toml::to_string(&toml_value) {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("TOML serialization error: {}", e)),
    }
}

/// Convert toml::Value to Kujo Value
#[allow(dead_code)]
fn toml_to_kujo_value(toml: toml::Value) -> Value {
    match toml {
        toml::Value::String(s) => Value::Str(Arc::new(s)),
        toml::Value::Integer(i) => Value::Int(i),
        toml::Value::Float(f) => Value::Float(f),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(dt) => Value::Str(Arc::new(dt.to_string())),
        toml::Value::Array(arr) => {
            let kujo_arr: Vec<Value> = arr.into_iter().map(toml_to_kujo_value).collect();
            Value::Array(Arc::new(kujo_arr))
        }
        toml::Value::Table(table) => {
            let mut kujo_dict = DictMap::default();
            for (key, val) in table {
                kujo_dict.insert(key.into(), toml_to_kujo_value(val));
            }
            Value::Dict(Arc::new(kujo_dict))
        }
    }
}

/// Convert Kujo Value to toml::Value
#[allow(dead_code)]
fn kujo_value_to_toml(value: &Value) -> Result<toml::Value, String> {
    match value {
        Value::Null => Ok(toml::Value::String(String::new())), // TOML doesn't have null, use empty string
        Value::Int(n) => Ok(toml::Value::Integer(*n)),
        Value::Float(n) => Ok(toml::Value::Float(*n)),
        Value::Str(s) => Ok(toml::Value::String(s.as_ref().clone())),
        Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        Value::Array(arr) => {
            let mut toml_arr = Vec::new();
            for item in arr.iter() {
                toml_arr.push(kujo_value_to_toml(item)?);
            }
            Ok(toml::Value::Array(toml_arr))
        }
        Value::Dict(dict) => {
            let mut toml_table = toml::map::Map::new();
            for (key, val) in dict.iter() {
                toml_table.insert(key.to_string(), kujo_value_to_toml(val)?);
            }
            Ok(toml::Value::Table(toml_table))
        }
        Value::FixedDict { keys, values } => {
            let mut toml_table = toml::map::Map::new();
            for (key, val) in keys.iter().zip(values.iter()) {
                toml_table.insert(key.to_string(), kujo_value_to_toml(val)?);
            }
            Ok(toml::Value::Table(toml_table))
        }
        Value::IntDict(dict) => {
            let mut toml_table = toml::map::Map::new();
            for (key, val) in dict.iter() {
                toml_table.insert(key.to_string(), kujo_value_to_toml(val)?);
            }
            Ok(toml::Value::Table(toml_table))
        }
        Value::DenseIntDict(values) => {
            let mut toml_table = toml::map::Map::new();
            for (index, val) in values.iter().enumerate() {
                toml_table.insert(index.to_string(), kujo_value_to_toml(val)?);
            }
            Ok(toml::Value::Table(toml_table))
        }
        _ => Err(format!("Cannot convert {:?} to TOML", value)),
    }
}

/// YAML functions
/// Parse a YAML string into a Kujo value
/// Infrastructure for yaml.parse() builtin
#[allow(dead_code)]
pub fn parse_yaml(yaml_str: &str) -> Result<Value, String> {
    match serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
        Ok(yaml_value) => Ok(yaml_to_kujo_value(yaml_value)),
        Err(e) => Err(format!("YAML parse error: {}", e)),
    }
}

/// Convert a Kujo value to a YAML string
/// Infrastructure for yaml.stringify() builtin
#[allow(dead_code)]
pub fn to_yaml(value: &Value) -> Result<String, String> {
    let yaml_value = kujo_value_to_yaml(value)?;
    match serde_yaml::to_string(&yaml_value) {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("YAML serialization error: {}", e)),
    }
}

/// Convert serde_yaml::Value to Kujo Value
#[allow(dead_code)]
fn yaml_to_kujo_value(yaml: serde_yaml::Value) -> Value {
    match yaml {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            // Preserve integer vs float distinction
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Int(0)
            }
        }
        serde_yaml::Value::String(s) => Value::Str(Arc::new(s)),
        serde_yaml::Value::Sequence(arr) => {
            let kujo_arr: Vec<Value> = arr.into_iter().map(yaml_to_kujo_value).collect();
            Value::Array(Arc::new(kujo_arr))
        }
        serde_yaml::Value::Mapping(map) => {
            let mut kujo_dict = DictMap::default();
            for (key, val) in map {
                if let serde_yaml::Value::String(key_str) = key {
                    kujo_dict.insert(key_str.into(), yaml_to_kujo_value(val));
                } else {
                    // Convert non-string keys to strings
                    let key_str = format!("{:?}", key);
                    kujo_dict.insert(key_str.into(), yaml_to_kujo_value(val));
                }
            }
            Value::Dict(Arc::new(kujo_dict))
        }
        serde_yaml::Value::Tagged(tagged) => {
            // Handle tagged values by converting the value itself
            yaml_to_kujo_value(tagged.value)
        }
    }
}

/// Convert Kujo Value to serde_yaml::Value
#[allow(dead_code)]
fn kujo_value_to_yaml(value: &Value) -> Result<serde_yaml::Value, String> {
    match value {
        Value::Null => Ok(serde_yaml::Value::Null),
        Value::Int(n) => Ok(serde_yaml::Value::Number((*n).into())),
        Value::Float(n) => Ok(serde_yaml::Value::Number(serde_yaml::Number::from(*n))),
        Value::Str(s) => Ok(serde_yaml::Value::String(s.as_ref().clone())),
        Value::Bool(b) => Ok(serde_yaml::Value::Bool(*b)),
        Value::Array(arr) => {
            let mut yaml_arr = Vec::new();
            for item in arr.iter() {
                yaml_arr.push(kujo_value_to_yaml(item)?);
            }
            Ok(serde_yaml::Value::Sequence(yaml_arr))
        }
        Value::Dict(dict) => {
            let mut yaml_map = serde_yaml::Mapping::new();
            for (key, val) in dict.iter() {
                yaml_map
                    .insert(serde_yaml::Value::String(key.to_string()), kujo_value_to_yaml(val)?);
            }
            Ok(serde_yaml::Value::Mapping(yaml_map))
        }
        Value::FixedDict { keys, values } => {
            let mut yaml_map = serde_yaml::Mapping::new();
            for (key, val) in keys.iter().zip(values.iter()) {
                yaml_map
                    .insert(serde_yaml::Value::String(key.to_string()), kujo_value_to_yaml(val)?);
            }
            Ok(serde_yaml::Value::Mapping(yaml_map))
        }
        Value::IntDict(dict) => {
            let mut yaml_map = serde_yaml::Mapping::new();
            for (key, val) in dict.iter() {
                yaml_map
                    .insert(serde_yaml::Value::String(key.to_string()), kujo_value_to_yaml(val)?);
            }
            Ok(serde_yaml::Value::Mapping(yaml_map))
        }
        Value::DenseIntDict(values) => {
            let mut yaml_map = serde_yaml::Mapping::new();
            for (index, val) in values.iter().enumerate() {
                yaml_map
                    .insert(serde_yaml::Value::String(index.to_string()), kujo_value_to_yaml(val)?);
            }
            Ok(serde_yaml::Value::Mapping(yaml_map))
        }
        _ => Err(format!("Cannot convert {:?} to YAML", value)),
    }
}

/// CSV functions
/// Parse a CSV string into a Kujo array of dictionaries
/// Each row becomes a dictionary with column headers as keys
/// Infrastructure for csv.parse() builtin
#[allow(dead_code)]
pub fn parse_csv(csv_str: &str) -> Result<Value, String> {
    let mut reader = csv::Reader::from_reader(csv_str.as_bytes());

    // Get headers
    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(e) => return Err(format!("CSV header error: {}", e)),
    };

    let mut rows = Vec::new();

    for result in reader.records() {
        match result {
            Ok(record) => {
                let mut row_dict = DictMap::default();
                for (i, field) in record.iter().enumerate() {
                    let header = headers.get(i).unwrap_or("unknown");
                    // Try to parse as number, otherwise keep as string
                    let value = if let Ok(num) = field.parse::<i64>() {
                        Value::Int(num)
                    } else if let Ok(num) = field.parse::<f64>() {
                        Value::Float(num)
                    } else {
                        Value::Str(Arc::new(field.to_string()))
                    };
                    row_dict.insert(header.to_string().into(), value);
                }
                rows.push(Value::Dict(Arc::new(row_dict)));
            }
            Err(e) => return Err(format!("CSV parse error: {}", e)),
        }
    }

    Ok(Value::Array(Arc::new(rows)))
}

/// Convert a Kujo array of dictionaries to a CSV string
/// Infrastructure for csv.stringify() builtin
#[allow(dead_code)]
pub fn to_csv(value: &Value) -> Result<String, String> {
    match value {
        Value::Array(rows) if !rows.is_empty() => {
            let mut wtr = csv::Writer::from_writer(vec![]);

            // Get headers from first row
            if let Some(Value::Dict(first_row)) = rows.first() {
                let headers: Vec<String> = first_row.keys().map(|key| key.to_string()).collect();

                if let Err(e) = wtr.write_record(&headers) {
                    return Err(format!("CSV write error: {}", e));
                }

                // Write each row
                for row_val in rows.iter() {
                    if let Value::Dict(row) = row_val {
                        let mut record = Vec::new();
                        for header in &headers {
                            let value_str = match row.get(header.as_str()) {
                                Some(Value::Int(n)) => n.to_string(),
                                Some(Value::Float(n)) => n.to_string(),
                                Some(Value::Str(s)) => s.as_ref().clone(),
                                Some(Value::Bool(b)) => b.to_string(),
                                Some(Value::Null) => String::new(),
                                _ => String::new(),
                            };
                            record.push(value_str);
                        }
                        if let Err(e) = wtr.write_record(&record) {
                            return Err(format!("CSV write error: {}", e));
                        }
                    } else {
                        return Err("CSV requires array of dictionaries".to_string());
                    }
                }

                match wtr.into_inner() {
                    Ok(bytes) => {
                        String::from_utf8(bytes).map_err(|e| format!("CSV encoding error: {}", e))
                    }
                    Err(e) => Err(format!("CSV write error: {}", e)),
                }
            } else {
                Err("CSV requires array of dictionaries".to_string())
            }
        }
        Value::Array(_) => Err("CSV requires non-empty array".to_string()),
        _ => Err("CSV requires array of dictionaries".to_string()),
    }
}

/// Date/Time functions
/// Get current Unix timestamp (seconds since epoch)
pub fn now() -> f64 {
    Utc::now().timestamp() as f64
}

/// Get current UTC time as an ISO-8601 UTC timestamp.
pub fn now_utc() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn safe_duration_since_unix_epoch(now: SystemTime) -> Duration {
    now.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO)
}

/// Get current Unix timestamp in seconds.
pub fn now_unix() -> i64 {
    safe_duration_since_unix_epoch(SystemTime::now()).as_secs() as i64
}

/// Get current timestamp in milliseconds since UNIX epoch
/// Returns the number of milliseconds elapsed since January 1, 1970 00:00:00 UTC
/// This is useful for timestamps and timing operations
pub fn current_timestamp() -> i64 {
    safe_duration_since_unix_epoch(SystemTime::now()).as_millis() as i64
}

/// High-resolution performance timer in milliseconds
/// Returns elapsed time in milliseconds since an arbitrary point in time
/// This is ideal for measuring performance and elapsed time between operations
/// Note: The starting point is arbitrary and consistent within the process lifetime
pub fn performance_now() -> f64 {
    // Use a static Instant that's initialized once for consistent measurements
    // This ensures performance_now() returns milliseconds since program start
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);

    start.elapsed().as_secs_f64() * 1000.0
}

/// High-resolution timer in microseconds since program start
/// Returns elapsed time with microsecond precision (1/1,000,000 second)
/// Ideal for measuring very fast operations and detailed performance analysis
pub fn time_us() -> f64 {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);

    start.elapsed().as_micros() as f64
}

/// High-resolution timer in nanoseconds since program start
/// Returns elapsed time with nanosecond precision (1/1,000,000,000 second)
/// Highest precision available - ideal for CPU-level performance analysis
pub fn time_ns() -> f64 {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);

    start.elapsed().as_nanos() as f64
}

/// Format a duration in milliseconds to a human-readable string
/// Automatically chooses the best unit (s, ms, μs, ns)
/// Examples: "1.23s", "456.78ms", "123.45μs", "789ns"
pub fn format_duration(ms: f64) -> String {
    if ms >= 1000.0 {
        // Format as seconds
        format!("{:.2}s", ms / 1000.0)
    } else if ms >= 1.0 {
        // Format as milliseconds
        format!("{:.2}ms", ms)
    } else if ms >= 0.001 {
        // Format as microseconds
        format!("{:.2}μs", ms * 1000.0)
    } else {
        // Format as nanoseconds
        format!("{:.0}ns", ms * 1_000_000.0)
    }
}

/// Calculate elapsed time between two timestamps
/// Returns the difference in milliseconds
pub fn elapsed(start: f64, end: f64) -> f64 {
    end - start
}

/// Format a Unix timestamp to a date string
/// Supports basic format: "YYYY-MM-DD HH:mm:ss"
pub fn format_date(timestamp: f64, format_str: &str) -> String {
    if !timestamp.is_finite() {
        return "Invalid timestamp: value must be a finite number".to_string();
    }
    if timestamp < i64::MIN as f64 || timestamp > i64::MAX as f64 {
        return "Invalid timestamp: value is out of supported range".to_string();
    }

    let Some(dt) = Utc.timestamp_opt(timestamp as i64, 0).single() else {
        return "Invalid timestamp: value is out of supported range".to_string();
    };

    // Simple format string replacement
    let result = format_str
        .replace("YYYY", &dt.format("%Y").to_string())
        .replace("MM", &dt.format("%m").to_string())
        .replace("DD", &dt.format("%d").to_string())
        .replace("HH", &dt.format("%H").to_string())
        .replace("mm", &dt.format("%M").to_string())
        .replace("ss", &dt.format("%S").to_string());

    result
}

/// Parse a date string to Unix timestamp
/// Supports format: "YYYY-MM-DD"
pub fn parse_date(date_str: &str, format: &str) -> Result<f64, String> {
    if "YYYY-MM-DD" != format {
        return Err(format!(
            "parse_date failed: unsupported format '{}' (expected 'YYYY-MM-DD')",
            format
        ));
    }

    let parsed_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|error| {
        format!("parse_date failed: '{}' does not match format 'YYYY-MM-DD' ({})", date_str, error)
    })?;

    let naive_datetime = parsed_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| format!("parse_date failed: '{}' is out of range", date_str))?;
    let dt: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

    Ok(dt.timestamp() as f64)
}

fn kv_store_path() -> Result<PathBuf, String> {
    if let Ok(path) = env::var("KUJO_KV_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    let cwd = env::current_dir().map_err(|error| {
        format!("Failed to determine current directory for kv store: {}", error)
    })?;
    Ok(cwd.join(".kujo_kv.json"))
}

fn load_kv_store(path: &Path) -> Result<HashMap<String, String>, String> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read kv store '{}': {}", path.display(), error))?;

    if contents.trim().is_empty() {
        return Ok(HashMap::new());
    }

    serde_json::from_str::<HashMap<String, String>>(&contents)
        .map_err(|error| format!("Failed to parse kv store '{}': {}", path.display(), error))
}

fn save_kv_store(path: &Path, data: &HashMap<String, String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("Failed to prepare kv store directory '{}': {}", parent.display(), error)
        })?;
    }

    let encoded = serde_json::to_string_pretty(data)
        .map_err(|error| format!("Failed to encode kv store: {}", error))?;
    fs::write(path, encoded)
        .map_err(|error| format!("Failed to write kv store '{}': {}", path.display(), error))
}

/// Persist a key/value pair in a lightweight local JSON store.
pub fn kv_set(key: &str, value: &str) -> Result<bool, String> {
    let path = kv_store_path()?;
    let mut data = load_kv_store(&path)?;
    data.insert(key.to_string(), value.to_string());
    save_kv_store(&path, &data)?;
    Ok(true)
}

/// Retrieve a value by key from the lightweight local JSON store.
pub fn kv_get(key: &str) -> Result<Option<String>, String> {
    let path = kv_store_path()?;
    let data = load_kv_store(&path)?;
    Ok(data.get(key).cloned())
}

/// System operation functions
/// Get environment variable value
/// Infrastructure for env.get() builtin
#[allow(dead_code)]
pub fn get_env(var_name: &str) -> String {
    env::var(var_name).unwrap_or_default()
}

/// Get environment variable or return default value if not set
/// Infrastructure for env.getOr() builtin
#[allow(dead_code)]
pub fn env_or(var_name: &str, default: &str) -> String {
    env::var(var_name).unwrap_or_else(|_| default.to_string())
}

/// Get environment variable and parse as integer
/// Infrastructure for env.getInt() builtin
#[allow(dead_code)]
pub fn env_int(var_name: &str) -> Result<i64, String> {
    match env::var(var_name) {
        Ok(val) => val.parse::<i64>().map_err(|_| {
            format!("Environment variable '{}' value '{}' is not a valid integer", var_name, val)
        }),
        Err(_) => Err(format!("Environment variable '{}' not found", var_name)),
    }
}

/// Get environment variable and parse as float
/// Infrastructure for env.getFloat() builtin
#[allow(dead_code)]
pub fn env_float(var_name: &str) -> Result<f64, String> {
    match env::var(var_name) {
        Ok(val) => val.parse::<f64>().map_err(|_| {
            format!("Environment variable '{}' value '{}' is not a valid float", var_name, val)
        }),
        Err(_) => Err(format!("Environment variable '{}' not found", var_name)),
    }
}

/// Get environment variable and parse as boolean
/// Accepts: "true", "1", "yes", "on" (case insensitive) as true
/// Infrastructure for env.getBool() builtin
#[allow(dead_code)]
pub fn env_bool(var_name: &str) -> Result<bool, String> {
    match env::var(var_name) {
        Ok(val) => {
            let val_lower = val.to_lowercase();
            match val_lower.as_str() {
                "true" | "1" | "yes" | "on" => Ok(true),
                "false" | "0" | "no" | "off" => Ok(false),
                _ => Err(format!(
                    "Environment variable '{}' value '{}' is not a valid bool",
                    var_name, val
                )),
            }
        }
        Err(_) => Err(format!("Environment variable '{}' not found", var_name)),
    }
}

/// Get required environment variable or error if not set
/// Infrastructure for env.require() builtin
#[allow(dead_code)]
pub fn env_required(var_name: &str) -> Result<String, String> {
    env::var(var_name)
        .map_err(|_| format!("Required environment variable '{}' is not set", var_name))
}

/// Set environment variable
/// Infrastructure for env.set() builtin
#[allow(dead_code)]
pub fn env_set(var_name: &str, value: &str) {
    env::set_var(var_name, value);
}

/// Get all environment variables as a HashMap
/// Infrastructure for env.list() builtin
#[allow(dead_code)]
pub fn env_list() -> HashMap<String, String> {
    env::vars().collect()
}

/// Get command-line arguments
pub fn get_args() -> Vec<String> {
    // Check if arguments were explicitly set via KUJO_SCRIPT_ARGS environment variable
    // This is used when arguments are passed via clap's trailing_var_arg
    if let Ok(args_str) = env::var("KUJO_SCRIPT_ARGS") {
        return args_str.split('\x1f').map(String::from).collect();
    }

    let all_args: Vec<String> = env::args().collect();

    // Filter out the kujo executable, subcommand, and script file
    // Example: ["kujo", "run", "script.kujo", "arg1", "arg2"] -> ["arg1", "arg2"]
    // Example: ["kujo", "script.kujo", "arg1"] -> ["arg1"]

    // Skip executable name
    let mut iter = all_args.iter().skip(1);

    // Check for subcommand (run, repl, etc.)
    if let Some(first) = iter.next() {
        if first == "run" || first == "check" || first == "format" {
            // Skip subcommand and script file
            iter.skip(1).cloned().collect()
        } else if first.ends_with(".kujo") {
            // No subcommand, just script file
            iter.cloned().collect()
        } else {
            // Return all remaining args
            std::iter::once(first.clone()).chain(iter.cloned()).collect()
        }
    } else {
        Vec::new()
    }
}

/// Sleep for specified milliseconds
/// Infrastructure for sleep() builtin
#[allow(dead_code)]
pub fn sleep_ms(ms: f64) {
    thread::sleep(Duration::from_millis(ms as u64));
}

fn validate_shell_command_text(command: &str) -> Result<(), String> {
    if command.trim().is_empty() {
        return Err(
            "command must not be empty; use spawn_process([...]) for structured argv execution"
                .to_string(),
        );
    }
    if command.contains('\0') {
        return Err("command contains NUL byte".to_string());
    }
    if command.contains('\n') || command.contains('\r') {
        return Err(
            "command contains newline; use spawn_process([...]) for structured argv execution"
                .to_string(),
        );
    }
    Ok(())
}

/// Execute a shell command and return output
/// Infrastructure for exec() builtin
#[allow(dead_code)]
pub fn execute_command(command: &str) -> String {
    if let Err(error) = validate_shell_command_text(command) {
        return format!("Error: {}", error);
    }

    if cfg!(target_os = "windows") {
        match Command::new("cmd").args(["/C", command]).output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(e) => format!("Error: {}", e),
        }
    } else {
        match Command::new("sh").arg("-c").arg(command).output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(e) => format!("Error: {}", e),
        }
    }
}

/// Path operation functions
/// Join path components
/// Infrastructure for path.join() builtin
#[allow(dead_code)]
pub fn join_path(parts: &[String]) -> String {
    let path: PathBuf = parts.iter().collect();
    path.to_string_lossy().to_string()
}

/// Get directory name from path
/// Infrastructure for path.dirname() builtin
#[allow(dead_code)]
pub fn dirname(path_str: &str) -> String {
    let path = Path::new(path_str);
    path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| String::from("/"))
}

/// Get base filename from path
/// Infrastructure for path.basename() builtin
#[allow(dead_code)]
pub fn basename(path_str: &str) -> String {
    let path = Path::new(path_str);
    path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()
}

/// Check if path exists
/// Infrastructure for path.exists() builtin
#[allow(dead_code)]
pub fn path_exists(path_str: &str) -> bool {
    Path::new(path_str).exists()
}

/// Regular expression functions
/// Check if string matches regex pattern
/// Infrastructure for regex.match() builtin
#[allow(dead_code)]
pub fn regex_match(text: &str, pattern: &str) -> bool {
    match Regex::new(pattern) {
        Ok(re) => re.is_match(text),
        Err(_) => false, // Invalid regex returns false
    }
}

/// Find all matches of regex pattern in text
/// Infrastructure for regex.findAll() builtin
#[allow(dead_code)]
pub fn regex_find_all(text: &str, pattern: &str) -> Vec<String> {
    match Regex::new(pattern) {
        Ok(re) => re.find_iter(text).map(|m| m.as_str().to_string()).collect(),
        Err(_) => vec![], // Invalid regex returns empty array
    }
}

/// Replace all matches of regex pattern with replacement string
/// Infrastructure for regex.replace() builtin
#[allow(dead_code)]
pub fn regex_replace(text: &str, pattern: &str, replacement: &str) -> String {
    match Regex::new(pattern) {
        Ok(re) => re.replace_all(text, replacement).to_string(),
        Err(_) => text.to_string(), // Invalid regex returns original text
    }
}

/// Split string by regex pattern
/// Infrastructure for regex.split() builtin
#[allow(dead_code)]
pub fn regex_split(text: &str, pattern: &str) -> Vec<String> {
    match Regex::new(pattern) {
        Ok(re) => re.split(text).map(|s| s.to_string()).collect(),
        Err(_) => vec![text.to_string()], // Invalid regex returns original text as single element
    }
}

/// Array functions
/// Insert an item at a specific index
pub fn array_insert(arr: Vec<Value>, index: i64, item: Value) -> Result<Vec<Value>, Arc<String>> {
    let idx = index as usize;
    let mut new_arr = arr;

    if idx > new_arr.len() {
        return Err(Arc::new(format!(
            "insert() index {} out of bounds for array of length {}",
            idx,
            new_arr.len()
        )));
    }

    new_arr.insert(idx, item);
    Ok(new_arr)
}

/// Remove the first occurrence of an item
pub fn array_remove(arr: Vec<Value>, item: &Value) -> Vec<Value> {
    let mut new_arr = arr;
    // Find position manually since Value doesn't implement PartialEq
    let pos = new_arr.iter().position(|x| match (x, item) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a.as_ref() == b.as_ref(),
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => false,
    });

    if let Some(idx) = pos {
        new_arr.remove(idx);
    }
    new_arr
}

/// Remove an item at a specific index
pub fn array_remove_at(arr: Vec<Value>, index: i64) -> Result<(Vec<Value>, Value), Arc<String>> {
    let idx = index as usize;
    let mut new_arr = arr;

    if idx >= new_arr.len() {
        return Err(Arc::new(format!(
            "remove_at() index {} out of bounds for array of length {}",
            idx,
            new_arr.len()
        )));
    }

    let removed = new_arr.remove(idx);
    Ok((new_arr, removed))
}

/// Clear all items from an array
pub fn array_clear() -> Vec<Value> {
    Vec::new()
}

/// Find the index of the first occurrence of an item
pub fn array_index_of(arr: &[Value], item: &Value) -> i64 {
    let pos = arr.iter().position(|x| match (x, item) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => false,
    });

    pos.map(|i| i as i64).unwrap_or(-1)
}

/// Check if an array contains an item
pub fn array_contains(arr: &[Value], item: &Value) -> bool {
    arr.iter().any(|x| match (x, item) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => false,
    })
}

/// Advanced array methods
/// Split array into chunks of specified size
/// Last chunk may be smaller if array length is not divisible by chunk_size
pub fn array_chunk(arr: &[Value], chunk_size: i64) -> Vec<Value> {
    if chunk_size <= 0 {
        return vec![Value::Array(Arc::new(arr.to_vec()))];
    }

    let size = chunk_size as usize;
    let chunks: Vec<Value> =
        arr.chunks(size).map(|chunk| Value::Array(Arc::new(chunk.to_vec()))).collect();

    chunks
}

/// Flatten nested arrays by one level
/// [[1,2], [3,4]] → [1,2,3,4]
pub fn array_flatten(arr: &[Value]) -> Vec<Value> {
    let mut result = Vec::new();

    for item in arr {
        match item {
            Value::Array(inner) => result.extend(inner.iter().cloned()),
            other => result.push(other.clone()),
        }
    }

    result
}

/// Zip two arrays together into array of pairs
/// [1,2,3].zip([4,5,6]) → [[1,4], [2,5], [3,6]]
pub fn array_zip(arr1: &[Value], arr2: &[Value]) -> Vec<Value> {
    arr1.iter()
        .zip(arr2.iter())
        .map(|(a, b)| Value::Array(Arc::new(vec![a.clone(), b.clone()])))
        .collect()
}

/// Add index to each element
/// ["a", "b"] → [[0, "a"], [1, "b"]]
pub fn array_enumerate(arr: &[Value]) -> Vec<Value> {
    arr.iter()
        .enumerate()
        .map(|(i, v)| Value::Array(Arc::new(vec![Value::Int(i as i64), v.clone()])))
        .collect()
}

/// Take first n elements
pub fn array_take(arr: &[Value], n: i64) -> Vec<Value> {
    if n <= 0 {
        return Vec::new();
    }

    let count = (n as usize).min(arr.len());
    arr[..count].to_vec()
}

/// Skip first n elements
pub fn array_skip(arr: &[Value], n: i64) -> Vec<Value> {
    if n <= 0 {
        return arr.to_vec();
    }

    let skip_count = (n as usize).min(arr.len());
    arr[skip_count..].to_vec()
}

/// Create sliding windows of specified size
/// [1,2,3,4].windows(2) → [[1,2], [2,3], [3,4]]
pub fn array_windows(arr: &[Value], window_size: i64) -> Vec<Value> {
    if window_size <= 0 || arr.len() < window_size as usize {
        return Vec::new();
    }

    let size = window_size as usize;
    arr.windows(size).map(|window| Value::Array(Arc::new(window.to_vec()))).collect()
}

/// Advanced dict methods
/// Invert a dictionary (swap keys and values)
/// Keys must convert to valid dict keys
pub fn dict_invert(dict: &DictMap) -> DictMap {
    let mut result = DictMap::default();

    for (key, value) in dict {
        let new_key = match value {
            Value::Str(s) => s.as_ref().clone(),
            Value::Int(n) => n.to_string(),
            Value::Float(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => continue, // Skip non-primitive values
        };
        result.insert(Arc::from(new_key), Value::Str(Arc::new(key.to_string())));
    }

    result
}

/// HTTP Client Functions
/// Make an HTTP GET request
/// Returns a dictionary with status, body, and headers
/// Infrastructure for http.get() builtin
#[allow(dead_code)]
pub fn http_get(url: &str) -> Result<DictMap, String> {
    network_policy::enforce_http_url_destination_policy(url, "HTTP GET")?;
    let url = url.to_string();
    network_policy::run_blocking_http_task("HTTP GET", move || {
        let client = network_policy::build_http_client(network_policy::default_http_timeout())?;
        let response = client.get(&url).send().map_err(|e| format!("HTTP GET failed: {}", e))?;
        http_response_to_dict("HTTP GET", response)
    })
}

fn http_response_to_dict(
    surface: &str,
    response: reqwest::blocking::Response,
) -> Result<DictMap, String> {
    let (status, _headers, body_bytes) =
        network_policy::read_http_response_bytes(response, surface)?;
    let mut result = DictMap::default();
    result.insert("status".into(), Value::Int(status as i64));
    result.insert(
        "body".into(),
        Value::Str(Arc::new(String::from_utf8_lossy(&body_bytes).to_string())),
    );
    Ok(result)
}

/// Make an HTTP POST request with JSON body
/// body_json should be a stringified JSON
/// Infrastructure for http.post() builtin
#[allow(dead_code)]
pub fn http_post(url: &str, body_json: &str) -> Result<DictMap, String> {
    network_policy::enforce_http_url_destination_policy(url, "HTTP POST")?;
    let url = url.to_string();
    let body_json = body_json.to_string();
    network_policy::run_blocking_http_task("HTTP POST", move || {
        let client = network_policy::build_http_client(network_policy::default_http_timeout())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .map_err(|e| format!("HTTP POST failed: {}", e))?;
        http_response_to_dict("HTTP POST", response)
    })
}

/// Make an HTTP PUT request with JSON body
/// Infrastructure for http.put() builtin
#[allow(dead_code)]
pub fn http_put(url: &str, body_json: &str) -> Result<DictMap, String> {
    network_policy::enforce_http_url_destination_policy(url, "HTTP PUT")?;
    let url = url.to_string();
    let body_json = body_json.to_string();
    network_policy::run_blocking_http_task("HTTP PUT", move || {
        let client = network_policy::build_http_client(network_policy::default_http_timeout())?;
        let response = client
            .put(&url)
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .map_err(|e| format!("HTTP PUT failed: {}", e))?;
        http_response_to_dict("HTTP PUT", response)
    })
}

/// Make an HTTP DELETE request
/// Infrastructure for http.delete() builtin
#[allow(dead_code)]
pub fn http_delete(url: &str) -> Result<DictMap, String> {
    network_policy::enforce_http_url_destination_policy(url, "HTTP DELETE")?;
    let url = url.to_string();
    network_policy::run_blocking_http_task("HTTP DELETE", move || {
        let client = network_policy::build_http_client(network_policy::default_http_timeout())?;
        let response =
            client.delete(&url).send().map_err(|e| format!("HTTP DELETE failed: {}", e))?;
        http_response_to_dict("HTTP DELETE", response)
    })
}

/// Make an HTTP GET request and return binary data
/// Infrastructure for http.getBinary() builtin
#[allow(dead_code)]
pub fn http_get_binary(url: &str) -> Result<Vec<u8>, String> {
    network_policy::enforce_http_url_destination_policy(url, "HTTP GET request")?;
    let url = url.to_string();
    network_policy::run_blocking_http_task("HTTP GET request", move || {
        let client = network_policy::build_http_client(network_policy::default_http_timeout())?;
        let response =
            client.get(&url).send().map_err(|e| format!("HTTP GET request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP GET failed with status: {}", response.status()));
        }
        let (_, _, body_bytes) =
            network_policy::read_http_response_bytes(response, "HTTP GET request")?;
        Ok(body_bytes)
    })
}

/// Encode bytes to base64 string
/// Infrastructure for base64.encode() builtin
#[allow(dead_code)]
pub fn encode_base64(bytes: &[u8]) -> String {
    general_purpose::STANDARD.encode(bytes)
}

/// Decode base64 string to bytes
/// Infrastructure for base64.decode() builtin
#[allow(dead_code)]
pub fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    general_purpose::STANDARD.decode(s).map_err(|e| format!("Base64 decode error: {}", e))
}

/// JWT Authentication Functions
/// JWT Claims structure for encoding/decoding
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    #[serde(flatten)]
    data: HashMap<String, serde_json::Value>,
}

/// Encode a JWT token from a dictionary payload and secret key
/// jwt_encode(payload_dict, secret_key) -> token string
pub fn jwt_encode(payload: &DictMap, secret: &str) -> Result<String, String> {
    // Convert Kujo dictionary to JSON claims
    let mut claims_data = HashMap::new();
    for (key, value) in payload {
        let json_value = kujo_value_to_json(value)
            .map_err(|e| format!("Failed to convert payload to JSON: {}", e))?;
        claims_data.insert(key.to_string(), json_value);
    }

    let claims = Claims { data: claims_data };

    // Encode JWT with HS256 algorithm
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| format!("JWT encoding error: {}", e))
}

/// Decode a JWT token and return the payload as a dictionary
/// jwt_decode(token, secret_key) -> payload dictionary
pub fn jwt_decode(token: &str, secret: &str) -> Result<DictMap, String> {
    // Create validation without requiring expiry
    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims.clear(); // Don't require any specific claims
    validation.validate_exp = false; // Don't validate expiration by default

    // Decode JWT
    let token_data =
        decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)
            .map_err(|e| format!("JWT decoding error: {}", e))?;

    // Convert claims back to Kujo dictionary
    let mut result = DictMap::default();
    for (key, json_value) in token_data.claims.data {
        result.insert(key.into(), json_to_kujo_value(json_value));
    }

    Ok(result)
}

/// Verify a JWT token signature and basic validity checks.
pub fn jwt_verify(token: &str, secret: &str) -> bool {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims.clear();
    validation.validate_exp = true;

    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation).is_ok()
}

/// OAuth2 Helper Functions
/// Create an OAuth2 authorization URL
/// oauth2_auth_url(client_id, redirect_uri, auth_url, scope) -> authorization URL
pub fn oauth2_auth_url(client_id: &str, redirect_uri: &str, auth_url: &str, scope: &str) -> String {
    // Generate a simple state parameter for CSRF protection
    let state = format!("{:x}", rand::random::<u64>());

    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        auth_url,
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(scope),
        state
    )
}

/// Exchange OAuth2 authorization code for access token
/// oauth2_get_token(code, client_id, client_secret, token_url, redirect_uri) -> token info dict
pub fn oauth2_get_token(
    code: &str,
    client_id: &str,
    client_secret: &str,
    token_url: &str,
    redirect_uri: &str,
) -> Result<DictMap, String> {
    network_policy::enforce_http_url_destination_policy(token_url, "OAuth2 token request")?;
    let code = code.to_string();
    let client_id = client_id.to_string();
    let client_secret = client_secret.to_string();
    let token_url = token_url.to_string();
    let redirect_uri = redirect_uri.to_string();

    network_policy::run_blocking_http_task("OAuth2 token request", move || {
        let client = network_policy::build_http_client(network_policy::default_http_timeout())?;
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ];

        let response = client
            .post(&token_url)
            .form(&params)
            .send()
            .map_err(|e| format!("OAuth2 token request failed: {}", e))?;

        let status = response.status().as_u16();
        let (_, _, body_bytes) =
            network_policy::read_http_response_bytes(response, "OAuth2 token request")?;
        let body = String::from_utf8_lossy(&body_bytes).to_string();

        if !(200..300).contains(&status) {
            return Err(format!("OAuth2 token request failed with status {}: {}", status, body));
        }

        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json) => {
                let mut result = DictMap::default();
                if let Some(obj) = json.as_object() {
                    for (key, val) in obj {
                        result.insert(key.clone().into(), json_to_kujo_value(val.clone()));
                    }
                }
                Ok(result)
            }
            Err(e) => Err(format!("Failed to parse OAuth2 token response: {}", e)),
        }
    })
}

/// HTTP Streaming Functions
/// Stream data structure to hold ongoing stream state
#[allow(dead_code)] // Infrastructure for future streaming enhancements
pub struct HttpStream {
    pub url: String,
    pub chunk_size: usize,
    pub position: usize,
    pub data: Vec<u8>,
}

/// Start an HTTP GET stream
/// http_get_stream(url) -> stream handle (as dictionary with internal state)
/// Infrastructure for http.getStream() builtin
#[allow(dead_code)]
pub fn http_get_stream(url: &str) -> Result<Vec<u8>, String> {
    // For now, we'll fetch the entire response but allow chunked reading
    // In a real implementation, this would use async streaming
    network_policy::enforce_http_url_destination_policy(url, "HTTP GET stream")?;
    let url = url.to_string();
    network_policy::run_blocking_http_task("HTTP GET stream", move || {
        let client = network_policy::build_http_client(network_policy::default_http_timeout())?;
        let response = client
            .get(&url)
            .send()
            .map_err(|e| format!("HTTP GET stream request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP GET stream failed with status: {}", response.status()));
        }
        let (_, _, body_bytes) =
            network_policy::read_http_response_bytes(response, "HTTP GET stream")?;
        Ok(body_bytes)
    })
}

/// Assert & Debug Functions
/// Assert that a condition is true, throw error if false
/// assert(condition, message) - Throws error with message if condition is false
pub fn assert_condition(condition: bool, message: Option<&str>) -> Result<(), String> {
    if !condition {
        let error_msg = message.unwrap_or("Assertion failed");
        return Err(error_msg.to_string());
    }
    Ok(())
}

/// Format a value for debug output
pub fn format_debug_value(value: &Value) -> String {
    match value {
        Value::Int(n) => format!("Int({})", n),
        Value::Float(n) => format!("Float({})", n),
        Value::Str(s) => format!("String(\"{}\")", s.as_ref()),
        Value::Bool(b) => format!("Bool({})", b),
        Value::Null => "Null".to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_debug_value).collect();
            format!("Array[{}]", items.join(", "))
        }
        Value::Dict(dict) => {
            let mut keys: Vec<&Arc<str>> = dict.keys().collect();
            keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
            let items: Vec<String> = keys
                .iter()
                .map(|k| format!("{}: {}", k, format_debug_value(dict.get(k.as_ref()).unwrap())))
                .collect();
            format!("Dict{{{}}}", items.join(", "))
        }
        Value::FixedDict { keys, values } => {
            let mut pairs: Vec<(&Arc<str>, &Value)> = keys.iter().zip(values.iter()).collect();
            pairs.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
            let items: Vec<String> =
                pairs.iter().map(|(k, v)| format!("{}: {}", k, format_debug_value(v))).collect();
            format!("Dict{{{}}}", items.join(", "))
        }
        Value::IntDict(dict) => {
            let mut keys: Vec<i64> = dict.keys().copied().collect();
            keys.sort();
            let items: Vec<String> = keys
                .iter()
                .map(|k| format!("{}: {}", k, format_debug_value(dict.get(k).unwrap())))
                .collect();
            format!("Dict{{{}}}", items.join(", "))
        }
        Value::DenseIntDict(values) => {
            let items: Vec<String> = values
                .iter()
                .enumerate()
                .map(|(index, value)| format!("{}: {}", index, format_debug_value(value)))
                .collect();
            format!("Dict{{{}}}", items.join(", "))
        }
        Value::DenseIntDictInt(values) => {
            let items: Vec<String> = values
                .iter()
                .enumerate()
                .map(|(index, value)| match value {
                    Some(value) => format!("{}: {}", index, value),
                    None => format!("{}: null", index),
                })
                .collect();
            format!("Dict{{{}}}", items.join(", "))
        }
        Value::DenseIntDictIntFull(values) => {
            let items: Vec<String> = values
                .iter()
                .enumerate()
                .map(|(index, value)| format!("{}: {}", index, value))
                .collect();
            format!("Dict{{{}}}", items.join(", "))
        }
        Value::Function(_, _, _) => "Function".to_string(),
        Value::AsyncFunction(_, _, _) => "AsyncFunction".to_string(),
        Value::NativeFunction(name) => format!("NativeFunction({})", name),
        Value::BytecodeFunction { chunk, .. } => {
            let name = chunk.name.as_deref().unwrap_or("<lambda>");
            format!("BytecodeFunction({})", name)
        }
        Value::BytecodeGenerator { state } => {
            let gen_state = state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            format!("BytecodeGenerator(exhausted: {})", gen_state.is_exhausted)
        }
        Value::ArrayMarker => "ArrayMarker".to_string(),
        Value::Struct { name, .. } => format!("Struct({})", name),
        Value::StructDef { name, .. } => format!("StructDef({})", name),
        Value::Tagged { tag, fields } => {
            let items: Vec<String> =
                fields.iter().map(|(k, v)| format!("{}: {}", k, format_debug_value(v))).collect();
            format!("{}{{ {} }}", tag, items.join(", "))
        }
        Value::Bytes(bytes) => format!("Bytes({} bytes)", bytes.len()),
        Value::Set(set) => {
            let items: Vec<String> = set.iter().map(format_debug_value).collect();
            format!("Set{{{}}}", items.join(", "))
        }
        Value::Queue(queue) => {
            let items: Vec<String> = queue.iter().map(format_debug_value).collect();
            format!("Queue[{}]", items.join(", "))
        }
        Value::Stack(stack) => {
            let items: Vec<String> = stack.iter().map(format_debug_value).collect();
            format!("Stack[{}]", items.join(", "))
        }
        Value::Return(val) => format!("Return({})", format_debug_value(val)),
        Value::Error(msg) => format!("Error(\"{}\")", msg),
        Value::ErrorObject { message, .. } => format!("ErrorObject(\"{}\")", message),
        Value::Enum(name) => format!("Enum({})", name),
        Value::Channel(_) => "Channel".to_string(),
        Value::HttpServer { host, port, .. } => {
            format!("HttpServer(host: {}, port: {})", host, port)
        }
        Value::HttpResponse { status, .. } => format!("HttpResponse(status: {})", status),
        Value::Database { db_type, .. } => format!("Database(type: {})", db_type),
        Value::DatabasePool { .. } => "DatabasePool".to_string(),
        Value::Image { format, .. } => format!("Image(format: {})", format),
        Value::ZipArchive { path, .. } => format!("ZipArchive(path: {})", path),
        Value::TcpListener { addr, .. } => format!("TcpListener(addr: {})", addr),
        Value::TcpStream { peer_addr, .. } => format!("TcpStream(peer: {})", peer_addr),
        Value::UdpSocket { addr, .. } => format!("UdpSocket(addr: {})", addr),
        Value::Result { is_ok, value } => {
            if *is_ok {
                format!("Ok({})", format_debug_value(value))
            } else {
                format!("Err({})", format_debug_value(value))
            }
        }
        Value::Option { is_some, value } => {
            if *is_some {
                format!("Some({})", format_debug_value(value))
            } else {
                "None".to_string()
            }
        }
        Value::GeneratorDef(params, _) => format!("GeneratorDef({:?})", params),
        Value::Generator { params, is_exhausted, .. } => {
            format!("Generator({:?}, exhausted: {})", params, is_exhausted)
        }
        Value::Iterator { source, .. } => format!("Iterator(source: {:?})", source),
        Value::Promise { cached_result, .. } => {
            let result = cached_result.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            match &*result {
                None => "Promise(Pending)".to_string(),
                Some(Ok(value)) => format!("Promise(Resolved: {})", format_debug_value(value)),
                Some(Err(err)) => format!("Promise(Rejected: {})", err),
            }
        }
        Value::TaskHandle { is_cancelled, .. } => {
            let cancelled = is_cancelled.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if *cancelled {
                "TaskHandle(Cancelled)".to_string()
            } else {
                "TaskHandle(Running)".to_string()
            }
        }
    }
}

/// Command-line argument parser
/// Represents an argument definition
#[derive(Debug, Clone)]
pub struct ArgumentDef {
    pub long_name: String,          // e.g., "--verbose"
    pub short_name: Option<String>, // e.g., "-v"
    pub arg_type: String,           // "bool", "string", "int", "float"
    pub required: bool,
    pub help: String,
    pub default: Option<String>,
}

/// Parse command-line arguments based on defined arguments
/// Returns a HashMap of argument names to values
pub fn parse_arguments(arg_defs: &[ArgumentDef], args: &[String]) -> Result<DictMap, String> {
    let mut result = DictMap::default();
    let mut i = 0;
    let mut positional_args: Vec<String> = Vec::new();

    // Track which arguments were found
    let mut found_args = std::collections::HashSet::new();

    while i < args.len() {
        let arg = &args[i];

        if arg.starts_with("--") || arg.starts_with('-') {
            // This is a flag or option
            let mut found = false;

            for def in arg_defs {
                let matches_long = arg == &def.long_name;
                let matches_short = def.short_name.as_ref().map_or(false, |s| arg == s);

                if matches_long || matches_short {
                    found = true;
                    let key = def.long_name.trim_start_matches("--").to_string();
                    found_args.insert(key.clone());

                    match def.arg_type.as_str() {
                        "bool" => {
                            // Flags don't consume next argument
                            result.insert(key.into(), Value::Bool(true));
                        }
                        "string" => {
                            // Consume next argument as string value
                            if i + 1 < args.len() {
                                i += 1;
                                result.insert(key.into(), Value::Str(Arc::new(args[i].clone())));
                            } else {
                                return Err(format!("Argument {} requires a value", def.long_name));
                            }
                        }
                        "int" => {
                            // Consume next argument and parse as int
                            if i + 1 < args.len() {
                                i += 1;
                                match args[i].parse::<i64>() {
                                    Ok(val) => {
                                        result.insert(key.into(), Value::Int(val));
                                    }
                                    Err(_) => {
                                        return Err(format!(
                                            "Argument {} requires an integer value, got: {}",
                                            def.long_name, args[i]
                                        ));
                                    }
                                }
                            } else {
                                return Err(format!("Argument {} requires a value", def.long_name));
                            }
                        }
                        "float" => {
                            // Consume next argument and parse as float
                            if i + 1 < args.len() {
                                i += 1;
                                match args[i].parse::<f64>() {
                                    Ok(val) => {
                                        result.insert(key.into(), Value::Float(val));
                                    }
                                    Err(_) => {
                                        return Err(format!(
                                            "Argument {} requires a float value, got: {}",
                                            def.long_name, args[i]
                                        ));
                                    }
                                }
                            } else {
                                return Err(format!("Argument {} requires a value", def.long_name));
                            }
                        }
                        _ => {
                            return Err(format!("Unknown argument type: {}", def.arg_type));
                        }
                    }
                    break;
                }
            }

            if !found {
                return Err(format!("Unknown argument: {}", arg));
            }
        } else {
            // Positional argument
            positional_args.push(arg.clone());
        }

        i += 1;
    }

    // Check for required arguments
    for def in arg_defs {
        let key = def.long_name.trim_start_matches("--").to_string();
        if def.required && !found_args.contains(&key) {
            return Err(format!("Required argument {} is missing", def.long_name));
        }

        // Apply defaults for missing optional arguments
        if !def.required && !found_args.contains(&key) {
            if let Some(default_val) = &def.default {
                let value = match def.arg_type.as_str() {
                    "bool" => Value::Bool(default_val == "true"),
                    "string" => Value::Str(Arc::new(default_val.clone())),
                    "int" => Value::Int(default_val.parse().unwrap_or(0)),
                    "float" => Value::Float(default_val.parse().unwrap_or(0.0)),
                    _ => Value::Str(Arc::new(default_val.clone())),
                };
                result.insert(key.into(), value);
            } else if def.arg_type == "bool" {
                // Bool flags default to false if not provided
                result.insert(key.into(), Value::Bool(false));
            } else {
                // For non-bool optional arguments without defaults, use Null
                result.insert(key.into(), Value::Null);
            }
        }
    }

    // Store positional arguments if any
    if !positional_args.is_empty() {
        result.insert(
            "_positional".into(),
            Value::Array(Arc::new(
                positional_args.into_iter().map(|s| Value::Str(Arc::new(s))).collect(),
            )),
        );
    }

    Ok(result)
}

/// Generate help text from argument definitions
pub fn generate_help(arg_defs: &[ArgumentDef], app_name: &str, description: &str) -> String {
    let mut help = format!("{}\n\n{}\n\nOptions:\n", app_name, description);

    for def in arg_defs {
        let mut line = String::new();

        // Short and long names
        if let Some(short) = &def.short_name {
            line.push_str(&format!("  {}, {}", short, def.long_name));
        } else {
            line.push_str(&format!("      {}", def.long_name));
        }

        // Type and required
        if def.arg_type != "bool" {
            line.push_str(&format!(" <{}>", def.arg_type));
        }
        if def.required {
            line.push_str(" (required)");
        }

        // Default value
        if let Some(default) = &def.default {
            line.push_str(&format!(" [default: {}]", default));
        }

        help.push_str(&line);
        help.push('\n');

        // Help text
        if !def.help.is_empty() {
            help.push_str(&format!("        {}\n", def.help));
        }
    }

    help
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_math_functions() {
        assert_eq!(abs(-5.0), 5.0);
        assert_eq!(abs(3.0), 3.0);
        assert_eq!(floor(3.7), 3.0);
        assert_eq!(ceil(3.2), 4.0);
        assert_eq!(round(3.5), 4.0);
        assert_eq!(min(5.0, 3.0), 3.0);
        assert_eq!(max(5.0, 3.0), 5.0);
    }

    #[test]
    fn test_string_functions() {
        assert_eq!(str_len("hello"), 5.0);
        assert_eq!(str_len("Free Tools → Plan"), 17.0);
        assert_eq!(substring("hello", 1.0, 4.0), "ell");
        assert_eq!(to_upper("hello"), "HELLO");
        assert_eq!(to_lower("HELLO"), "hello");
        assert_eq!(trim("  hello  "), "hello");
        assert!(contains("hello world", "world"));
        assert!(!contains("hello", "xyz"));
        assert_eq!(replace("hello world", "world", "rust"), "hello rust");

        // New string functions
        assert!(starts_with("hello world", "hello"));
        assert!(!starts_with("hello world", "world"));
        assert!(ends_with("test.kujo", ".kujo"));
        assert!(!ends_with("test.kujo", ".py"));
        assert_eq!(index_of("hello", "ll"), 2.0);
        assert_eq!(index_of("a→b", "b"), 2.0);
        assert_eq!(index_of("hello", "xyz"), -1.0);
        assert_eq!(repeat("ha", 3.0), "hahaha");
        assert_eq!(split("a,b,c", ","), vec!["a", "b", "c"]);
        assert_eq!(join(&["a".to_string(), "b".to_string(), "c".to_string()], ","), "a,b,c");
    }

    #[test]
    fn test_to_json_fixed_dict_serializes_successfully() {
        let value = Value::FixedDict {
            keys: Arc::new(vec![Arc::<str>::from("title"), Arc::<str>::from("count")]),
            values: vec![Value::Str(Arc::new("Kujo".to_string())), Value::Int(2)],
        };

        let encoded = to_json(&value).expect("FixedDict should serialize to JSON");
        let decoded: serde_json::Value =
            serde_json::from_str(&encoded).expect("Serialized JSON should parse");

        assert_eq!(decoded["title"], serde_json::Value::String("Kujo".to_string()));
        assert_eq!(decoded["count"], serde_json::Value::Number(2.into()));
    }

    #[test]
    fn test_to_json_array_does_not_duplicate_entries() {
        let value = Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));

        let encoded = to_json(&value).expect("Array should serialize to JSON");
        let decoded: serde_json::Value =
            serde_json::from_str(&encoded).expect("Serialized JSON should parse");

        let items = decoded.as_array().expect("Decoded array JSON should contain an array root");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], serde_json::Value::Number(1.into()));
        assert_eq!(items[1], serde_json::Value::Number(2.into()));
        assert_eq!(items[2], serde_json::Value::Number(3.into()));
    }

    #[test]
    fn test_to_json_pretty_includes_newlines_and_indentation() {
        let value = Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)]));

        let encoded = to_json_pretty(&value).expect("Array should serialize to pretty JSON");
        assert!(encoded.contains('\n'));
        assert!(encoded.contains("  1"));

        let decoded: serde_json::Value =
            serde_json::from_str(&encoded).expect("Pretty JSON should parse");
        assert_eq!(decoded, serde_json::json!([1, 2]));
    }

    #[test]
    fn test_json_roundtrip_array_of_objects_preserves_cardinality() {
        let raw = r#"{"items":[{"id":1},{"id":2}],"ok":true}"#;
        let parsed = parse_json(raw).expect("Input JSON should parse");
        let encoded = to_json(&parsed).expect("Roundtrip JSON should serialize");
        let decoded: serde_json::Value =
            serde_json::from_str(&encoded).expect("Roundtrip JSON output should parse");

        let items =
            decoded["items"].as_array().expect("items should remain an array after roundtrip");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], serde_json::Value::Number(1.into()));
        assert_eq!(items[1]["id"], serde_json::Value::Number(2.into()));
    }

    #[test]
    fn test_format_date_rejects_non_finite_timestamp_without_panicking() {
        let formatted = format_date(f64::NAN, "YYYY-MM-DD");
        assert_eq!(formatted, "Invalid timestamp: value must be a finite number");
    }

    #[test]
    fn test_format_date_rejects_out_of_range_timestamp_without_panicking() {
        let formatted = format_date(i64::MAX as f64, "YYYY-MM-DD");
        assert_eq!(formatted, "Invalid timestamp: value is out of supported range");
    }

    #[test]
    fn test_safe_duration_since_unix_epoch_clamps_pre_epoch_values_to_zero() {
        let pre_epoch = UNIX_EPOCH
            .checked_sub(Duration::from_secs(1))
            .expect("pre-epoch test timestamp should be constructible");
        let duration = safe_duration_since_unix_epoch(pre_epoch);
        assert_eq!(duration, Duration::ZERO);
    }

    #[test]
    fn test_safe_duration_since_unix_epoch_returns_positive_duration_for_post_epoch() {
        let post_epoch = UNIX_EPOCH + Duration::from_secs(42);
        let duration = safe_duration_since_unix_epoch(post_epoch);
        assert_eq!(duration, Duration::from_secs(42));
    }
}
