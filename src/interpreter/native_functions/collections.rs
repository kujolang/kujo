// File: src/interpreter/native_functions/collections.rs
//
// Collection manipulation native functions (arrays, dicts, sets)

use crate::builtins;
use crate::interpreter::{DictMap, IntDictMap, Interpreter, Value};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

fn strict_arity_error(function_name: &str, expected: usize, got: usize) -> Value {
    Value::Error(format!(
        "{} expects {} argument{}, got {}",
        function_name,
        expected,
        if 1 == expected { "" } else { "s" },
        got
    ))
}

fn fixed_dict_to_dict(keys: &[Arc<str>], values: &[Value]) -> DictMap {
    let mut dict = DictMap::default();
    for (key, value) in keys.iter().cloned().zip(values.iter().cloned()) {
        dict.insert(key, value);
    }
    dict
}

fn parse_slice_bound(value: &Value, label: &str) -> Result<i64, Value> {
    match value {
        Value::Int(number) => Ok(*number),
        Value::Float(number) if number.is_finite() => Ok(*number as i64),
        _ => Err(Value::Error(format!("slice() {} must be a number", label))),
    }
}

fn normalize_slice_bounds(len: usize, start: i64, end: i64) -> (usize, usize) {
    let len_i64 = len as i64;
    let start_idx = if start < 0 { len_i64 + start } else { start };
    let start_idx = start_idx.max(0).min(len_i64);
    let end_idx = if end < 0 { len_i64 + end } else { end };
    let end_idx = end_idx.max(start_idx).min(len_i64);
    (start_idx as usize, end_idx as usize)
}

pub fn handle(interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        // Polymorphic len function - handles arrays, dicts, sets, queues, stacks, bytes
        "len" => match arg_values.first() {
            Some(Value::Array(arr)) => Value::Int(arr.len() as i64),
            Some(Value::Dict(dict)) => Value::Int(dict.len() as i64),
            Some(Value::IntDict(dict)) => Value::Int(dict.len() as i64),
            Some(Value::DenseIntDict(values)) => Value::Int(values.len() as i64),
            Some(Value::DenseIntDictInt(values)) => Value::Int(values.len() as i64),
            Some(Value::DenseIntDictIntFull(values)) => Value::Int(values.len() as i64),
            Some(Value::Bytes(bytes)) => Value::Int(bytes.len() as i64),
            Some(Value::Set(set)) => Value::Int(set.len() as i64),
            Some(Value::Queue(queue)) => Value::Int(queue.len() as i64),
            Some(Value::Stack(stack)) => Value::Int(stack.len() as i64),
            Some(Value::Str(_)) => return None, // Let strings module handle this
            Some(_) => Value::Error(
                "len() requires an array, dict, bytes, set, queue, stack, or string".to_string(),
            ),
            None => Value::Error("len() requires 1 argument".to_string()),
        },

        // Polymorphic contains - handles both strings and arrays
        "contains" => match (arg_values.first(), arg_values.get(1)) {
            (Some(Value::Array(arr)), Some(item)) => {
                Value::Bool(builtins::array_contains(&**arr, item))
            }
            _ => return None, // Let strings module handle string case
        },

        // Polymorphic index_of - handles both strings and arrays
        "index_of" => match (arg_values.first(), arg_values.get(1)) {
            (Some(Value::Array(arr)), Some(item)) => {
                Value::Int(builtins::array_index_of(&**arr, item))
            }
            _ => return None, // Let strings module handle string case
        },

        // Array functions
        "push" | "append" => {
            if 2 != arg_values.len() {
                strict_arity_error(name, 2, arg_values.len())
            } else if let Some(Value::Array(arr)) = arg_values.first().cloned() {
                if let Some(item) = arg_values.get(1).cloned() {
                    let mut arr_clone = arr;
                    let arr_mut = Arc::make_mut(&mut arr_clone);
                    arr_mut.push(item);
                    Value::Array(arr_clone)
                } else {
                    Value::Array(arr)
                }
            } else {
                Value::Error(format!("{}() requires an array as the first argument", name))
            }
        }

        "pop" => {
            if 1 != arg_values.len() {
                strict_arity_error("pop", 1, arg_values.len())
            } else if let Some(Value::Array(arr)) = arg_values.first().cloned() {
                let mut arr_clone = arr;
                let arr_mut = Arc::make_mut(&mut arr_clone);
                let popped = arr_mut.pop().unwrap_or(Value::Int(0));
                Value::Array(Arc::new(vec![Value::Array(arr_clone), popped]))
            } else {
                Value::Error("pop() requires an array argument".to_string())
            }
        }

        "slice" => {
            if 3 != arg_values.len() {
                strict_arity_error("slice", 3, arg_values.len())
            } else {
                let start = match arg_values.get(1) {
                    Some(value) => match parse_slice_bound(value, "start") {
                        Ok(number) => number,
                        Err(error) => return Some(error),
                    },
                    None => return Some(Value::Error("slice() requires 3 arguments".to_string())),
                };
                let end = match arg_values.get(2) {
                    Some(value) => match parse_slice_bound(value, "end") {
                        Ok(number) => number,
                        Err(error) => return Some(error),
                    },
                    None => return Some(Value::Error("slice() requires 3 arguments".to_string())),
                };

                match arg_values.first() {
                    Some(Value::Array(arr)) => {
                        let (start_idx, end_idx) = normalize_slice_bounds(arr.len(), start, end);
                        Value::Array(Arc::new(arr[start_idx..end_idx].to_vec()))
                    }
                    Some(Value::Bytes(bytes)) => {
                        let (start_idx, end_idx) = normalize_slice_bounds(bytes.len(), start, end);
                        Value::Bytes(bytes[start_idx..end_idx].to_vec())
                    }
                    _ => Value::Error(
                        "slice() requires array or bytes and numeric start/end arguments"
                            .to_string(),
                    ),
                }
            }
        }

        "concat" => {
            if 2 != arg_values.len() {
                strict_arity_error("concat", 2, arg_values.len())
            } else if let (Some(Value::Array(arr1)), Some(Value::Array(arr2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = Vec::with_capacity(arr1.len() + arr2.len());
                result.extend((**arr1).iter().cloned());
                result.extend((**arr2).iter().cloned());
                Value::Array(Arc::new(result))
            } else {
                Value::Error("concat() requires two array arguments".to_string())
            }
        }

        "insert" => {
            if 3 != arg_values.len() {
                strict_arity_error("insert", 3, arg_values.len())
            } else if let (Some(Value::Array(arr)), Some(index_val), Some(item)) =
                (arg_values.first().cloned(), arg_values.get(1), arg_values.get(2).cloned())
            {
                let index = match index_val {
                    Value::Int(n) => *n,
                    Value::Float(n) => *n as i64,
                    _ => return Some(Value::Error("insert() index must be a number".to_string())),
                };

                match builtins::array_insert((*arr).clone(), index, item) {
                    Ok(new_arr) => Value::Array(Arc::new(new_arr)),
                    Err(e) => Value::Error((*e).clone()),
                }
            } else {
                Value::Error("insert() requires 3 arguments: array, index, and item".to_string())
            }
        }

        "remove" => {
            if 2 != arg_values.len() {
                strict_arity_error("remove", 2, arg_values.len())
            } else {
                match (arg_values.first().cloned(), arg_values.get(1)) {
                    (Some(Value::Array(arr)), Some(item)) => {
                        Value::Array(Arc::new(builtins::array_remove((*arr).clone(), item)))
                    }
                    (Some(Value::Dict(dict)), Some(Value::Str(key))) => {
                        let mut dict_clone = dict.clone();
                        let dict_mut = Arc::make_mut(&mut dict_clone);
                        let removed = dict_mut.remove(key.as_str()).unwrap_or(Value::Int(0));
                        Value::Array(Arc::new(vec![Value::Dict(dict_clone), removed]))
                    }
                    (Some(Value::FixedDict { keys, values }), Some(Value::Str(key))) => {
                        let mut dict = fixed_dict_to_dict(keys.as_ref(), values.as_ref());
                        let removed = dict.remove(key.as_str()).unwrap_or(Value::Int(0));
                        Value::Array(Arc::new(vec![Value::Dict(Arc::new(dict)), removed]))
                    }
                    (Some(Value::IntDict(dict)), Some(key_val)) => {
                        let int_key = match key_val {
                            Value::Int(i) => Some(*i),
                            Value::Str(key) => key.parse::<i64>().ok(),
                            _ => None,
                        };
                        let mut dict_clone = dict.clone();
                        let dict_mut = Arc::make_mut(&mut dict_clone);
                        let removed = if let Some(key) = int_key {
                            dict_mut.remove(&key).unwrap_or(Value::Int(0))
                        } else {
                            Value::Int(0)
                        };
                        Value::Array(Arc::new(vec![Value::IntDict(dict_clone), removed]))
                    }
                    (Some(Value::DenseIntDict(values)), Some(key_val)) => {
                        let int_key = match key_val {
                            Value::Int(i) => Some(*i),
                            Value::Str(key) => key.parse::<i64>().ok(),
                            _ => None,
                        };
                        if let Some(key) = int_key {
                            if key >= 0 && (key as usize) < values.len() {
                                let mut int_dict = IntDictMap::default();
                                int_dict.reserve(values.len());
                                for (index, value) in values.iter().enumerate() {
                                    int_dict.insert(index as i64, value.clone());
                                }
                                let removed = int_dict.remove(&key).unwrap_or(Value::Int(0));
                                Value::Array(Arc::new(vec![
                                    Value::IntDict(Arc::new(int_dict)),
                                    removed,
                                ]))
                            } else {
                                Value::Array(Arc::new(vec![
                                    Value::DenseIntDict(values),
                                    Value::Int(0),
                                ]))
                            }
                        } else {
                            Value::Array(Arc::new(vec![Value::DenseIntDict(values), Value::Int(0)]))
                        }
                    }
                    (Some(Value::DenseIntDictInt(values)), Some(key_val)) => {
                        let int_key = match key_val {
                            Value::Int(i) => Some(*i),
                            Value::Str(key) => key.parse::<i64>().ok(),
                            _ => None,
                        };
                        if let Some(key) = int_key {
                            if key >= 0 && (key as usize) < values.len() {
                                let mut int_dict = IntDictMap::default();
                                int_dict.reserve(values.len());
                                for (index, value) in values.iter().enumerate() {
                                    int_dict.insert(
                                        index as i64,
                                        (*value).map(Value::Int).unwrap_or(Value::Null),
                                    );
                                }
                                let removed = int_dict.remove(&key).unwrap_or(Value::Int(0));
                                Value::Array(Arc::new(vec![
                                    Value::IntDict(Arc::new(int_dict)),
                                    removed,
                                ]))
                            } else {
                                Value::Array(Arc::new(vec![
                                    Value::DenseIntDictInt(values),
                                    Value::Int(0),
                                ]))
                            }
                        } else {
                            Value::Array(Arc::new(vec![
                                Value::DenseIntDictInt(values),
                                Value::Int(0),
                            ]))
                        }
                    }
                    (Some(Value::DenseIntDictIntFull(values)), Some(key_val)) => {
                        let int_key = match key_val {
                            Value::Int(i) => Some(*i),
                            Value::Str(key) => key.parse::<i64>().ok(),
                            _ => None,
                        };
                        if let Some(key) = int_key {
                            if key >= 0 && (key as usize) < values.len() {
                                let mut int_dict = IntDictMap::default();
                                int_dict.reserve(values.len());
                                for (index, value) in values.iter().enumerate() {
                                    int_dict.insert(index as i64, Value::Int(*value));
                                }
                                let removed = int_dict.remove(&key).unwrap_or(Value::Int(0));
                                Value::Array(Arc::new(vec![
                                    Value::IntDict(Arc::new(int_dict)),
                                    removed,
                                ]))
                            } else {
                                Value::Array(Arc::new(vec![
                                    Value::DenseIntDictIntFull(values),
                                    Value::Int(0),
                                ]))
                            }
                        } else {
                            Value::Array(Arc::new(vec![
                                Value::DenseIntDictIntFull(values),
                                Value::Int(0),
                            ]))
                        }
                    }
                    _ => Value::Error(
                        "remove() requires an array or dictionary-like value with a valid key/item"
                            .to_string(),
                    ),
                }
            }
        }

        "remove_at" => {
            if 2 != arg_values.len() {
                strict_arity_error("remove_at", 2, arg_values.len())
            } else if let (Some(Value::Array(arr)), Some(index_val)) =
                (arg_values.first().cloned(), arg_values.get(1))
            {
                let index = match index_val {
                    Value::Int(n) => *n,
                    Value::Float(n) => *n as i64,
                    _ => {
                        return Some(Value::Error("remove_at() index must be a number".to_string()))
                    }
                };

                match builtins::array_remove_at((*arr).clone(), index) {
                    Ok((new_arr, removed)) => {
                        Value::Array(Arc::new(vec![Value::Array(Arc::new(new_arr)), removed]))
                    }
                    Err(e) => Value::Error((*e).clone()),
                }
            } else {
                Value::Error("remove_at() requires 2 arguments: array and index".to_string())
            }
        }

        "clear" => {
            if 1 != arg_values.len() {
                strict_arity_error("clear", 1, arg_values.len())
            } else {
                match arg_values.first() {
                    Some(Value::Array(_)) => Value::Array(Arc::new(builtins::array_clear())),
                    Some(Value::Dict(_)) => Value::Dict(Arc::new(DictMap::default())),
                    Some(Value::FixedDict { .. }) => Value::Dict(Arc::new(DictMap::default())),
                    _ => Value::Error("clear() requires an array or dict argument".to_string()),
                }
            }
        }

        // Array higher-order functions that need interpreter
        "map" => {
            if 2 != arg_values.len() {
                return Some(strict_arity_error("map", 2, arg_values.len()));
            }

            if arg_values.len() < 2 {
                return Some(Value::Error(
                    "map requires two arguments: array and function".to_string(),
                ));
            }

            let (array, func) = match (arg_values.first(), arg_values.get(1)) {
                (
                    Some(Value::Array(arr)),
                    Some(func @ Value::Function(_, _, _))
                    | Some(func @ Value::BytecodeFunction { .. }),
                ) => (arr.clone(), func.clone()),
                _ => {
                    if std::env::var("DEBUG_VM").is_ok() {
                        eprintln!(
                            "map arg types: first={:?}, second={:?}",
                            arg_values.first(),
                            arg_values.get(1)
                        );
                    }
                    return Some(Value::Error("map expects an array and a function".to_string()));
                }
            };

            let mut result = Vec::new();
            for element in array.iter() {
                let func_result = interp.call_user_function(&func, &[element.clone()]);
                result.push(func_result);
            }
            Value::Array(Arc::new(result))
        }

        "filter" => {
            if 2 != arg_values.len() {
                return Some(strict_arity_error("filter", 2, arg_values.len()));
            }

            if arg_values.len() < 2 {
                return Some(Value::Error(
                    "filter requires two arguments: array and function".to_string(),
                ));
            }

            let (array, func) = match (arg_values.first(), arg_values.get(1)) {
                (
                    Some(Value::Array(arr)),
                    Some(func @ Value::Function(_, _, _))
                    | Some(func @ Value::BytecodeFunction { .. }),
                ) => (arr.clone(), func.clone()),
                _ => {
                    return Some(Value::Error("filter expects an array and a function".to_string()))
                }
            };

            let mut result = Vec::new();
            for element in array.iter() {
                let func_result = interp.call_user_function(&func, &[element.clone()]);
                if func_result.is_truthy() {
                    result.push(element.clone());
                }
            }
            Value::Array(Arc::new(result))
        }

        "reduce" => {
            if 3 != arg_values.len() {
                return Some(strict_arity_error("reduce", 3, arg_values.len()));
            }

            if arg_values.len() < 3 {
                return Some(Value::Error(
                    "reduce requires three arguments: array, initial value, and function"
                        .to_string(),
                ));
            }

            let (array, initial, func) =
                match (arg_values.first(), arg_values.get(1), arg_values.get(2)) {
                    (
                        Some(Value::Array(arr)),
                        Some(init),
                        Some(func @ Value::Function(_, _, _))
                        | Some(func @ Value::BytecodeFunction { .. }),
                    ) => (arr.clone(), init.clone(), func.clone()),
                    _ => {
                        return Some(Value::Error(
                            "reduce expects an array, an initial value, and a function".to_string(),
                        ))
                    }
                };

            let mut accumulator = initial;
            for element in array.iter() {
                accumulator = interp.call_user_function(&func, &[accumulator, element.clone()]);
            }
            accumulator
        }

        "find" => {
            if 2 != arg_values.len() {
                return Some(strict_arity_error("find", 2, arg_values.len()));
            }

            if arg_values.len() < 2 {
                return Some(Value::Error(
                    "find requires two arguments: array and function".to_string(),
                ));
            }

            let (array, func) = match (arg_values.first(), arg_values.get(1)) {
                (
                    Some(Value::Array(arr)),
                    Some(func @ Value::Function(_, _, _))
                    | Some(func @ Value::BytecodeFunction { .. }),
                ) => (arr.clone(), func.clone()),
                _ => return Some(Value::Error("find expects an array and a function".to_string())),
            };

            for element in array.iter() {
                let func_result = interp.call_user_function(&func, &[element.clone()]);
                if func_result.is_truthy() {
                    return Some(element.clone());
                }
            }
            Value::Int(0)
        }

        "any" => {
            if 2 != arg_values.len() {
                return Some(strict_arity_error("any", 2, arg_values.len()));
            }

            if arg_values.len() < 2 {
                return Some(Value::Error(
                    "any requires two arguments: array and function".to_string(),
                ));
            }

            let (array, func) = match (arg_values.first(), arg_values.get(1)) {
                (
                    Some(Value::Array(arr)),
                    Some(func @ Value::Function(_, _, _))
                    | Some(func @ Value::BytecodeFunction { .. }),
                ) => (arr.clone(), func.clone()),
                _ => return Some(Value::Error("any expects an array and a function".to_string())),
            };

            for element in array.iter() {
                let func_result = interp.call_user_function(&func, &[element.clone()]);
                if func_result.is_truthy() {
                    return Some(Value::Bool(true));
                }
            }
            Value::Bool(false)
        }

        "all" => {
            if 2 != arg_values.len() {
                return Some(strict_arity_error("all", 2, arg_values.len()));
            }

            if arg_values.len() < 2 {
                return Some(Value::Error(
                    "all requires two arguments: array and function".to_string(),
                ));
            }

            let (array, func) = match (arg_values.first(), arg_values.get(1)) {
                (
                    Some(Value::Array(arr)),
                    Some(func @ Value::Function(_, _, _))
                    | Some(func @ Value::BytecodeFunction { .. }),
                ) => (arr.clone(), func.clone()),
                _ => return Some(Value::Error("all expects an array and a function".to_string())),
            };

            for element in array.iter() {
                let func_result = interp.call_user_function(&func, &[element.clone()]);
                if !func_result.is_truthy() {
                    return Some(Value::Bool(false));
                }
            }
            Value::Bool(true)
        }

        "sort" => {
            if 1 != arg_values.len() {
                strict_arity_error("sort", 1, arg_values.len())
            } else if let Some(Value::Array(arr)) = arg_values.first() {
                let mut sorted = (**arr).clone();
                sorted.sort_by(|a, b| match (a, b) {
                    (Value::Int(x), Value::Int(y)) => x.cmp(y),
                    (Value::Float(x), Value::Float(y)) => {
                        x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Value::Int(x), Value::Float(y)) => {
                        (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Value::Float(x), Value::Int(y)) => {
                        x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Value::Str(x), Value::Str(y)) => x.as_ref().cmp(y.as_ref()),
                    _ => std::cmp::Ordering::Equal,
                });
                Value::Array(Arc::new(sorted))
            } else {
                Value::Error("sort requires an array argument".to_string())
            }
        }

        "reverse" => {
            if 1 != arg_values.len() {
                strict_arity_error("reverse", 1, arg_values.len())
            } else if let Some(Value::Array(arr)) = arg_values.first() {
                let mut reversed = (**arr).clone();
                reversed.reverse();
                Value::Array(Arc::new(reversed))
            } else {
                Value::Error("reverse requires an array argument".to_string())
            }
        }

        "unique" => {
            if 1 != arg_values.len() {
                strict_arity_error("unique", 1, arg_values.len())
            } else if let Some(Value::Array(arr)) = arg_values.first() {
                let mut seen = HashSet::new();
                let mut result = Vec::new();

                for element in arr.iter() {
                    let key = format!("{:?}", element);
                    if seen.insert(key) {
                        result.push(element.clone());
                    }
                }
                Value::Array(Arc::new(result))
            } else {
                Value::Error("unique requires an array argument".to_string())
            }
        }

        "sum" => {
            if 1 != arg_values.len() {
                strict_arity_error("sum", 1, arg_values.len())
            } else if let Some(Value::Array(arr)) = arg_values.first() {
                let mut int_sum: i64 = 0;
                let mut float_sum: f64 = 0.0;
                let mut has_float = false;

                for element in arr.iter() {
                    match element {
                        Value::Int(n) => {
                            if has_float {
                                float_sum += *n as f64;
                            } else {
                                int_sum += n;
                            }
                        }
                        Value::Float(n) => {
                            if !has_float {
                                float_sum = int_sum as f64;
                                has_float = true;
                            }
                            float_sum += n;
                        }
                        _ => {}
                    }
                }

                if has_float {
                    Value::Float(float_sum)
                } else {
                    Value::Int(int_sum)
                }
            } else {
                Value::Error("sum requires an array argument".to_string())
            }
        }

        "chunk" => {
            if 2 != arg_values.len() {
                strict_arity_error("chunk", 2, arg_values.len())
            } else if let (Some(Value::Array(arr)), Some(size_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let size = match size_val {
                    Value::Int(n) => *n,
                    Value::Float(n) => *n as i64,
                    _ => return Some(Value::Error("chunk() size must be a number".to_string())),
                };
                Value::Array(Arc::new(builtins::array_chunk(&**arr, size)))
            } else {
                Value::Error("chunk() requires 2 arguments: array and size".to_string())
            }
        }

        "flatten" => {
            if 1 != arg_values.len() {
                strict_arity_error("flatten", 1, arg_values.len())
            } else if let Some(Value::Array(arr)) = arg_values.first() {
                Value::Array(Arc::new(builtins::array_flatten(&**arr)))
            } else {
                Value::Error("flatten() requires an array argument".to_string())
            }
        }

        "zip" => {
            if 2 != arg_values.len() {
                strict_arity_error("zip", 2, arg_values.len())
            } else if let (Some(Value::Array(arr1)), Some(Value::Array(arr2))) =
                (arg_values.first(), arg_values.get(1))
            {
                Value::Array(Arc::new(builtins::array_zip(&**arr1, &**arr2)))
            } else {
                Value::Error("zip() requires 2 array arguments".to_string())
            }
        }

        "enumerate" => {
            if 1 != arg_values.len() {
                strict_arity_error("enumerate", 1, arg_values.len())
            } else if let Some(Value::Array(arr)) = arg_values.first() {
                Value::Array(Arc::new(builtins::array_enumerate(&**arr)))
            } else {
                Value::Error("enumerate() requires an array argument".to_string())
            }
        }

        "take" => {
            if 2 != arg_values.len() {
                strict_arity_error("take", 2, arg_values.len())
            } else if let (Some(Value::Array(arr)), Some(n_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let n = match n_val {
                    Value::Int(n) => *n,
                    Value::Float(n) => *n as i64,
                    _ => return Some(Value::Error("take() count must be a number".to_string())),
                };
                Value::Array(Arc::new(builtins::array_take(&**arr, n)))
            } else {
                Value::Error("take() requires 2 arguments: array and count".to_string())
            }
        }

        "skip" => {
            if 2 != arg_values.len() {
                strict_arity_error("skip", 2, arg_values.len())
            } else if let (Some(Value::Array(arr)), Some(n_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let n = match n_val {
                    Value::Int(n) => *n,
                    Value::Float(n) => *n as i64,
                    _ => return Some(Value::Error("skip() count must be a number".to_string())),
                };
                Value::Array(Arc::new(builtins::array_skip(&**arr, n)))
            } else {
                Value::Error("skip() requires 2 arguments: array and count".to_string())
            }
        }

        "windows" => {
            if 2 != arg_values.len() {
                strict_arity_error("windows", 2, arg_values.len())
            } else if let (Some(Value::Array(arr)), Some(size_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let size = match size_val {
                    Value::Int(n) => *n,
                    Value::Float(n) => *n as i64,
                    _ => return Some(Value::Error("windows() size must be a number".to_string())),
                };
                Value::Array(Arc::new(builtins::array_windows(&**arr, size)))
            } else {
                Value::Error("windows() requires 2 arguments: array and size".to_string())
            }
        }

        "range" => match builtins::range(&arg_values) {
            Ok(arr) => Value::Array(Arc::new(arr)),
            Err(e) => Value::Error(e),
        },

        // Dict functions
        "keys" => {
            if let Some(Value::Dict(dict)) = arg_values.first() {
                let mut keys: Vec<String> = Vec::with_capacity(dict.len());
                for key in dict.keys() {
                    keys.push(key.to_string());
                }
                keys.sort();
                let keys: Vec<Value> = keys.into_iter().map(|k| Value::Str(Arc::new(k))).collect();
                Value::Array(Arc::new(keys))
            } else if let Some(Value::FixedDict { keys, .. }) = arg_values.first() {
                let mut key_strings: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
                key_strings.sort();
                let keys: Vec<Value> =
                    key_strings.into_iter().map(|k| Value::Str(Arc::new(k))).collect();
                Value::Array(Arc::new(keys))
            } else if let Some(Value::IntDict(dict)) = arg_values.first() {
                let mut keys: Vec<i64> = dict.keys().copied().collect();
                keys.sort();
                let keys: Vec<Value> =
                    keys.into_iter().map(|k| Value::Str(Arc::new(k.to_string()))).collect();
                Value::Array(Arc::new(keys))
            } else if let Some(Value::DenseIntDict(values)) = arg_values.first() {
                let keys: Vec<Value> =
                    (0..values.len()).map(|k| Value::Str(Arc::new(k.to_string()))).collect();
                Value::Array(Arc::new(keys))
            } else if let Some(Value::DenseIntDictInt(values)) = arg_values.first() {
                let keys: Vec<Value> =
                    (0..values.len()).map(|k| Value::Str(Arc::new(k.to_string()))).collect();
                Value::Array(Arc::new(keys))
            } else if let Some(Value::DenseIntDictIntFull(values)) = arg_values.first() {
                let keys: Vec<Value> =
                    (0..values.len()).map(|k| Value::Str(Arc::new(k.to_string()))).collect();
                Value::Array(Arc::new(keys))
            } else {
                Value::Array(Arc::new(vec![]))
            }
        }

        "values" => {
            if let Some(Value::Dict(dict)) = arg_values.first() {
                let mut keys: Vec<&Arc<str>> = Vec::with_capacity(dict.len());
                for key in dict.keys() {
                    keys.push(key);
                }
                keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                let vals: Vec<Value> =
                    keys.iter().map(|k| dict.get(k.as_ref()).unwrap().clone()).collect();
                Value::Array(Arc::new(vals))
            } else if let Some(Value::FixedDict { keys, values }) = arg_values.first() {
                let mut pairs: Vec<(&Arc<str>, &Value)> = keys.iter().zip(values.iter()).collect();
                pairs.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
                let vals: Vec<Value> = pairs.iter().map(|(_, v)| (*v).clone()).collect();
                Value::Array(Arc::new(vals))
            } else if let Some(Value::IntDict(dict)) = arg_values.first() {
                let mut keys: Vec<i64> = dict.keys().copied().collect();
                keys.sort();
                let vals: Vec<Value> = keys.iter().map(|k| dict.get(k).unwrap().clone()).collect();
                Value::Array(Arc::new(vals))
            } else if let Some(Value::DenseIntDict(values)) = arg_values.first() {
                Value::Array(Arc::new(values.iter().cloned().collect()))
            } else if let Some(Value::DenseIntDictInt(values)) = arg_values.first() {
                Value::Array(Arc::new(
                    values
                        .iter()
                        .map(|value| (*value).map(Value::Int).unwrap_or(Value::Null))
                        .collect(),
                ))
            } else if let Some(Value::DenseIntDictIntFull(values)) = arg_values.first() {
                Value::Array(Arc::new(values.iter().map(|value| Value::Int(*value)).collect()))
            } else {
                Value::Array(Arc::new(vec![]))
            }
        }

        "has_key" => {
            if let (Some(Value::Dict(dict)), Some(Value::Str(key))) =
                (arg_values.first(), arg_values.get(1))
            {
                Value::Int(if dict.contains_key(key.as_str()) { 1 } else { 0 })
            } else if let (Some(Value::FixedDict { keys, .. }), Some(Value::Str(key))) =
                (arg_values.first(), arg_values.get(1))
            {
                Value::Int(if keys.iter().any(|k| k.as_ref() == key.as_str()) { 1 } else { 0 })
            } else if let (Some(Value::IntDict(dict)), Some(key_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                Value::Int(if int_key.is_some() && dict.contains_key(&int_key.unwrap()) {
                    1
                } else {
                    0
                })
            } else if let (Some(Value::DenseIntDict(values)), Some(key_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                let has_key =
                    int_key.map(|key| key >= 0 && (key as usize) < values.len()).unwrap_or(false);
                Value::Int(if has_key { 1 } else { 0 })
            } else if let (Some(Value::DenseIntDictInt(values)), Some(key_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                let has_key =
                    int_key.map(|key| key >= 0 && (key as usize) < values.len()).unwrap_or(false);
                Value::Int(if has_key { 1 } else { 0 })
            } else if let (Some(Value::DenseIntDictIntFull(values)), Some(key_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                let has_key =
                    int_key.map(|key| key >= 0 && (key as usize) < values.len()).unwrap_or(false);
                Value::Int(if has_key { 1 } else { 0 })
            } else {
                Value::Int(0)
            }
        }

        "items" => {
            if let Some(Value::Dict(dict)) = arg_values.first() {
                let mut keys: Vec<&Arc<str>> = Vec::with_capacity(dict.len());
                for key in dict.keys() {
                    keys.push(key);
                }
                keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
                let items: Vec<Value> = keys
                    .iter()
                    .map(|k| {
                        Value::Array(Arc::new(vec![
                            Value::Str(Arc::new(k.to_string())),
                            dict.get(k.as_ref()).unwrap().clone(),
                        ]))
                    })
                    .collect();
                Value::Array(Arc::new(items))
            } else if let Some(Value::FixedDict { keys, values }) = arg_values.first() {
                let mut pairs: Vec<(&Arc<str>, &Value)> = keys.iter().zip(values.iter()).collect();
                pairs.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
                let items: Vec<Value> = pairs
                    .iter()
                    .map(|(k, v)| {
                        Value::Array(Arc::new(vec![
                            Value::Str(Arc::new(k.to_string())),
                            (*v).clone(),
                        ]))
                    })
                    .collect();
                Value::Array(Arc::new(items))
            } else if let Some(Value::IntDict(dict)) = arg_values.first() {
                let mut keys: Vec<i64> = dict.keys().copied().collect();
                keys.sort();
                let items: Vec<Value> = keys
                    .iter()
                    .map(|k| {
                        Value::Array(Arc::new(vec![
                            Value::Str(Arc::new(k.to_string())),
                            dict.get(k).unwrap().clone(),
                        ]))
                    })
                    .collect();
                Value::Array(Arc::new(items))
            } else if let Some(Value::DenseIntDict(values)) = arg_values.first() {
                let items: Vec<Value> = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        Value::Array(Arc::new(vec![
                            Value::Str(Arc::new(index.to_string())),
                            value.clone(),
                        ]))
                    })
                    .collect();
                Value::Array(Arc::new(items))
            } else if let Some(Value::DenseIntDictInt(values)) = arg_values.first() {
                let items: Vec<Value> = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        Value::Array(Arc::new(vec![
                            Value::Str(Arc::new(index.to_string())),
                            (*value).map(Value::Int).unwrap_or(Value::Null),
                        ]))
                    })
                    .collect();
                Value::Array(Arc::new(items))
            } else if let Some(Value::DenseIntDictIntFull(values)) = arg_values.first() {
                let items: Vec<Value> = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        Value::Array(Arc::new(vec![
                            Value::Str(Arc::new(index.to_string())),
                            Value::Int(*value),
                        ]))
                    })
                    .collect();
                Value::Array(Arc::new(items))
            } else {
                Value::Array(Arc::new(vec![]))
            }
        }

        "get" => {
            if let (Some(Value::Dict(dict)), Some(Value::Str(key))) =
                (arg_values.first(), arg_values.get(1))
            {
                let default = arg_values.get(2).cloned().unwrap_or(Value::Null);
                dict.get(key.as_str()).cloned().unwrap_or(default)
            } else if let (Some(Value::FixedDict { keys, values }), Some(Value::Str(key))) =
                (arg_values.first(), arg_values.get(1))
            {
                let default = arg_values.get(2).cloned().unwrap_or(Value::Null);
                let idx = keys.iter().position(|k| k.as_ref() == key.as_str());
                idx.and_then(|i| values.get(i).cloned()).unwrap_or(default)
            } else if let (Some(Value::IntDict(dict)), Some(key_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let default = arg_values.get(2).cloned().unwrap_or(Value::Null);
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                if let Some(key) = int_key {
                    dict.get(&key).cloned().unwrap_or(default)
                } else {
                    default
                }
            } else if let (Some(Value::DenseIntDict(values)), Some(key_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let default = arg_values.get(2).cloned().unwrap_or(Value::Null);
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                if let Some(key) = int_key {
                    if key < 0 {
                        default
                    } else {
                        values.get(key as usize).cloned().unwrap_or(default)
                    }
                } else {
                    default
                }
            } else if let (Some(Value::DenseIntDictInt(values)), Some(key_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let default = arg_values.get(2).cloned().unwrap_or(Value::Null);
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                if let Some(key) = int_key {
                    if key < 0 {
                        default
                    } else {
                        match values.get(key as usize) {
                            Some(value) => (*value).map(Value::Int).unwrap_or(Value::Null),
                            None => default,
                        }
                    }
                } else {
                    default
                }
            } else if let (Some(Value::DenseIntDictIntFull(values)), Some(key_val)) =
                (arg_values.first(), arg_values.get(1))
            {
                let default = arg_values.get(2).cloned().unwrap_or(Value::Null);
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                if let Some(key) = int_key {
                    if key < 0 {
                        default
                    } else {
                        values.get(key as usize).map(|value| Value::Int(*value)).unwrap_or(default)
                    }
                } else {
                    default
                }
            } else {
                Value::Null
            }
        }

        "merge" => {
            if let (Some(Value::Dict(dict1)), Some(Value::Dict(dict2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**dict1).clone();
                for (k, v) in dict2.iter() {
                    result.insert(k.clone(), v.clone());
                }
                Value::Dict(Arc::new(result))
            } else if let (
                Some(Value::FixedDict { keys: keys1, values: values1 }),
                Some(Value::Dict(dict2)),
            ) = (arg_values.first(), arg_values.get(1))
            {
                let mut result = fixed_dict_to_dict(keys1.as_ref(), values1.as_ref());
                for (k, v) in dict2.iter() {
                    result.insert(k.clone(), v.clone());
                }
                Value::Dict(Arc::new(result))
            } else if let (
                Some(Value::Dict(dict1)),
                Some(Value::FixedDict { keys: keys2, values: values2 }),
            ) = (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**dict1).clone();
                for (k, v) in fixed_dict_to_dict(keys2.as_ref(), values2.as_ref()) {
                    result.insert(k, v);
                }
                Value::Dict(Arc::new(result))
            } else if let (
                Some(Value::FixedDict { keys: keys1, values: values1 }),
                Some(Value::FixedDict { keys: keys2, values: values2 }),
            ) = (arg_values.first(), arg_values.get(1))
            {
                let mut result = fixed_dict_to_dict(keys1.as_ref(), values1.as_ref());
                for (k, v) in fixed_dict_to_dict(keys2.as_ref(), values2.as_ref()) {
                    result.insert(k, v);
                }
                Value::Dict(Arc::new(result))
            } else if let (Some(Value::IntDict(dict1)), Some(Value::IntDict(dict2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**dict1).clone();
                for (k, v) in dict2.iter() {
                    result.insert(*k, v.clone());
                }
                Value::IntDict(Arc::new(result))
            } else if let (Some(Value::DenseIntDict(values1)), Some(Value::DenseIntDict(values2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**values1).clone();
                if values2.len() > result.len() {
                    result.resize(values2.len(), Value::Null);
                }
                for (index, value) in values2.iter().enumerate() {
                    result[index] = value.clone();
                }
                Value::DenseIntDict(Arc::new(result))
            } else if let (
                Some(Value::DenseIntDictInt(values1)),
                Some(Value::DenseIntDictInt(values2)),
            ) = (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**values1).clone();
                if values2.len() > result.len() {
                    result.resize(values2.len(), None);
                }
                for (index, value) in values2.iter().enumerate() {
                    result[index] = *value;
                }
                Value::DenseIntDictInt(Arc::new(result))
            } else if let (
                Some(Value::DenseIntDictIntFull(values1)),
                Some(Value::DenseIntDictIntFull(values2)),
            ) = (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**values1).clone();
                if values2.len() > result.len() {
                    result.resize(values2.len(), 0);
                }
                for (index, value) in values2.iter().enumerate() {
                    result[index] = *value;
                }
                Value::DenseIntDictIntFull(Arc::new(result))
            } else {
                Value::Dict(Arc::new(DictMap::default()))
            }
        }

        "invert" => {
            if let Some(Value::Dict(dict)) = arg_values.first() {
                Value::Dict(Arc::new(builtins::dict_invert(&**dict)))
            } else if let Some(Value::FixedDict { keys, values }) = arg_values.first() {
                let dict = fixed_dict_to_dict(keys.as_ref(), values.as_ref());
                let inverted = builtins::dict_invert(&dict);
                Value::Dict(Arc::new(inverted))
            } else if let Some(Value::IntDict(dict)) = arg_values.first() {
                let mut inverted = DictMap::default();
                for (k, v) in dict.iter() {
                    inverted.insert(k.to_string().into(), v.clone());
                }
                Value::Dict(Arc::new(inverted))
            } else if let Some(Value::DenseIntDict(values)) = arg_values.first() {
                let mut inverted = DictMap::default();
                for (index, value) in values.iter().enumerate() {
                    inverted.insert(index.to_string().into(), value.clone());
                }
                Value::Dict(Arc::new(inverted))
            } else if let Some(Value::DenseIntDictInt(values)) = arg_values.first() {
                let mut inverted = DictMap::default();
                for (index, value) in values.iter().enumerate() {
                    inverted.insert(
                        index.to_string().into(),
                        (*value).map(Value::Int).unwrap_or(Value::Null),
                    );
                }
                Value::Dict(Arc::new(inverted))
            } else if let Some(Value::DenseIntDictIntFull(values)) = arg_values.first() {
                let mut inverted = DictMap::default();
                for (index, value) in values.iter().enumerate() {
                    inverted.insert(index.to_string().into(), Value::Int(*value));
                }
                Value::Dict(Arc::new(inverted))
            } else {
                Value::Error("invert() requires a dict argument".to_string())
            }
        }

        "update" => {
            if let (Some(Value::Dict(dict1)), Some(Value::Dict(dict2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**dict1).clone();
                for (k, v) in dict2.iter() {
                    result.insert(k.clone(), v.clone());
                }
                Value::Dict(Arc::new(result))
            } else if let (
                Some(Value::FixedDict { keys: keys1, values: values1 }),
                Some(Value::FixedDict { keys: keys2, values: values2 }),
            ) = (arg_values.first(), arg_values.get(1))
            {
                let mut result = fixed_dict_to_dict(keys1.as_ref(), values1.as_ref());
                for (k, v) in fixed_dict_to_dict(keys2.as_ref(), values2.as_ref()) {
                    result.insert(k, v);
                }
                Value::Dict(Arc::new(result))
            } else if let (Some(Value::FixedDict { keys, values }), Some(Value::Dict(dict2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = fixed_dict_to_dict(keys.as_ref(), values.as_ref());
                for (k, v) in dict2.iter() {
                    result.insert(k.clone(), v.clone());
                }
                Value::Dict(Arc::new(result))
            } else if let (Some(Value::Dict(dict1)), Some(Value::FixedDict { keys, values })) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**dict1).clone();
                for (k, v) in fixed_dict_to_dict(keys.as_ref(), values.as_ref()) {
                    result.insert(k, v);
                }
                Value::Dict(Arc::new(result))
            } else if let (Some(Value::IntDict(dict1)), Some(Value::IntDict(dict2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**dict1).clone();
                for (k, v) in dict2.iter() {
                    result.insert(*k, v.clone());
                }
                Value::IntDict(Arc::new(result))
            } else if let (Some(Value::DenseIntDict(values1)), Some(Value::DenseIntDict(values2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**values1).clone();
                if values2.len() > result.len() {
                    result.resize(values2.len(), Value::Null);
                }
                for (index, value) in values2.iter().enumerate() {
                    result[index] = value.clone();
                }
                Value::DenseIntDict(Arc::new(result))
            } else if let (
                Some(Value::DenseIntDictInt(values1)),
                Some(Value::DenseIntDictInt(values2)),
            ) = (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**values1).clone();
                if values2.len() > result.len() {
                    result.resize(values2.len(), None);
                }
                for (index, value) in values2.iter().enumerate() {
                    result[index] = *value;
                }
                Value::DenseIntDictInt(Arc::new(result))
            } else if let (
                Some(Value::DenseIntDictIntFull(values1)),
                Some(Value::DenseIntDictIntFull(values2)),
            ) = (arg_values.first(), arg_values.get(1))
            {
                let mut result = (**values1).clone();
                if values2.len() > result.len() {
                    result.resize(values2.len(), 0);
                }
                for (index, value) in values2.iter().enumerate() {
                    result[index] = *value;
                }
                Value::DenseIntDictIntFull(Arc::new(result))
            } else {
                Value::Dict(Arc::new(DictMap::default()))
            }
        }

        "get_default" => {
            if let (Some(Value::Dict(dict)), Some(Value::Str(key)), Some(default_val)) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                if let Some(value) = dict.get(key.as_str()) {
                    value.clone()
                } else {
                    default_val.clone()
                }
            } else if let (
                Some(Value::FixedDict { keys, values }),
                Some(Value::Str(key)),
                Some(default_val),
            ) = (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                if let Some(index) = keys.iter().position(|k| k.as_ref() == key.as_str()) {
                    values.get(index).cloned().unwrap_or_else(|| default_val.clone())
                } else {
                    default_val.clone()
                }
            } else if let (Some(Value::IntDict(dict)), Some(key_val), Some(default_val)) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                if let Some(key) = int_key {
                    dict.get(&key).cloned().unwrap_or(default_val.clone())
                } else {
                    default_val.clone()
                }
            } else if let (Some(Value::DenseIntDict(values)), Some(key_val), Some(default_val)) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                if let Some(key) = int_key {
                    if key < 0 {
                        default_val.clone()
                    } else {
                        values.get(key as usize).cloned().unwrap_or(default_val.clone())
                    }
                } else {
                    default_val.clone()
                }
            } else if let (
                Some(Value::DenseIntDictIntFull(values)),
                Some(key_val),
                Some(default_val),
            ) = (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                if let Some(key) = int_key {
                    if key < 0 {
                        default_val.clone()
                    } else {
                        values
                            .get(key as usize)
                            .map(|value| Value::Int(*value))
                            .unwrap_or(default_val.clone())
                    }
                } else {
                    default_val.clone()
                }
            } else if let (Some(Value::DenseIntDictInt(values)), Some(key_val), Some(default_val)) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                let int_key = match key_val {
                    Value::Int(i) => Some(*i),
                    Value::Str(key) => key.parse::<i64>().ok(),
                    _ => None,
                };
                if let Some(key) = int_key {
                    if key < 0 {
                        default_val.clone()
                    } else {
                        match values.get(key as usize) {
                            Some(value) => (*value).map(Value::Int).unwrap_or(Value::Null),
                            None => default_val.clone(),
                        }
                    }
                } else {
                    default_val.clone()
                }
            } else {
                Value::Error(
                    "get_default() requires 3 arguments: dict, key, default_value".to_string(),
                )
            }
        }

        // Set functions
        "Set" => {
            if arg_values.len() > 1 {
                Value::Error("Set constructor takes at most 1 argument".to_string())
            } else if let Some(Value::Array(items)) = arg_values.first() {
                let mut set_items: Vec<Value> = Vec::new();
                for item in items.iter() {
                    let exists =
                        set_items.iter().any(|value| Interpreter::values_equal(value, item));
                    if !exists {
                        set_items.push(item.clone());
                    }
                }
                Value::Set(set_items)
            } else if arg_values.is_empty() {
                Value::Set(Vec::new())
            } else {
                Value::Error("Set constructor requires an array argument".to_string())
            }
        }

        "set_add" => {
            if let (Some(Value::Set(mut set)), Some(item)) =
                (arg_values.first().cloned(), arg_values.get(1).cloned())
            {
                let exists = set.iter().any(|v| Interpreter::values_equal(v, &item));
                if !exists {
                    set.push(item);
                }
                Value::Set(set)
            } else {
                Value::Set(Vec::new())
            }
        }

        "set_has" => {
            if let (Some(Value::Set(set)), Some(item)) = (arg_values.first(), arg_values.get(1)) {
                let exists = set.iter().any(|v| Interpreter::values_equal(v, item));
                Value::Bool(exists)
            } else {
                Value::Bool(false)
            }
        }

        "set_remove" => {
            if let (Some(Value::Set(mut set)), Some(item)) =
                (arg_values.first().cloned(), arg_values.get(1))
            {
                set.retain(|v| !Interpreter::values_equal(v, item));
                Value::Set(set)
            } else {
                Value::Set(Vec::new())
            }
        }

        "set_union" => {
            if let (Some(Value::Set(set1)), Some(Value::Set(set2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let mut result = set1.clone();
                for item in set2 {
                    let exists = result.iter().any(|v| Interpreter::values_equal(v, item));
                    if !exists {
                        result.push(item.clone());
                    }
                }
                Value::Set(result)
            } else {
                Value::Set(Vec::new())
            }
        }

        "set_intersect" => {
            if let (Some(Value::Set(set1)), Some(Value::Set(set2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let result: Vec<Value> = set1
                    .iter()
                    .filter(|v| set2.iter().any(|v2| Interpreter::values_equal(v, v2)))
                    .cloned()
                    .collect();
                Value::Set(result)
            } else {
                Value::Set(Vec::new())
            }
        }

        "set_difference" => {
            if let (Some(Value::Set(set1)), Some(Value::Set(set2))) =
                (arg_values.first(), arg_values.get(1))
            {
                let result: Vec<Value> = set1
                    .iter()
                    .filter(|v| !set2.iter().any(|v2| Interpreter::values_equal(v, v2)))
                    .cloned()
                    .collect();
                Value::Set(result)
            } else {
                Value::Set(Vec::new())
            }
        }

        "set_to_array" => {
            if let Some(Value::Set(set)) = arg_values.first() {
                Value::Array(Arc::new(set.clone()))
            } else {
                Value::Array(Arc::new(Vec::new()))
            }
        }

        // Queue functions
        "Queue" => {
            if let Some(Value::Array(arr)) = arg_values.first() {
                let mut queue = VecDeque::new();
                for item in arr.iter() {
                    queue.push_back(item.clone());
                }
                Value::Queue(queue)
            } else {
                Value::Queue(VecDeque::new())
            }
        }

        "queue_enqueue" => {
            if let (Some(Value::Queue(mut queue)), Some(item)) =
                (arg_values.first().cloned(), arg_values.get(1).cloned())
            {
                queue.push_back(item);
                Value::Queue(queue)
            } else {
                Value::Queue(VecDeque::new())
            }
        }

        "queue_dequeue" => {
            if let Some(Value::Queue(mut queue)) = arg_values.first().cloned() {
                if let Some(item) = queue.pop_front() {
                    Value::Array(Arc::new(vec![Value::Queue(queue), item]))
                } else {
                    Value::Array(Arc::new(vec![Value::Queue(queue), Value::Null]))
                }
            } else {
                Value::Array(Arc::new(vec![Value::Queue(VecDeque::new()), Value::Null]))
            }
        }

        "queue_peek" => {
            if let Some(Value::Queue(queue)) = arg_values.first() {
                queue.front().cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }

        "queue_is_empty" => {
            if let Some(Value::Queue(queue)) = arg_values.first() {
                Value::Bool(queue.is_empty())
            } else {
                Value::Bool(true)
            }
        }

        "queue_size" => {
            if arg_values.len() != 1 {
                Value::Error(format!(
                    "queue_size expects 1 argument (queue), got {}",
                    arg_values.len()
                ))
            } else if let Some(Value::Queue(queue)) = arg_values.first() {
                Value::Int(queue.len() as i64)
            } else {
                Value::Error("queue_size requires a Queue argument".to_string())
            }
        }

        "queue_to_array" => {
            if let Some(Value::Queue(queue)) = arg_values.first() {
                Value::Array(Arc::new(queue.iter().cloned().collect()))
            } else {
                Value::Array(Arc::new(Vec::new()))
            }
        }

        // Stack functions
        "Stack" => {
            if let Some(Value::Array(arr)) = arg_values.first() {
                Value::Stack((**arr).clone())
            } else {
                Value::Stack(Vec::new())
            }
        }

        "stack_push" => {
            if let (Some(Value::Stack(mut stack)), Some(item)) =
                (arg_values.first().cloned(), arg_values.get(1).cloned())
            {
                stack.push(item);
                Value::Stack(stack)
            } else {
                Value::Stack(Vec::new())
            }
        }

        "stack_pop" => {
            if let Some(Value::Stack(mut stack)) = arg_values.first().cloned() {
                if let Some(item) = stack.pop() {
                    Value::Array(Arc::new(vec![Value::Stack(stack), item]))
                } else {
                    Value::Array(Arc::new(vec![Value::Stack(stack), Value::Null]))
                }
            } else {
                Value::Array(Arc::new(vec![Value::Stack(Vec::new()), Value::Null]))
            }
        }

        "stack_peek" => {
            if let Some(Value::Stack(stack)) = arg_values.first() {
                stack.last().cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }

        "stack_is_empty" => {
            if let Some(Value::Stack(stack)) = arg_values.first() {
                Value::Bool(stack.is_empty())
            } else {
                Value::Bool(true)
            }
        }

        "stack_size" => {
            if arg_values.len() != 1 {
                Value::Error(format!(
                    "stack_size expects 1 argument (stack), got {}",
                    arg_values.len()
                ))
            } else if let Some(Value::Stack(stack)) = arg_values.first() {
                Value::Int(stack.len() as i64)
            } else {
                Value::Error("stack_size requires a Stack argument".to_string())
            }
        }

        "stack_to_array" => {
            if let Some(Value::Stack(stack)) = arg_values.first() {
                Value::Array(Arc::new(stack.clone()))
            } else {
                Value::Array(Arc::new(Vec::new()))
            }
        }

        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::interpreter::{Interpreter, LeakyFunctionBody, Value};
    use std::sync::Arc;

    #[test]
    fn test_array_and_higher_order_collection_strict_arity_contracts() {
        let mut interpreter = Interpreter::new();
        let array_arg = Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
        let function_arg = Value::Function(vec![], LeakyFunctionBody::new(Vec::new()), None);

        let strict_arity_cases: Vec<(&str, Vec<Value>, &str)> = vec![
            ("push", vec![array_arg.clone()], "expects 2 arguments"),
            ("pop", vec![], "expects 1 argument"),
            ("slice", vec![array_arg.clone(), Value::Int(0)], "expects 3 arguments"),
            (
                "concat",
                vec![array_arg.clone(), array_arg.clone(), Value::Int(7)],
                "expects 2 arguments",
            ),
            (
                "insert",
                vec![array_arg.clone(), Value::Int(0), Value::Int(9), Value::Int(1)],
                "expects 3 arguments",
            ),
            (
                "remove",
                vec![array_arg.clone(), Value::Int(2), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "remove_at",
                vec![array_arg.clone(), Value::Int(0), Value::Int(1)],
                "expects 2 arguments",
            ),
            ("clear", vec![], "expects 1 argument"),
            (
                "map",
                vec![array_arg.clone(), function_arg.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "filter",
                vec![array_arg.clone(), function_arg.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "reduce",
                vec![array_arg.clone(), Value::Int(0), function_arg.clone(), Value::Int(1)],
                "expects 3 arguments",
            ),
            (
                "find",
                vec![array_arg.clone(), function_arg.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "any",
                vec![array_arg.clone(), function_arg.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            (
                "all",
                vec![array_arg.clone(), function_arg.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            ("sort", vec![array_arg.clone(), Value::Int(1)], "expects 1 argument"),
            ("reverse", vec![array_arg.clone(), Value::Int(1)], "expects 1 argument"),
            ("unique", vec![array_arg.clone(), Value::Int(1)], "expects 1 argument"),
            ("sum", vec![array_arg.clone(), Value::Int(1)], "expects 1 argument"),
            ("chunk", vec![array_arg.clone(), Value::Int(2), Value::Int(1)], "expects 2 arguments"),
            ("flatten", vec![array_arg.clone(), Value::Int(1)], "expects 1 argument"),
            (
                "zip",
                vec![array_arg.clone(), array_arg.clone(), Value::Int(1)],
                "expects 2 arguments",
            ),
            ("enumerate", vec![array_arg.clone(), Value::Int(1)], "expects 1 argument"),
            ("take", vec![array_arg.clone(), Value::Int(2), Value::Int(1)], "expects 2 arguments"),
            ("skip", vec![array_arg.clone(), Value::Int(2), Value::Int(1)], "expects 2 arguments"),
            (
                "windows",
                vec![array_arg.clone(), Value::Int(2), Value::Int(1)],
                "expects 2 arguments",
            ),
        ];

        for (name, args, expected_message) in strict_arity_cases {
            let result = handle(&mut interpreter, name, &args).expect("handler should match");
            assert!(
                matches!(result, Value::Error(ref message) if message.contains(expected_message)),
                "Expected strict-arity contract for {} to contain '{}', got {:?}",
                name,
                expected_message,
                result
            );
        }
    }

    #[test]
    fn test_array_collection_behavior_contracts() {
        let mut interpreter = Interpreter::new();

        let sort_input = Value::Array(Arc::new(vec![Value::Int(3), Value::Int(1), Value::Int(2)]));
        let sort_result =
            handle(&mut interpreter, "sort", &[sort_input]).expect("handler should match");
        assert!(matches!(sort_result, Value::Array(values) if values.len() == 3
            && matches!(&values[0], Value::Int(1))
            && matches!(&values[1], Value::Int(2))
            && matches!(&values[2], Value::Int(3))));

        let sum_result = handle(
            &mut interpreter,
            "sum",
            &[Value::Array(Arc::new(vec![Value::Int(2), Value::Int(3), Value::Int(5)]))],
        )
        .expect("handler should match");
        assert!(matches!(sum_result, Value::Int(10)));

        let zip_result = handle(
            &mut interpreter,
            "zip",
            &[
                Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])),
                Value::Array(Arc::new(vec![Value::Int(10), Value::Int(20)])),
            ],
        )
        .expect("handler should match");
        assert!(matches!(zip_result, Value::Array(values) if values.len() == 2
            && matches!(&values[0], Value::Array(pair) if pair.len() == 2
                && matches!(&pair[0], Value::Int(1))
                && matches!(&pair[1], Value::Int(10)))
            && matches!(&values[1], Value::Array(pair) if pair.len() == 2
                && matches!(&pair[0], Value::Int(2))
                && matches!(&pair[1], Value::Int(20)))));
    }

    #[test]
    fn test_collection_helpers_reject_wrong_types_instead_of_silent_fallbacks() {
        let mut interpreter = Interpreter::new();

        let len_wrong_type = handle(&mut interpreter, "len", &[Value::Null]).expect("handler");
        assert!(
            matches!(len_wrong_type, Value::Error(message) if message.contains("len() requires"))
        );

        let push_wrong_type =
            handle(&mut interpreter, "push", &[Value::Null, Value::Int(1)]).expect("handler");
        assert!(
            matches!(push_wrong_type, Value::Error(message) if message.contains("push() requires an array"))
        );

        let pop_wrong_type = handle(&mut interpreter, "pop", &[Value::Null]).expect("handler");
        assert!(
            matches!(pop_wrong_type, Value::Error(message) if message.contains("pop() requires an array"))
        );

        let slice_wrong_type =
            handle(&mut interpreter, "slice", &[Value::Null, Value::Int(0), Value::Int(1)])
                .expect("handler");
        assert!(
            matches!(slice_wrong_type, Value::Error(message) if message.contains("slice() requires array or bytes and numeric start/end arguments"))
        );

        let concat_wrong_type =
            handle(&mut interpreter, "concat", &[Value::Array(Arc::new(vec![])), Value::Null])
                .expect("handler");
        assert!(
            matches!(concat_wrong_type, Value::Error(message) if message.contains("concat() requires two array arguments"))
        );

        let clear_wrong_type = handle(&mut interpreter, "clear", &[Value::Null]).expect("handler");
        assert!(
            matches!(clear_wrong_type, Value::Error(message) if message.contains("clear() requires an array or dict"))
        );
    }

    #[test]
    fn test_fixed_dict_merge_clear_remove_contracts() {
        let mut interpreter = Interpreter::new();

        let fixed_left = Value::FixedDict {
            keys: Arc::new(vec![Arc::from("a"), Arc::from("b")]),
            values: vec![Value::Int(1), Value::Int(2)],
        };
        let fixed_right = Value::FixedDict {
            keys: Arc::new(vec![Arc::from("b"), Arc::from("c")]),
            values: vec![Value::Int(99), Value::Int(3)],
        };

        let merged = handle(&mut interpreter, "merge", &[fixed_left.clone(), fixed_right.clone()])
            .expect("merge handler should match");
        assert!(matches!(merged, Value::Dict(dict) if matches!(dict.get("a"), Some(Value::Int(1)))
                && matches!(dict.get("b"), Some(Value::Int(99)))
                && matches!(dict.get("c"), Some(Value::Int(3)))));

        let cleared = handle(&mut interpreter, "clear", std::slice::from_ref(&fixed_left))
            .expect("clear handler should match");
        assert!(matches!(cleared, Value::Dict(dict) if dict.is_empty()));

        let removed = handle(
            &mut interpreter,
            "remove",
            &[fixed_left, Value::Str(Arc::new("b".to_string()))],
        )
        .expect("remove handler should match");
        assert!(matches!(removed, Value::Array(values) if values.len() == 2
                && matches!(&values[0], Value::Dict(dict) if matches!(dict.get("a"), Some(Value::Int(1))) && !dict.contains_key("b"))
                && matches!(&values[1], Value::Int(2))));
    }

    #[test]
    fn test_fixed_dict_invert_update_get_default_contracts() {
        let mut interpreter = Interpreter::new();

        let left = Value::FixedDict {
            keys: Arc::new(vec![Arc::from("a"), Arc::from("b")]),
            values: vec![Value::Int(1), Value::Int(2)],
        };
        let right = Value::FixedDict {
            keys: Arc::new(vec![Arc::from("b"), Arc::from("c")]),
            values: vec![Value::Int(99), Value::Int(3)],
        };

        let inverted = handle(&mut interpreter, "invert", std::slice::from_ref(&left))
            .expect("invert handler should match");
        assert!(
            matches!(inverted, Value::Dict(dict) if matches!(dict.get("1"), Some(Value::Str(value)) if value.as_ref() == "a")
                && matches!(dict.get("2"), Some(Value::Str(value)) if value.as_ref() == "b"))
        );

        let updated = handle(&mut interpreter, "update", &[left.clone(), right.clone()])
            .expect("update handler should match");
        assert!(
            matches!(updated, Value::Dict(dict) if matches!(dict.get("a"), Some(Value::Int(1)))
                && matches!(dict.get("b"), Some(Value::Int(99)))
                && matches!(dict.get("c"), Some(Value::Int(3))))
        );

        let get_existing = handle(
            &mut interpreter,
            "get_default",
            &[left.clone(), Value::Str(Arc::new("a".to_string())), Value::Int(0)],
        )
        .expect("get_default existing should match");
        assert!(matches!(get_existing, Value::Int(1)));

        let get_missing = handle(
            &mut interpreter,
            "get_default",
            &[right, Value::Str(Arc::new("missing".to_string())), Value::Int(123)],
        )
        .expect("get_default missing should match");
        assert!(matches!(get_missing, Value::Int(123)));
    }
}
