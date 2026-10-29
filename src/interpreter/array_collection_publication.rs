//! Bounded collection-valued replacements in a captured array. Payloads stay
//! private until publication; snapshots and replaced owners retain their values.
use super::dictionary_publication::CollectionPayload;
use super::Value;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(super) struct ArrayCollectionPublication<T> {
    name: Box<str>,
    base: Arc<Vec<Value>>,
    indices: [u32; 2],
    value: T,
    second: Option<T>,
}

impl<T: CollectionPayload> ArrayCollectionPublication<T> {
    pub(super) fn prepare(name: String, base: Arc<Vec<Value>>, index: usize, value: T) -> Self {
        // Admission, including exact key capacity, is checked before allocating
        // a payload or taking the private environment's entry.
        debug_assert_eq!(name.len(), name.capacity());
        debug_assert!(index < base.len());
        debug_assert!(index <= u32::MAX as usize);
        Self { name: name.into_boxed_str(), base, indices: [index as u32, 0], value, second: None }
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn len(&self) -> usize {
        self.base.len()
    }

    pub(super) fn matches_index(&self, index: usize) -> bool {
        self.indices[0] as usize == index
    }

    pub(super) fn is_single(&self) -> bool {
        self.second.is_none()
    }

    pub(super) fn into_batch(
        self,
        capacity: u16,
    ) -> super::array_batch_publication::ArrayBatchPublication {
        use super::array_batch_publication::{ArrayBatchPublication, BatchValue};
        let Self { name, base, indices, value, second } = self;
        let mut batch = ArrayBatchPublication::new(name, base, capacity);
        assert!(batch.record(indices[0] as usize, BatchValue::from_owned(value.into_value())));
        if let Some(value) = second {
            assert!(batch.record(indices[1] as usize, BatchValue::from_owned(value.into_value())));
        }
        batch
    }

    /// Two same-kind handles fit the existing journal payload. Reject a third
    /// distinct index atomically so ordinary mutation can publish both writes.
    pub(super) fn try_record(&mut self, index: usize, value: T) -> bool {
        if index >= self.base.len() || index > u32::MAX as usize {
            return false;
        }
        if self.matches_index(index) {
            self.value = value;
        } else if self.second.is_none() || self.indices[1] as usize == index {
            self.indices[1] = index as u32;
            self.second = Some(value);
        } else {
            return false;
        }
        true
    }

    pub(super) fn payload_mut(&mut self) -> &mut T {
        &mut self.value
    }

    #[cfg(test)]
    pub(super) fn replace(&mut self, value: T) {
        self.value = value;
    }

    pub(super) fn with_payload<U: CollectionPayload>(
        self,
        value: U,
    ) -> ArrayCollectionPublication<U> {
        assert!(self.is_single(), "payload-kind conversion requires a single pending write");
        ArrayCollectionPublication {
            name: self.name,
            base: self.base,
            indices: self.indices,
            value,
            second: None,
        }
    }

    pub(super) fn get(&self, index: usize) -> Option<Value> {
        if self.matches_index(index) {
            Some(self.value.value())
        } else if self.indices[1] as usize == index && self.second.is_some() {
            Some(self.second.as_ref().unwrap().value())
        } else {
            self.base.get(index).cloned()
        }
    }

    pub(super) fn snapshot_value(&self) -> Value {
        let mut base = self.base.clone();
        let values = Arc::make_mut(&mut base);
        values[self.indices[0] as usize] = self.value.value();
        if let Some(second) = &self.second {
            values[self.indices[1] as usize] = second.value();
        }
        Value::Array(base)
    }

    pub(super) fn publish(self, owner: Option<&mut Value>) -> (String, Value) {
        let Self { name, mut base, indices, value, second } = self;
        if let Some(Value::Array(public)) = owner {
            if Arc::ptr_eq(&base, public) {
                drop(base);
                let values = Arc::make_mut(public);
                values[indices[0] as usize] = value.into_value();
                if let Some(second) = second {
                    values[indices[1] as usize] = second.into_value();
                }
                return (name.into_string(), Value::Array(public.clone()));
            }
        }
        let values = Arc::make_mut(&mut base);
        values[indices[0] as usize] = value.into_value();
        if let Some(second) = second {
            values[indices[1] as usize] = second.into_value();
        }
        (name.into_string(), Value::Array(base))
    }

    pub(super) fn discard(self) -> (String, Value) {
        (self.name.into_string(), Value::Array(self.base))
    }
}

#[cfg(test)]
mod tests {
    use super::super::dictionary_publication::CollectionValue;
    use super::*;

    #[test]
    fn pair_storage_fits_existing_capture_payload() {
        let limit = std::mem::size_of::<std::collections::HashSet<(usize, String)>>();
        assert!(std::mem::size_of::<ArrayCollectionPublication<Arc<Vec<Value>>>>() <= limit);
        assert!(
            std::mem::size_of::<ArrayCollectionPublication<Arc<super::super::value::DictMap>>>()
                <= limit
        );
        assert!(std::mem::size_of::<ArrayCollectionPublication<Box<CollectionValue>>>() <= limit);
    }

    #[test]
    fn paired_updates_preserve_snapshots_and_reject_overflow_atomically() {
        let mut owner = Value::array(vec![Value::Null; 3]);
        let Value::Array(base) = &owner else { unreachable!() };
        let pointer = Arc::as_ptr(base);
        let payload = |n| Arc::new(vec![Value::Int(n)]);
        let mut pending =
            ArrayCollectionPublication::prepare("state".into(), base.clone(), 0, payload(1));
        assert!(pending.try_record(1, payload(2)));
        assert!(!pending.is_single());
        let saved = pending.snapshot_value();
        assert!(pending.try_record(0, payload(3)));
        assert!(pending.try_record(1, payload(4)));
        assert!(!pending.try_record(2, payload(5)));
        assert!(!pending.try_record(usize::MAX, payload(6)));
        assert!(Value::equals(&pending.get(0).unwrap(), &Value::array(vec![Value::Int(3)])));
        assert!(Value::equals(&pending.get(1).unwrap(), &Value::array(vec![Value::Int(4)])));
        assert!(matches!(pending.get(2), Some(Value::Null)));
        let (_, result) = pending.publish(Some(&mut owner));
        let Value::Array(values) = &result else { unreachable!() };
        assert_eq!(Arc::as_ptr(values), pointer);
        assert!(Value::equals(
            &result,
            &Value::array(vec![
                Value::array(vec![Value::Int(3)]),
                Value::array(vec![Value::Int(4)]),
                Value::Null
            ])
        ));
        assert!(Value::equals(
            &saved,
            &Value::array(vec![
                Value::array(vec![Value::Int(1)]),
                Value::array(vec![Value::Int(2)]),
                Value::Null
            ])
        ));
    }

    #[test]
    fn collection_publication_preserves_snapshots_replacements_and_unique_storage() {
        for replace_owner in [false, true] {
            let mut owner = Value::array(vec![Value::Int(0), Value::Int(1)]);
            let Value::Array(base) = &owner else { unreachable!() };
            let pointer = Arc::as_ptr(base);
            let mut pending = ArrayCollectionPublication::prepare(
                "state".into(),
                base.clone(),
                1,
                Arc::new(vec![Value::Int(2)]),
            );
            assert_eq!(pending.len(), 2);
            assert!(pending.matches_index(1));
            assert!(!pending.matches_index(0));
            assert!(pending.get(2).is_none());
            let first = pending.snapshot_value();
            pending.replace(Arc::new(vec![Value::Int(3)]));
            assert!(Value::equals(
                &first,
                &Value::array(vec![Value::Int(0), Value::array(vec![Value::Int(2)])])
            ));
            let pending = pending.with_payload(Box::new(CollectionValue::from_owned_value(
                Value::Dict(Arc::new(Default::default())),
            )));
            if replace_owner {
                owner = Value::array(vec![Value::Int(99)]);
            }
            let (_, result) = pending.publish(Some(&mut owner));
            let Value::Array(values) = &result else { unreachable!() };
            assert!(matches!(values[1], Value::Dict(_)));
            if replace_owner {
                assert!(Value::equals(&owner, &Value::array(vec![Value::Int(99)])));
            } else {
                assert_eq!(Arc::as_ptr(values), pointer);
                assert!(Value::equals(&owner, &result));
            }
            assert!(Value::equals(
                &first,
                &Value::array(vec![Value::Int(0), Value::array(vec![Value::Int(2)])])
            ));
        }
    }

    #[test]
    fn late_alias_and_discard_preserve_the_original_array() {
        let mut owner = Value::array(vec![Value::Int(0)]);
        let Value::Array(base) = &owner else { unreachable!() };
        let pending = ArrayCollectionPublication::prepare(
            "state".into(),
            base.clone(),
            0,
            Arc::new(vec![Value::Int(1)]),
        );
        let alias = owner.clone();
        let discarded = pending.clone().discard().1;
        let (_, result) = pending.publish(Some(&mut owner));
        assert!(Value::equals(&alias, &discarded));
        assert!(Value::equals(&alias, &Value::array(vec![Value::Int(0)])));
        assert!(Value::equals(&owner, &result));
        assert!(Value::equals(&result, &Value::array(vec![Value::array(vec![Value::Int(1)])])));
    }
}
