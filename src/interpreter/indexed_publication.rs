//! Private, bounded indexed updates for a captured global array. This storage
//! must not grow the ordinary capture journal or expose a new public Value kind.
use super::Value;
use std::sync::Arc;

// The upper three bits encode a numeric scalar kind. Larger indices and other
// value kinds use ordinary array mutation; this is not a language index limit.
const INDEX_BITS: u32 = 29;
const INDEX_MASK: u32 = (1 << INDEX_BITS) - 1;

#[derive(Clone, Debug)]
pub(super) struct IndexedPublication {
    name: Box<str>,
    base: Arc<Vec<Value>>,
    words: [u64; 2],
    entries: [u32; 2],
}

impl IndexedPublication {
    fn encode(index: usize, value: &Value) -> Option<(u32, u64)> {
        if index > INDEX_MASK as usize {
            return None;
        }
        let (kind, word) = match value {
            Value::Int(value) => (1, *value as u64),
            Value::Float(value) => (2, value.to_bits()),
            Value::Bool(value) => (3, u64::from(*value)),
            Value::Null => (4, 0),
            _ => return None,
        };
        Some(((kind << INDEX_BITS) | index as u32, word))
    }

    fn decode(entry: u32, word: u64) -> Value {
        match entry >> INDEX_BITS {
            1 => Value::Int(word as i64),
            2 => Value::Float(f64::from_bits(word)),
            3 => Value::Bool(word != 0),
            4 => Value::Null,
            _ => unreachable!("empty or invalid indexed publication entry"),
        }
    }

    /// Takes a private map entry without allocating another copy of its name.
    /// A name with spare capacity is returned unchanged rather than shrunk.
    pub(super) fn prepare(
        name: String,
        base: Arc<Vec<Value>>,
        index: usize,
        value: &Value,
    ) -> Result<Self, (String, Arc<Vec<Value>>)> {
        let encoded = Self::encode(index, value);
        if name.capacity() != name.len() || index >= base.len() || encoded.is_none() {
            return Err((name, base));
        }
        let (entry, word) = encoded.unwrap();
        Ok(Self { name: name.into_boxed_str(), base, words: [word, 0], entries: [entry, 0] })
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn len(&self) -> usize {
        self.base.len()
    }

    pub(super) fn get(&self, index: usize) -> Option<Value> {
        if index <= INDEX_MASK as usize {
            if let Some(slot) = self
                .entries
                .iter()
                .position(|entry| *entry != 0 && (*entry & INDEX_MASK) as usize == index)
            {
                return Some(Self::decode(self.entries[slot], self.words[slot]));
            }
        }
        self.base.get(index).cloned()
    }

    /// Rejection is atomic: callers can materialize and use the ordinary path.
    pub(super) fn try_record(&mut self, index: usize, value: &Value) -> bool {
        let Some((entry, word)) = Self::encode(index, value) else { return false };
        if index >= self.base.len() {
            return false;
        }
        let slot = self
            .entries
            .iter()
            .position(|entry| *entry != 0 && (*entry & INDEX_MASK) as usize == index)
            .or_else(|| self.entries.iter().position(|entry| *entry == 0));
        let Some(slot) = slot else { return false };
        self.entries[slot] = entry;
        self.words[slot] = word;
        true
    }

    /// Check capacity before consuming a scalar journal for mixed-pair promotion.
    pub(super) fn can_promote_pair(&self, index: usize) -> bool {
        index < self.base.len()
            && index <= u32::MAX as usize
            && self
                .entries
                .iter()
                .any(|entry| *entry == 0 || (*entry & INDEX_MASK) as usize == index)
    }

    pub(super) fn into_mixed(
        self,
    ) -> super::array_collection_publication::ArrayCollectionPublication<
        super::array_batch_publication::BatchValue,
    > {
        use super::array_batch_publication::BatchValue;
        use super::array_collection_publication::ArrayCollectionPublication;
        let Self { name, base, words, entries } = self;
        assert_ne!(entries[0], 0);
        let mut pair = ArrayCollectionPublication::prepare(
            name.into_string(),
            base,
            (entries[0] & INDEX_MASK) as usize,
            BatchValue::from_owned(Self::decode(entries[0], words[0])),
        );
        if entries[1] != 0 {
            assert!(pair.try_record(
                (entries[1] & INDEX_MASK) as usize,
                BatchValue::from_owned(Self::decode(entries[1], words[1])),
            ));
        }
        pair
    }

    /// Transfer existing scalar writes without publishing or detaching the owner.
    pub(super) fn into_batch(
        self,
        capacity: u16,
    ) -> super::array_batch_publication::ArrayBatchPublication {
        use super::array_batch_publication::{ArrayBatchPublication, BatchValue};
        let Self { name, base, words, entries } = self;
        let mut batch = ArrayBatchPublication::new(name, base, capacity);
        for (entry, word) in entries.into_iter().zip(words) {
            if entry != 0 {
                assert!(batch.record(
                    (entry & INDEX_MASK) as usize,
                    BatchValue::from_owned(Self::decode(entry, word)),
                ));
            }
        }
        batch
    }

    fn apply(base: &mut Arc<Vec<Value>>, entries: [u32; 2], words: [u64; 2]) {
        let values = Arc::make_mut(base);
        for (entry, word) in entries.into_iter().zip(words) {
            if entry != 0 {
                values[(entry & INDEX_MASK) as usize] = Self::decode(entry, word);
            }
        }
    }

    #[cfg(test)]
    pub(super) fn materialize(self) -> (String, Value) {
        self.publish(None)
    }

    pub(super) fn snapshot_value(&self) -> Value {
        let mut base = self.base.clone();
        Self::apply(&mut base, self.entries, self.words);
        Value::Array(base)
    }

    /// The caller holds the public owner lock only for publication, never while
    /// executing user code. A replaced owner must not become the private base.
    pub(super) fn publish(self, owner: Option<&mut Value>) -> (String, Value) {
        let Self { name, mut base, words, entries } = self;
        if let Some(Value::Array(public)) = owner {
            if Arc::ptr_eq(&base, public) {
                drop(base);
                Self::apply(public, entries, words);
                return (name.into_string(), Value::Array(public.clone()));
            }
        }
        Self::apply(&mut base, entries, words);
        (name.into_string(), Value::Array(base))
    }

    /// Rollback/storage recycling can keep the original key without applying
    /// discarded changes or retaining the array in the idle pool.
    pub(super) fn discard(self) -> (String, Value) {
        (self.name.into_string(), Value::Array(self.base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn mixed_pair_promotion_preserves_scalar_bits_and_capacity() {
        use super::super::array_batch_publication::BatchValue;
        for value in [Value::Int(i64::MIN), Value::Float(-0.0), Value::Bool(true), Value::Null] {
            let owner = original();
            let pending =
                IndexedPublication::prepare("state".into(), base(&owner), 0, &value).unwrap();
            assert!(pending.can_promote_pair(1));
            assert!(!pending.can_promote_pair(128));
            let mut pair = pending.into_mixed();
            assert!(pair.try_record(1, BatchValue::Str(Arc::new("entry".into()))));
            let saved = pair.snapshot_value();
            assert!(!pair.can_record(2));
            assert!(pair.try_record(0, BatchValue::Int(11)));
            let Value::Array(values) = saved else { panic!() };
            match (&values[0], &value) {
                (Value::Int(a), Value::Int(b)) => assert_eq!(a, b),
                (Value::Float(a), Value::Float(b)) => assert_eq!(a.to_bits(), b.to_bits()),
                (Value::Bool(a), Value::Bool(b)) => assert_eq!(a, b),
                (Value::Null, Value::Null) => {}
                _ => panic!("scalar changed during mixed promotion"),
            }
            assert!(matches!(&values[1], Value::Str(s) if s.as_str() == "entry"));
            assert_eq!(item(&owner, 0), 0);
        }
    }

    fn base(value: &Value) -> Arc<Vec<Value>> {
        let Value::Array(values) = value else { panic!("expected array") };
        values.clone()
    }
    fn item(value: &Value, index: usize) -> i64 {
        let Value::Array(values) = value else { panic!("expected array") };
        let Value::Int(value) = values[index] else { panic!("expected integer") };
        value
    }
    fn original() -> Value {
        Value::Array(Arc::new((0..128).map(Value::Int).collect()))
    }

    #[test]
    fn layout_fits_existing_capture_journal_payload() {
        assert!(
            std::mem::size_of::<IndexedPublication>()
                <= std::mem::size_of::<HashSet<(usize, String)>>()
        );
        assert!(
            std::mem::align_of::<IndexedPublication>()
                <= std::mem::align_of::<HashSet<(usize, String)>>()
        );
    }

    #[test]
    fn promotion_preserves_scalar_bits_snapshots_and_replaced_owners() {
        use super::super::array_batch_publication::BatchValue;
        for value in [Value::Int(i64::MIN), Value::Float(-0.0), Value::Bool(true), Value::Null] {
            let mut owner = original();
            let snapshot = owner.clone();
            let mut pending =
                IndexedPublication::prepare("state".into(), base(&owner), 0, &value).unwrap();
            assert!(pending.try_record(1, &Value::Int(19)));
            let mut batch = pending.into_batch(3);
            match (batch.get(0).unwrap(), &value) {
                (Value::Int(a), Value::Int(b)) => assert_eq!(a, *b),
                (Value::Float(a), Value::Float(b)) => assert_eq!(a.to_bits(), b.to_bits()),
                (Value::Bool(a), Value::Bool(b)) => assert_eq!(a, *b),
                (Value::Null, Value::Null) => {}
                _ => panic!("scalar changed during promotion"),
            }
            assert!(batch.record(2, BatchValue::Int(29)));
            let before = batch.snapshot_value();
            assert!(batch.record(1, BatchValue::Int(39)));
            assert!(!batch.record(128, BatchValue::Int(99)));
            owner = Value::array(vec![Value::Int(100); 128]);
            let (_, result) = batch.publish(Some(&mut owner));
            assert_eq!(item(&owner, 1), 100);
            assert_eq!(item(&result, 1), 39);
            assert_eq!(item(&result, 2), 29);
            assert_eq!(item(&before, 1), 19);
            assert_eq!(item(&snapshot, 1), 1);
        }
    }

    #[test]
    fn name_and_unique_public_storage_are_reused_only_at_publication() {
        let mut owner = original();
        let pointer = Arc::as_ptr(&base(&owner));
        let name = String::from("state");
        let key_pointer = name.as_ptr();
        let mut pending =
            IndexedPublication::prepare(name, base(&owner), 0, &Value::Int(10)).unwrap();
        assert_eq!(pending.name().as_ptr(), key_pointer);
        assert_eq!(pending.len(), 128);
        assert!(pending.try_record(0, &Value::Int(11)));
        assert!(pending.try_record(1, &Value::Int(12)));
        assert_eq!(item(&owner, 0), 0);
        assert!(matches!(pending.get(0), Some(Value::Int(11))));
        assert!(matches!(pending.get(127), Some(Value::Int(127))));
        let (key, value) = pending.publish(Some(&mut owner));
        assert_eq!(key.as_ptr(), key_pointer);
        assert_eq!(Arc::as_ptr(&base(&owner)), pointer);
        assert_eq!(item(&owner, 0), 11);
        assert_eq!(item(&value, 1), 12);
    }

    #[test]
    fn late_observer_and_owner_replacement_preserve_both_snapshots() {
        let mut owner = original();
        let pending =
            IndexedPublication::prepare("state".into(), base(&owner), 0, &Value::Int(9)).unwrap();
        let observer = owner.clone();
        let (_, result) = pending.publish(Some(&mut owner));
        assert_eq!(item(&observer, 0), 0);
        assert_eq!(item(&owner, 0), 9);
        drop(result);
        let pending =
            IndexedPublication::prepare("state".into(), base(&owner), 1, &Value::Int(8)).unwrap();
        let old = owner.clone();
        owner = Value::Array(Arc::new(vec![Value::Int(100); 128]));
        let replacement = owner.clone();
        let (_, result) = pending.publish(Some(&mut owner));
        assert_eq!(item(&owner, 0), 100);
        owner = result;
        assert_eq!(item(&old, 1), 1);
        assert_eq!(item(&replacement, 0), 100);
        assert_eq!(item(&owner, 0), 9);
        assert_eq!(item(&owner, 1), 8);
    }

    #[test]
    fn unsupported_updates_are_atomic_and_materialization_is_independent() {
        let owner = original();
        let mut pending =
            IndexedPublication::prepare("state".into(), base(&owner), 0, &Value::Int(10)).unwrap();
        assert!(pending.try_record(1, &Value::Int(20)));
        assert!(!pending.try_record(2, &Value::Int(30)));
        assert!(!pending.try_record(0, &Value::Str(Arc::new("no".into()))));
        assert!(!pending.try_record(128, &Value::Int(30)));
        assert!(!pending.try_record(INDEX_MASK as usize + 1, &Value::Int(30)));
        let snapshot = pending.clone().materialize().1;
        assert!(pending.try_record(0, &Value::Int(11)));
        let current = pending.materialize().1;
        assert_eq!(item(&snapshot, 0), 10);
        assert_eq!(item(&current, 0), 11);
        assert_eq!(item(&current, 1), 20);
        assert_eq!(item(&current, 2), 2);
        assert_eq!(item(&owner, 0), 0);
    }

    #[test]
    fn scalar_bits_and_constructor_rejections_preserve_values() {
        let owner = original();
        let mut pending =
            IndexedPublication::prepare("state".into(), base(&owner), 0, &Value::Null).unwrap();
        for value in [
            Value::Int(i64::MIN),
            Value::Int(i64::MAX),
            Value::Float(-0.0),
            Value::Float(f64::from_bits(0x7ff8000000000011)),
            Value::Bool(false),
            Value::Bool(true),
            Value::Null,
        ] {
            assert!(pending.try_record(0, &value));
            let actual = pending.get(0).unwrap();
            match (&actual, &value) {
                (Value::Float(a), Value::Float(b)) => assert_eq!(a.to_bits(), b.to_bits()),
                _ => assert!(Value::equals(&actual, &value)),
            }
        }
        let mut name = String::with_capacity(64);
        name.push_str("state");
        let capacity = name.capacity();
        let pointer = name.as_ptr();
        let (name, returned) =
            IndexedPublication::prepare(name, base(&owner), 0, &Value::Int(1)).unwrap_err();
        assert_eq!(name.capacity(), capacity);
        assert_eq!(name.as_ptr(), pointer);
        assert!(Arc::ptr_eq(&returned, &base(&owner)));
        assert!(
            IndexedPublication::prepare("state".into(), base(&owner), 128, &Value::Int(1)).is_err()
        );
        let (_, unchanged) = pending.discard();
        assert_eq!(item(&unchanged, 0), 0);
    }

    #[test]
    fn operation_sequences_match_ordinary_private_copy_publication() {
        // Enumerate ordering, not just individual boundaries: observers may
        // arrive before or after replacement, escape, or overflow fallback.
        // This is a storage test, not a substitute for runtime boundary tests.
        for sequence in 0..7_usize.pow(5) {
            let mut owner = original();
            let mut ordinary_owner = original();
            let mut ordinary = ordinary_owner.clone();
            let mut pending = Some(
                IndexedPublication::prepare("state".into(), base(&owner), 0, &Value::Int(10))
                    .unwrap(),
            );
            let mut flat = None;
            let Value::Array(values) = &mut ordinary else { unreachable!() };
            Arc::make_mut(values)[0] = Value::Int(10);
            let mut snapshots = Vec::new();
            let mut operations = sequence;
            for step in 0..5 {
                let operation = operations % 7;
                operations /= 7;
                match operation {
                    0..=3 => {
                        let index = operation.min(2);
                        let value = if operation == 3 {
                            Value::Array(Arc::new(vec![Value::Int(step)]))
                        } else {
                            Value::Int(100 + step)
                        };
                        if !pending.as_mut().is_some_and(|p| p.try_record(index, &value)) {
                            if let Some(journal) = pending.take() {
                                flat = Some(journal.materialize().1);
                            }
                            let Some(Value::Array(values)) = &mut flat else { unreachable!() };
                            Arc::make_mut(values)[index] = value.clone();
                        }
                        let Value::Array(values) = &mut ordinary else { unreachable!() };
                        Arc::make_mut(values)[index] = value;
                    }
                    4 => snapshots.push((owner.clone(), ordinary_owner.clone())),
                    5 => {
                        if let Some(journal) = pending.take() {
                            flat = Some(journal.materialize().1);
                        }
                        snapshots.push((flat.as_ref().unwrap().clone(), ordinary.clone()));
                    }
                    6 => {
                        owner = Value::Array(Arc::new(vec![Value::Int(-step); 128]));
                        ordinary_owner = Value::Array(Arc::new(vec![Value::Int(-step); 128]));
                    }
                    _ => unreachable!(),
                }
                assert!(Value::equals(&owner, &ordinary_owner), "early publication {sequence}");
                let Value::Array(expected) = &ordinary else { unreachable!() };
                for (index, expected) in expected.iter().enumerate() {
                    let actual = if let Some(journal) = &pending {
                        journal.get(index).unwrap()
                    } else {
                        let Some(Value::Array(values)) = &flat else { unreachable!() };
                        values[index].clone()
                    };
                    assert!(Value::equals(&actual, expected), "private read {sequence}/{step}");
                }
            }
            owner = if let Some(journal) = pending {
                journal.publish(Some(&mut owner)).1
            } else {
                flat.unwrap()
            };
            assert!(Value::equals(&owner, &ordinary), "publication {sequence}");
            for (actual, expected) in snapshots {
                assert!(Value::equals(&actual, &expected), "escaped snapshot {sequence}");
            }
        }
    }
}
