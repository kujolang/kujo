//! Fallible synchronous callback APIs with operation-owned runtime state.
use super::{Interpreter, Value};
use crate::{builtins, runtime_limits};
use std::sync::{Arc, Mutex};

fn callable(value: &Value) -> bool {
    matches!(value, Value::Function(..) | Value::BytecodeFunction { .. } | Value::NativeFunction(_))
}

/// One native operation owns a VM for its fixed synchronous bytecode callback.
/// Interpreter-dependent callbacks borrow and return the caller's loader state.
struct Callback<'a> {
    function: &'a Value,
    vm: Option<Box<crate::vm::VM>>,
    initialized: bool,
}

impl<'a> Callback<'a> {
    fn new(function: &'a Value) -> Self {
        Self { function, vm: None, initialized: false }
    }

    fn invoke(&mut self, interp: &mut Interpreter, args: &[Value]) -> Value {
        if !self.initialized {
            self.initialized = true;
            if matches!(self.function, Value::BytecodeFunction { .. }) {
                let globals = interp
                    .vm_globals
                    .clone()
                    .unwrap_or_else(|| Arc::new(Mutex::new(interp.env.clone())));
                self.vm = crate::vm::VM::reusable_sync_callback(
                    self.function,
                    globals,
                    interp.capability_policy().clone(),
                    interp.output_buffer(),
                    interp.task_cancellation.clone(),
                    interp.vm_jit_requested,
                )
                .map(Box::new);
            }
        }
        if let Some(vm) = self.vm.as_mut() {
            return vm
                .call_reused_sync_callback(self.function.clone(), args, &mut interp.module_loader)
                .unwrap_or_else(Value::Error);
        }
        match self.function {
            Value::NativeFunction(name) => super::call_native_function(interp, name, args),
            _ => interp.call_user_function(self.function, args),
        }
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
    if name == "sort_by" {
        return Some(sort_by(interp, args));
    }
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
    let mut callback = Callback::new(callback);
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
        let replacement = callback.invoke(interp, &[Value::Array(Arc::new(groups))]);
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

fn sort_by(interp: &mut Interpreter, args: &[Value]) -> Value {
    let [Value::Array(items), callback] = args else {
        return Value::Error("sort_by expects an array and function".into());
    };
    if !callable(callback) {
        return Value::Error("sort_by expects a synchronous function".into());
    }
    let mut callback = Callback::new(callback);
    // Sort indices, not deep values. Explicit fallible merges avoid Rust sort's
    // comparator panics and stop at the first callback failure.
    let mut order: Vec<usize> = (0..items.len()).collect();
    let mut scratch = order.clone();
    let mut width = 1;
    while width < items.len() {
        for start in (0..items.len()).step_by(2 * width) {
            let middle = (start + width).min(items.len());
            let end = (start + 2 * width).min(items.len());
            let (mut left, mut right) = (start, middle);
            for slot in &mut scratch[start..end] {
                let take_left = if right == end {
                    true
                } else if left == middle {
                    false
                } else {
                    match callback
                        .invoke(interp, &[items[order[left]].clone(), items[order[right]].clone()])
                    {
                        Value::Int(result) => result <= 0,
                        error @ (Value::Error(_) | Value::ErrorObject { .. }) => return error,
                        _ => {
                            return Value::Error("sort_by comparator must return an integer".into())
                        }
                    }
                };
                if take_left {
                    *slot = order[left];
                    left += 1;
                } else {
                    *slot = order[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(&mut order, &mut scratch);
        width *= 2;
    }
    Value::Array(Arc::new(order.into_iter().map(|index| items[index].clone()).collect()))
}
