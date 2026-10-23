//! Serialize borrowed runtime values without constructing a second owned DOM.
use super::{kujo_value_to_json, MAX_JSON_NESTING_DEPTH};
use crate::interpreter::Value;
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use std::collections::BTreeMap;

pub(super) fn encode(value: &Value, pretty: bool) -> Result<String, String> {
    // Validate in the historical conversion order, including overwritten fixed
    // keys. This preserves which error is reported when multiple values fail.
    validate(value, 0)?;
    let view = JsonView(value);
    let result =
        if pretty { serde_json::to_string_pretty(&view) } else { serde_json::to_string(&view) };
    result.map_err(|error| format!("JSON serialization error: {error}"))
}

fn check_depth(depth: usize) -> Result<(), String> {
    if depth > MAX_JSON_NESTING_DEPTH {
        Err(format!("JSON value exceeds maximum nesting depth of {}", MAX_JSON_NESTING_DEPTH))
    } else {
        Ok(())
    }
}

fn validate(value: &Value, depth: usize) -> Result<(), String> {
    check_depth(depth)?;
    match value {
        Value::Null | Value::Int(_) | Value::Str(_) | Value::Secret(_) | Value::Bool(_) => Ok(()),
        Value::Float(value) if value.is_finite() => Ok(()),
        Value::Float(value) => Err(format!("Cannot convert non-finite float {} to JSON", value)),
        Value::Array(values) | Value::DenseIntDict(values) => {
            for value in values.iter() {
                validate(value, depth + 1)?;
            }
            Ok(())
        }
        Value::Dict(values) => {
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_by(|(left, _), (right, _)| left.as_ref().cmp(right.as_ref()));
            for (_, value) in entries {
                validate(value, depth + 1)?;
            }
            Ok(())
        }
        Value::FixedDict { keys, values } => {
            for (_, value) in keys.iter().zip(values.iter()) {
                validate(value, depth + 1)?;
            }
            Ok(())
        }
        Value::IntDict(values) => {
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_by_key(|(key, _)| **key);
            for (_, value) in entries {
                validate(value, depth + 1)?;
            }
            Ok(())
        }
        Value::DenseIntDictInt(values) => {
            if !values.is_empty() {
                check_depth(depth + 1)?;
            }
            Ok(())
        }
        Value::DenseIntDictIntFull(values) => {
            if !values.is_empty() {
                check_depth(depth + 1)?;
            }
            Ok(())
        }
        _ => Err(format!("Cannot convert {:?} to JSON", value)),
    }
}

struct JsonView<'a>(&'a Value);
impl Serialize for JsonView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Null => serializer.serialize_none(),
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::Int(value) => serializer.serialize_i64(*value),
            Value::Float(value) => serializer.serialize_f64(*value),
            Value::Str(value) => serializer.serialize_str(value),
            Value::Secret(_) => serializer.serialize_str("***"),
            Value::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values.iter() {
                    sequence.serialize_element(&JsonView(value))?;
                }
                sequence.end()
            }
            // serde_json's existing DOM uses lexicographically ordered object
            // keys. Borrow keys/values where possible and retain duplicate-key
            // last-write behavior for internal fixed dictionaries.
            Value::Dict(values) => {
                let mut ordered: Vec<_> = values.iter().collect();
                ordered.sort_by(|(left, _), (right, _)| left.as_ref().cmp(right.as_ref()));
                let mut map = serializer.serialize_map(Some(ordered.len()))?;
                for (key, value) in ordered {
                    map.serialize_entry(key.as_ref(), &JsonView(value))?;
                }
                map.end()
            }
            Value::FixedDict { keys, values } => {
                let mut ordered = BTreeMap::new();
                for (key, value) in keys.iter().zip(values.iter()) {
                    ordered.insert(key.as_ref(), JsonView(value));
                }
                ordered.serialize(serializer)
            }
            Value::IntDict(values) => values
                .iter()
                .map(|(key, value)| (key.to_string(), JsonView(value)))
                .collect::<BTreeMap<_, _>>()
                .serialize(serializer),
            Value::DenseIntDict(values) => values
                .iter()
                .enumerate()
                .map(|(key, value)| (key.to_string(), JsonView(value)))
                .collect::<BTreeMap<_, _>>()
                .serialize(serializer),
            Value::DenseIntDictInt(values) => values
                .iter()
                .enumerate()
                .map(|(key, value)| (key.to_string(), value))
                .collect::<BTreeMap<_, _>>()
                .serialize(serializer),
            Value::DenseIntDictIntFull(values) => values
                .iter()
                .enumerate()
                .map(|(key, value)| (key.to_string(), value))
                .collect::<BTreeMap<_, _>>()
                .serialize(serializer),
            // Validation rejects these before serialization. Keep a checked
            // fallback for future Value variants rather than inventing output.
            value => {
                kujo_value_to_json(value).map_err(serde::ser::Error::custom)?.serialize(serializer)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::{DictMap, IntDictMap};
    use std::sync::Arc;

    fn assert_matches_owned(value: &Value) {
        for pretty in [false, true] {
            let expected = kujo_value_to_json(value).and_then(|dom| {
                let encoded = if pretty {
                    serde_json::to_string_pretty(&dom)
                } else {
                    serde_json::to_string(&dom)
                };
                encoded.map_err(|error| format!("JSON serialization error: {error}"))
            });
            assert_eq!(encode(value, pretty), expected, "pretty={pretty}, value={value:?}");
        }
    }

    #[test]
    fn borrowed_json_matches_owned_conversion_for_all_representations() {
        let scalar = vec![
            Value::Null,
            Value::Bool(true),
            Value::Int(i64::MIN),
            Value::Int(i64::MAX),
            Value::Float(-0.0),
            Value::Float(1.0),
            Value::Float(f64::MIN_POSITIVE),
            Value::Float(f64::MAX),
            Value::Str(Arc::new("quote\" newline\n slash\\ café 🦀".into())),
            Value::Secret(Arc::new("must-not-leak".into())),
        ];
        for value in &scalar {
            assert_matches_owned(value);
        }
        let array = Value::Array(Arc::new(scalar));
        assert_matches_owned(&array);
        let dictionary: DictMap =
            [("z".into(), array.clone()), ("a".into(), Value::Int(2))].into_iter().collect();
        assert_matches_owned(&Value::Dict(Arc::new(dictionary)));
        let fixed = Value::FixedDict {
            keys: Arc::new(vec!["z".into(), "a".into(), "z".into()]),
            values: vec![Value::Int(1), array, Value::Int(3)],
        };
        assert_matches_owned(&fixed);
        let integers: IntDictMap =
            [(2, Value::Int(2)), (10, Value::Int(10)), (-1, Value::Null)].into_iter().collect();
        assert_matches_owned(&Value::IntDict(Arc::new(integers)));
        assert_matches_owned(&Value::DenseIntDict(Arc::new((0..12).map(Value::Int).collect())));
        assert_matches_owned(&Value::DenseIntDictInt(Arc::new(vec![Some(1), None, Some(3)])));
        assert_matches_owned(&Value::DenseIntDictIntFull(Arc::new((0..12).collect())));
    }

    #[test]
    fn borrowed_json_preserves_depth_errors_and_first_invalid_value() {
        for value in [
            Value::Float(f64::NAN),
            Value::Float(f64::INFINITY),
            Value::Float(f64::NEG_INFINITY),
            Value::Bytes(Arc::new(vec![255])),
        ] {
            assert_matches_owned(&value);
        }
        let fixed = Value::FixedDict {
            keys: Arc::new(vec!["z".into(), "a".into(), "z".into()]),
            values: vec![Value::Bytes(Arc::new(vec![1])), Value::Float(f64::NAN), Value::Int(3)],
        };
        assert_matches_owned(&fixed);
        let integers: IntDictMap =
            [(2, Value::Bytes(Arc::new(vec![2]))), (10, Value::Float(f64::NAN))]
                .into_iter()
                .collect();
        assert_matches_owned(&Value::IntDict(Arc::new(integers)));
        for leaf in [
            Value::Int(1),
            Value::DenseIntDictInt(Arc::new(vec![None])),
            Value::DenseIntDictIntFull(Arc::new(vec![1])),
        ] {
            let mut value = leaf;
            for _ in 0..=MAX_JSON_NESTING_DEPTH + 1 {
                assert_matches_owned(&value);
                value = Value::Array(Arc::new(vec![value]));
            }
        }
    }
}
