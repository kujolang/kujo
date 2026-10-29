//! A bounded collection-valued replacement in a captured array. Payloads stay
//! private until publication; snapshots and replaced owners retain their values.
use super::dictionary_publication::CollectionPayload;
use super::Value;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(super) struct ArrayCollectionPublication<T> {
    name: Box<str>,
    base: Arc<Vec<Value>>,
    index: usize,
    value: T,
}

impl<T: CollectionPayload> ArrayCollectionPublication<T> {
    pub(super) fn prepare(name: String, base: Arc<Vec<Value>>, index: usize, value: T) -> Self {
        // Admission, including exact key capacity, is checked before allocating
        // a payload or taking the private environment's entry.
        debug_assert_eq!(name.len(), name.capacity());
        debug_assert!(index < base.len());
        Self { name: name.into_boxed_str(), base, index, value }
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn len(&self) -> usize {
        self.base.len()
    }

    pub(super) fn matches_index(&self, index: usize) -> bool {
        self.index == index
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
        ArrayCollectionPublication { name: self.name, base: self.base, index: self.index, value }
    }

    pub(super) fn get(&self, index: usize) -> Option<Value> {
        if self.index == index {
            Some(self.value.value())
        } else {
            self.base.get(index).cloned()
        }
    }

    pub(super) fn snapshot_value(&self) -> Value {
        let mut base = self.base.clone();
        Arc::make_mut(&mut base)[self.index] = self.value.value();
        Value::Array(base)
    }

    pub(super) fn publish(self, owner: Option<&mut Value>) -> (String, Value) {
        let Self { name, mut base, index, value } = self;
        if let Some(Value::Array(public)) = owner {
            if Arc::ptr_eq(&base, public) {
                drop(base);
                Arc::make_mut(public)[index] = value.into_value();
                return (name.into_string(), Value::Array(public.clone()));
            }
        }
        Arc::make_mut(&mut base)[index] = value.into_value();
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
