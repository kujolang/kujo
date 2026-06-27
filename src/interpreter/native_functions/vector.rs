// File: src/interpreter/native_functions/vector.rs
//
// Native vector math helpers for numeric Kujo arrays.

use crate::interpreter::{DictMap, Value};
use rayon::prelude::*;
use std::sync::Arc;

const MAX_VECTOR_DIMENSIONS: usize = 100_000;
const MAX_MATRIX_ROWS: usize = 100_000;
const MAX_MATRIX_CELLS: usize = 5_000_000;
const VEC_TOP_K_PARALLEL_THRESHOLD: usize = 1024;

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "vec_dot" => handle_vec_dot(arg_values),
        "vec_norm" => handle_vec_norm(arg_values),
        "vec_normalize" => handle_vec_normalize(arg_values),
        "vec_cosine" => handle_vec_cosine(arg_values),
        "vec_top_k" => handle_vec_top_k(arg_values),
        _ => return None,
    };

    Some(result)
}

fn handle_vec_dot(arg_values: &[Value]) -> Value {
    if arg_values.len() != 2 {
        return Value::Error(format!("vec_dot() expects 2 arguments, got {}", arg_values.len()));
    }

    let left = match numeric_vector("vec_dot", "a", &arg_values[0]) {
        Ok(vector) => vector,
        Err(error) => return error,
    };
    let right = match numeric_vector("vec_dot", "b", &arg_values[1]) {
        Ok(vector) => vector,
        Err(error) => return error,
    };

    match dot_checked("vec_dot", &left, &right) {
        Ok(result) => Value::Float(result),
        Err(error) => error,
    }
}

fn handle_vec_norm(arg_values: &[Value]) -> Value {
    if arg_values.len() != 1 {
        return Value::Error(format!("vec_norm() expects 1 argument, got {}", arg_values.len()));
    }

    let vector = match numeric_vector("vec_norm", "a", &arg_values[0]) {
        Ok(vector) => vector,
        Err(error) => return error,
    };

    match norm_checked("vec_norm", &vector) {
        Ok(result) => Value::Float(result),
        Err(error) => error,
    }
}

fn handle_vec_normalize(arg_values: &[Value]) -> Value {
    if arg_values.len() != 1 {
        return Value::Error(format!(
            "vec_normalize() expects 1 argument, got {}",
            arg_values.len()
        ));
    }

    let vector = match numeric_vector("vec_normalize", "a", &arg_values[0]) {
        Ok(vector) => vector,
        Err(error) => return error,
    };

    let norm = match norm_checked("vec_normalize", &vector) {
        Ok(result) => result,
        Err(error) => return error,
    };

    if norm == 0.0 {
        return Value::Array(Arc::new(vector.into_iter().map(|_| Value::Float(0.0)).collect()));
    }

    let normalized: Vec<Value> =
        vector.into_iter().map(|value| Value::Float(value / norm)).collect();
    Value::Array(Arc::new(normalized))
}

fn handle_vec_cosine(arg_values: &[Value]) -> Value {
    if arg_values.len() != 2 {
        return Value::Error(format!("vec_cosine() expects 2 arguments, got {}", arg_values.len()));
    }

    let left = match numeric_vector("vec_cosine", "a", &arg_values[0]) {
        Ok(vector) => vector,
        Err(error) => return error,
    };
    let right = match numeric_vector("vec_cosine", "b", &arg_values[1]) {
        Ok(vector) => vector,
        Err(error) => return error,
    };

    match cosine_checked("vec_cosine", &left, &right) {
        Ok(result) => Value::Float(result),
        Err(error) => error,
    }
}

fn handle_vec_top_k(arg_values: &[Value]) -> Value {
    if arg_values.len() != 3 {
        return Value::Error(format!("vec_top_k() expects 3 arguments, got {}", arg_values.len()));
    }

    let query = match numeric_vector("vec_top_k", "query", &arg_values[0]) {
        Ok(vector) => vector,
        Err(error) => return error,
    };
    let matrix = match numeric_matrix("vec_top_k", "matrix", &arg_values[1], query.len()) {
        Ok(matrix) => matrix,
        Err(error) => return error,
    };
    let k = match non_negative_usize("vec_top_k", "k", &arg_values[2]) {
        Ok(k) => k,
        Err(error) => return error,
    };

    let limit = k.min(matrix.len());
    if limit == 0 {
        return Value::Array(Arc::new(Vec::new()));
    }

    let mut scored = top_k_scores(&query, &matrix);
    if scored.iter().any(|(_, score)| !score.is_finite()) {
        return Value::Error("vec_top_k() score is not finite; reduce input magnitude".to_string());
    }
    scored.sort_by(|left, right| right.1.total_cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    scored.truncate(limit);

    Value::Array(Arc::new(scored.into_iter().map(score_entry_value).collect()))
}

fn top_k_scores(query: &[f64], matrix: &[Vec<f64>]) -> Vec<(usize, f64)> {
    if matrix.len() >= VEC_TOP_K_PARALLEL_THRESHOLD {
        matrix.par_iter().enumerate().map(|(index, row)| (index, cosine_raw(query, row))).collect()
    } else {
        matrix.iter().enumerate().map(|(index, row)| (index, cosine_raw(query, row))).collect()
    }
}

fn numeric_vector(function: &str, arg_name: &str, value: &Value) -> Result<Vec<f64>, Value> {
    let Value::Array(values) = value else {
        return Err(Value::Error(format!(
            "{function}() expects '{arg_name}' to be an array of finite numbers"
        )));
    };

    if values.len() > MAX_VECTOR_DIMENSIONS {
        return Err(Value::Error(format!(
            "{function}() '{arg_name}' exceeds vector dimension limit ({MAX_VECTOR_DIMENSIONS})"
        )));
    }

    values
        .iter()
        .enumerate()
        .map(|(index, value)| finite_number(function, &format!("{arg_name}[{index}]"), value))
        .collect()
}

fn numeric_matrix(
    function: &str,
    arg_name: &str,
    value: &Value,
    expected_dimensions: usize,
) -> Result<Vec<Vec<f64>>, Value> {
    let Value::Array(rows) = value else {
        return Err(Value::Error(format!(
            "{function}() expects '{arg_name}' to be an array of numeric arrays"
        )));
    };

    if rows.len() > MAX_MATRIX_ROWS {
        return Err(Value::Error(format!(
            "{function}() '{arg_name}' exceeds matrix row limit ({MAX_MATRIX_ROWS})"
        )));
    }
    if expected_dimensions > 0 && rows.len().saturating_mul(expected_dimensions) > MAX_MATRIX_CELLS
    {
        return Err(Value::Error(format!(
            "{function}() '{arg_name}' exceeds matrix cell limit ({MAX_MATRIX_CELLS})"
        )));
    }

    rows.iter()
        .enumerate()
        .map(|(row_index, row)| {
            let vector = numeric_vector(function, &format!("{arg_name}[{row_index}]"), row)?;
            if vector.len() != expected_dimensions {
                return Err(Value::Error(format!(
                    "{function}() dimension mismatch: query has {expected_dimensions} dimensions but matrix row {row_index} has {}",
                    vector.len()
                )));
            }
            Ok(vector)
        })
        .collect()
}

fn finite_number(function: &str, location: &str, value: &Value) -> Result<f64, Value> {
    let number = match value {
        Value::Int(n) => *n as f64,
        Value::Float(n) => *n,
        _ => {
            return Err(Value::Error(format!(
                "{function}() expects {location} to be a finite number"
            )))
        }
    };

    if !number.is_finite() {
        return Err(Value::Error(format!("{function}() expects {location} to be finite")));
    }

    Ok(number)
}

fn non_negative_usize(function: &str, arg_name: &str, value: &Value) -> Result<usize, Value> {
    match value {
        Value::Int(n) if *n >= 0 => Ok(*n as usize),
        _ => Err(Value::Error(format!(
            "{function}() expects '{arg_name}' to be a non-negative integer"
        ))),
    }
}

fn dot_checked(function: &str, left: &[f64], right: &[f64]) -> Result<f64, Value> {
    if left.len() != right.len() {
        return Err(Value::Error(format!(
            "{function}() dimension mismatch: left has {} dimensions but right has {}",
            left.len(),
            right.len()
        )));
    }

    let result = dot_raw(left, right);
    ensure_finite(function, result)
}

fn norm_checked(function: &str, vector: &[f64]) -> Result<f64, Value> {
    let squared = dot_raw(vector, vector);
    let result = squared.sqrt();
    ensure_finite(function, result)
}

fn cosine_checked(function: &str, left: &[f64], right: &[f64]) -> Result<f64, Value> {
    if left.len() != right.len() {
        return Err(Value::Error(format!(
            "{function}() dimension mismatch: left has {} dimensions but right has {}",
            left.len(),
            right.len()
        )));
    }
    ensure_finite(function, cosine_raw(left, right))
}

fn dot_raw(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn cosine_raw(left: &[f64], right: &[f64]) -> f64 {
    let left_norm = dot_raw(left, left).sqrt();
    let right_norm = dot_raw(right, right).sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    (dot_raw(left, right) / (left_norm * right_norm)).clamp(-1.0, 1.0)
}

fn ensure_finite(function: &str, value: f64) -> Result<f64, Value> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Value::Error(format!("{function}() result is not finite; reduce input magnitude")))
    }
}

fn score_entry_value((index, score): (usize, f64)) -> Value {
    let mut map = DictMap::default();
    map.insert(Arc::<str>::from("index"), Value::Int(index as i64));
    map.insert(Arc::<str>::from("score"), Value::Float(score));
    Value::Dict(Arc::new(map))
}

#[cfg(test)]
mod tests {
    use super::{handle, VEC_TOP_K_PARALLEL_THRESHOLD};
    use crate::interpreter::Value;
    use std::sync::Arc;

    fn arr(values: &[f64]) -> Value {
        Value::Array(Arc::new(values.iter().map(|value| Value::Float(*value)).collect()))
    }

    fn int_arr(values: &[i64]) -> Value {
        Value::Array(Arc::new(values.iter().map(|value| Value::Int(*value)).collect()))
    }

    fn matrix(rows: Vec<Value>) -> Value {
        Value::Array(Arc::new(rows))
    }

    fn expect_float(value: Value) -> f64 {
        match value {
            Value::Float(value) => value,
            other => panic!("expected float, got {other:?}"),
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= 1e-9, "expected {expected}, got {actual}");
    }

    #[test]
    fn vector_dot_norm_cosine_and_normalize_are_numerically_correct() {
        let dot = handle("vec_dot", &[int_arr(&[1, 2, 3]), arr(&[4.0, 5.0, 6.0])]).unwrap();
        assert_close(expect_float(dot), 32.0);

        let norm = handle("vec_norm", &[arr(&[3.0, 4.0])]).unwrap();
        assert_close(expect_float(norm), 5.0);

        let cosine = handle("vec_cosine", &[arr(&[1.0, 1.0]), arr(&[1.0, 0.0])]).unwrap();
        assert_close(expect_float(cosine), std::f64::consts::FRAC_1_SQRT_2);

        let normalized = handle("vec_normalize", &[arr(&[3.0, 4.0])]).unwrap();
        match normalized {
            Value::Array(values) => {
                assert_close(expect_float(values[0].clone()), 0.6);
                assert_close(expect_float(values[1].clone()), 0.8);
            }
            other => panic!("expected normalized array, got {other:?}"),
        }
    }

    #[test]
    fn vector_cosine_handles_orthogonal_identical_and_zero_vectors() {
        let orthogonal = handle("vec_cosine", &[arr(&[1.0, 0.0]), arr(&[0.0, 1.0])]).unwrap();
        assert_close(expect_float(orthogonal), 0.0);

        let identical = handle("vec_cosine", &[arr(&[2.0, 2.0]), arr(&[2.0, 2.0])]).unwrap();
        assert_close(expect_float(identical), 1.0);

        let zero = handle("vec_cosine", &[arr(&[0.0, 0.0]), arr(&[2.0, 2.0])]).unwrap();
        assert_close(expect_float(zero), 0.0);

        let normalized_zero = handle("vec_normalize", &[arr(&[0.0, 0.0])]).unwrap();
        match normalized_zero {
            Value::Array(values) => {
                assert!(values.iter().all(|value| matches!(value, Value::Float(0.0))));
            }
            other => panic!("expected normalized zero array, got {other:?}"),
        }
    }

    #[test]
    fn vector_top_k_returns_indices_sorted_by_descending_cosine() {
        let query = arr(&[1.0, 0.0]);
        let rows = matrix(vec![arr(&[0.0, 1.0]), arr(&[1.0, 0.0]), arr(&[0.5, 0.0])]);

        let result = handle("vec_top_k", &[query, rows, Value::Int(2)]).unwrap();
        match result {
            Value::Array(entries) => {
                assert_eq!(entries.len(), 2);
                let first = match &entries[0] {
                    Value::Dict(map) => map,
                    other => panic!("expected dict entry, got {other:?}"),
                };
                let second = match &entries[1] {
                    Value::Dict(map) => map,
                    other => panic!("expected dict entry, got {other:?}"),
                };
                assert!(matches!(first.get("index"), Some(Value::Int(1))));
                assert!(matches!(second.get("index"), Some(Value::Int(2))));
                assert!(
                    matches!(first.get("score"), Some(Value::Float(score)) if score.is_finite())
                );
            }
            other => panic!("expected top_k array, got {other:?}"),
        }
    }

    #[test]
    fn vector_top_k_returns_all_when_k_exceeds_rows_and_parallel_path_matches_contract() {
        let query = arr(&[1.0, 0.0]);
        let mut rows = Vec::new();
        for index in 0..(VEC_TOP_K_PARALLEL_THRESHOLD + 1) {
            rows.push(if index == 7 { arr(&[1.0, 0.0]) } else { arr(&[0.0, 1.0]) });
        }

        let result = handle("vec_top_k", &[query, matrix(rows), Value::Int(2_000)]).unwrap();
        match result {
            Value::Array(entries) => {
                assert_eq!(entries.len(), VEC_TOP_K_PARALLEL_THRESHOLD + 1);
                let first = match &entries[0] {
                    Value::Dict(map) => map,
                    other => panic!("expected dict entry, got {other:?}"),
                };
                assert!(matches!(first.get("index"), Some(Value::Int(7))));
            }
            other => panic!("expected top_k array, got {other:?}"),
        }
    }

    #[test]
    fn vector_functions_reject_dimension_mismatch_non_numeric_and_non_finite_inputs() {
        let mismatch = handle("vec_dot", &[arr(&[1.0]), arr(&[1.0, 2.0])]).unwrap();
        assert!(
            matches!(mismatch, Value::Error(message) if message.contains("dimension mismatch"))
        );

        let non_numeric = handle(
            "vec_norm",
            &[Value::Array(Arc::new(vec![Value::Str(Arc::new("x".to_string()))]))],
        )
        .unwrap();
        assert!(matches!(non_numeric, Value::Error(message) if message.contains("finite number")));

        let non_finite = handle("vec_norm", &[arr(&[f64::NAN])]).unwrap();
        assert!(matches!(non_finite, Value::Error(message) if message.contains("finite")));

        let non_finite_result = handle("vec_norm", &[arr(&[f64::MAX, f64::MAX])]).unwrap();
        assert!(
            matches!(non_finite_result, Value::Error(message) if message.contains("not finite"))
        );

        let row_mismatch =
            handle("vec_top_k", &[arr(&[1.0, 0.0]), matrix(vec![arr(&[1.0])]), Value::Int(1)])
                .unwrap();
        assert!(
            matches!(row_mismatch, Value::Error(message) if message.contains("dimension mismatch"))
        );

        let top_k_non_finite =
            handle("vec_top_k", &[arr(&[f64::MAX]), matrix(vec![arr(&[f64::MAX])]), Value::Int(1)])
                .unwrap();
        assert!(
            matches!(top_k_non_finite, Value::Error(message) if message.contains("not finite"))
        );
    }
}
