// File: src/interpreter/native_functions/concurrency.rs
//
// Concurrency-related native functions (spawn, channels, etc.)

use crate::interpreter::{Interpreter, Value};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, MutexGuard};

fn shared_value_store() -> &'static Mutex<HashMap<String, Arc<Mutex<Value>>>> {
    static SHARED_VALUE_STORE: OnceLock<Mutex<HashMap<String, Arc<Mutex<Value>>>>> =
        OnceLock::new();
    SHARED_VALUE_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_or_concurrency_error<'a, T>(
    mutex: &'a Mutex<T>,
    context: &str,
) -> Result<MutexGuard<'a, T>, Value> {
    mutex.lock().map_err(|_| Value::Error(format!("{}: shared state lock poisoned", context)))
}

pub fn handle(_interp: &mut Interpreter, name: &str, _arg_values: &[Value]) -> Option<Value> {
    let arg_values = _arg_values;
    let result = match name {
        "channel" => {
            // channel() - creates a new channel for thread communication
            if arg_values.len() != 0 {
                return Some(Value::Error(format!(
                    "channel() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            use std::sync::mpsc;
            let (sender, receiver) = mpsc::channel();
            #[allow(clippy::arc_with_non_send_sync)]
            let channel = Arc::new(Mutex::new((sender, receiver)));
            Value::Channel(channel)
        }
        "shared_set" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "shared_set requires (key, value) arguments".to_string(),
                ));
            }

            let key = match &arg_values[0] {
                Value::Str(s) => s.as_ref().clone(),
                _ => {
                    return Some(Value::Error("shared_set key must be a string".to_string()));
                }
            };

            let value = arg_values[1].clone();
            let mut store = match lock_or_concurrency_error(shared_value_store(), "shared_set") {
                Ok(guard) => guard,
                Err(error) => return Some(error),
            };
            store.insert(key, Arc::new(Mutex::new(value)));
            Value::Bool(true)
        }
        "shared_get" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("shared_get requires one key argument".to_string()));
            }

            let key = match &arg_values[0] {
                Value::Str(s) => s.as_ref().clone(),
                _ => {
                    return Some(Value::Error("shared_get key must be a string".to_string()));
                }
            };

            let store = match lock_or_concurrency_error(shared_value_store(), "shared_get") {
                Ok(guard) => guard,
                Err(error) => return Some(error),
            };
            if let Some(cell) = store.get(&key) {
                let value = match lock_or_concurrency_error(cell, "shared_get.value") {
                    Ok(guard) => guard.clone(),
                    Err(error) => return Some(error),
                };
                value
            } else {
                Value::Error(format!("shared_get key '{}' not found", key))
            }
        }
        "shared_has" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("shared_has requires one key argument".to_string()));
            }

            let key = match &arg_values[0] {
                Value::Str(s) => s.as_ref().clone(),
                _ => {
                    return Some(Value::Error("shared_has key must be a string".to_string()));
                }
            };

            let store = match lock_or_concurrency_error(shared_value_store(), "shared_has") {
                Ok(guard) => guard,
                Err(error) => return Some(error),
            };
            Value::Bool(store.contains_key(&key))
        }
        "shared_delete" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("shared_delete requires one key argument".to_string()));
            }

            let key = match &arg_values[0] {
                Value::Str(s) => s.as_ref().clone(),
                _ => {
                    return Some(Value::Error("shared_delete key must be a string".to_string()));
                }
            };

            let mut store = match lock_or_concurrency_error(shared_value_store(), "shared_delete") {
                Ok(guard) => guard,
                Err(error) => return Some(error),
            };
            Value::Bool(store.remove(&key).is_some())
        }
        "shared_add_int" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "shared_add_int requires (key, delta) arguments".to_string(),
                ));
            }

            let key = match &arg_values[0] {
                Value::Str(s) => s.as_ref().clone(),
                _ => {
                    return Some(Value::Error("shared_add_int key must be a string".to_string()));
                }
            };

            let delta = match &arg_values[1] {
                Value::Int(n) => *n,
                _ => {
                    return Some(Value::Error("shared_add_int delta must be an int".to_string()));
                }
            };

            let store = match lock_or_concurrency_error(shared_value_store(), "shared_add_int") {
                Ok(guard) => guard,
                Err(error) => return Some(error),
            };
            if let Some(cell) = store.get(&key) {
                let mut value = match lock_or_concurrency_error(cell, "shared_add_int.value") {
                    Ok(guard) => guard,
                    Err(error) => return Some(error),
                };
                match &mut *value {
                    Value::Int(current) => {
                        *current += delta;
                        Value::Int(*current)
                    }
                    _ => Value::Error(format!(
                        "shared_add_int requires key '{}' to reference an int",
                        key
                    )),
                }
            } else {
                Value::Error(format!("shared_add_int key '{}' not found", key))
            }
        }

        _ => return None,
    };

    Some(result)
}
