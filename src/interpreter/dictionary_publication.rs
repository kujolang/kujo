//! Dictionary keys updated without copying their captured public owner.
//! The first key stays inline; a second key promotes to a sparse write table.
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

/// Array and dictionary entries retain their existing Arc directly. Larger
/// Value variants use a Box so every capture payload remains pointer-sized.
/// Borrowed reads never allocate a replacement Box.
#[derive(Clone, Debug)]
pub(super) struct DictionaryCollectionPublication<T = Box<CollectionValue>> {
    name: Box<str>,
    base: Arc<DictMap>,
    key: Arc<str>,
    value: T,
}

// Keep the boxed fallback proportional to a collection handle, not the largest
// runtime Value variant. Struct names retain their text without spare capacity.
#[derive(Clone, Debug)]
pub(super) enum CollectionValue {
    Array(Arc<Vec<Value>>),
    Dict(Arc<DictMap>),
    Struct { name: Box<str>, fields: Arc<std::collections::HashMap<String, Value>> },
    FixedDict { keys: Arc<Vec<Arc<str>>>, values: Arc<Vec<Value>> },
    IntDict(Arc<super::value::IntDictMap>),
    DenseIntDict(Arc<super::value::DenseIntDict>),
    DenseIntDictInt(Arc<super::value::DenseIntDictInt>),
    DenseIntDictIntFull(Arc<super::value::DenseIntDictIntFull>),
}

impl CollectionValue {
    pub(super) fn from_owned_value(value: Value) -> Self {
        match value {
            Value::Array(v) => Self::Array(v),
            Value::Dict(v) => Self::Dict(v),
            Value::Struct { name, fields } => Self::Struct { name: name.into_boxed_str(), fields },
            Value::FixedDict { keys, values } => Self::FixedDict { keys, values },
            Value::IntDict(v) => Self::IntDict(v),
            Value::DenseIntDict(v) => Self::DenseIntDict(v),
            Value::DenseIntDictInt(v) => Self::DenseIntDictInt(v),
            Value::DenseIntDictIntFull(v) => Self::DenseIntDictIntFull(v),
            _ => unreachable!("admitted collection payload"),
        }
    }

    pub(super) fn from_value(value: &Value) -> Self {
        match value {
            Value::Array(v) => Self::Array(v.clone()),
            Value::Dict(v) => Self::Dict(v.clone()),
            Value::Struct { name, fields } => {
                Self::Struct { name: name.clone().into_boxed_str(), fields: fields.clone() }
            }
            Value::FixedDict { keys, values } => {
                Self::FixedDict { keys: keys.clone(), values: values.clone() }
            }
            Value::IntDict(v) => Self::IntDict(v.clone()),
            Value::DenseIntDict(v) => Self::DenseIntDict(v.clone()),
            Value::DenseIntDictInt(v) => Self::DenseIntDictInt(v.clone()),
            Value::DenseIntDictIntFull(v) => Self::DenseIntDictIntFull(v.clone()),
            _ => unreachable!("admitted collection payload"),
        }
    }
    pub(super) fn into_value(self) -> Value {
        match self {
            Self::Array(v) => Value::Array(v),
            Self::Dict(v) => Value::Dict(v),
            Self::Struct { name, fields } => Value::Struct { name: name.into_string(), fields },
            Self::FixedDict { keys, values } => Value::FixedDict { keys, values },
            Self::IntDict(v) => Value::IntDict(v),
            Self::DenseIntDict(v) => Value::DenseIntDict(v),
            Self::DenseIntDictInt(v) => Value::DenseIntDictInt(v),
            Self::DenseIntDictIntFull(v) => Value::DenseIntDictIntFull(v),
        }
    }
}

pub(super) trait CollectionPayload: Clone + std::fmt::Debug {
    fn into_value(self) -> Value;
    fn value(&self) -> Value {
        self.clone().into_value()
    }
}

impl CollectionPayload for Arc<Vec<Value>> {
    fn into_value(self) -> Value {
        Value::Array(self)
    }
}

impl CollectionPayload for Arc<DictMap> {
    fn into_value(self) -> Value {
        Value::Dict(self)
    }
}

impl CollectionPayload for Box<CollectionValue> {
    fn into_value(self) -> Value {
        (*self).into_value()
    }
    fn value(&self) -> Value {
        self.as_ref().clone().into_value()
    }
}

impl DictionaryCollectionPublication {
    pub(super) fn supports(value: &Value) -> bool {
        matches!(value, Value::Array(_) | Value::Struct { .. }) || value.dictionary_len().is_some()
    }

    pub(super) fn prepare(
        name: String,
        base: Arc<DictMap>,
        key: &str,
        value: &Value,
    ) -> Result<Self, (String, Arc<DictMap>)> {
        // Do not allocate payload storage on a rejected admission path.
        if name.capacity() != name.len() || !Self::supports(value) {
            return Err((name, base));
        }
        Self::prepare_payload(name, base, key, Box::new(CollectionValue::from_value(value)))
    }

    pub(super) fn replace(&mut self, value: &Value) {
        debug_assert!(Self::supports(value));
        *self.value = CollectionValue::from_value(value);
    }
}

impl<T: CollectionPayload> DictionaryCollectionPublication<T> {
    pub(super) fn with_collection(self, value: &Value) -> DictionaryCollectionPublication {
        self.with_payload(Box::new(CollectionValue::from_value(value)))
    }
    pub(super) fn prepare_payload(
        name: String,
        base: Arc<DictMap>,
        key: &str,
        value: T,
    ) -> Result<Self, (String, Arc<DictMap>)> {
        if name.capacity() != name.len() {
            return Err((name, base));
        }
        let key = base.get_key_value(key).map_or_else(|| Arc::from(key), |(key, _)| key.clone());
        Ok(Self { name: name.into_boxed_str(), base, key, value })
    }

    pub(super) fn with_payload<U: CollectionPayload>(
        self,
        value: U,
    ) -> DictionaryCollectionPublication<U> {
        DictionaryCollectionPublication { name: self.name, base: self.base, key: self.key, value }
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }
    pub(super) fn matches_key(&self, key: &str) -> bool {
        self.key.as_ref() == key
    }
    pub(super) fn get(&self, key: &str) -> Option<Value> {
        if self.matches_key(key) {
            Some(self.value.value())
        } else {
            self.base.get(key).cloned()
        }
    }
    pub(super) fn with_scalar<U: DictionaryScalar>(self, value: U) -> DictionaryPublication<U> {
        DictionaryPublication { name: self.name, base: self.base, key: self.key, value }
    }
    fn apply(base: &mut Arc<DictMap>, key: &Arc<str>, value: Value) {
        let map = Arc::make_mut(base);
        if let Some(slot) = map.get_mut(key.as_ref()) {
            *slot = value;
        } else {
            map.insert(key.clone(), value);
        }
    }
    pub(super) fn snapshot_value(&self) -> Value {
        let mut base = self.base.clone();
        Self::apply(&mut base, &self.key, self.value.value());
        Value::Dict(base)
    }
    pub(super) fn publish(self, owner: Option<&mut Value>) -> (String, Value) {
        let Self { name, mut base, key, value } = self;
        if let Some(Value::Dict(public)) = owner {
            if Arc::ptr_eq(&base, public) {
                drop(base);
                Self::apply(public, &key, value.into_value());
                return (name.into_string(), Value::Dict(public.clone()));
            }
        }
        Self::apply(&mut base, &key, value.into_value());
        (name.into_string(), Value::Dict(base))
    }
    pub(super) fn discard(self) -> (String, Value) {
        (self.name.into_string(), Value::Dict(self.base))
    }
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
        let key = base.get_key_value(key).map_or_else(|| Arc::from(key), |(key, _)| key.clone());
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

    pub(super) fn contains_key(&self, key: &str) -> bool {
        self.base.contains_key(key)
    }

    pub(super) fn can_insert(&self, writes: u16) -> bool {
        insertion_can_pay(&self.base, writes)
    }

    pub(super) fn into_many(self, key: &str, value: &Value) -> DictionaryManyPublication {
        let mut writes = ScalarWrites::default();
        writes.insert(self.key, ScalarValue::from_value(&self.value.into_value()));
        let stored =
            self.base.get_key_value(key).map_or_else(|| Arc::from(key), |(key, _)| key.clone());
        writes.insert(stored, ScalarValue::from_value(value));
        DictionaryManyPublication { name: self.name, base: self.base, writes }
    }

    pub(super) fn matches_key(&self, key: &str) -> bool {
        key == self.key.as_ref()
    }

    pub(super) fn can_pair(&self, key: &str, value: &Value) -> bool {
        // The pair borrows canonical keys from the base at publication. Keep an
        // inserted key's owned allocation in the scalar or sparse journal.
        self.base.contains_key(self.key.as_ref())
            && self.base.contains_key(key)
            && DictionaryPairPublication::encode(&self.key, &self.value.into_value()).is_some()
            && DictionaryPairPublication::encode(key, value).is_some()
    }

    pub(super) fn into_pair(self, key: &str, value: &Value) -> DictionaryPairPublication {
        let first = DictionaryPairPublication::encode(&self.key, &self.value.into_value())
            .expect("admitted first numeric dictionary key");
        let second = DictionaryPairPublication::encode(key, value)
            .expect("admitted second numeric dictionary key");
        DictionaryPairPublication {
            name: self.name,
            base: self.base,
            entries: [first.0, second.0],
            words: [first.1, second.1],
        }
    }

    pub(super) fn with_collection(self, value: &Value) -> DictionaryCollectionPublication {
        debug_assert!(DictionaryCollectionPublication::supports(value));
        self.with_collection_payload(Box::new(CollectionValue::from_value(value)))
    }

    pub(super) fn with_collection_payload<U: CollectionPayload>(
        self,
        value: U,
    ) -> DictionaryCollectionPublication<U> {
        DictionaryCollectionPublication { name: self.name, base: self.base, key: self.key, value }
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

    fn apply(base: &mut Arc<DictMap>, key: &Arc<str>, value: T) {
        let target = Arc::make_mut(base);
        if let Some(slot) = target.get_mut(key.as_ref()) {
            *slot = value.into_value();
        } else {
            target.insert(key.clone(), value.into_value());
        }
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
    fn inserted_key_keeps_owner_snapshots_and_promotion_independent() {
        let base = original();
        let mut owner = Value::Dict(base.clone());
        let pending = DictionaryPublication::prepare("state".into(), base, "0", 9).unwrap();
        assert!(owner.dictionary_get("0").is_none());
        assert!(matches!(pending.get("0"), Some(Value::Int(9))));
        assert!(!pending.can_pair("1", &Value::Int(10)));
        let snapshot = pending.snapshot_value();
        let discarded = pending.clone().discard().1;
        assert!(discarded.dictionary_get("0").is_none());
        let mut many = pending.into_many("1", &Value::Bool(false));
        many.record("0", &Value::Null);
        let (_, published) = many.publish(Some(&mut owner));
        assert!(matches!(snapshot.dictionary_get("0"), Some(Value::Int(9))));
        assert!(snapshot.dictionary_get("1").is_none());
        assert!(matches!(published.dictionary_get("0"), Some(Value::Null)));
        assert!(matches!(owner.dictionary_get("1"), Some(Value::Bool(false))));
        assert!(matches!(discarded.dictionary_get("key"), Some(Value::Int(1))));
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
        let mut name = String::with_capacity(64);
        name.push_str("state");
        let pointer = name.as_ptr();
        let capacity = name.capacity();
        let base_pointer = Arc::as_ptr(&base);
        let (name, returned) = DictionaryPublication::prepare(name, base, "key", 9).unwrap_err();
        assert_eq!(Arc::as_ptr(&returned), base_pointer);
        assert_eq!(name.as_ptr(), pointer);
        assert_eq!(name.capacity(), capacity);
    }
}

// Pending writes are admitted only for scalar values. Keeping a full Value in
// each hash bucket wastes space proportional to the number of updated keys.
#[derive(Clone, Debug)]
enum ScalarValue {
    String(Arc<String>),
    Bytes(Arc<Vec<u8>>),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}
impl ScalarValue {
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Str(value) => Self::String(value.clone()),
            Value::Bytes(value) => Self::Bytes(value.clone()),
            Value::Int(value) => Self::Int(*value),
            Value::Float(value) => Self::Float(*value),
            Value::Bool(value) => Self::Bool(*value),
            Value::Null => Self::Null,
            _ => unreachable!("admitted scalar dictionary write"),
        }
    }
    fn into_value(&self) -> Value {
        match self {
            Self::String(value) => Value::Str(value.clone()),
            Self::Bytes(value) => Value::Bytes(value.clone()),
            Self::Int(value) => Value::Int(*value),
            Self::Float(value) => Value::Float(*value),
            Self::Bool(value) => Value::Bool(*value),
            Self::Null => Value::Null,
        }
    }
}
type ScalarWrites =
    std::collections::HashMap<Arc<str>, ScalarValue, std::hash::BuildHasherDefault<ahash::AHasher>>;

#[cfg(test)]
mod pair_tests {
    use super::*;

    #[test]
    fn pair_fits_existing_payload_and_preserves_canonical_signed_keys() {
        assert_eq!(
            std::mem::size_of::<DictionaryPairPublication>(),
            std::mem::size_of::<DictionaryPublication>()
        );
        for key in [-(1i64 << 28), -1, 0, (1 << 28) - 1] {
            let text = key.to_string();
            let (entry, word) =
                DictionaryPairPublication::encode(&text, &Value::Int(i64::MIN)).unwrap();
            assert_eq!(DictionaryPairPublication::key(entry), key);
            assert!(matches!(DictionaryPairPublication::value(entry, word), Value::Int(i64::MIN)));
        }
        for key in ["01", "+1", "-0", "268435456", "-268435457", "x"] {
            assert!(DictionaryPairPublication::encode(key, &Value::Null).is_none(), "{key}");
        }
    }

    #[test]
    fn pair_publication_keeps_snapshot_and_float_identity_without_owner_detachment() {
        let base = Arc::new(DictMap::from_iter([
            (Arc::from("-1"), Value::Int(0)),
            (Arc::from("2"), Value::Int(2)),
            (Arc::from("02"), Value::Int(99)),
        ]));
        let pointer = Arc::as_ptr(&base);
        let mut owner = Value::Dict(base.clone());
        let first = DictionaryPublication::prepare("state".into(), base, "-1", 7).unwrap();
        let bits = 0xfff8_0000_0000_0042;
        assert!(first.can_pair("2", &Value::Float(f64::from_bits(bits))));
        let mut pair = first.into_pair("2", &Value::Float(f64::from_bits(bits)));
        let snapshot = pair.snapshot_value();
        pair.record("-1", &Value::Bool(false));
        pair.record("2", &Value::Null);
        assert!(!pair.can_record("02", &Value::Int(3)));
        assert!(matches!(pair.get("02"), Some(Value::Int(99))));
        assert!(matches!(owner.dictionary_get("-1"), Some(Value::Int(0))));
        let (_, updated) = pair.publish(Some(&mut owner));
        assert!(matches!(updated, Value::Dict(ref map) if Arc::as_ptr(map)==pointer));
        assert!(matches!(owner.dictionary_get("-1"), Some(Value::Bool(false))));
        assert!(matches!(snapshot.dictionary_get("2"), Some(Value::Float(v)) if v.to_bits()==bits));
    }

    #[test]
    fn pair_promotion_and_discard_do_not_change_original_values() {
        let base = Arc::new(DictMap::from_iter([
            (Arc::from("0"), Value::Int(0)),
            (Arc::from("1"), Value::Int(1)),
        ]));
        let pair = DictionaryPublication::prepare("state".into(), base.clone(), "0", 4)
            .unwrap()
            .into_pair("1", &Value::Bool(true));
        let (_, discarded) = pair.clone().discard();
        assert!(matches!(discarded.dictionary_get("0"), Some(Value::Int(0))));
        let mut table = pair.into_many();
        table.record("new", &Value::Bytes(Arc::new(vec![1, 2])));
        let (_, updated) = table.publish(None);
        assert!(matches!(updated.dictionary_get("0"), Some(Value::Int(4))));
        assert!(matches!(updated.dictionary_get("1"), Some(Value::Bool(true))));
        assert!(
            matches!(updated.dictionary_get("new"), Some(Value::Bytes(v)) if v.as_slice()==[1,2])
        );
        assert!(matches!(base["0"], Value::Int(0)));
    }
}

/// Two canonical numeric keys fit in the existing capture payload without a
/// table allocation. The signed 29-bit key range is an optimization boundary;
/// all other keys keep ordinary dictionary behavior.
#[derive(Clone, Debug)]
pub(super) struct DictionaryPairPublication {
    name: Box<str>,
    base: Arc<DictMap>,
    words: [u64; 2],
    entries: [u32; 2],
}

impl DictionaryPairPublication {
    const KEY_BITS: u32 = 29;
    const KEY_MASK: u32 = (1 << Self::KEY_BITS) - 1;

    fn numeric_key(key: &str) -> Option<i64> {
        let number = key.parse::<i64>().ok()?;
        if !(-(1 << 28)..(1 << 28)).contains(&number)
            || !with_integer_key(number, |canonical| canonical == key)
        {
            return None;
        }
        Some(number)
    }

    fn encode(key: &str, value: &Value) -> Option<(u32, u64)> {
        let key = Self::numeric_key(key)?;
        let (kind, word) = match value {
            Value::Int(value) => (1, *value as u64),
            Value::Float(value) => (2, value.to_bits()),
            Value::Bool(value) => (3, u64::from(*value)),
            Value::Null => (4, 0),
            _ => return None,
        };
        Some(((kind << Self::KEY_BITS) | ((key as u32) & Self::KEY_MASK), word))
    }

    fn key(entry: u32) -> i64 {
        ((entry << (32 - Self::KEY_BITS)) as i32 >> (32 - Self::KEY_BITS)) as i64
    }

    fn value(entry: u32, word: u64) -> Value {
        match entry >> Self::KEY_BITS {
            1 => Value::Int(word as i64),
            2 => Value::Float(f64::from_bits(word)),
            3 => Value::Bool(word != 0),
            4 => Value::Null,
            _ => unreachable!("admitted dictionary pair scalar"),
        }
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn get(&self, key: &str) -> Option<Value> {
        if let Some(number) = Self::numeric_key(key) {
            if let Some(slot) = self.entries.iter().position(|entry| Self::key(*entry) == number) {
                return Some(Self::value(self.entries[slot], self.words[slot]));
            }
        }
        self.base.get(key).cloned()
    }

    pub(super) fn can_record(&self, key: &str, value: &Value) -> bool {
        Self::encode(key, value).is_some_and(|(entry, _)| {
            self.entries.iter().any(|old| Self::key(*old) == Self::key(entry))
        })
    }

    pub(super) fn record(&mut self, key: &str, value: &Value) {
        let (entry, word) = Self::encode(key, value).expect("admitted pair update");
        let slot = self
            .entries
            .iter()
            .position(|old| Self::key(*old) == Self::key(entry))
            .expect("existing pair key");
        self.entries[slot] = entry;
        self.words[slot] = word;
    }

    pub(super) fn into_many(self) -> DictionaryManyPublication {
        let mut writes = ScalarWrites::default();
        for (entry, word) in self.entries.into_iter().zip(self.words) {
            with_integer_key(Self::key(entry), |key| {
                let key = self.base.get_key_value(key).expect("existing pair key").0.clone();
                writes.insert(key, ScalarValue::from_value(&Self::value(entry, word)));
            });
        }
        DictionaryManyPublication { name: self.name, base: self.base, writes }
    }

    fn apply(base: &mut Arc<DictMap>, entries: [u32; 2], words: [u64; 2]) {
        let target = Arc::make_mut(base);
        for (entry, word) in entries.into_iter().zip(words) {
            with_integer_key(Self::key(entry), |key| {
                *target.get_mut(key).expect("existing pair key") = Self::value(entry, word);
            });
        }
    }

    pub(super) fn snapshot_value(&self) -> Value {
        let mut base = self.base.clone();
        Self::apply(&mut base, self.entries, self.words);
        Value::Dict(base)
    }

    pub(super) fn publish(self, owner: Option<&mut Value>) -> (String, Value) {
        let Self { name, mut base, entries, words } = self;
        if let Some(Value::Dict(public)) = owner {
            if Arc::ptr_eq(&base, public) {
                drop(base);
                Self::apply(public, entries, words);
                return (name.into_string(), Value::Dict(public.clone()));
            }
        }
        Self::apply(&mut base, entries, words);
        (name.into_string(), Value::Dict(base))
    }

    pub(super) fn discard(self) -> (String, Value) {
        (self.name.into_string(), Value::Dict(self.base))
    }
}

/// Promote only when a helper admits a second key. Single-key calls
/// retain their inline journal; this table scales with writes, not owner width.
#[derive(Clone, Debug)]
pub(super) struct DictionaryManyPublication {
    name: Box<str>,
    base: Arc<DictMap>,
    writes: ScalarWrites,
}

impl DictionaryManyPublication {
    pub(super) fn name(&self) -> &str {
        &self.name
    }
    pub(super) fn contains_key(&self, key: &str) -> bool {
        self.base.contains_key(key)
    }
    pub(super) fn get(&self, key: &str) -> Option<Value> {
        self.writes.get(key).map(|value| value.into_value()).or_else(|| self.base.get(key).cloned())
    }
    pub(super) fn record(&mut self, key: &str, value: &Value) {
        if let Some(previous) = self.writes.get_mut(key) {
            *previous = ScalarValue::from_value(value);
        } else {
            let stored = self
                .base
                .get_key_value(key)
                .map(|(key, _)| key.clone())
                .unwrap_or_else(|| Arc::from(key));
            self.writes.insert(stored, ScalarValue::from_value(value));
        }
    }
    fn apply(writes: &ScalarWrites, base: &mut Arc<DictMap>) {
        let target = Arc::make_mut(base);
        for (key, value) in writes {
            if let Some(entry) = target.get_mut(key) {
                *entry = value.into_value();
            } else {
                target.insert(key.clone(), value.into_value());
            }
        }
    }
    pub(super) fn snapshot_value(&self) -> Value {
        let mut base = self.base.clone();
        Self::apply(&self.writes, &mut base);
        Value::Dict(base)
    }
    pub(super) fn publish(self, owner: Option<&mut Value>) -> (String, Value) {
        let Self { name, mut base, writes } = self;
        if let Some(Value::Dict(public)) = owner {
            if Arc::ptr_eq(&base, public) {
                drop(base);
                Self::apply(&writes, public);
                return (name.into_string(), Value::Dict(public.clone()));
            }
        }
        Self::apply(&writes, &mut base);
        (name.into_string(), Value::Dict(base))
    }
    pub(super) fn discard(self) -> (String, Value) {
        (self.name.into_string(), Value::Dict(self.base))
    }
}

#[cfg(test)]
mod many_tests {
    use super::*;

    #[test]
    fn journal_strings_and_insertions_preserve_snapshots_and_owner_visibility() {
        let mut owner = Value::Dict(Arc::new(DictMap::from_iter([
            (Arc::from("a"), Value::Int(1)),
            (Arc::from("b"), Value::Int(2)),
        ])));
        let Value::Dict(base) = &owner else { unreachable!() };
        let mut pending = DictionaryPublication::prepare("state".into(), base.clone(), "a", 10)
            .unwrap()
            .into_many("b", &Value::Int(20));
        let text = Arc::new("retained string".to_owned());
        pending.record("a", &Value::Str(text.clone()));
        pending.record("new", &Value::Str(text.clone()));
        assert!(matches!(owner.dictionary_get("a"), Some(Value::Int(1))));
        assert!(owner.dictionary_get("new").is_none());
        let saved = pending.snapshot_value();
        pending.record("new", &Value::Null);
        assert!(matches!(pending.get("new"), Some(Value::Null)));
        pending.publish(Some(&mut owner));
        assert!(
            matches!(owner.dictionary_get("a"), Some(Value::Str(value)) if Arc::ptr_eq(&value, &text))
        );
        assert!(matches!(owner.dictionary_get("new"), Some(Value::Null)));
        assert!(
            matches!(saved.dictionary_get("new"), Some(Value::Str(value)) if Arc::ptr_eq(&value, &text))
        );
    }

    #[test]
    fn compact_journal_scalars_preserve_exact_values() {
        assert!(std::mem::size_of::<ScalarValue>() <= 16);
        for value in [
            Value::Int(i64::MIN),
            Value::Int(i64::MAX),
            Value::Float(-0.0),
            Value::Float(f64::from_bits(0xfff8_0000_0000_0042)),
            Value::Bool(false),
            Value::Bool(true),
            Value::Null,
            Value::Bytes(Arc::new(vec![0, 1, 255])),
        ] {
            let restored = ScalarValue::from_value(&value).into_value();
            match (value, restored) {
                (Value::Int(a), Value::Int(b)) => assert_eq!(a, b),
                (Value::Float(a), Value::Float(b)) => assert_eq!(a.to_bits(), b.to_bits()),
                (Value::Bool(a), Value::Bool(b)) => assert_eq!(a, b),
                (Value::Null, Value::Null) => {}
                (Value::Bytes(a), Value::Bytes(b)) => assert!(Arc::ptr_eq(&a, &b)),
                _ => panic!("scalar kind changed"),
            }
        }
    }

    #[test]
    fn multiple_keys_preserve_aliases_snapshots_and_owner_storage() {
        let mut map = DictMap::default();
        for i in 0..128 {
            map.insert(Arc::from(i.to_string()), Value::Int(i));
        }
        map.insert(Arc::from("01"), Value::Int(999));
        let mut owner = Value::Dict(Arc::new(map));
        let Value::Dict(base) = &owner else { unreachable!() };
        let pointer = Arc::as_ptr(base);
        let first =
            DictionaryPublication::prepare("state".into(), base.clone(), "0", 1000).unwrap();
        let mut pending = first.into_many("1", &Value::Float(-0.0));
        let saved = pending.snapshot_value();
        for i in 2..128 {
            pending.record(&i.to_string(), &Value::Bool(true));
        }
        pending.record("0", &Value::Null);
        assert!(!pending.contains_key("missing"));
        assert!(matches!(pending.get("01"), Some(Value::Int(999))));
        let (_, current) = pending.publish(Some(&mut owner));
        let Value::Dict(current) = current else { unreachable!() };
        assert_eq!(Arc::as_ptr(&current), pointer);
        assert!(matches!(current["0"], Value::Null));
        assert!(matches!(current["127"], Value::Bool(true)));
        assert!(matches!(current["1"], Value::Float(v) if v.to_bits() == (-0.0_f64).to_bits()));
        let Value::Dict(saved) = saved else { unreachable!() };
        assert!(matches!(saved["0"], Value::Int(1000)));
        assert!(matches!(saved["127"], Value::Int(127)));
    }

    #[test]
    fn multiple_key_discard_and_replaced_owner_preserve_originals() {
        let base = Arc::new(DictMap::from_iter([
            (Arc::from("a"), Value::Int(1)),
            (Arc::from("b"), Value::Int(2)),
        ]));
        let pending = DictionaryPublication::prepare("state".into(), base.clone(), "a", 10)
            .unwrap()
            .into_many("b", &Value::Null);
        let (_, discarded) = pending.clone().discard();
        assert!(matches!(discarded, Value::Dict(ref map) if matches!(map["a"],Value::Int(1))));
        let mut other = Value::Dict(Arc::new(DictMap::default()));
        let (_, updated) = pending.publish(Some(&mut other));
        assert!(matches!(other, Value::Dict(ref map) if map.is_empty()));
        assert!(
            matches!(updated, Value::Dict(ref map) if matches!(map["a"],Value::Int(10)) && matches!(map["b"],Value::Null))
        );
        assert!(matches!(base["a"], Value::Int(1)));
    }
}

/// Bound temporary table allocation traffic before admitting the first new key.
/// A sparse table grows geometrically at 7/8 load, starting at four buckets.
/// Sum every possible allocation, including control bytes, rather than comparing
/// only the final table with the owner. Use the full Value storage avoided by a
/// map clone as a conservative budget; shared key and Arc overhead are excluded.
/// The caller supplies a proven upper bound on writes for this activation.
pub(super) fn insertion_can_pay(base: &DictMap, writes: u16) -> bool {
    if writes == 0 || base.is_empty() {
        return false;
    }
    if writes == 1 {
        return true; // The one-key journal stays inline.
    }
    let bucket_bytes = std::mem::size_of::<(Arc<str>, ScalarValue)>() + 1;
    let mut buckets = 4usize;
    let mut traffic = 0usize;
    loop {
        // Sixteen trailing control bytes also cover the smaller scalar group
        // implementations; overestimating only retains ordinary fallback.
        traffic = traffic.saturating_add(buckets.saturating_mul(bucket_bytes) + 16);
        if buckets - buckets / 8 >= usize::from(writes) && buckets > 4 {
            break;
        }
        if buckets == 4 && writes <= 3 {
            break;
        }
        buckets *= 2;
    }
    traffic <= base.capacity().saturating_mul(std::mem::size_of::<Value>())
}

#[cfg(test)]
mod insertion_budget_tests {
    use super::*;
    #[test]
    fn dense_small_owners_fall_back_and_sparse_large_owners_pay() {
        for width in [0, 1, 2, 128, 2048] {
            let base: DictMap =
                (0..width).map(|k| (Arc::from(k.to_string()), Value::Int(k))).collect();
            assert!(!insertion_can_pay(&base, 0));
            assert_eq!(insertion_can_pay(&base, 1), width != 0);
            assert_eq!(insertion_can_pay(&base, 2), width != 0);
            assert_eq!(insertion_can_pay(&base, 64), width >= 128);
        }
    }
}

#[cfg(test)]
mod collection_entry_tests {
    use super::*;

    #[test]
    fn compact_collection_payloads_preserve_every_supported_representation() {
        assert!(std::mem::size_of::<CollectionValue>() <= 4 * std::mem::size_of::<usize>());
        let values = [
            Value::Struct {
                name: "Item".into(),
                fields: Arc::new([("value".into(), Value::Int(3))].into_iter().collect()),
            },
            Value::FixedDict {
                keys: Arc::new(vec![Arc::from("value")]),
                values: Arc::new(vec![Value::Int(3)]),
            },
            Value::IntDict(Arc::new([(0, Value::Int(3))].into_iter().collect())),
            Value::DenseIntDict(Arc::new(vec![Value::Int(3)])),
            Value::DenseIntDictInt(Arc::new(vec![Some(3)])),
            Value::DenseIntDictIntFull(Arc::new(vec![3])),
        ];
        for expected in values {
            let pending =
                DictionaryCollectionPublication::prepare("state".into(), base(), "key", &expected)
                    .unwrap();
            let read = pending.get("key").unwrap();
            assert_eq!(std::mem::discriminant(&read), std::mem::discriminant(&expected));
            assert!(Value::equals(&read, &expected));
            let snapshot = pending.snapshot_value();
            let (_, published) = pending.publish(None);
            for container in [snapshot, published] {
                let actual = container.dictionary_get("key").unwrap();
                assert_eq!(std::mem::discriminant(&actual), std::mem::discriminant(&expected));
                assert!(Value::equals(&actual, &expected));
            }
        }
    }

    #[test]
    fn typed_collection_payloads_reuse_storage_and_preserve_snapshots() {
        let array = Arc::new(vec![Value::Int(7)]);
        let root = base();
        let mut owner = Value::Dict(root.clone());
        let pending = DictionaryCollectionPublication::prepare_payload(
            "state".into(),
            root,
            "key",
            array.clone(),
        )
        .unwrap();
        let Some(Value::Array(read)) = pending.get("key") else { panic!() };
        assert!(Arc::ptr_eq(&read, &array));
        let snapshot = pending.snapshot_value();
        let map = Arc::new([(Arc::from("nested"), Value::Int(8))].into_iter().collect::<DictMap>());
        let pending = pending.with_payload(map.clone());
        let Some(Value::Dict(read)) = pending.get("key") else { panic!() };
        assert!(Arc::ptr_eq(&read, &map));
        let Value::Array(saved) = snapshot.dictionary_get("key").unwrap() else { panic!() };
        assert!(Arc::ptr_eq(&saved, &array));
        let (_, published) = pending.publish(Some(&mut owner));
        for value in [owner, published] {
            let Value::Dict(current) = value.dictionary_get("key").unwrap() else { panic!() };
            assert!(Arc::ptr_eq(&current, &map));
        }
        assert!(matches!(array[0], Value::Int(7)));
    }

    fn base() -> Arc<DictMap> {
        Arc::new([(Arc::from("key"), Value::array(vec![Value::Int(1)]))].into_iter().collect())
    }

    #[test]
    fn collection_entry_preserves_snapshots_discard_and_type_transitions() {
        let original = base();
        let mut owner = Value::Dict(original.clone());
        let mut pending = DictionaryCollectionPublication::prepare(
            "state".into(),
            original,
            "key",
            &Value::array(vec![Value::Int(2)]),
        )
        .unwrap();
        let payload = pending.value.as_ref() as *const CollectionValue;
        let saved = pending.snapshot_value();
        pending.replace(&Value::dict([(Arc::from("nested"), Value::Int(3))].into_iter().collect()));
        assert_eq!(payload, pending.value.as_ref() as *const CollectionValue);
        let Value::Array(old) = saved.dictionary_get("key").unwrap() else { panic!() };
        assert!(matches!(old[0], Value::Int(2)));
        let (_, discarded) = pending.clone().discard();
        let Value::Array(old) = discarded.dictionary_get("key").unwrap() else { panic!() };
        assert!(matches!(old[0], Value::Int(1)));
        let pending = pending.with_scalar(-0.0_f64);
        assert!(
            matches!(pending.get("key"), Some(Value::Float(v)) if v.to_bits()==(-0.0_f64).to_bits())
        );
        let pending = pending.with_collection(&Value::array(vec![Value::Int(4)]));
        let (_, result) = pending.publish(Some(&mut owner));
        for current in [owner, result] {
            let Value::Array(items) = current.dictionary_get("key").unwrap() else { panic!() };
            assert!(matches!(items[0], Value::Int(4)));
        }
        assert!(matches!(old[0], Value::Int(1)));
    }

    #[test]
    fn collection_entry_keeps_replacement_owners_and_external_aliases() {
        let original = base();
        let retained = Value::Dict(original.clone());
        let pending = DictionaryCollectionPublication::prepare(
            "state".into(),
            original,
            "key",
            &Value::array(vec![retained.clone()]),
        )
        .unwrap();
        let mut replacement =
            Value::dict([(Arc::from("replacement"), Value::Bool(true))].into_iter().collect());
        let (_, result) = pending.publish(Some(&mut replacement));
        assert!(matches!(replacement.dictionary_get("replacement"), Some(Value::Bool(true))));
        assert!(replacement.dictionary_get("key").is_none());
        let Value::Array(items) = result.dictionary_get("key").unwrap() else { panic!() };
        let Value::Array(old) = items[0].dictionary_get("key").unwrap() else { panic!() };
        assert!(matches!(old[0], Value::Int(1)));
        let Value::Array(old) = retained.dictionary_get("key").unwrap() else { panic!() };
        assert!(matches!(old[0], Value::Int(1)));
        let mut name = String::with_capacity(128);
        name.push_str("state");
        assert!(DictionaryCollectionPublication::prepare(
            name,
            base(),
            "key",
            &Value::array(vec![])
        )
        .is_err());
    }
}
