//! Fallible synchronous callback APIs. Always use the ordinary runtime bridge.
use super::{Interpreter, Value};
use crate::{builtins, runtime_limits};
use std::sync::Arc;

fn callable(value: &Value) -> bool {
    matches!(value, Value::Function(..) | Value::BytecodeFunction { .. } | Value::NativeFunction(_))
}

fn invoke(interp: &mut Interpreter, callback: &Value, args: &[Value]) -> Value {
    match callback {
        Value::NativeFunction(name) => super::call_native_function(interp, name, args),
        _ => interp.call_user_function(callback, args),
    }
}

fn append(output: &mut String, text: &str) -> Result<(), Value> {
    if text.len() > runtime_limits::MAX_FILE_IO_BYTES.saturating_sub(output.len()) {
        return Err(Value::Error("regex_replace_with output exceeds 8388608 bytes".into()));
    }
    output.push_str(text);
    Ok(())
}

pub fn handle(interp: &mut Interpreter, name: &str, args: &[Value]) -> Option<Value> {
    if name != "regex_replace_with" {
        return None;
    }
    let [Value::Str(text), Value::Str(pattern), callback] = args else {
        return Some(Value::Error("regex_replace_with expects text, pattern, and function".into()));
    };
    if !callable(callback) {
        return Some(Value::Error("regex_replace_with expects a synchronous function".into()));
    }
    if text.len() > runtime_limits::MAX_FILE_IO_BYTES {
        return Some(Value::Error("regex_replace_with input exceeds 8388608 bytes".into()));
    }
    let Some(regex) = builtins::cached_regex(pattern) else {
        return Some(Value::Error("regex_replace_with invalid regex pattern".into()));
    };
    let mut output = String::new();
    let mut end = 0;
    for (count, captures) in regex.captures_iter(text).enumerate() {
        if count >= runtime_limits::MAX_GENERATED_SEQUENCE_ITEMS {
            return Some(Value::Error("regex_replace_with exceeds 1000000 matches".into()));
        }
        let matched = captures.get(0).expect("whole regex match");
        if let Err(error) = append(&mut output, &text[end..matched.start()]) {
            return Some(error);
        }
        let mut groups = Vec::new();
        let mut capture_bytes = 0usize;
        for capture in captures.iter() {
            groups.push(if let Some(value) = capture {
                capture_bytes += value.len();
                if capture_bytes > runtime_limits::MAX_FILE_IO_BYTES {
                    return Some(Value::Error(
                        "regex_replace_with captures exceed 8388608 bytes".into(),
                    ));
                }
                Value::Str(Arc::new(value.as_str().to_string()))
            } else {
                Value::Null
            });
        }
        let replacement = invoke(interp, callback, &[Value::Array(Arc::new(groups))]);
        match replacement {
            Value::Str(replacement) => {
                if let Err(error) = append(&mut output, &replacement) {
                    return Some(error);
                }
            }
            error @ (Value::Error(_) | Value::ErrorObject { .. }) => return Some(error),
            _ => {
                return Some(Value::Error(
                    "regex_replace_with callback must return a string".into(),
                ))
            }
        }
        end = matched.end();
    }
    if let Err(error) = append(&mut output, &text[end..]) {
        return Some(error);
    }
    Some(Value::Str(Arc::new(output)))
}
