//! Sparse collection batches. Capacity is validated before admission; recording
//! never grows the table, and publication moves values into the matching owner.
use super::dictionary_publication::CollectionValue;
use super::Value;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(super) enum BatchValue {
    Collection(CollectionValue),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Str(Arc<String>),
    Bytes(Arc<Vec<u8>>),
}
impl BatchValue {
    pub(super) fn supports(value: &Value) -> bool {
        super::dictionary_publication::DictionaryCollectionPublication::supports(value)
            || matches!(
                value,
                Value::Int(_)
                    | Value::Float(_)
                    | Value::Bool(_)
                    | Value::Null
                    | Value::Str(_)
                    | Value::Bytes(_)
            )
    }
    pub(super) fn from_owned(value: Value) -> Self {
        match value {
            Value::Int(n) => Self::Int(n),
            Value::Float(n) => Self::Float(n),
            Value::Bool(n) => Self::Bool(n),
            Value::Null => Self::Null,
            Value::Str(s) => Self::Str(s),
            Value::Bytes(b) => Self::Bytes(b),
            value => Self::Collection(CollectionValue::from_owned_value(value)),
        }
    }
    fn into_value(self) -> Value {
        match self {
            Self::Collection(v) => v.into_value(),
            Self::Int(n) => Value::Int(n),
            Self::Float(n) => Value::Float(n),
            Self::Bool(n) => Value::Bool(n),
            Self::Null => Value::Null,
            Self::Str(s) => Value::Str(s),
            Self::Bytes(b) => Value::Bytes(b),
        }
    }
}

impl super::dictionary_publication::CollectionPayload for BatchValue {
    fn into_value(self) -> Value {
        BatchValue::into_value(self)
    }
}

// This private, bounded journal hashes validated integer indices, not public
// dictionary keys. Explicit seeds avoid AHasher::default's process-wide lazy
// seed allocation on the first otherwise allocation-neutral array update.
#[derive(Clone, Debug, Default)]
struct IndexHashBuilder;
impl BuildHasher for IndexHashBuilder {
    type Hasher = ahash::AHasher;

    fn build_hasher(&self) -> Self::Hasher {
        ahash::RandomState::with_seeds(0, 0, 0, 0).build_hasher()
    }
}

type Writes = HashMap<usize, BatchValue, IndexHashBuilder>;
#[derive(Clone, Debug)]
pub(super) struct ArrayBatchPublication {
    name: Box<str>,
    base: Arc<Vec<Value>>,
    writes: Writes,
}
impl ArrayBatchPublication {
    pub(super) fn new(name: Box<str>, base: Arc<Vec<Value>>, capacity: u16) -> Self {
        debug_assert!(capacity > 2);
        Self {
            name,
            base,
            writes: Writes::with_capacity_and_hasher(usize::from(capacity), Default::default()),
        }
    }
    pub(super) fn name(&self) -> &str {
        &self.name
    }
    pub(super) fn len(&self) -> usize {
        self.base.len()
    }
    pub(super) fn can_record(&self, index: usize) -> bool {
        index < self.base.len()
            && (self.writes.len() < self.writes.capacity() || self.writes.contains_key(&index))
    }
    pub(super) fn record(&mut self, index: usize, value: BatchValue) -> bool {
        if index >= self.base.len() {
            return false;
        }
        if let Some(previous) = self.writes.get_mut(&index) {
            *previous = value;
        } else {
            if self.writes.len() == self.writes.capacity() {
                return false;
            }
            self.writes.insert(index, value);
        }
        true
    }
    pub(super) fn get(&self, index: usize) -> Option<Value> {
        self.writes
            .get(&index)
            .map(|v| v.clone().into_value())
            .or_else(|| self.base.get(index).cloned())
    }
    pub(super) fn snapshot_value(&self) -> Value {
        let mut base = self.base.clone();
        let values = Arc::make_mut(&mut base);
        for (&index, value) in &self.writes {
            values[index] = value.clone().into_value();
        }
        Value::Array(base)
    }
    pub(super) fn publish(self, owner: Option<&mut Value>) -> (String, Value) {
        let Self { name, mut base, writes } = self;
        if let Some(Value::Array(public)) = owner {
            if Arc::ptr_eq(&base, public) {
                drop(base);
                let values = Arc::make_mut(public);
                for (index, value) in writes {
                    values[index] = value.into_value();
                }
                return (name.into_string(), Value::Array(public.clone()));
            }
        }
        let values = Arc::make_mut(&mut base);
        for (index, value) in writes {
            values[index] = value.into_value();
        }
        (name.into_string(), Value::Array(base))
    }
    pub(super) fn discard(self) -> (String, Value) {
        (self.name.into_string(), Value::Array(self.base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dense_batches_reuse_storage_and_preserve_snapshots() {
        let mut owner = Value::array(vec![Value::Null; 1024]);
        let Value::Array(base) = &owner else { unreachable!() };
        let pointer = Arc::as_ptr(base);
        let mut pending = ArrayBatchPublication::new("state".into(), base.clone(), 1024);
        let capacity = pending.writes.capacity();
        for index in 0..1024 {
            assert!(pending.record(index, BatchValue::Int(index as i64)));
        }
        let saved = pending.snapshot_value();
        for index in (0..1024).rev() {
            assert!(pending.record(
                index,
                BatchValue::from_owned(Value::array(vec![Value::Int(index as i64)]))
            ));
        }
        assert_eq!(pending.writes.capacity(), capacity);
        assert!(!pending.record(usize::MAX, BatchValue::Null));
        let (_, value) = pending.publish(Some(&mut owner));
        let Value::Array(values) = value else { unreachable!() };
        assert_eq!(Arc::as_ptr(&values), pointer);
        let Value::Array(saved) = saved else { unreachable!() };
        for index in 0..1024 {
            assert!(matches!(saved[index], Value::Int(n) if n == index as i64));
            assert!(Value::equals(&values[index], &Value::array(vec![Value::Int(index as i64)])));
        }
    }
    #[test]
    fn full_batches_reject_new_entries_without_losing_updates_or_owners() {
        let original = Value::array(vec![Value::Null; 64]);
        let Value::Array(base) = &original else { unreachable!() };
        let mut pending = ArrayBatchPublication::new("state".into(), base.clone(), 3);
        let capacity = pending.writes.capacity();
        for index in 0..capacity {
            assert!(pending.record(index, BatchValue::Int(index as i64)));
        }
        assert!(!pending.record(capacity, BatchValue::Int(99)));
        assert!(pending.record(0, BatchValue::Bool(true)));
        let mut replacement = Value::array(vec![Value::Int(77)]);
        let (_, result) = pending.publish(Some(&mut replacement));
        assert!(Value::equals(&replacement, &Value::array(vec![Value::Int(77)])));
        let Value::Array(result) = result else { unreachable!() };
        assert!(matches!(result[0], Value::Bool(true)));
        assert!(matches!(result[capacity], Value::Null));
        assert!(base.iter().all(|v| matches!(v, Value::Null)));
    }
}
