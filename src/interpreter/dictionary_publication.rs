//! One existing dictionary key updated without copying its captured public owner.
//! The key is an existing Arc, so overflow/fallback adds no journal allocation.
use super::value::DictMap;
use super::Value;
use std::sync::Arc;

/// Match ordinary decimal dictionary keys without allocating on admission or
/// rejection. Twenty bytes cover every i64, including the sign of i64::MIN.
pub(super) fn with_integer_key<T>(index: i64, lookup: impl FnOnce(&str) -> T) -> T {
    use std::io::Write;
    let mut bytes = [0u8; 20];
    let mut remaining = bytes.as_mut_slice();
    write!(remaining, "{index}").expect("i64 decimal fits in twenty bytes");
    let length = 20 - remaining.len();
    lookup(std::str::from_utf8(&bytes[..length]).expect("decimal digits are UTF-8"))
}

#[derive(Clone, Debug)]
pub(super) struct DictionaryPublication<T = i64> {
    name: Box<str>,
    base: Arc<DictMap>,
    key: Arc<str>,
    value: T,
}

// The kind belongs to CaptureWrites' existing discriminant, not this payload.
// Keeping each value concrete avoids growing every captured call frame.
pub(super) trait DictionaryScalar: Copy + std::fmt::Debug {
    fn into_value(self) -> Value;
}

impl DictionaryScalar for i64 {
    fn into_value(self) -> Value {
        Value::Int(self)
    }
}
impl DictionaryScalar for f64 {
    fn into_value(self) -> Value {
        Value::Float(self)
    }
}
impl DictionaryScalar for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}
impl DictionaryScalar for () {
    fn into_value(self) -> Value {
        Value::Null
    }
}

impl DictionaryPublication<i64> {
    pub(super) fn prepare(
        name: String,
        base: Arc<DictMap>,
        key: &str,
        value: i64,
    ) -> Result<Self, (String, Arc<DictMap>)> {
        Self::prepare_scalar(name, base, key, value)
    }
}

impl<T: DictionaryScalar> DictionaryPublication<T> {
    pub(super) fn prepare_scalar(
        name: String,
        base: Arc<DictMap>,
        key: &str,
        value: T,
    ) -> Result<Self, (String, Arc<DictMap>)> {
        if name.capacity() != name.len() {
            return Err((name, base));
        }
        let Some((key, _)) = base.get_key_value(key) else { return Err((name, base)) };
        let key = key.clone();
        Ok(Self { name: name.into_boxed_str(), base, key, value })
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn get(&self, key: &str) -> Option<Value> {
        if key == self.key.as_ref() {
            Some(self.value.into_value())
        } else {
            self.base.get(key).cloned()
        }
    }

    pub(super) fn matches_key(&self, key: &str) -> bool {
        key == self.key.as_ref()
    }

    pub(super) fn with_value<U: DictionaryScalar>(self, value: U) -> DictionaryPublication<U> {
        DictionaryPublication { name: self.name, base: self.base, key: self.key, value }
    }

    #[cfg(test)]
    pub(super) fn try_record(&mut self, key: &str, value: T) -> bool {
        if key != self.key.as_ref() {
            return false;
        }
        self.value = value;
        true
    }

    fn apply(base: &mut Arc<DictMap>, key: &str, value: T) {
        *Arc::make_mut(base).get_mut(key).expect("admitted existing dictionary key") =
            value.into_value();
    }

    pub(super) fn snapshot_value(&self) -> Value {
        let mut base = self.base.clone();
        Self::apply(&mut base, &self.key, self.value);
        Value::Dict(base)
    }

    pub(super) fn publish(self, owner: Option<&mut Value>) -> (String, Value) {
        let Self { name, mut base, key, value } = self;
        if let Some(Value::Dict(public)) = owner {
            if Arc::ptr_eq(&base, public) {
                drop(base);
                Self::apply(public, &key, value);
                return (name.into_string(), Value::Dict(public.clone()));
            }
        }
        Self::apply(&mut base, &key, value);
        (name.into_string(), Value::Dict(base))
    }

    pub(super) fn discard(self) -> (String, Value) {
        (self.name.into_string(), Value::Dict(self.base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_type_changes_move_storage_without_canonicalizing_float_bits() {
        let base = original();
        let mut owner = Value::Dict(base.clone());
        let pending =
            DictionaryPublication::prepare("state".into(), base, "key", i64::MIN).unwrap();
        let pointers = (pending.name.as_ptr(), Arc::as_ptr(&pending.base), pending.key.as_ptr());
        let pending = pending.with_value(f64::from_bits(0xfff8_0000_0000_0042));
        let Some(Value::Float(value)) = pending.get("key") else { panic!() };
        assert_eq!(value.to_bits(), 0xfff8_0000_0000_0042);
        let pending = pending.with_value(-0.0_f64);
        let snapshot = pending.snapshot_value();
        let pending = pending.with_value(false);
        assert!(matches!(pending.get("key"), Some(Value::Bool(false))));
        let pending = pending.with_value(());
        assert!(matches!(pending.get("key"), Some(Value::Null)));
        assert!(pending.get("missing").is_none());
        let pending = pending.with_value(i64::MAX);
        assert_eq!(
            pointers,
            (pending.name.as_ptr(), Arc::as_ptr(&pending.base), pending.key.as_ptr())
        );
        let (_, result) = pending.publish(Some(&mut owner));
        assert!(matches!(result.dictionary_get("key"), Some(Value::Int(i64::MAX))));
        let Some(Value::Float(value)) = snapshot.dictionary_get("key") else { panic!() };
        assert_eq!(value.to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn scalar_payloads_fit_existing_storage() {
        let original = std::mem::size_of::<DictionaryPublication>();
        assert!(std::mem::size_of::<DictionaryPublication<f64>>() <= original);
        assert!(std::mem::size_of::<DictionaryPublication<bool>>() <= original);
        assert!(std::mem::size_of::<DictionaryPublication<()>>() <= original);
    }

    #[test]
    fn integer_keys_use_canonical_decimal_at_both_limits() {
        for index in [i64::MIN, -100, -1, 0, 1, 100, i64::MAX] {
            with_integer_key(index, |key| assert_eq!(key, index.to_string()));
        }
        with_integer_key(1, |key| {
            assert_ne!(key, "01");
            assert_ne!(key, "+1");
        });
    }

    fn original() -> Arc<DictMap> {
        Arc::new(
            [(Arc::from("key"), Value::Int(1)), (Arc::from("other"), Value::Int(2))]
                .into_iter()
                .collect(),
        )
    }

    #[test]
    fn reuses_key_and_unique_owner_without_early_publication() {
        let base = original();
        let pointer = Arc::as_ptr(&base);
        let key = base.get_key_value("key").unwrap().0.clone();
        let mut owner = Value::Dict(base.clone());
        let mut pending = DictionaryPublication::prepare("state".into(), base, "key", 9).unwrap();
        assert!(Arc::ptr_eq(&key, &pending.key));
        assert!(pending.try_record("key", i64::MAX));
        assert!(!pending.try_record("other", 8));
        assert!(matches!(owner.dictionary_get("key"), Some(Value::Int(1))));
        assert!(matches!(pending.get("key"), Some(Value::Int(i64::MAX))));
        assert!(matches!(pending.get("other"), Some(Value::Int(2))));
        let (_, result) = pending.publish(Some(&mut owner));
        let Value::Dict(current) = result else { panic!() };
        assert_eq!(Arc::as_ptr(&current), pointer);
        assert!(matches!(current["key"], Value::Int(i64::MAX)));
    }

    #[test]
    fn snapshots_replacement_and_rollback_keep_original_values() {
        let mut owner = Value::Dict(original());
        let Value::Dict(base) = &owner else { panic!() };
        let pending =
            DictionaryPublication::prepare("state".into(), base.clone(), "key", 9).unwrap();
        let observer = owner.clone();
        let snapshot = pending.snapshot_value();
        let mut next = pending.clone();
        assert!(next.try_record("key", i64::MIN));
        next.publish(Some(&mut owner));
        assert!(matches!(observer.dictionary_get("key"), Some(Value::Int(1))));
        assert!(matches!(snapshot.dictionary_get("key"), Some(Value::Int(9))));
        assert!(matches!(owner.dictionary_get("key"), Some(Value::Int(i64::MIN))));
        owner = Value::Null;
        let result = pending.clone().publish(Some(&mut owner)).1;
        assert!(matches!(owner, Value::Null));
        assert!(matches!(result.dictionary_get("key"), Some(Value::Int(9))));
        let original = pending.discard().1;
        assert!(matches!(original.dictionary_get("key"), Some(Value::Int(1))));
    }

    #[test]
    fn constructor_rejections_are_atomic_and_payload_fits() {
        assert!(
            std::mem::size_of::<DictionaryPublication>()
                <= std::mem::size_of::<std::collections::HashSet<(usize, String)>>()
        );
        let base = original();
        let pointer = Arc::as_ptr(&base);
        let (name, returned) =
            DictionaryPublication::prepare("state".into(), base, "missing", 9).unwrap_err();
        assert_eq!(name, "state");
        assert_eq!(Arc::as_ptr(&returned), pointer);
        let mut name = String::with_capacity(64);
        name.push_str("state");
        let pointer = name.as_ptr();
        let capacity = name.capacity();
        let (name, _) = DictionaryPublication::prepare(name, returned, "key", 9).unwrap_err();
        assert_eq!(name.as_ptr(), pointer);
        assert_eq!(name.capacity(), capacity);
    }
}
