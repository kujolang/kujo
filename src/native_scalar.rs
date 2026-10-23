//! Immediate native payloads. No handles, retained objects, or heap allocation.
use crate::interpreter::Value;

pub(crate) const INTEGER: i64 = 0;
pub(crate) const BOOLEAN: i64 = 1;
pub(crate) const NULL: i64 = 2;
pub(crate) const FLOAT: i64 = 3;

pub(crate) fn encode(value: &Value) -> Option<(i64, i64)> {
    match value {
        Value::Int(value) => Some((*value, INTEGER)),
        Value::Bool(value) => Some((i64::from(*value), BOOLEAN)),
        Value::Null => Some((0, NULL)),
        Value::Float(value) => Some((value.to_bits() as i64, FLOAT)),
        _ => None,
    }
}

pub(crate) fn decode(value: i64, kind: i64) -> Value {
    match kind {
        INTEGER => Value::Int(value),
        BOOLEAN => Value::Bool(value != 0),
        NULL => Value::Null,
        FLOAT => Value::Float(f64::from_bits(value as u64)),
        _ => Value::Error(format!("Invalid native scalar kind: {kind}")),
    }
}
