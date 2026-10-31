//! Private nested writes retained in the assignment path, with bounded row overflow.
use super::array_batch_publication::{BatchValue, IndexHashBuilder};
use super::assignment::Access;
use super::dictionary_publication::CollectionPayload;
use super::dictionary_publication::{
    with_integer_key, CollectionValue, DictionaryCollectionPublication,
};
use super::Value;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
enum Scalar {
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl Scalar {
    fn from_value(value: &Value) -> Option<Self> {
        Some(match value {
            Value::Int(value) => Self::Int(*value),
            Value::Float(value) => Self::Float(*value),
            Value::Bool(value) => Self::Bool(*value),
            Value::Null => Self::Null,
            _ => return None,
        })
    }

    fn value(self) -> Value {
        match self {
            Self::Int(value) => Value::Int(value),
            Self::Float(value) => Value::Float(value),
            Self::Bool(value) => Value::Bool(value),
            Self::Null => Value::Null,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum RowKey<'a> {
    Field(&'a str),
    String(&'a str),
    Integer(i64),
}

impl<'a> RowKey<'a> {
    fn from_access(access: &'a Access) -> Option<Self> {
        match access {
            Access::Field(field) => Some(Self::Field(field)),
            Access::Index(Value::Str(key)) => Some(Self::String(key)),
            Access::Index(Value::Int(key)) => Some(Self::Integer(*key)),
            _ => None,
        }
    }

    fn same(self, other: Self, root: &Value) -> bool {
        if let Value::Array(rows) = root {
            return match (self, other) {
                (Self::Integer(a), Self::Integer(b)) => {
                    let a = NestedPublication::resolve(a, rows.len());
                    a.is_some() && a == NestedPublication::resolve(b, rows.len())
                }
                _ => false,
            };
        }
        match (self, other) {
            (Self::Field(a), Self::Field(b)) | (Self::String(a), Self::String(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Integer(a), Self::String(b)) | (Self::String(b), Self::Integer(a)) => {
                with_integer_key(a, |key| key == b)
            }
            _ => false,
        }
    }

    fn value(self, root: &Value) -> Option<&Value> {
        match (self, root) {
            (Self::Field(field), Value::Struct { fields, .. }) => fields.get(field),
            (Self::String(key), Value::Dict(map)) => map.get(key),
            (Self::Integer(key), Value::Dict(map)) => with_integer_key(key, |key| map.get(key)),
            (Self::Integer(key), Value::Array(rows)) => {
                rows.get(NestedPublication::resolve(key, rows.len())?)
            }
            _ => None,
        }
    }

    fn row(self, root: &Value) -> Option<&Arc<Vec<Value>>> {
        if let Value::Array(row) = self.value(root)? {
            Some(row)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
enum Selector {
    Field(String),
    String(Arc<String>),
    Integer(i64),
}

impl Selector {
    fn key(&self) -> RowKey<'_> {
        match self {
            Self::Field(field) => RowKey::Field(field),
            Self::String(key) => RowKey::String(key),
            Self::Integer(key) => RowKey::Integer(*key),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum CollectionKey {
    Array(usize),
    Dictionary(Arc<str>),
}

/// One collection replacement reuses the existing assignment-path allocation.
/// Keep scalar batches separate so their inline capacity and layout do not grow.
#[derive(Clone, Debug)]
pub(crate) struct CollectionWrite {
    selector: Selector,
    key: CollectionKey,
    value: CollectionValue,
}

impl CollectionWrite {
    fn key(leaf: &Value, key: RowKey<'_>) -> Option<CollectionKey> {
        match (leaf, key) {
            (Value::Array(row), RowKey::Integer(index)) => {
                NestedPublication::resolve(index, row.len()).map(CollectionKey::Array)
            }
            (Value::Dict(map), RowKey::String(key)) => {
                map.get_key_value(key).map(|(key, _)| CollectionKey::Dictionary(key.clone()))
            }
            (Value::Dict(map), RowKey::Integer(index)) => with_integer_key(index, |key| {
                map.get_key_value(key).map(|(key, _)| CollectionKey::Dictionary(key.clone()))
            }),
            _ => None,
        }
    }

    fn read(&self, root: &Value, owner: RowKey<'_>, key: RowKey<'_>) -> Option<Value> {
        if !self.selector.key().same(owner, root) {
            return None;
        }
        let leaf = self.selector.key().value(root)?;
        if Self::key(leaf, key).as_ref() == Some(&self.key) {
            return Some(self.value.clone().into_value());
        }
        // Keep ordinary missing-key/index diagnostics on the fallback path.
        key.value(leaf).cloned()
    }

    fn slot<'a>(&self, root: &'a mut Value) -> &'a mut Value {
        let leaf = match (&self.selector, root) {
            (Selector::Field(field), Value::Struct { fields, .. }) => {
                Arc::make_mut(fields).get_mut(field)
            }
            (Selector::String(key), Value::Dict(map)) => Arc::make_mut(map).get_mut(key.as_str()),
            (Selector::Integer(index), Value::Dict(map)) => {
                with_integer_key(*index, |key| Arc::make_mut(map).get_mut(key))
            }
            (Selector::Integer(index), Value::Array(rows)) => {
                let index =
                    NestedPublication::resolve(*index, rows.len()).expect("validated owner");
                Arc::make_mut(rows).get_mut(index)
            }
            _ => unreachable!("validated collection owner"),
        }
        .expect("validated collection leaf");
        match (&self.key, leaf) {
            (CollectionKey::Array(index), Value::Array(row)) => &mut Arc::make_mut(row)[*index],
            (CollectionKey::Dictionary(key), Value::Dict(map)) => {
                Arc::make_mut(map).get_mut(key.as_ref()).expect("validated replacement key")
            }
            _ => unreachable!("validated collection replacement"),
        }
    }

    fn apply(&self, root: &mut Value) {
        *self.slot(root) = self.value.clone().into_value();
    }

    fn apply_owned(self, root: &mut Value) {
        let slot = self.slot(root);
        *slot = self.value.into_value();
    }
}

/// Private sparse row payloads; retain no extra owner and never grow after admission.
#[derive(Clone, Debug)]
pub(crate) struct CollectionRowWrite {
    selector: Selector,
    writes: std::collections::HashMap<usize, BatchValue, IndexHashBuilder>,
}
impl CollectionRowWrite {
    fn new(selector: Selector, hint: u16) -> Self {
        Self {
            selector,
            writes: std::collections::HashMap::with_capacity_and_hasher(
                usize::from(hint),
                Default::default(),
            ),
        }
    }
    fn record(&mut self, index: usize, value: &mut Value) -> bool {
        if !BatchValue::supports(value) {
            return false;
        }
        if let Some(previous) = self.writes.get_mut(&index) {
            *previous = BatchValue::from_owned(std::mem::replace(value, Value::Null));
        } else {
            if self.writes.len() == self.writes.capacity() {
                return false;
            }
            self.writes
                .insert(index, BatchValue::from_owned(std::mem::replace(value, Value::Null)));
        }
        true
    }
    fn row_mut<'a>(selector: &Selector, root: &'a mut Value) -> &'a mut Vec<Value> {
        let leaf = match (selector, root) {
            (Selector::Field(k), Value::Struct { fields, .. }) => Arc::make_mut(fields).get_mut(k),
            (Selector::String(k), Value::Dict(map)) => Arc::make_mut(map).get_mut(k.as_str()),
            (Selector::Integer(k), Value::Dict(map)) => {
                with_integer_key(*k, |k| Arc::make_mut(map).get_mut(k))
            }
            (Selector::Integer(k), Value::Array(rows)) => {
                let index = NestedPublication::resolve(*k, rows.len()).expect("validated owner");
                Arc::make_mut(rows).get_mut(index)
            }
            _ => unreachable!("validated row owner"),
        };
        let Some(Value::Array(row)) = leaf else { unreachable!("validated row") };
        Arc::make_mut(row)
    }
    fn apply(&self, root: &mut Value) {
        let row = Self::row_mut(&self.selector, root);
        for (index, value) in &self.writes {
            row[*index] = value.clone().into_value();
        }
    }
    fn apply_owned(self, root: &mut Value) {
        let row = Self::row_mut(&self.selector, root);
        for (index, value) in self.writes {
            row[index] = value.into_value();
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RowWrite {
    selector: Selector,
    entries: [Option<RowEntry>; 4],
}

#[derive(Clone, Copy, Debug)]
struct RowEntry {
    index: usize,
    value: Scalar,
}

impl RowWrite {
    fn apply(&self, root: &mut Value) {
        let leaf = match (&self.selector, root) {
            (Selector::Field(field), Value::Struct { fields, .. }) => {
                Arc::make_mut(fields).get_mut(field)
            }
            (Selector::String(key), Value::Dict(map)) => Arc::make_mut(map).get_mut(key.as_str()),
            (Selector::Integer(key), Value::Dict(map)) => {
                with_integer_key(*key, |key| Arc::make_mut(map).get_mut(key))
            }
            (Selector::Integer(key), Value::Array(rows)) => {
                let index = NestedPublication::resolve(*key, rows.len()).expect("validated row");
                Arc::make_mut(rows).get_mut(index)
            }
            _ => unreachable!("validated nested publication root"),
        };
        let Some(Value::Array(row)) = leaf else { unreachable!("validated row") };
        let row = Arc::make_mut(row);
        for entry in self.entries.iter().flatten() {
            row[entry.index] = entry.value.value();
        }
    }
}

/// Allocate only after the inline journal fills in a proven scalar region.
/// Keys are resolved indices, so negative aliases coalesce before storage.
#[derive(Clone, Debug)]
pub(crate) struct RowManyWrite {
    selector: Selector,
    writes: std::collections::HashMap<usize, Scalar, std::hash::BuildHasherDefault<ahash::AHasher>>,
}

impl RowManyWrite {
    fn from_inline(write: RowWrite, hint: u16) -> Self {
        let mut writes = std::collections::HashMap::with_capacity_and_hasher(
            usize::from(hint).max(5),
            Default::default(),
        );
        for entry in write.entries.into_iter().flatten() {
            writes.insert(entry.index, entry.value);
        }
        Self { selector: write.selector, writes }
    }

    fn apply(&self, root: &mut Value) {
        let leaf = match (&self.selector, root) {
            (Selector::Field(field), Value::Struct { fields, .. }) => {
                Arc::make_mut(fields).get_mut(field)
            }
            (Selector::String(key), Value::Dict(map)) => Arc::make_mut(map).get_mut(key.as_str()),
            (Selector::Integer(key), Value::Dict(map)) => {
                with_integer_key(*key, |key| Arc::make_mut(map).get_mut(key))
            }
            (Selector::Integer(key), Value::Array(rows)) => {
                let index = NestedPublication::resolve(*key, rows.len()).expect("validated owner");
                Arc::make_mut(rows).get_mut(index)
            }
            _ => unreachable!("validated row owner"),
        };
        let Some(Value::Array(row)) = leaf else { unreachable!("validated row") };
        let row = Arc::make_mut(row);
        for (index, value) in &self.writes {
            row[*index] = value.value();
        }
    }
}

/// Up to four scalar dictionary leaf writes reuse assignment path storage.
/// Existing canonical keys are shared; an inserted key is allocated once at preparation.
#[derive(Clone, Debug)]
pub(crate) struct DictionaryWrite {
    selector: Selector,
    keys: [Option<Arc<str>>; 4],
    words: [u64; 4],
    kinds: u8,
}

impl DictionaryWrite {
    fn new(selector: Selector, key: Arc<str>, value: Scalar) -> Self {
        let mut write =
            Self { selector, keys: [Some(key), None, None, None], words: [0; 4], kinds: 0 };
        write.set_value(0, value);
        write
    }

    fn set_value(&mut self, slot: usize, value: Scalar) {
        let (kind, word) = match value {
            Scalar::Int(value) => (0, value as u64),
            Scalar::Float(value) => (1, value.to_bits()),
            Scalar::Bool(value) => (2, u64::from(value)),
            Scalar::Null => (3, 0),
        };
        self.words[slot] = word;
        self.kinds = (self.kinds & !(3 << (slot * 2))) | (kind << (slot * 2));
    }

    fn value(&self, slot: usize) -> Value {
        match (self.kinds >> (slot * 2)) & 3 {
            0 => Value::Int(self.words[slot] as i64),
            1 => Value::Float(f64::from_bits(self.words[slot])),
            2 => Value::Bool(self.words[slot] != 0),
            3 => Value::Null,
            _ => unreachable!(),
        }
    }

    fn record(&mut self, root: &Value, key: &str, value: Scalar) -> bool {
        if let Some(slot) = self.keys.iter().position(|stored| stored.as_deref() == Some(key)) {
            self.set_value(slot, value);
            return true;
        }
        // Reject overflow before allocating an insertion key or changing any entry.
        let Some(slot) = self.keys.iter().position(Option::is_none) else { return false };
        let Some(Value::Dict(map)) = self.selector.key().value(root) else { return false };
        self.keys[slot] =
            Some(map.get_key_value(key).map_or_else(|| Arc::from(key), |(key, _)| key.clone()));
        self.set_value(slot, value);
        true
    }

    fn apply(&self, root: &mut Value) {
        let leaf = match (&self.selector, root) {
            (Selector::Field(field), Value::Struct { fields, .. }) => {
                Arc::make_mut(fields).get_mut(field)
            }
            (Selector::String(key), Value::Dict(map)) => Arc::make_mut(map).get_mut(key.as_str()),
            (Selector::Integer(key), Value::Dict(map)) => {
                with_integer_key(*key, |key| Arc::make_mut(map).get_mut(key))
            }
            (Selector::Integer(key), Value::Array(rows)) => {
                let index = NestedPublication::resolve(*key, rows.len()).expect("validated owner");
                Arc::make_mut(rows).get_mut(index)
            }
            _ => unreachable!("validated dictionary publication root"),
        };
        let Some(Value::Dict(map)) = leaf else { unreachable!("validated dictionary leaf") };
        let map = Arc::make_mut(map);
        for (slot, key) in self.keys.iter().enumerate() {
            if let Some(key) = key {
                if let Some(existing) = map.get_mut(key.as_ref()) {
                    *existing = self.value(slot);
                } else {
                    map.insert(key.clone(), self.value(slot));
                }
            }
        }
    }

    fn read(&self, root: &Value, owner: RowKey<'_>, key: &str) -> Option<Value> {
        if !self.selector.key().same(owner, root) {
            return None;
        }
        let Value::Dict(map) = self.selector.key().value(root)? else { return None };
        if let Some(slot) = self.keys.iter().position(|stored| stored.as_deref() == Some(key)) {
            Some(self.value(slot))
        } else {
            // Missing keys retain ordinary diagnostics and evaluation order.
            map.get(key).cloned()
        }
    }
}

/// Overflow storage grows with distinct writes, never with the untouched leaf.
/// The inline four-key representation remains allocation-free.
#[derive(Clone, Debug)]
pub(crate) struct DictionaryManyWrite {
    selector: Selector,
    writes:
        std::collections::HashMap<Arc<str>, Scalar, std::hash::BuildHasherDefault<ahash::AHasher>>,
}

impl DictionaryManyWrite {
    fn from_inline(write: DictionaryWrite, spill_hint: u16) -> Self {
        let mut writes = std::collections::HashMap::with_capacity_and_hasher(
            usize::from(spill_hint).max(5),
            Default::default(),
        );
        for (slot, key) in write.keys.iter().enumerate() {
            if let Some(key) = key {
                writes.insert(key.clone(), Scalar::from_value(&write.value(slot)).unwrap());
            }
        }
        Self { selector: write.selector, writes }
    }

    fn record(&mut self, root: &Value, key: &str, value: Scalar) {
        if let Some(previous) = self.writes.get_mut(key) {
            *previous = value;
            return;
        }
        let Some(Value::Dict(map)) = self.selector.key().value(root) else {
            unreachable!("validated dictionary leaf")
        };
        let key = map.get_key_value(key).map_or_else(|| Arc::from(key), |(key, _)| key.clone());
        self.writes.insert(key, value);
    }

    fn read(&self, root: &Value, owner: RowKey<'_>, key: &str) -> Option<Value> {
        if !self.selector.key().same(owner, root) {
            return None;
        }
        let Value::Dict(map) = self.selector.key().value(root)? else { return None };
        self.writes.get(key).map(|value| value.value()).or_else(|| map.get(key).cloned())
    }

    fn apply(&self, root: &mut Value) {
        let leaf = match (&self.selector, root) {
            (Selector::Field(field), Value::Struct { fields, .. }) => {
                Arc::make_mut(fields).get_mut(field)
            }
            (Selector::String(key), Value::Dict(map)) => Arc::make_mut(map).get_mut(key.as_str()),
            (Selector::Integer(key), Value::Dict(map)) => {
                with_integer_key(*key, |key| Arc::make_mut(map).get_mut(key))
            }
            (Selector::Integer(key), Value::Array(rows)) => {
                let index = NestedPublication::resolve(*key, rows.len()).expect("validated owner");
                Arc::make_mut(rows).get_mut(index)
            }
            _ => unreachable!("validated dictionary publication root"),
        };
        let Some(Value::Dict(map)) = leaf else { unreachable!("validated dictionary leaf") };
        let map = Arc::make_mut(map);
        for (key, value) in &self.writes {
            if let Some(existing) = map.get_mut(key.as_ref()) {
                *existing = value.value();
            } else {
                map.insert(key.clone(), value.value());
            }
        }
    }
}

/// Mixed dictionary payloads retain only changed keys and their owned values.
/// Admission/promotion is separate; this storage does not retain another root.
#[derive(Clone, Debug)]
pub(crate) struct CollectionDictionaryWrite {
    selector: Selector,
    writes: std::collections::HashMap<
        Arc<str>,
        BatchValue,
        std::hash::BuildHasherDefault<ahash::AHasher>,
    >,
    limit: u16,
}

impl CollectionDictionaryWrite {
    #[cfg(test)]
    fn new(selector: Selector, limit: u16) -> Self {
        Self::with_known_keys(selector, limit, usize::from(limit.min(4)))
    }

    fn with_known_keys(selector: Selector, limit: u16, known_keys: usize) -> Self {
        debug_assert!(known_keys <= usize::from(limit));
        Self {
            selector,
            writes: std::collections::HashMap::with_capacity_and_hasher(
                known_keys,
                Default::default(),
            ),
            limit,
        }
    }

    fn record(&mut self, root: &Value, key: &str, value: &mut Value) -> bool {
        if !BatchValue::supports(value) {
            return false;
        }
        let Some(Value::Dict(base)) = self.selector.key().value(root) else {
            return false;
        };
        // Rewrites must not reserve, even when the table is full.
        if let Some(previous) = self.writes.get_mut(key) {
            *previous = BatchValue::from_owned(std::mem::replace(value, Value::Null));
            return true;
        }
        if self.writes.len() >= usize::from(self.limit)
            || (!base.contains_key(key) && !collection_insertion_can_pay(base, self.limit))
        {
            return false;
        }
        let key = base.get_key_value(key).map_or_else(|| Arc::from(key), |(key, _)| key.clone());
        self.writes.insert(key, BatchValue::from_owned(std::mem::replace(value, Value::Null)));
        true
    }

    fn read(&self, root: &Value, owner: RowKey<'_>, key: &str) -> Option<Value> {
        if !self.selector.key().same(owner, root) {
            return None;
        }
        let Value::Dict(base) = self.selector.key().value(root)? else { return None };
        self.writes.get(key).map(|v| v.clone().into_value()).or_else(|| base.get(key).cloned())
    }

    fn dictionary_mut<'a>(
        selector: &Selector,
        root: &'a mut Value,
    ) -> &'a mut super::value::DictMap {
        let leaf = match (selector, root) {
            (Selector::Field(key), Value::Struct { fields, .. }) => {
                Arc::make_mut(fields).get_mut(key)
            }
            (Selector::String(key), Value::Dict(map)) => Arc::make_mut(map).get_mut(key.as_str()),
            (Selector::Integer(key), Value::Dict(map)) => {
                with_integer_key(*key, |key| Arc::make_mut(map).get_mut(key))
            }
            (Selector::Integer(key), Value::Array(rows)) => {
                let index = NestedPublication::resolve(*key, rows.len()).expect("validated owner");
                Arc::make_mut(rows).get_mut(index)
            }
            _ => unreachable!("validated dictionary owner"),
        };
        let Some(Value::Dict(map)) = leaf else { unreachable!("validated dictionary leaf") };
        Arc::make_mut(map)
    }

    fn apply(&self, root: &mut Value) {
        let map = Self::dictionary_mut(&self.selector, root);
        for (key, value) in &self.writes {
            if let Some(slot) = map.get_mut(key.as_ref()) {
                *slot = value.clone().into_value();
            } else {
                map.insert(key.clone(), value.clone().into_value());
            }
        }
    }

    fn apply_owned(self, root: &mut Value) {
        let map = Self::dictionary_mut(&self.selector, root);
        for (key, value) in self.writes {
            if let Some(slot) = map.get_mut(key.as_ref()) {
                *slot = value.into_value();
            } else {
                map.insert(key, value.into_value());
            }
        }
    }
}

/// Two Vec slots hold the original root and the write metadata. Admission moves
/// existing path/key storage; it does not allocate a journal or clone the root.
#[derive(Clone, Debug)]
pub(super) struct NestedPublication {
    name: String,
    path: Vec<Access>,
}

/// Include the mixed payload's actual bucket size and all geometric growth.
/// A successful estimate is only a profitability hint; ownership checks remain separate.
pub(super) fn collection_insertion_can_pay(base: &super::value::DictMap, writes: u16) -> bool {
    if writes == 0 || base.is_empty() {
        return false;
    }
    let bucket_bytes = std::mem::size_of::<(Arc<str>, BatchValue)>() + 1;
    let mut buckets = 4usize;
    let mut traffic = 0usize;
    loop {
        traffic = traffic.saturating_add(buckets.saturating_mul(bucket_bytes) + 16);
        if (buckets == 4 && writes <= 3)
            || (buckets > 4 && buckets - buckets / 8 >= usize::from(writes))
        {
            break;
        }
        buckets *= 2;
    }
    traffic <= base.capacity().saturating_mul(std::mem::size_of::<Value>())
}
impl NestedPublication {
    fn collection_insertion_admission(
        root: &Value,
        path: &[Access],
        value: &Value,
        policy: super::nested_spill_proof::SpillPolicy,
    ) -> bool {
        let text_or_bytes = matches!(value, Value::Str(_) | Value::Bytes(_));
        if !policy.collection_rows
            || policy.capacity < 2
            || !(DictionaryCollectionPublication::supports(value) || text_or_bytes)
        {
            return false;
        }
        let Some((owner, path)) = Self::admission_owner(root, path) else { return false };
        let [first, Access::Index(index)] = path else { return false };
        let Some(Value::Dict(base)) = RowKey::from_access(first).and_then(|key| key.value(owner))
        else {
            return false;
        };
        if Arc::strong_count(base) != 1 || !collection_insertion_can_pay(base, policy.capacity) {
            return false;
        }
        let exists = match index {
            Value::Str(key) => base.contains_key(key.as_str()),
            Value::Int(key) => with_integer_key(*key, |key| base.contains_key(key)),
            _ => return false,
        };
        // Existing collection values retain their compact single-write route.
        // Text/bytes need the mixed journal even for the first replacement.
        if exists {
            text_or_bytes
        } else {
            policy.allow_initial_insertion
        }
    }

    pub(super) fn can_prepare_with_policy(
        root: &Value,
        path: &[Access],
        value: &Value,
        policy: super::nested_spill_proof::SpillPolicy,
    ) -> bool {
        Self::can_prepare(root, path, value, policy.allow_initial_insertion)
            || Self::collection_insertion_admission(root, path, value, policy)
    }

    pub(super) fn prepare_owned_with_policy(
        name: String,
        root: Value,
        mut path: Vec<Access>,
        value: &mut Value,
        policy: super::nested_spill_proof::SpillPolicy,
    ) -> Result<Self, (String, Value, Vec<Access>)> {
        if !Self::collection_insertion_admission(&root, &path, value, policy) {
            return Self::prepare_owned(name, root, path, value);
        }
        // Admission checks every owner and the budget before consuming a value
        // or allocating a missing key. Replacements reuse the canonical key.
        let (owner, leaf_path) = Self::admission_owner(&root, &path).expect("admitted owner");
        let Value::Dict(base) = RowKey::from_access(&leaf_path[0])
            .and_then(|key| key.value(owner))
            .expect("admitted dictionary")
        else {
            unreachable!("admitted dictionary")
        };
        let key: Arc<str> = match path.last().expect("admitted key") {
            Access::Index(Value::Str(key)) => base
                .get_key_value(key.as_str())
                .map(|(key, _)| key.clone())
                .unwrap_or_else(|| Arc::from(key.as_str())),
            Access::Index(Value::Int(key)) => with_integer_key(*key, |key| {
                base.get_key_value(key)
                    .map(|(key, _)| key.clone())
                    .unwrap_or_else(|| Arc::from(key))
            }),
            _ => unreachable!("admitted key"),
        };
        path.pop();
        let selector = match path.pop().expect("admitted insertion owner") {
            Access::Field(field) => Selector::Field(field),
            Access::Index(Value::Str(key)) => Selector::String(key),
            Access::Index(Value::Int(key)) => Selector::Integer(key),
            _ => unreachable!("admitted insertion selector"),
        };
        let mut write = CollectionDictionaryWrite::with_known_keys(selector, policy.capacity, 1);
        write.writes.insert(key, BatchValue::from_owned(std::mem::replace(value, Value::Null)));
        path.insert(0, Access::Index(root));
        path.push(Access::DeferredCollectionDictionary(write));
        Ok(Self { name, path })
    }

    fn resolve(index: i64, length: usize) -> Option<usize> {
        let resolved = if index < 0 { length as i64 + index } else { index };
        (resolved >= 0 && (resolved as usize) < length).then_some(resolved as usize)
    }

    fn owner_count(value: &Value) -> Option<usize> {
        match value {
            Value::Struct { fields, .. } => Some(Arc::strong_count(fields)),
            Value::Dict(map) => Some(Arc::strong_count(map)),
            Value::Array(rows) => Some(Arc::strong_count(rows)),
            _ => None,
        }
    }

    /// Only the root is shared with the suspended owner. Every traversed
    /// intermediate container must be unique; retained aliases keep fallback.
    fn admission_owner<'a>(
        root: &'a Value,
        path: &'a [Access],
    ) -> Option<(&'a Value, &'a [Access])> {
        if path.len() < 2 || Self::owner_count(root) != Some(2) {
            return None;
        }
        let mut owner = root;
        for access in &path[..path.len() - 2] {
            owner = RowKey::from_access(access)?.value(owner)?;
            if Self::owner_count(owner) != Some(1) {
                return None;
            }
        }
        Some((owner, &path[path.len() - 2..]))
    }

    fn admission(root: &Value, path: &[Access], value: &Value) -> Option<(usize, Scalar)> {
        let (root, path) = Self::admission_owner(root, path)?;
        let [first, Access::Index(Value::Int(index))] = path else { return None };
        let row = RowKey::from_access(first)?.row(root)?;
        if Arc::strong_count(row) != 1 {
            return None;
        }
        Some((Self::resolve(*index, row.len())?, Scalar::from_value(value)?))
    }

    fn dictionary_admission(
        root: &Value,
        path: &[Access],
        value: &Value,
    ) -> Option<(Option<Arc<str>>, Scalar)> {
        let (root, path) = Self::admission_owner(root, path)?;
        let [first, Access::Index(index)] = path else { return None };
        let Value::Dict(map) = RowKey::from_access(first)?.value(root)? else { return None };
        if Arc::strong_count(map) != 1 {
            return None;
        }
        let value = Scalar::from_value(value)?;
        let key = match index {
            Value::Str(key) => map.get_key_value(key.as_str()).map(|(key, _)| key.clone()),
            Value::Int(index) => {
                with_integer_key(*index, |key| map.get_key_value(key).map(|(key, _)| key.clone()))
            }
            _ => return None,
        };
        // Empty leaves retain ordinary mutation. Eligibility checks must not allocate
        // a missing key: can_prepare and prepare both inspect the same assignment.
        if key.is_none() && map.is_empty() {
            return None;
        }
        Some((key, value))
    }

    fn collection_admission(root: &Value, path: &[Access], value: &Value) -> Option<CollectionKey> {
        if !DictionaryCollectionPublication::supports(value) {
            return None;
        }
        let (root, path) = Self::admission_owner(root, path)?;
        let [first, last] = path else { return None };
        let leaf = RowKey::from_access(first)?.value(root)?;
        if Self::owner_count(leaf) != Some(1) {
            return None;
        }
        CollectionWrite::key(leaf, RowKey::from_access(last)?)
    }

    pub(super) fn can_prepare(
        root: &Value,
        path: &[Access],
        value: &Value,
        allow_initial_insertion: bool,
    ) -> bool {
        Self::collection_admission(root, path, value).is_some()
            || Self::admission(root, path, value).is_some()
            || Self::dictionary_admission(root, path, value)
                .is_some_and(|(key, _)| key.is_some() || allow_initial_insertion)
    }

    #[cfg(test)]
    pub(super) fn prepare(
        name: String,
        root: Value,
        path: Vec<Access>,
        value: &Value,
    ) -> Result<Self, (String, Value, Vec<Access>)> {
        Self::prepare_owned(name, root, path, &mut value.clone())
    }

    /// Consume an admitted collection only after every owner/path check succeeds.
    /// Rejection leaves the assignment value available to ordinary mutation.
    pub(super) fn prepare_owned(
        name: String,
        root: Value,
        mut path: Vec<Access>,
        value: &mut Value,
    ) -> Result<Self, (String, Value, Vec<Access>)> {
        let collection = Self::collection_admission(&root, &path, value);
        let dictionary = Self::dictionary_admission(&root, &path, value);
        let row = if dictionary.is_none() { Self::admission(&root, &path, value) } else { None };
        if dictionary.is_none() && row.is_none() && collection.is_none() {
            return Err((name, root, path));
        }
        let dictionary = dictionary.map(|(key, value)| {
            let key = key.unwrap_or_else(|| match path.last().expect("admitted dictionary key") {
                Access::Index(Value::Str(key)) => Arc::from(key.as_str()),
                Access::Index(Value::Int(index)) => with_integer_key(*index, |key| Arc::from(key)),
                _ => unreachable!("admitted dictionary index"),
            });
            (key, value)
        });
        path.pop(); // Leaf key/index is retained in the write metadata.
        let selector = match path.pop().expect("admitted owner selector") {
            Access::Field(field) => Selector::Field(field),
            Access::Index(Value::Str(key)) => Selector::String(key),
            Access::Index(Value::Int(key)) => Selector::Integer(key),
            _ => unreachable!("admitted selector"),
        };
        // Reuse the original allocation: root + prefix + write has the same
        // length as the resolved assignment path, including for deep owners.
        path.insert(0, Access::Index(root));
        let write = if let Some(key) = collection {
            Access::DeferredCollection(CollectionWrite {
                selector,
                key,
                value: CollectionValue::from_owned_value(std::mem::replace(value, Value::Null)),
            })
        } else if let Some((key, value)) = dictionary {
            Access::DeferredDictionary(DictionaryWrite::new(selector, key, value))
        } else {
            let (index, value) = row.unwrap();
            Access::DeferredRow(RowWrite {
                selector,
                entries: [Some(RowEntry { index, value }), None, None, None],
            })
        };
        path.push(write);
        Ok(Self { name, path })
    }

    fn storage(&self) -> (&Value, &[Access], &Access) {
        let Some((Access::Index(root), rest)) = self.path.split_first() else { unreachable!() };
        let (write, prefix) = rest.split_last().expect("nested write");
        (root, prefix, write)
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    fn read_at(root: &Value, write: &Access, owner: RowKey<'_>, key: RowKey<'_>) -> Option<Value> {
        match write {
            Access::DeferredCollection(write) => write.read(root, owner, key),
            Access::DeferredCollectionRow(write) => {
                let RowKey::Integer(index) = key else { return None };
                if !write.selector.key().same(owner, root) {
                    return None;
                }
                let row = write.selector.key().row(root)?;
                Some(match Self::resolve(index, row.len()) {
                    Some(index) => write
                        .writes
                        .get(&index)
                        .map_or_else(|| row[index].clone(), |v| v.clone().into_value()),
                    None => Value::Error(format!("Index out of bounds: {index}")),
                })
            }
            Access::DeferredCollectionDictionary(write) => match key {
                RowKey::Integer(index) => {
                    with_integer_key(index, |key| write.read(root, owner, key))
                }
                RowKey::String(key) => write.read(root, owner, key),
                _ => None,
            },
            Access::DeferredDictionary(write) => match key {
                RowKey::Integer(index) => {
                    with_integer_key(index, |key| write.read(root, owner, key))
                }
                RowKey::String(key) => write.read(root, owner, key),
                _ => None,
            },
            Access::DeferredDictionaryMany(write) => match key {
                RowKey::Integer(index) => {
                    with_integer_key(index, |key| write.read(root, owner, key))
                }
                RowKey::String(key) => write.read(root, owner, key),
                _ => None,
            },
            Access::DeferredRow(write) => {
                let RowKey::Integer(index) = key else { return None };
                if !write.selector.key().same(owner, root) {
                    return None;
                }
                let row = write.selector.key().row(root).expect("validated row");
                Some(match Self::resolve(index, row.len()) {
                    Some(index) => write
                        .entries
                        .iter()
                        .flatten()
                        .find(|entry| entry.index == index)
                        .map_or_else(|| row[index].clone(), |entry| entry.value.value()),
                    None => Value::Error(format!("Index out of bounds: {index}")),
                })
            }
            Access::DeferredRowMany(write) => {
                let RowKey::Integer(index) = key else { return None };
                if !write.selector.key().same(owner, root) {
                    return None;
                }
                let row = write.selector.key().row(root).expect("validated row");
                Some(match Self::resolve(index, row.len()) {
                    Some(index) => {
                        write.writes.get(&index).map_or_else(|| row[index].clone(), |v| v.value())
                    }
                    None => Value::Error(format!("Index out of bounds: {index}")),
                })
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn read(&self, key: RowKey<'_>, index: i64) -> Option<Value> {
        let (root, prefix, write) = self.storage();
        if !prefix.is_empty() {
            return None;
        }
        Self::read_at(root, write, key, RowKey::Integer(index))
    }

    pub(super) fn read_string(&self, owner: RowKey<'_>, key: &str) -> Option<Value> {
        let (root, prefix, write) = self.storage();
        if !prefix.is_empty() {
            return None;
        }
        Self::read_at(root, write, owner, RowKey::String(key))
    }

    /// Match only pure selectors against the retained prefix, without creating
    /// a root snapshot or allocating another path. Effectful syntax falls back.
    pub(super) fn read_expression<'a>(
        &self,
        object: &'a crate::ast::Expr,
        leaf: RowKey<'_>,
        resolve: impl Fn(&'a crate::ast::Expr) -> Option<RowKey<'a>>,
    ) -> Option<Value> {
        use crate::ast::Expr;
        fn matches<'a, 'r>(
            expr: &'a Expr,
            root: &'r Value,
            name: &str,
            prefix: &[Access],
            resolve: &impl Fn(&'a Expr) -> Option<RowKey<'a>>,
        ) -> Option<&'r Value> {
            if prefix.is_empty() {
                return matches!(expr, Expr::Identifier(n) if n == name).then_some(root);
            }
            let (object, key) = match expr {
                Expr::FieldAccess { object, field } => (object.as_ref(), RowKey::Field(field)),
                Expr::IndexAccess { object, index } => (object.as_ref(), resolve(index)?),
                _ => return None,
            };
            let owner = matches(object, root, name, &prefix[..prefix.len() - 1], resolve)?;
            let expected = RowKey::from_access(prefix.last()?)?;
            if !expected.same(key, owner) {
                return None;
            }
            expected.value(owner)
        }
        let (root, prefix, write) = self.storage();
        let (parent, owner_key) = match object {
            Expr::FieldAccess { object, field } => (object.as_ref(), RowKey::Field(field)),
            Expr::IndexAccess { object, index } => (object.as_ref(), resolve(index)?),
            _ => return None,
        };
        let owner = matches(parent, root, &self.name, prefix, &resolve)?;
        Self::read_at(owner, write, owner_key, leaf)
    }

    #[cfg(test)]
    pub(super) fn try_record(&mut self, path: &[Access], value: &Value) -> bool {
        self.try_record_with_batching(path, value, 0)
    }

    #[cfg(test)]
    pub(super) fn try_record_with_batching(
        &mut self,
        path: &[Access],
        value: &Value,
        spill_hint: u16,
    ) -> bool {
        self.try_record_owned_with_batching(path, &mut value.clone(), spill_hint)
    }

    /// A failed record must leave both the pending write and input unchanged.
    #[cfg(test)]
    pub(super) fn try_record_owned_with_batching(
        &mut self,
        path: &[Access],
        value: &mut Value,
        spill_hint: u16,
    ) -> bool {
        self.try_record_owned_with_policy(
            path,
            value,
            super::nested_spill_proof::SpillPolicy { capacity: spill_hint, ..Default::default() },
        )
    }
    // None leaves compact scalar/single-collection handling in its existing path.
    fn record_collection_dictionary(
        root: &Value,
        write: &mut Access,
        path: &[Access],
        value: &mut Value,
        limit: u16,
    ) -> Option<bool> {
        let [first, Access::Index(index)] = path else { return None };
        let owner = RowKey::from_access(first)?;
        let selector = match &*write {
            Access::DeferredCollection(w) if matches!(w.key, CollectionKey::Dictionary(_)) => {
                &w.selector
            }
            Access::DeferredDictionary(w) => &w.selector,
            Access::DeferredDictionaryMany(w) => &w.selector,
            Access::DeferredCollectionDictionary(w) => &w.selector,
            _ => return None,
        };
        if !selector.key().same(owner, root) {
            return Some(false);
        }
        let Some(Value::Dict(base)) = selector.key().value(root) else { return Some(false) };
        let mut record = |key: &str| {
            if let Access::DeferredCollectionDictionary(w) = write {
                return Some(w.record(root, key, value));
            }
            if Scalar::from_value(value).is_some()
                && matches!(
                    write,
                    Access::DeferredDictionary(_) | Access::DeferredDictionaryMany(_)
                )
            {
                return None;
            }
            let (count, contains) = match &*write {
                Access::DeferredCollection(w) => {
                    let CollectionKey::Dictionary(old) = &w.key else { unreachable!() };
                    (1, old.as_ref() == key)
                }
                Access::DeferredDictionary(w) => (
                    w.keys.iter().flatten().count(),
                    w.keys.iter().flatten().any(|old| old.as_ref() == key),
                ),
                Access::DeferredDictionaryMany(w) => (w.writes.len(), w.writes.contains_key(key)),
                _ => unreachable!(),
            };
            if count == 1
                && contains
                && DictionaryCollectionPublication::supports(value)
                && matches!(write, Access::DeferredDictionary(_))
            {
                let Access::DeferredDictionary(old) =
                    std::mem::replace(write, Access::Field(String::new()))
                else {
                    unreachable!()
                };
                let key = old.keys.into_iter().flatten().next().expect("single scalar key");
                *write = Access::DeferredCollection(CollectionWrite {
                    selector: old.selector,
                    key: CollectionKey::Dictionary(key),
                    value: CollectionValue::from_owned_value(std::mem::replace(value, Value::Null)),
                });
                return Some(true);
            }
            if let Access::DeferredCollection(_) = write {
                if contains {
                    if DictionaryCollectionPublication::supports(value) {
                        return None;
                    }
                    if let Some(scalar) = Scalar::from_value(value) {
                        let Access::DeferredCollection(old) =
                            std::mem::replace(write, Access::Field(String::new()))
                        else {
                            unreachable!()
                        };
                        let CollectionKey::Dictionary(key) = old.key else { unreachable!() };
                        *write = Access::DeferredDictionary(DictionaryWrite::new(
                            old.selector,
                            key,
                            scalar,
                        ));
                        return Some(true);
                    }
                }
            }
            if count > usize::from(limit)
                || (!contains && count == usize::from(limit))
                || (!contains
                    && !base.contains_key(key)
                    && !collection_insertion_can_pay(base, limit))
            {
                return Some(false);
            }
            // Reserve only keys already present plus the incoming distinct key,
            // never the source loop bound. Promotion copies the existing journal
            // once without intermediate table growth.
            let known_keys = count + usize::from(!contains);
            // Every possible rejection precedes replacing the old journal.
            let old = std::mem::replace(write, Access::Field(String::new()));
            let mut batch = match old {
                Access::DeferredCollection(w) => {
                    let CollectionKey::Dictionary(key) = w.key else { unreachable!() };
                    let mut batch =
                        CollectionDictionaryWrite::with_known_keys(w.selector, limit, known_keys);
                    batch.writes.insert(key, BatchValue::Collection(w.value));
                    batch
                }
                Access::DeferredDictionary(w) => {
                    let values: [Value; 4] = std::array::from_fn(|slot| w.value(slot));
                    let mut batch =
                        CollectionDictionaryWrite::with_known_keys(w.selector, limit, known_keys);
                    for (key, value) in w.keys.into_iter().zip(values) {
                        if let Some(key) = key {
                            batch.writes.insert(key, BatchValue::from_owned(value));
                        }
                    }
                    batch
                }
                Access::DeferredDictionaryMany(w) => {
                    let mut batch =
                        CollectionDictionaryWrite::with_known_keys(w.selector, limit, known_keys);
                    for (key, value) in w.writes {
                        batch.writes.insert(key, BatchValue::from_owned(value.value()));
                    }
                    batch
                }
                _ => unreachable!(),
            };
            assert!(batch.record(root, key, value), "validated dictionary promotion");
            *write = Access::DeferredCollectionDictionary(batch);
            Some(true)
        };
        match index {
            Value::Str(key) => record(key),
            Value::Int(key) => with_integer_key(*key, record),
            _ => Some(false),
        }
    }

    pub(super) fn try_record_owned_with_policy(
        &mut self,
        path: &[Access],
        value: &mut Value,
        policy: super::nested_spill_proof::SpillPolicy,
    ) -> bool {
        let spill_hint = policy.capacity;
        let Some((Access::Index(root), rest)) = self.path.split_first_mut() else { unreachable!() };
        let (write, prefix) = rest.split_last_mut().expect("nested write");
        if path.len() != prefix.len() + 2 {
            return false;
        }
        let mut owner: &Value = root;
        for (stored, actual) in prefix.iter().zip(path.iter()) {
            let Some(expected) = RowKey::from_access(stored) else { return false };
            let Some(actual) = RowKey::from_access(actual) else { return false };
            if !expected.same(actual, owner) {
                return false;
            }
            let Some(child) = expected.value(owner) else { return false };
            owner = child;
        }
        let root = owner;
        let path = &path[prefix.len()..];
        if policy.collection_rows && spill_hint > 0 && BatchValue::supports(value) {
            if let Some(recorded) =
                Self::record_collection_dictionary(root, write, path, value, spill_hint)
            {
                return recorded;
            }
        }
        // Promotion is opt-in for a proven collection-bearing row region. All
        // validation precedes moving the old journal or the incoming payload.
        if policy.collection_rows && spill_hint > 0 && BatchValue::supports(value) {
            if let [first, Access::Index(Value::Int(raw))] = path {
                if let Some(actual) = RowKey::from_access(first) {
                    let selector = match write {
                        Access::DeferredCollection(w)
                            if matches!(w.key, CollectionKey::Array(_)) =>
                        {
                            Some(&w.selector)
                        }
                        Access::DeferredRow(w) => Some(&w.selector),
                        Access::DeferredRowMany(w) => Some(&w.selector),
                        Access::DeferredCollectionRow(w) => Some(&w.selector),
                        _ => None,
                    };
                    if let Some(selector) = selector {
                        if !selector.key().same(actual, root) {
                            return false;
                        }
                        let Some(row) = selector.key().row(root) else { return false };
                        let Some(index) = Self::resolve(*raw, row.len()) else { return false };
                        if let Access::DeferredCollectionRow(w) = write {
                            return w.record(index, value);
                        }
                        // Replacing an existing scalar slot needs no promotion,
                        // even if a later statement may write a collection.
                        if let Some(scalar) = Scalar::from_value(value) {
                            match write {
                                Access::DeferredRow(w) => {
                                    if let Some(entry) =
                                        w.entries.iter_mut().flatten().find(|e| e.index == index)
                                    {
                                        entry.value = scalar;
                                        return true;
                                    }
                                }
                                Access::DeferredRowMany(w) => {
                                    if let Some(entry) = w.writes.get_mut(&index) {
                                        *entry = scalar;
                                        return true;
                                    }
                                }
                                _ => {}
                            }
                        }
                        // Keep an ordinary single collection overwrite allocation-free.
                        let same_collection = matches!(write, Access::DeferredCollection(w)
                            if w.key == CollectionKey::Array(index) && DictionaryCollectionPublication::supports(value));
                        if !same_collection {
                            let prior_count = match write {
                                Access::DeferredCollection(_) => 1,
                                Access::DeferredRow(w) => w.entries.iter().flatten().count(),
                                Access::DeferredRowMany(w) => w.writes.len(),
                                _ => unreachable!(),
                            };
                            let contains = match write {
                                Access::DeferredCollection(w) => {
                                    w.key == CollectionKey::Array(index)
                                }
                                Access::DeferredRow(w) => {
                                    w.entries.iter().flatten().any(|e| e.index == index)
                                }
                                Access::DeferredRowMany(w) => w.writes.contains_key(&index),
                                _ => unreachable!(),
                            };
                            if prior_count > usize::from(spill_hint)
                                || (prior_count == usize::from(spill_hint) && !contains)
                            {
                                return false;
                            }
                            let old = std::mem::replace(write, Access::Field(String::new()));
                            let mut batch = match old {
                                Access::DeferredCollection(w) => {
                                    let CollectionKey::Array(index) = w.key else { unreachable!() };
                                    let mut batch = CollectionRowWrite::new(w.selector, spill_hint);
                                    batch.writes.insert(index, BatchValue::Collection(w.value));
                                    batch
                                }
                                Access::DeferredRow(w) => {
                                    let mut batch = CollectionRowWrite::new(w.selector, spill_hint);
                                    for entry in w.entries.into_iter().flatten() {
                                        batch.writes.insert(
                                            entry.index,
                                            BatchValue::from_owned(entry.value.value()),
                                        );
                                    }
                                    batch
                                }
                                Access::DeferredRowMany(w) => {
                                    let mut batch = CollectionRowWrite::new(w.selector, spill_hint);
                                    for (index, value) in w.writes {
                                        batch
                                            .writes
                                            .insert(index, BatchValue::from_owned(value.value()));
                                    }
                                    batch
                                }
                                _ => unreachable!(),
                            };
                            assert!(batch.record(index, value), "validated batch capacity");
                            *write = Access::DeferredCollectionRow(batch);
                            return true;
                        }
                    }
                }
            }
        }
        if matches!(
            write,
            Access::DeferredCollectionRow(_) | Access::DeferredCollectionDictionary(_)
        ) {
            return false;
        }
        if let Access::DeferredCollection(write) = write {
            let [first, last] = path else { return false };
            let Some(owner) = RowKey::from_access(first) else { return false };
            let Some(key) = RowKey::from_access(last) else { return false };
            if !write.selector.key().same(owner, root)
                || !DictionaryCollectionPublication::supports(value)
            {
                return false;
            }
            let Some(leaf) = write.selector.key().value(root) else { return false };
            if CollectionWrite::key(leaf, key).as_ref() != Some(&write.key) {
                return false;
            }
            write.value = CollectionValue::from_owned_value(std::mem::replace(value, Value::Null));
            return true;
        }
        if matches!(write, Access::DeferredDictionary(_) | Access::DeferredDictionaryMany(_)) {
            let [first, Access::Index(index)] = path else { return false };
            let Some(owner) = RowKey::from_access(first) else { return false };
            let Some(value) = Scalar::from_value(value) else { return false };
            let selector = match write {
                Access::DeferredDictionary(write) => &write.selector,
                Access::DeferredDictionaryMany(write) => &write.selector,
                _ => unreachable!(),
            };
            if !selector.key().same(owner, root) {
                return false;
            }
            let mut record = |key: &str| {
                if spill_hint == 0 {
                    if let Access::DeferredDictionary(inline) = write {
                        return inline.record(root, key, value);
                    }
                }
                if let Access::DeferredDictionary(inline) = write {
                    if inline.record(root, key, value) {
                        return true;
                    }
                    let Access::DeferredDictionary(inline) =
                        std::mem::replace(write, Access::Field(String::new()))
                    else {
                        unreachable!()
                    };
                    *write = Access::DeferredDictionaryMany(DictionaryManyWrite::from_inline(
                        inline, spill_hint,
                    ));
                }
                let Access::DeferredDictionaryMany(many) = write else { unreachable!() };
                many.record(root, key, value);
                true
            };
            return match index {
                Value::Str(key) => record(key),
                Value::Int(index) => with_integer_key(*index, record),
                _ => return false,
            };
        }

        let [first, Access::Index(Value::Int(index))] = path else { return false };
        let Some(key) = RowKey::from_access(first) else { return false };
        let Some(value) = Scalar::from_value(value) else { return false };
        let selector = match write {
            Access::DeferredRow(write) => &write.selector,
            Access::DeferredRowMany(write) => &write.selector,
            _ => unreachable!(),
        };
        let row = selector.key().row(root).expect("validated row");
        if !selector.key().same(key, root) {
            return false;
        }
        let Some(index) = Self::resolve(*index, row.len()) else { return false };
        // Coalesce aliases before looking for an unused slot. A full journal
        // must still accept repeated writes; rejection leaves it untouched.
        if let Access::DeferredRow(inline) = write {
            let slot = inline
                .entries
                .iter()
                .position(|entry| entry.is_some_and(|entry| entry.index == index))
                .or_else(|| inline.entries.iter().position(Option::is_none));
            if let Some(slot) = slot {
                inline.entries[slot] = Some(RowEntry { index, value });
                return true;
            }
            if spill_hint == 0 {
                return false;
            }
            let Access::DeferredRow(inline) =
                std::mem::replace(write, Access::Field(String::new()))
            else {
                unreachable!()
            };
            *write = Access::DeferredRowMany(RowManyWrite::from_inline(inline, spill_hint));
        }
        let Access::DeferredRowMany(many) = write else { unreachable!() };
        many.writes.insert(index, value);
        true
    }

    fn path_owner_mut<'a>(prefix: &[Access], mut root: &'a mut Value) -> &'a mut Value {
        for first in prefix {
            root = match (first, root) {
                (Access::Field(field), Value::Struct { fields, .. }) => {
                    Arc::make_mut(fields).get_mut(field)
                }
                (Access::Index(Value::Str(key)), Value::Dict(map)) => {
                    Arc::make_mut(map).get_mut(key.as_str())
                }
                (Access::Index(Value::Int(key)), Value::Dict(map)) => {
                    with_integer_key(*key, |key| Arc::make_mut(map).get_mut(key))
                }
                (Access::Index(Value::Int(key)), Value::Array(rows)) => {
                    let index = Self::resolve(*key, rows.len()).expect("validated prefix");
                    Arc::make_mut(rows).get_mut(index)
                }
                _ => unreachable!("validated prefix"),
            }
            .expect("validated prefix child");
        }
        root
    }

    fn apply_path(prefix: &[Access], write: &Access, root: &mut Value) {
        let root = Self::path_owner_mut(prefix, root);
        match write {
            Access::DeferredCollection(write) => write.apply(root),
            Access::DeferredCollectionRow(write) => write.apply(root),
            Access::DeferredCollectionDictionary(write) => write.apply(root),
            Access::DeferredRow(write) => write.apply(root),
            Access::DeferredRowMany(write) => write.apply(root),
            Access::DeferredDictionary(write) => write.apply(root),
            Access::DeferredDictionaryMany(write) => write.apply(root),
            _ => unreachable!("nested publication write"),
        }
    }

    fn apply_owned_path(prefix: &[Access], write: Access, root: &mut Value) {
        match write {
            Access::DeferredCollectionRow(write) => {
                write.apply_owned(Self::path_owner_mut(prefix, root));
            }
            Access::DeferredCollectionDictionary(write) => {
                write.apply_owned(Self::path_owner_mut(prefix, root));
            }
            Access::DeferredCollection(write) => {
                write.apply_owned(Self::path_owner_mut(prefix, root));
            }
            write => Self::apply_path(prefix, &write, root),
        }
    }

    pub(super) fn snapshot_value(&self) -> Value {
        let (root, prefix, write) = self.storage();
        let mut root = root.clone();
        Self::apply_path(prefix, write, &mut root);
        root
    }

    pub(super) fn publish(mut self, owner: Option<&mut Value>) -> (String, Value) {
        let write = self.path.pop().expect("nested write");
        let Access::Index(mut root) = self.path.remove(0) else { unreachable!() };
        if let Some(owner) = owner {
            let same = match (&root, &*owner) {
                (Value::Dict(a), Value::Dict(b)) => Arc::ptr_eq(a, b),
                (Value::Array(a), Value::Array(b)) => Arc::ptr_eq(a, b),
                (Value::Struct { name: a, fields: af }, Value::Struct { name: b, fields: bf }) => {
                    a == b && Arc::ptr_eq(af, bf)
                }
                _ => false,
            };
            if same {
                drop(root);
                Self::apply_owned_path(&self.path, write, owner);
                return (self.name, owner.clone());
            }
        }
        Self::apply_owned_path(&self.path, write, &mut root);
        (self.name, root)
    }

    pub(super) fn discard(mut self) -> (String, Value) {
        let Access::Index(root) = self.path.remove(0) else { unreachable!() };
        (self.name, root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_row_table_is_bounded_and_full_tables_accept_alias_rewrites() {
        let mut batch = CollectionRowWrite::new(Selector::Integer(0), 3);
        let capacity = batch.writes.capacity();
        for index in 0..capacity {
            assert!(batch.record(index, &mut Value::Int(index as i64)));
        }
        let mut extra = Value::array(vec![Value::Int(99)]);
        assert!(!batch.record(capacity, &mut extra));
        assert!(matches!(extra, Value::Array(_)));
        assert_eq!(batch.writes.len(), capacity);
        assert!(batch.record(0, &mut extra));
        assert!(matches!(extra, Value::Null));
        assert_eq!(batch.writes.capacity(), capacity);
        let mut owner = Value::array(vec![Value::array(vec![Value::Null; capacity])]);
        batch.apply_owned(&mut owner);
        let Value::Array(rows) = owner else { unreachable!() };
        let Value::Array(row) = &rows[0] else { unreachable!() };
        let Value::Array(first) = &row[0] else { unreachable!() };
        assert!(matches!(first[0], Value::Int(99)));
    }

    #[test]
    fn collection_policy_uses_existing_padding() {
        #[allow(dead_code)]
        struct PreviousPolicy {
            capacity: u16,
            allow_initial_insertion: bool,
        }
        assert_eq!(
            std::mem::size_of::<super::super::nested_spill_proof::SpillPolicy>(),
            std::mem::size_of::<PreviousPolicy>()
        );
    }

    #[test]
    fn collection_row_promotion_moves_values_and_rejects_invalid_writes_atomically() {
        let policy = super::super::nested_spill_proof::SpillPolicy {
            capacity: 3,
            collection_rows: true,
            ..Default::default()
        };
        for scalar_first in [false, true] {
            let mut owner = Value::array(vec![Value::array(vec![Value::Int(0); 8])]);
            let Value::Array(rows) = &owner else { unreachable!() };
            let root_pointer = Arc::as_ptr(rows);
            let Value::Array(row) = &rows[0] else { unreachable!() };
            let row_pointer = Arc::as_ptr(row);
            let mut first =
                if scalar_first { Value::Int(10) } else { Value::array(vec![Value::Int(10)]) };
            let mut pending = NestedPublication::prepare_owned(
                "state".into(),
                owner.clone(),
                array_path(0, 0),
                &mut first,
            )
            .unwrap();
            let mut payload = Value::array(vec![Value::Int(20)]);
            let Value::Array(values) = &payload else { unreachable!() };
            let payload_pointer = Arc::as_ptr(values);
            assert!(pending.try_record_owned_with_policy(&array_path(0, 1), &mut payload, policy));
            assert!(matches!(payload, Value::Null));
            let Some(Value::Array(values)) = pending.read(RowKey::Integer(0), 1) else { panic!() };
            assert_eq!(Arc::as_ptr(&values), payload_pointer);
            drop(values);
            assert!(pending.try_record_owned_with_policy(
                &array_path(0, 2),
                &mut Value::Int(30),
                policy
            ));
            let saved = pending.snapshot_value();
            let mut rejected = Value::array(vec![Value::Int(99)]);
            for path in [array_path(0, 8), array_path(1, 0)] {
                assert!(!pending.try_record_owned_with_policy(&path, &mut rejected, policy));
                assert!(matches!(rejected, Value::Array(_)));
            }
            assert!(!pending.try_record_owned_with_policy(
                &array_path(0, 1),
                &mut rejected,
                Default::default()
            ));
            assert!(matches!(rejected, Value::Array(_)));
            assert!(pending.try_record_owned_with_policy(
                &array_path(-1, -7),
                &mut rejected,
                policy
            ));
            assert!(matches!(rejected, Value::Null));
            pending.publish(Some(&mut owner));
            let Value::Array(rows) = &owner else { unreachable!() };
            assert_eq!(Arc::as_ptr(rows), root_pointer);
            let Value::Array(row) = &rows[0] else { unreachable!() };
            assert_eq!(Arc::as_ptr(row), row_pointer);
            let Value::Array(current) = &row[1] else { unreachable!() };
            assert!(matches!(current[0], Value::Int(99)));
            let Value::Array(saved) = saved else { unreachable!() };
            let Value::Array(saved_row) = &saved[0] else { unreachable!() };
            let Value::Array(old) = &saved_row[1] else { unreachable!() };
            assert!(matches!(old[0], Value::Int(20)));
        }
    }

    fn array_path(row: i64, index: i64) -> Vec<Access> {
        vec![Access::Index(Value::Int(row)), Access::Index(Value::Int(index))]
    }

    #[test]
    fn array_row_publication_coalesces_selector_aliases_and_preserves_snapshots() {
        let mut owner = Value::array(vec![Value::array(vec![Value::Int(0); 8]), Value::Null]);
        let Value::Array(rows) = &owner else { unreachable!() };
        let root_pointer = Arc::as_ptr(rows);
        let Value::Array(row) = &rows[0] else { unreachable!() };
        let row_pointer = Arc::as_ptr(row);
        let mut pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            array_path(-2, -8),
            &Value::Int(10),
        )
        .unwrap();
        assert!(pending.try_record(&array_path(0, 0), &Value::Float(-0.0)));
        for index in 1..4 {
            assert!(pending.try_record(&array_path(0, index), &Value::Int(index)));
        }
        let snapshot = pending.snapshot_value();
        assert!(!pending.try_record(&array_path(0, 4), &Value::Int(99)));
        assert!(!pending.try_record(&array_path(1, 0), &Value::Int(99)));
        assert!(!pending.try_record(&array_path(-3, 0), &Value::Int(99)));
        assert!(!pending.try_record(&array_path(0, 8), &Value::Int(99)));
        assert!(pending.read(RowKey::String("0"), 0).is_none());
        assert!(pending.read(RowKey::Integer(2), 0).is_none());
        assert!(pending.read(RowKey::Integer(i64::MIN), 0).is_none());
        assert!(
            matches!(pending.read(RowKey::Integer(0), 0), Some(Value::Float(v)) if v.to_bits() == (-0.0_f64).to_bits())
        );
        assert!(pending.try_record(&array_path(-2, -8), &Value::Bool(true)));
        let (_, current) = pending.publish(Some(&mut owner));
        let Value::Array(rows) = &owner else { unreachable!() };
        assert_eq!(Arc::as_ptr(rows), root_pointer);
        let Value::Array(row) = &rows[0] else { unreachable!() };
        assert_eq!(Arc::as_ptr(row), row_pointer);
        assert!(matches!(row[0], Value::Bool(true)));
        let Value::Array(saved) = snapshot else { unreachable!() };
        let Value::Array(saved_row) = &saved[0] else { unreachable!() };
        assert!(matches!(saved_row[0], Value::Float(v) if v.to_bits() == (-0.0_f64).to_bits()));
        assert!(matches!(current, Value::Array(_)));
    }

    #[test]
    fn array_row_admission_rejects_real_aliases_and_invalid_selectors() {
        let row = Value::array(vec![Value::Int(0); 8]);
        let owner = Value::array(vec![row.clone()]);
        assert!(NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            array_path(0, 0),
            &Value::Int(1)
        )
        .is_err());
        drop(row);
        for selector in
            [Value::Str(Arc::new("0".into())), Value::Int(1), Value::Int(-2), Value::Null]
        {
            assert!(NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                vec![Access::Index(selector), Access::Index(Value::Int(0))],
                &Value::Int(1)
            )
            .is_err());
        }
        let snapshot = owner.clone();
        assert!(NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            array_path(0, 0),
            &Value::Int(1)
        )
        .is_err());
        drop(snapshot);
        let pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            array_path(0, 0),
            &Value::Int(1),
        )
        .unwrap();
        let (_, unchanged) = pending.discard();
        let Value::Array(rows) = unchanged else { unreachable!() };
        let Value::Array(row) = &rows[0] else { unreachable!() };
        assert!(matches!(row[0], Value::Int(0)));
    }

    #[test]
    fn bounded_row_entries_do_not_grow_assignment_storage() {
        #[allow(dead_code)]
        enum LegacyAccess {
            Index(Value),
            Field(String),
            DeferredRow(LegacyRow),
        }
        #[allow(dead_code)]
        struct LegacyRow {
            selector: Selector,
            index: usize,
            value: Scalar,
        }
        assert_eq!(std::mem::size_of::<Access>(), std::mem::size_of::<LegacyAccess>());
        assert_eq!(std::mem::align_of::<Access>(), std::mem::align_of::<LegacyAccess>());
    }

    #[test]
    fn bounded_row_entries_coalesce_and_reject_overflow_atomically() {
        for dictionary in [false, true] {
            let mut owner = root(dictionary);
            let mut pending = NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path(dictionary, 0),
                &Value::Int(10),
            )
            .unwrap();
            for index in 1..4 {
                assert!(pending.try_record(&path(dictionary, index), &Value::Int(index + 10)));
            }
            let private = pending.snapshot_value();
            assert!(!pending.try_record(&path(dictionary, 4), &Value::Int(99)));
            assert!(!pending.try_record(&path(dictionary, 2048), &Value::Int(99)));
            assert!(pending.try_record(&path(dictionary, -2048), &Value::Int(20)));
            for index in 0..4 {
                assert!(matches!(pending.read(key(dictionary), index), Some(Value::Int(v))
                    if v == if index == 0 { 20 } else { index + 10 }));
                assert_eq!(element(&owner, dictionary, index as usize), 0);
                assert_eq!(element(&private, dictionary, index as usize), index + 10);
            }
            let (_, current) = pending.publish(Some(&mut owner));
            assert_eq!(element(&current, dictionary, 0), 20);
            assert_eq!(element(&owner, dictionary, 3), 13);
            assert_eq!(element(&owner, dictionary, 4), 0);
            assert_eq!(element(&private, dictionary, 0), 10);
        }
    }

    fn root(dictionary: bool) -> Value {
        let row = Value::array(vec![Value::Int(0); 2048]);
        if dictionary {
            Value::Dict(Arc::new([(Arc::<str>::from("1"), row)].into_iter().collect()))
        } else {
            Value::Struct {
                name: "State".into(),
                fields: Arc::new([("row".into(), row)].into_iter().collect()),
            }
        }
    }

    fn path(dictionary: bool, index: i64) -> Vec<Access> {
        vec![
            if dictionary { Access::Index(Value::Int(1)) } else { Access::Field("row".into()) },
            Access::Index(Value::Int(index)),
        ]
    }

    fn key(dictionary: bool) -> RowKey<'static> {
        if dictionary {
            RowKey::String("1")
        } else {
            RowKey::Field("row")
        }
    }

    fn element(root: &Value, dictionary: bool, index: usize) -> i64 {
        let Value::Int(value) = key(dictionary).row(root).unwrap()[index] else { panic!() };
        value
    }

    #[test]
    fn nested_publication_reuses_path_and_unique_public_row() {
        for dictionary in [false, true] {
            let mut owner = root(dictionary);
            let before = Arc::as_ptr(key(dictionary).row(&owner).unwrap());
            let accesses = path(dictionary, -1);
            let storage = (accesses.as_ptr(), accesses.capacity());
            let name = String::from("state");
            let name_storage = name.as_ptr();
            let mut pending =
                NestedPublication::prepare(name, owner.clone(), accesses, &Value::Int(7)).unwrap();
            assert_eq!((pending.path.as_ptr(), pending.path.capacity()), storage);
            assert_eq!(pending.name.as_ptr(), name_storage);
            assert_eq!(pending.name(), "state");
            assert_eq!(element(&owner, dictionary, 2047), 0);
            assert!(matches!(pending.read(key(dictionary), -1), Some(Value::Int(7))));
            assert!(pending.try_record(&path_for_last(dictionary), &Value::Int(9)));
            assert!(pending.try_record(&path(dictionary, 0), &Value::Int(88)));
            assert!(!pending.try_record(&path_for_last(dictionary), &Value::array(vec![])));
            let (_, current) = pending.publish(Some(&mut owner));
            assert_eq!(Arc::as_ptr(key(dictionary).row(&owner).unwrap()), before);
            assert_eq!(element(&current, dictionary, 2047), 9);
            assert_eq!(element(&owner, dictionary, 2047), 9);
            assert_eq!(element(&owner, dictionary, 0), 88);
        }
    }

    fn path_for_last(dictionary: bool) -> Vec<Access> {
        path(dictionary, 2047)
    }

    #[test]
    fn nested_publication_late_observers_and_private_snapshots_are_isolated() {
        for dictionary in [false, true] {
            let mut owner = root(dictionary);
            let pending = NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path(dictionary, 0),
                &Value::Int(5),
            )
            .unwrap();
            let public_snapshot = owner.clone();
            let escaped_row = key(dictionary).row(&owner).unwrap().clone();
            let private_snapshot = pending.snapshot_value();
            assert_eq!(element(&public_snapshot, dictionary, 0), 0);
            assert_eq!(element(&private_snapshot, dictionary, 0), 5);
            let (_, current) = pending.publish(Some(&mut owner));
            assert_eq!(element(&current, dictionary, 0), 5);
            assert_eq!(element(&public_snapshot, dictionary, 0), 0);
            assert!(matches!(escaped_row[0], Value::Int(0)));
        }
    }

    #[test]
    fn nested_publication_rejects_existing_snapshots_and_invalid_paths_atomically() {
        for dictionary in [false, true] {
            let owner = root(dictionary);
            let snapshot = owner.clone();
            let path = path(dictionary, 0);
            let pointer = path.as_ptr();
            let (_, returned, returned_path) =
                NestedPublication::prepare("state".into(), owner.clone(), path, &Value::Int(4))
                    .unwrap_err();
            assert_eq!(pointer, returned_path.as_ptr());
            assert_eq!(element(&returned, dictionary, 0), 0);
            drop(returned);
            drop(snapshot);
            assert!(NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path_for_last(dictionary),
                &Value::str_ref("no")
            )
            .is_err());
            assert!(NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                vec![],
                &Value::Int(4)
            )
            .is_err());
            assert!(NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                self::path(dictionary, 2048),
                &Value::Int(4)
            )
            .is_err());
            let escaped = key(dictionary).row(&owner).unwrap().clone();
            assert!(NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path_for_last(dictionary),
                &Value::Int(4)
            )
            .is_err());
            assert!(matches!(escaped[2047], Value::Int(0)));
            assert_eq!(element(&owner, dictionary, 0), 0);
        }
    }

    #[test]
    fn nested_publication_owner_replacement_and_discard_keep_original_base() {
        for dictionary in [false, true] {
            let mut owner = root(dictionary);
            let pending = NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path(dictionary, 0),
                &Value::Int(5),
            )
            .unwrap();
            owner = root(dictionary);
            let (_, result) = pending.publish(Some(&mut owner));
            assert_eq!(element(&owner, dictionary, 0), 0);
            assert_eq!(element(&result, dictionary, 0), 5);
            let pending = NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path(dictionary, 0),
                &Value::Int(8),
            )
            .unwrap();
            let (_, discarded) = pending.discard();
            assert_eq!(element(&discarded, dictionary, 0), 0);
        }
    }

    #[test]
    fn nested_publication_keys_scalars_and_errors_keep_their_meaning() {
        let owner = root(true);
        let mut pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(true, 0),
            &Value::Int(i64::MIN),
        )
        .unwrap();
        assert!(pending.read(RowKey::String("01"), 0).is_none());
        assert!(pending.read(RowKey::Field("1"), 0).is_none());
        assert!(matches!(pending.read(RowKey::Integer(1), 0), Some(Value::Int(i64::MIN))));
        assert!(
            matches!(pending.read(RowKey::Integer(1), 2048), Some(Value::Error(s)) if s == "Index out of bounds: 2048")
        );
        let aliases = vec![Access::Index(Value::str_ref("1")), Access::Index(Value::Int(0))];
        assert!(pending.try_record(&aliases, &Value::Float(-0.0)));
        assert!(
            matches!(pending.read(RowKey::Integer(1), 0), Some(Value::Float(v)) if v.to_bits() == (-0.0f64).to_bits())
        );
        assert!(pending.try_record(&aliases, &Value::Bool(true)));
        assert!(matches!(pending.read(RowKey::Integer(1), 0), Some(Value::Bool(true))));
        assert!(pending.try_record(&aliases, &Value::Null));
        assert!(matches!(pending.read(RowKey::Integer(1), 0), Some(Value::Null)));
    }

    #[test]
    fn nested_publication_private_escape_then_write_keeps_both_snapshots() {
        for dictionary in [false, true] {
            let mut owner = root(dictionary);
            let mut pending = NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path(dictionary, 0),
                &Value::Int(7),
            )
            .unwrap();
            let private = pending.snapshot_value();
            let public = owner.clone();
            let private_row = key(dictionary).row(&private).unwrap().clone();
            assert!(pending.try_record(&path(dictionary, 0), &Value::Int(9)));
            let (_, result) = pending.publish(Some(&mut owner));
            assert_eq!(element(&result, dictionary, 0), 9);
            assert_eq!(element(&owner, dictionary, 0), 9);
            assert_eq!(element(&public, dictionary, 0), 0);
            assert_eq!(element(&private, dictionary, 0), 7);
            assert!(matches!(private_row[0], Value::Int(7)));
        }
    }

    #[test]
    fn nested_publication_rejects_rows_shared_by_two_fields() {
        for dictionary in [false, true] {
            let mut owner = root(dictionary);
            let alias = Value::Array(key(dictionary).row(&owner).unwrap().clone());
            match &mut owner {
                Value::Struct { fields, .. } => {
                    Arc::make_mut(fields).insert("other".into(), alias);
                }
                Value::Dict(map) => {
                    Arc::make_mut(map).insert("other".into(), alias);
                }
                _ => unreachable!(),
            }
            assert!(NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path(dictionary, 0),
                &Value::Int(5),
            )
            .is_err());
            assert_eq!(element(&owner, dictionary, 0), 0);
        }
    }

    #[test]
    fn nested_publication_struct_name_replacement_is_not_the_original_owner() {
        let mut owner = root(false);
        let pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(false, 0),
            &Value::Int(5),
        )
        .unwrap();
        let Value::Struct { name, .. } = &mut owner else { unreachable!() };
        *name = "Replacement".into();
        let (_, result) = pending.publish(Some(&mut owner));
        assert_eq!(element(&owner, false, 0), 0);
        assert_eq!(element(&result, false, 0), 5);
        assert!(matches!(result, Value::Struct { name, .. } if name == "State"));
    }

    #[test]
    fn nested_publication_path_storage_does_not_grow() {
        #[allow(dead_code)]
        enum PreviousAccess {
            Index(Value),
            Field(String),
        }
        assert_eq!(std::mem::size_of::<Access>(), std::mem::size_of::<PreviousAccess>());
        assert_eq!(std::mem::align_of::<Access>(), std::mem::align_of::<PreviousAccess>());
        assert_eq!(std::mem::size_of::<NestedPublication>(), 48);
    }
}

#[cfg(test)]
mod dictionary_tests {
    use super::*;
    use crate::interpreter::value::DictMap;

    fn leaf() -> Value {
        Value::dict(
            [(Arc::<str>::from("0"), Value::Int(1)), (Arc::<str>::from("01"), Value::Int(2))]
                .into_iter()
                .collect::<DictMap>(),
        )
    }
    fn path(key: Value) -> Vec<Access> {
        vec![Access::Index(Value::Int(0)), Access::Index(key)]
    }

    #[test]
    fn dictionary_leaf_preserves_owner_identity_aliases_and_exact_scalars() {
        let mut owner = Value::array(vec![leaf()]);
        let Value::Array(rows) = &owner else { unreachable!() };
        let owner_pointer = Arc::as_ptr(rows);
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        let leaf_pointer = Arc::as_ptr(map);
        let mut pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(Value::Int(0)),
            &Value::Float(-0.0),
        )
        .unwrap();
        assert!(
            matches!(pending.read_string(RowKey::Integer(-1), "0"), Some(Value::Float(v)) if v.to_bits()==(-0.0f64).to_bits())
        );
        let saved = pending.snapshot_value();
        assert!(pending.try_record(&path(Value::str_ref("0")), &Value::Bool(true)));
        assert!(pending.try_record(&path(Value::str_ref("01")), &Value::Int(50)));
        assert!(matches!(pending.read_string(RowKey::Integer(0), "01"), Some(Value::Int(50))));
        assert!(!pending.try_record(&path(Value::Int(0)), &Value::array(vec![])));
        assert!(matches!(pending.read(RowKey::Integer(0), 0), Some(Value::Bool(true))));
        let (_, published) = pending.publish(Some(&mut owner));
        let Value::Array(rows) = &owner else { unreachable!() };
        assert_eq!(Arc::as_ptr(rows), owner_pointer);
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        assert_eq!(Arc::as_ptr(map), leaf_pointer);
        assert!(matches!(map.get("0"), Some(Value::Bool(true))));
        let Value::Array(rows) = saved else { unreachable!() };
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        assert!(matches!(map.get("0"), Some(Value::Float(v)) if v.to_bits()==(-0.0f64).to_bits()));
        assert!(matches!(published, Value::Array(_)));
    }

    #[test]
    fn replacing_existing_keys_does_not_grow_full_leaf_tables() {
        for width in [3, 7, 14, 28] {
            for batched in [false, true] {
                if batched && width < 5 {
                    continue;
                }
                let mut map = DictMap::with_capacity_and_hasher(width, Default::default());
                for i in 0..width {
                    map.insert(Arc::from(i.to_string()), Value::Int(i as i64));
                }
                assert_eq!(map.len(), map.capacity());
                let capacity = map.capacity();
                let mut owner = Value::array(vec![Value::dict(map)]);
                let mut pending = NestedPublication::prepare(
                    "state".into(),
                    owner.clone(),
                    path(Value::Int(0)),
                    &Value::Int(42),
                )
                .unwrap();
                if batched {
                    for i in 1..5 {
                        assert!(pending.try_record_with_batching(
                            &path(Value::Int(i)),
                            &Value::Int(42 + i),
                            5
                        ));
                    }
                }
                let (_, published) = pending.publish(Some(&mut owner));
                for value in [owner, published] {
                    let Value::Array(rows) = value else { unreachable!() };
                    let Value::Dict(map) = &rows[0] else { unreachable!() };
                    assert_eq!(map.capacity(), capacity, "width={width} batched={batched}");
                    assert_eq!(map.len(), width);
                    assert!(matches!(map.get("0"), Some(Value::Int(42))));
                }
            }
        }
    }

    #[test]
    fn initial_dictionary_key_preserves_snapshot_discard_and_unique_publication() {
        let mut owner = Value::array(vec![leaf()]);
        let captured = owner.clone();
        assert!(NestedPublication::can_prepare(
            &captured,
            &path(Value::Int(0)),
            &Value::Int(9),
            false
        ));
        assert!(!NestedPublication::can_prepare(
            &captured,
            &path(Value::Int(2)),
            &Value::Int(9),
            false
        ));
        assert!(NestedPublication::can_prepare(
            &captured,
            &path(Value::Int(2)),
            &Value::Int(9),
            true
        ));
        drop(captured);
        let pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(Value::Int(2)),
            &Value::Bool(false),
        )
        .unwrap();
        assert!(matches!(pending.read(RowKey::Integer(0), 2), Some(Value::Bool(false))));
        let saved = pending.snapshot_value();
        let (_, discarded) = pending.clone().discard();
        let Value::Array(rows) = discarded else { unreachable!() };
        assert!(rows[0].dictionary_get("2").is_none());
        drop(rows);
        let (_, published) = pending.publish(Some(&mut owner));
        for value in [saved, owner, published] {
            let Value::Array(rows) = value else { unreachable!() };
            assert!(matches!(rows[0].dictionary_get("2"), Some(Value::Bool(false))));
            assert!(matches!(rows[0].dictionary_get("01"), Some(Value::Int(2))));
        }
        let empty = Value::array(vec![Value::dict(DictMap::default())]);
        assert!(NestedPublication::prepare(
            "state".into(),
            empty.clone(),
            path(Value::Int(0)),
            &Value::Null,
        )
        .is_err());
    }

    #[test]
    fn dictionary_leaf_rejects_shared_roots_leaves_and_preserves_discard() {
        let shared = leaf();
        let owner = Value::array(vec![shared.clone()]);
        assert!(NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(Value::Int(0)),
            &Value::Int(9)
        )
        .is_err());
        drop(shared);
        let snapshot = owner.clone();
        assert!(NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(Value::Int(0)),
            &Value::Int(9)
        )
        .is_err());
        drop(snapshot);
        assert!(NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(Value::Bool(false)),
            &Value::Int(9)
        )
        .is_err());
        let pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(Value::Int(0)),
            &Value::Null,
        )
        .unwrap();
        let (_, unchanged) = pending.discard();
        let Value::Array(rows) = unchanged else { unreachable!() };
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        assert!(matches!(map.get("0"), Some(Value::Int(1))));
    }
}

#[cfg(test)]
mod dictionary_batch_tests {
    use super::*;
    use crate::interpreter::value::DictMap;

    fn path(key: &str) -> Vec<Access> {
        vec![Access::Index(Value::Int(0)), Access::Index(Value::str_ref(key))]
    }

    #[test]
    fn dictionary_batch_fits_storage_and_preserves_full_scalar_bits() {
        assert!(std::mem::size_of::<DictionaryWrite>() <= std::mem::size_of::<Value>());
        let leaf = Value::dict(DictMap::from_iter([
            (Arc::from("0"), Value::Int(1)),
            (Arc::from("01"), Value::Int(2)),
        ]));
        let mut owner = Value::array(vec![leaf]);
        let mut pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path("0"),
            &Value::Int(i64::MIN),
        )
        .unwrap();
        let bits = 0xfff8_0000_0000_0042;
        assert!(pending.try_record(&path("01"), &Value::Float(f64::from_bits(bits))));
        assert!(pending.try_record(&path("new"), &Value::Bool(true)));
        assert!(pending.try_record(&path("null"), &Value::Null));
        assert!(!pending.try_record(&path("overflow"), &Value::Int(5)));
        let saved = pending.snapshot_value();
        assert!(pending.try_record(&path("0"), &Value::Int(i64::MAX)));
        assert!(pending.try_record(&path("new"), &Value::Float(-0.0)));
        assert!(
            matches!(pending.read_string(RowKey::Integer(-1), "01"), Some(Value::Float(v)) if v.to_bits()==bits)
        );
        assert!(matches!(pending.read(RowKey::Integer(0), 0), Some(Value::Int(i64::MAX))));
        assert!(
            matches!(pending.read_string(RowKey::Integer(0), "new"), Some(Value::Float(v)) if v.to_bits()==(-0.0f64).to_bits())
        );
        assert!(pending.read_string(RowKey::Integer(0), "overflow").is_none());
        pending.publish(Some(&mut owner));
        let Value::Array(rows) = owner else { unreachable!() };
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        assert!(matches!(map.get("0"), Some(Value::Int(i64::MAX))));
        assert!(matches!(map.get("null"), Some(Value::Null)));
        assert!(!map.contains_key("overflow"));
        let Value::Array(rows) = saved else { unreachable!() };
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        assert!(matches!(map.get("0"), Some(Value::Int(i64::MIN))));
        assert!(matches!(map.get("new"), Some(Value::Bool(true))));
        assert!(matches!(map.get("01"), Some(Value::Float(v)) if v.to_bits()==bits));
    }
}

#[cfg(test)]
mod dictionary_spill_tests {
    use super::*;
    use crate::interpreter::value::DictMap;

    fn path(key: &str) -> Vec<Access> {
        vec![Access::Index(Value::Int(0)), Access::Index(Value::str_ref(key))]
    }

    #[test]
    fn spill_preserves_scalars_snapshots_and_unique_owner_storage() {
        assert!(std::mem::size_of::<DictionaryManyWrite>() <= std::mem::size_of::<Value>());
        let mut owner = Value::array(vec![Value::dict(DictMap::from_iter(
            (0..300).map(|i| (Arc::from(i.to_string()), Value::Int(i))),
        ))]);
        let Value::Array(rows) = &owner else { unreachable!() };
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        let pointer = Arc::as_ptr(map);
        let mut pending =
            NestedPublication::prepare("state".into(), owner.clone(), path("0"), &Value::Int(1000))
                .unwrap();
        for i in 1..257 {
            assert!(pending.try_record_with_batching(
                &path(&i.to_string()),
                &Value::Int(i + 1000),
                257
            ));
        }
        let [_, Access::DeferredDictionaryMany(many)] = pending.path.as_slice() else {
            panic!("did not spill")
        };
        assert_eq!(many.writes.len(), 257);
        let bits = 0xfff8_0000_0000_0042;
        for (key, value) in [
            ("0", Value::Int(i64::MIN)),
            ("1", Value::Float(f64::from_bits(bits))),
            ("2", Value::Float(-0.0)),
            ("3", Value::Bool(true)),
            ("4", Value::Null),
        ] {
            assert!(pending.try_record_with_batching(&path(key), &value, 257));
        }
        assert!(!pending.try_record_with_batching(&path("invalid"), &Value::array(vec![]), 257));
        let saved = pending.snapshot_value();
        assert!(pending.try_record_with_batching(&path("0"), &Value::Int(i64::MAX), 257));
        assert!(matches!(pending.read(RowKey::Integer(-1), 0), Some(Value::Int(i64::MAX))));
        assert!(
            matches!(pending.read_string(RowKey::Integer(0), "1"), Some(Value::Float(v)) if v.to_bits() == bits)
        );
        pending.publish(Some(&mut owner));
        let Value::Array(rows) = &owner else { unreachable!() };
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        assert_eq!(Arc::as_ptr(map), pointer);
        assert!(matches!(map.get("0"), Some(Value::Int(i64::MAX))));
        assert!(
            matches!(map.get("2"), Some(Value::Float(v)) if v.to_bits() == (-0.0f64).to_bits())
        );
        assert!(matches!(map.get("3"), Some(Value::Bool(true))));
        assert!(matches!(map.get("4"), Some(Value::Null)));
        let Value::Array(saved) = saved else { unreachable!() };
        let Value::Dict(saved) = &saved[0] else { unreachable!() };
        assert!(matches!(saved.get("0"), Some(Value::Int(i64::MIN))));
        for i in 5..300 {
            let expected = if i < 257 { i + 1000 } else { i };
            assert!(
                matches!(map.get(i.to_string().as_str()), Some(Value::Int(v)) if *v == expected)
            );
            assert!(
                matches!(saved.get(i.to_string().as_str()), Some(Value::Int(v)) if *v == expected)
            );
        }
    }

    #[test]
    fn discard_releases_spill_keys_without_changing_owner() {
        let owner =
            Value::array(vec![Value::dict(DictMap::from_iter([(Arc::from("0"), Value::Int(0))]))]);
        let mut pending =
            NestedPublication::prepare("state".into(), owner.clone(), path("0"), &Value::Int(1))
                .unwrap();
        for i in 1..6 {
            assert!(pending.try_record_with_batching(&path(&i.to_string()), &Value::Int(i), 257));
        }
        let [_, Access::DeferredDictionaryMany(many)] = pending.path.as_slice() else {
            panic!("did not spill")
        };
        let key = Arc::downgrade(many.writes.get_key_value("5").unwrap().0);
        let (_, unchanged) = pending.discard();
        assert!(key.upgrade().is_none());
        let Value::Array(rows) = unchanged else { unreachable!() };
        let Value::Dict(map) = &rows[0] else { unreachable!() };
        assert_eq!(map.len(), 1);
        assert!(matches!(map.get("0"), Some(Value::Int(0))));
    }
}

#[cfg(test)]
mod deep_tests {
    use super::*;
    use crate::ast::{Expr, Stmt};
    use crate::interpreter::value::DictMap;

    fn leaf(dictionary: bool) -> Value {
        if dictionary {
            Value::dict(DictMap::from_iter([
                (Arc::from("0"), Value::Int(0)),
                (Arc::from("1"), Value::Int(1)),
            ]))
        } else {
            Value::array(vec![Value::Int(0), Value::Int(1)])
        }
    }

    fn inner(leaf: Value) -> Value {
        Value::array(vec![Value::Struct {
            name: "Inner".into(),
            fields: Arc::new(std::collections::HashMap::from([("row".into(), leaf)])),
        }])
    }

    fn root(inner: Value) -> Value {
        Value::dict(DictMap::from_iter([(Arc::from("top"), inner)]))
    }

    fn path(index: i64) -> Vec<Access> {
        vec![
            Access::Index(Value::str_ref("top")),
            Access::Index(Value::Int(-1)),
            Access::Field("row".into()),
            Access::Index(Value::Int(index)),
        ]
    }

    fn row(root: &Value) -> &Value {
        let inner = RowKey::String("top").value(root).unwrap();
        let object = RowKey::Integer(0).value(inner).unwrap();
        RowKey::Field("row").value(object).unwrap()
    }

    fn value(root: &Value, index: i64) -> &Value {
        RowKey::Integer(index).value(row(root)).unwrap()
    }

    fn pointer(root: &Value) -> usize {
        match row(root) {
            Value::Array(row) => Arc::as_ptr(row) as usize,
            Value::Dict(map) => Arc::as_ptr(map) as usize,
            _ => unreachable!(),
        }
    }

    fn object(source: &str) -> Expr {
        let parsed = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty());
        let Stmt::Return(Some(Expr::IndexAccess { object, .. })) =
            parsed.stmts.into_iter().next().unwrap()
        else {
            panic!("expected indexed return")
        };
        *object
    }

    fn selector(expr: &Expr) -> Option<RowKey<'_>> {
        match expr {
            Expr::Int(value) => Some(RowKey::Integer(*value)),
            Expr::String(value) => Some(RowKey::String(value)),
            _ => None,
        }
    }

    #[test]
    fn deep_mixed_paths_reuse_storage_and_preserve_snapshots_and_scalar_bits() {
        for dictionary in [false, true] {
            let mut owner = root(inner(leaf(dictionary)));
            let backing = pointer(&owner);
            let path = path(0);
            let allocation = path.as_ptr();
            let capacity = path.capacity();
            let mut pending =
                NestedPublication::prepare("state".into(), owner.clone(), path, &Value::Int(42))
                    .unwrap();
            assert_eq!(pending.path.as_ptr(), allocation);
            assert_eq!(pending.path.capacity(), capacity);
            let object = object("return state[\"top\"][0].row[0]");
            assert!(matches!(
                pending.read_expression(&object, RowKey::Integer(0), selector),
                Some(Value::Int(42))
            ));
            let snapshot = pending.snapshot_value();
            let mut alias_path = super::deep_tests::path(0);
            alias_path[1] = Access::Index(Value::Int(0));
            if dictionary {
                alias_path[3] = Access::Index(Value::str_ref("0"));
            }
            assert!(pending.try_record(&alias_path, &Value::Int(100)));
            let bits = 0xfff8_0000_0000_0042;
            assert!(pending
                .try_record(&super::deep_tests::path(1), &Value::Float(f64::from_bits(bits))));
            assert!(!pending.try_record(&super::deep_tests::path(1), &Value::array(vec![])));
            assert!(matches!(
                pending.read_expression(&object, RowKey::Integer(0), selector),
                Some(Value::Int(100))
            ));
            pending.publish(Some(&mut owner));
            assert_eq!(pointer(&owner), backing);
            assert!(matches!(value(&owner, 0), Value::Int(100)));
            assert!(matches!(value(&owner, 1), Value::Float(v) if v.to_bits()==bits));
            assert!(matches!(value(&snapshot, 0), Value::Int(42)));
            assert!(matches!(value(&snapshot, 1), Value::Int(1)));
        }
    }

    #[test]
    fn deep_admission_rejects_shared_intermediates_and_leaves() {
        for dictionary in [false, true] {
            let retained = inner(leaf(dictionary));
            let owner = root(retained.clone());
            assert!(NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path(0),
                &Value::Int(9)
            )
            .is_err());
            assert!(matches!(value(&owner, 0), Value::Int(0)));
            let retained = leaf(dictionary);
            let owner = root(inner(retained.clone()));
            assert!(NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path(0),
                &Value::Int(9)
            )
            .is_err());
            assert!(matches!(value(&owner, 0), Value::Int(0)));
        }
    }

    #[test]
    fn deep_reads_reject_effectful_or_different_prefixes_and_discard_is_unchanged() {
        let owner = root(inner(leaf(false)));
        let pending =
            NestedPublication::prepare("state".into(), owner.clone(), path(0), &Value::Int(9))
                .unwrap();
        for expression in [
            "return state[\"top\"][callback()].row[0]",
            "return state[\"other\"][0].row[0]",
            "return other[\"top\"][0].row[0]",
        ] {
            assert!(pending
                .read_expression(&object(expression), RowKey::Integer(0), selector)
                .is_none());
        }
        let (_, unchanged) = pending.discard();
        assert_eq!(pointer(&unchanged), pointer(&owner));
        assert!(matches!(value(&unchanged, 0), Value::Int(0)));
    }
}

#[cfg(test)]
mod row_spill_tests {
    use super::*;
    fn path(i: i64) -> Vec<Access> {
        vec![Access::Index(Value::Int(0)), Access::Index(Value::Int(i))]
    }
    #[test]
    fn row_overflow_coalesces_aliases_preserves_snapshots_and_reuses_leaf() {
        let mut owner = Value::array(vec![Value::array((0..128).map(Value::Int).collect())]);
        let before = RowKey::Integer(0).row(&owner).unwrap().as_ptr();
        let mut pending =
            NestedPublication::prepare("state".into(), owner.clone(), path(0), &Value::Int(42))
                .unwrap();
        for i in 1..64 {
            assert!(pending.try_record_with_batching(&path(i), &Value::Int(i + 42), 64));
        }
        let snapshot = pending.snapshot_value();
        assert!(pending.try_record_with_batching(
            &path(-128),
            &Value::Float(f64::from_bits(0x7ff8000000000042)),
            64
        ));
        assert!(!pending.try_record_with_batching(&path(128), &Value::Int(9), 64));
        assert!(!pending.try_record_with_batching(&path(1), &Value::array(vec![]), 64));
        assert!(matches!(pending.read(RowKey::Integer(0), 63), Some(Value::Int(105))));
        pending.publish(Some(&mut owner));
        let row = RowKey::Integer(0).row(&owner).unwrap();
        assert_eq!(row.as_ptr(), before);
        assert!(matches!(row[0], Value::Float(v) if v.to_bits() == 0x7ff8000000000042));
        assert!(matches!(RowKey::Integer(0).row(&snapshot).unwrap()[0], Value::Int(42)));
        for i in 1..128 {
            assert!(
                matches!(row[i], Value::Int(n) if n == if i < 64 {i as i64+42} else {i as i64})
            );
        }
    }
}

#[cfg(test)]
mod collection_tests {
    use super::*;

    fn payload(value: i64) -> Value {
        Value::Struct {
            name: Box::<str>::from("Item").into_string(),
            fields: [("value".to_owned(), Value::Int(value))]
                .into_iter()
                .collect::<std::collections::HashMap<_, _>>()
                .into(),
        }
    }

    #[test]
    fn owned_collection_publication_moves_name_storage_and_preserves_snapshots() {
        for publish_to_owner in [false, true] {
            let mut owner = Value::array(vec![Value::array(vec![Value::Null])]);
            let mut value = payload(7);
            let Value::Struct { name, .. } = &value else { unreachable!() };
            assert_eq!(name.len(), name.capacity());
            let name_pointer = name.as_ptr();
            let pending =
                NestedPublication::prepare_owned("state".into(), owner.clone(), path(), &mut value)
                    .unwrap();
            assert!(matches!(value, Value::Null));
            let snapshot = pending.snapshot_value();
            let (_, published) = pending.publish(publish_to_owner.then_some(&mut owner));
            let Value::Array(rows) = &published else { unreachable!() };
            let Value::Array(row) = &rows[0] else { unreachable!() };
            let Value::Struct { name, .. } = &row[0] else { unreachable!() };
            assert_eq!(name.as_ptr(), name_pointer);
            assert!(Value::equals(&snapshot, &published));
            let Value::Array(rows) = &snapshot else { unreachable!() };
            let Value::Array(row) = &rows[0] else { unreachable!() };
            let Value::Struct { name, .. } = &row[0] else { unreachable!() };
            assert_ne!(name.as_ptr(), name_pointer);
        }
    }

    #[test]
    fn owned_collection_rejections_leave_input_intact_and_success_moves_replacement() {
        let owner = Value::array(vec![Value::array(vec![Value::Null, Value::Null])]);
        let mut value = payload(1);
        let invalid = vec![Access::Index(Value::Int(0)), Access::Index(Value::Bool(true))];
        assert!(NestedPublication::prepare_owned(
            "state".into(),
            owner.clone(),
            invalid,
            &mut value
        )
        .is_err());
        assert!(Value::equals(&value, &payload(1)));
        let mut pending =
            NestedPublication::prepare_owned("state".into(), owner.clone(), path(), &mut value)
                .unwrap();
        let saved = pending.snapshot_value();
        let mut next = payload(2);
        let Value::Struct { name, .. } = &next else { unreachable!() };
        let pointer = name.as_ptr();
        assert!(!pending.try_record_owned_with_batching(
            &[Access::Index(Value::Int(0)), Access::Index(Value::Int(1))],
            &mut next,
            0
        ));
        assert!(Value::equals(&next, &payload(2)));
        assert!(pending.try_record_owned_with_batching(&path(), &mut next, 0));
        assert!(matches!(next, Value::Null));
        let (_, published) = pending.publish(None);
        let Value::Array(rows) = &published else { unreachable!() };
        let Value::Array(row) = &rows[0] else { unreachable!() };
        let Value::Struct { name, .. } = &row[0] else { unreachable!() };
        assert_eq!(name.as_ptr(), pointer);
        let Value::Array(rows) = &saved else { unreachable!() };
        let Value::Array(row) = &rows[0] else { unreachable!() };
        assert!(Value::equals(&row[0], &payload(1)));
    }

    fn path() -> Vec<Access> {
        vec![Access::Index(Value::Int(0)), Access::Index(Value::Int(0))]
    }

    #[test]
    fn collection_replacement_reuses_path_and_owner_storage() {
        for dictionary in [false, true] {
            let leaf = if dictionary {
                Value::dict(
                    [(Arc::from("0"), Value::array(vec![Value::Int(1)]))].into_iter().collect(),
                )
            } else {
                Value::array(vec![Value::array(vec![Value::Int(1)])])
            };
            let mut owner = Value::array(vec![leaf]);
            let Value::Array(rows) = &owner else { unreachable!() };
            let pointer = Arc::as_ptr(rows);
            let row_pointer = match &rows[0] {
                Value::Array(v) => Arc::as_ptr(v) as *const (),
                Value::Dict(v) => Arc::as_ptr(v) as *const (),
                _ => unreachable!(),
            };
            let path = path();
            let path_pointer = path.as_ptr();
            let pending = NestedPublication::prepare(
                "state".into(),
                owner.clone(),
                path,
                &Value::array(vec![Value::Int(2)]),
            )
            .unwrap();
            assert_eq!(pending.path.as_ptr(), path_pointer);
            let (_, result) = pending.publish(Some(&mut owner));
            let Value::Array(rows) = &owner else { unreachable!() };
            assert_eq!(Arc::as_ptr(rows), pointer);
            let current_pointer = match &rows[0] {
                Value::Array(v) => Arc::as_ptr(v) as *const (),
                Value::Dict(v) => Arc::as_ptr(v) as *const (),
                _ => unreachable!(),
            };
            assert_eq!(row_pointer, current_pointer);
            assert!(Value::equals(&result, &owner));
        }
    }

    #[test]
    fn collection_replacement_preserves_snapshots_and_rejects_other_keys_atomically() {
        let mut owner = Value::array(vec![Value::array(vec![Value::Null, Value::Null])]);
        let mut pending = NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(),
            &Value::array(vec![Value::Int(2)]),
        )
        .unwrap();
        let snapshot = pending.snapshot_value();
        let alias_path = vec![Access::Index(Value::Int(-1)), Access::Index(Value::Int(-2))];
        let replacement = Value::dict([(Arc::from("value"), Value::Int(3))].into_iter().collect());
        assert!(pending.try_record(&alias_path, &replacement));
        assert!(!pending.try_record(
            &[Access::Index(Value::Int(0)), Access::Index(Value::Int(1))],
            &replacement
        ));
        assert!(!pending.try_record(&path(), &Value::Int(99)));
        assert!(Value::equals(&pending.read(RowKey::Integer(0), 0).unwrap(), &replacement));
        let (_, discarded) = pending.clone().discard();
        assert!(Value::equals(&discarded, &owner));
        let (_, published) = pending.publish(Some(&mut owner));
        assert!(Value::equals(&published, &owner));
        let Value::Array(rows) = snapshot else { unreachable!() };
        let Value::Array(row) = &rows[0] else { unreachable!() };
        let Value::Array(value) = &row[0] else { unreachable!() };
        assert!(matches!(value[0], Value::Int(2)));
        assert!(matches!(row[1], Value::Null));
    }

    #[test]
    fn deep_collection_replacement_requires_unique_intermediate_owners() {
        let mut owner = Value::array(vec![Value::array(vec![Value::array(vec![Value::Null])])]);
        let path = || {
            vec![
                Access::Index(Value::Int(0)),
                Access::Index(Value::Int(0)),
                Access::Index(Value::Int(0)),
            ]
        };
        let value = Value::array(vec![Value::Int(7)]);
        let Value::Array(rows) = &owner else { unreachable!() };
        let alias = rows[0].clone();
        assert!(NestedPublication::prepare("state".into(), owner.clone(), path(), &value).is_err());
        drop(alias);
        let pending =
            NestedPublication::prepare("state".into(), owner.clone(), path(), &value).unwrap();
        let snapshot = pending.snapshot_value();
        let (_, result) = pending.publish(Some(&mut owner));
        assert!(Value::equals(&snapshot, &result));
        assert!(Value::equals(&owner, &result));
    }
}

#[cfg(test)]
mod collection_dictionary_storage_tests {
    use super::*;

    fn root() -> Value {
        let leaf =
            Value::dict((0..128).map(|i| (Arc::from(i.to_string()), Value::Int(0))).collect());
        Value::dict([(Arc::from("inner"), leaf)].into_iter().collect())
    }

    fn leaf(root: &Value) -> &Arc<super::super::value::DictMap> {
        let Value::Dict(parent) = root else { panic!("dictionary root") };
        let Value::Dict(leaf) = &parent["inner"] else { panic!("dictionary leaf") };
        leaf
    }

    #[test]
    fn mixed_dictionary_storage_preserves_snapshots_and_moves_into_unique_leaf() {
        let mut root = root();
        let original = root.clone();
        let mut journal =
            CollectionDictionaryWrite::new(Selector::String(Arc::new("inner".into())), 3);
        let mut array = Value::array(vec![Value::Int(42)]);
        assert!(journal.record(&root, "0", &mut array));
        assert!(matches!(array, Value::Null));
        assert!(Arc::ptr_eq(
            journal.writes.get_key_value("0").unwrap().0,
            leaf(&root).get_key_value("0").unwrap().0
        ));
        let mut map = Value::dict([(Arc::from("v"), Value::Int(7))].into_iter().collect());
        assert!(journal.record(&root, "1", &mut map));
        assert!(matches!(map, Value::Null));
        let mut snapshot = root.clone();
        journal.apply(&mut snapshot);
        let mut replacement = Value::Bool(true);
        assert!(journal.record(&root, "0", &mut replacement));
        assert!(matches!(
            journal.read(&root, RowKey::String("inner"), "0"),
            Some(Value::Bool(true))
        ));
        assert!(journal.read(&root, RowKey::String("other"), "0").is_none());
        assert!(
            matches!(&leaf(&snapshot)["0"], Value::Array(items) if matches!(items[0], Value::Int(42)))
        );
        assert!(matches!(&leaf(&original)["0"], Value::Int(0)));
        drop(original);
        let leaf_address = Arc::as_ptr(leaf(&root));
        journal.apply_owned(&mut root);
        assert_eq!(Arc::as_ptr(leaf(&root)), leaf_address, "unique leaf must not detach");
        assert!(matches!(&leaf(&root)["0"], Value::Bool(true)));
        assert!(
            matches!(&leaf(&root)["1"], Value::Dict(items) if matches!(items["v"], Value::Int(7)))
        );
        assert!(matches!(&leaf(&root)["127"], Value::Int(0)));
        assert!(
            matches!(&leaf(&snapshot)["0"], Value::Array(items) if matches!(items[0], Value::Int(42)))
        );
    }

    #[test]
    fn dictionary_storage_bounds_distinct_keys_and_rewrites_full_tables() {
        let root = root();
        let selector = Selector::String(Arc::new("inner".into()));
        let upper_bound = CollectionDictionaryWrite::new(selector.clone(), u16::MAX);
        assert!(upper_bound.writes.capacity() <= 7, "do not reserve for a loop's write count");
        let mut journal = CollectionDictionaryWrite::new(selector, 7);
        for i in 0..7 {
            let mut value = Value::array(vec![Value::Int(i)]);
            assert!(journal.record(&root, &i.to_string(), &mut value));
        }
        let capacity = journal.writes.capacity();
        for i in 0..1024 {
            let mut value = Value::Int(i);
            assert!(journal.record(&root, "0", &mut value));
            assert_eq!(journal.writes.capacity(), capacity);
        }
        let mut rejected = Value::array(vec![Value::Int(99)]);
        assert!(!journal.record(&root, "new", &mut rejected));
        assert!(matches!(&rejected, Value::Array(items) if matches!(items[0], Value::Int(99))));
        let mut unsupported = Value::Error("unchanged".into());
        assert!(!journal.record(&root, "0", &mut unsupported));
        assert!(matches!(unsupported, Value::Error(message) if message=="unchanged"));
        assert!(matches!(
            journal.read(&root, RowKey::String("inner"), "0"),
            Some(Value::Int(1023))
        ));
        assert!(std::mem::size_of::<CollectionDictionaryWrite>() <= std::mem::size_of::<Value>());
    }
}

#[cfg(test)]
mod collection_dictionary_promotion_tests {
    use super::*;
    fn root() -> Value {
        Value::dict(
            [(
                Arc::from("inner"),
                Value::dict((0..128).map(|i| (Arc::from(i.to_string()), Value::Int(0))).collect()),
            )]
            .into_iter()
            .collect(),
        )
    }
    fn path(key: i64) -> Vec<Access> {
        vec![Access::Index(Value::Str(Arc::new("inner".into()))), Access::Index(Value::Int(key))]
    }
    fn leaf(root: &Value) -> &Arc<super::super::value::DictMap> {
        let Value::Dict(parent) = root else { panic!("root") };
        let Value::Dict(leaf) = &parent["inner"] else { panic!("leaf") };
        leaf
    }
    fn policy(capacity: u16) -> super::super::nested_spill_proof::SpillPolicy {
        super::super::nested_spill_proof::SpillPolicy {
            capacity,
            collection_rows: true,
            ..Default::default()
        }
    }
    #[test]
    fn dictionary_promotion_preserves_inline_many_and_collection_history() {
        for count in [1usize, 4, 6] {
            let mut owner = root();
            let pointer = Arc::as_ptr(leaf(&owner));
            let mut first =
                if count == 1 { Value::array(vec![Value::Int(10)]) } else { Value::Int(10) };
            let mut pending = NestedPublication::prepare_owned(
                "state".into(),
                owner.clone(),
                path(0),
                &mut first,
            )
            .unwrap();
            for key in 1..count {
                assert!(pending.try_record_with_batching(
                    &path(key as i64),
                    &Value::Int(10 + key as i64),
                    7
                ));
            }
            let before = pending.snapshot_value();
            let mut incoming = Value::array(vec![Value::Int(99)]);
            assert!(!pending.try_record_owned_with_policy(
                &path(count as i64),
                &mut incoming,
                policy(1)
            ));
            assert!(matches!(&incoming,Value::Array(items) if matches!(items[0],Value::Int(99))));
            assert!(pending.try_record_owned_with_policy(
                &path(count as i64),
                &mut incoming,
                policy((count + 1) as u16)
            ));
            assert!(matches!(incoming, Value::Null));
            let Some(Access::DeferredCollectionDictionary(batch)) = pending.path.last() else {
                panic!("batch not selected")
            };
            assert_eq!(batch.writes.len(), count + 1);
            let after = pending.snapshot_value();
            let mut replacement = Value::Bool(false);
            assert!(pending.try_record_owned_with_policy(
                &path(0),
                &mut replacement,
                policy((count + 1) as u16)
            ));
            let bad = vec![
                Access::Index(Value::Str(Arc::new("other".into()))),
                Access::Index(Value::Int(0)),
            ];
            let mut rejected = Value::array(vec![Value::Int(123)]);
            assert!(!pending.try_record_owned_with_policy(&bad, &mut rejected, policy(7)));
            assert!(matches!(&rejected,Value::Array(items) if matches!(items[0],Value::Int(123))));
            pending.publish(Some(&mut owner));
            assert_eq!(Arc::as_ptr(leaf(&owner)), pointer);
            assert!(matches!(&leaf(&owner)["0"], Value::Bool(false)));
            assert!(
                matches!(&leaf(&owner)[count.to_string().as_str()],Value::Array(items) if matches!(items[0],Value::Int(99)))
            );
            assert!(matches!(&leaf(&before)[count.to_string().as_str()], Value::Int(0)));
            if count == 1 {
                assert!(
                    matches!(&leaf(&after)["0"],Value::Array(items) if matches!(items[0],Value::Int(10)))
                );
            } else {
                for key in 0..count {
                    assert!(
                        matches!(&leaf(&after)[key.to_string().as_str()],Value::Int(n) if *n==10+key as i64)
                    );
                }
            }
        }
    }
    #[test]
    fn promotion_reservation_tracks_known_keys_not_logical_limit() {
        let root = root();
        for count in [2usize, 7, 8, 32, 64] {
            let mut batch = CollectionDictionaryWrite::with_known_keys(
                Selector::String(Arc::new("inner".into())),
                u16::MAX,
                count,
            );
            let reserved = batch.writes.capacity();
            assert!(reserved >= count && reserved < count * 2);
            for key in 0..count {
                let mut value = Value::array(vec![Value::Int(key as i64)]);
                assert!(batch.record(&root, &key.to_string(), &mut value));
                assert_eq!(batch.writes.capacity(), reserved);
            }
            for key in 0..count {
                let mut value = Value::Bool(true);
                assert!(batch.record(&root, &key.to_string(), &mut value));
                assert_eq!(batch.writes.capacity(), reserved);
            }
        }
    }

    #[test]
    fn first_collection_insertion_preserves_missing_keys_snapshots_and_owned_publication() {
        for deep in [false, true] {
            let mut owner = if deep {
                Value::dict([(Arc::from("outer"), root())].into_iter().collect())
            } else {
                root()
            };
            let make_path = |key: i64| {
                let mut p = path(key);
                if deep {
                    p.insert(0, Access::Index(Value::Str(Arc::new("outer".into()))));
                }
                p
            };
            let get_leaf = |value: &Value| -> Arc<super::super::value::DictMap> {
                let Value::Dict(map) = value else { panic!("root") };
                if deep {
                    leaf(&map["outer"]).clone()
                } else {
                    leaf(value).clone()
                }
            };
            let pointer = Arc::as_ptr(&get_leaf(&owner));
            let mut value = Value::array(vec![Value::Int(7)]);
            let original = owner.clone();
            assert!(NestedPublication::can_prepare_with_policy(
                &original,
                &make_path(200),
                &value,
                policy(2)
            ));
            let mut pending = NestedPublication::prepare_owned_with_policy(
                "state".into(),
                original,
                make_path(200),
                &mut value,
                policy(2),
            )
            .unwrap();
            assert!(matches!(value, Value::Null));
            let before = pending.snapshot_value();
            assert!(
                matches!(&get_leaf(&before)["200"],Value::Array(a) if matches!(a[0],Value::Int(7)))
            );
            assert!(!get_leaf(&owner).contains_key("200"));
            let mut second = Value::dict([(Arc::from("v"), Value::Int(8))].into_iter().collect());
            assert!(pending.try_record_owned_with_policy(&make_path(201), &mut second, policy(2)));
            assert!(matches!(second, Value::Null));
            let mut rejected = Value::array(vec![Value::Int(9)]);
            assert!(!pending.try_record_owned_with_policy(
                &make_path(202),
                &mut rejected,
                policy(2)
            ));
            assert!(matches!(&rejected,Value::Array(a) if matches!(a[0],Value::Int(9))));
            pending.publish(Some(&mut owner));
            assert_eq!(Arc::as_ptr(&get_leaf(&owner)), pointer);
            assert_eq!(get_leaf(&owner).len(), 130);
            assert!(!get_leaf(&before).contains_key("201"));
            assert!(!get_leaf(&owner).contains_key("202"));
        }
    }

    #[test]
    fn initial_text_and_bytes_reuse_keys_and_preserve_snapshots() {
        for mut value in [Value::Str(Arc::new("text".into())), Value::Bytes(Arc::new(vec![0, 255]))]
        {
            let mut owner = root();
            let canonical = leaf(&owner).get_key_value("0").unwrap().0.clone();
            let expected = value.clone();
            let no_insertion = super::super::nested_spill_proof::SpillPolicy {
                allow_initial_insertion: false,
                ..policy(2)
            };
            let mut pending = NestedPublication::prepare_owned_with_policy(
                "state".into(),
                owner.clone(),
                path(0),
                &mut value,
                no_insertion,
            )
            .unwrap();
            assert!(matches!(value, Value::Null));
            let Some(Access::DeferredCollectionDictionary(batch)) = pending.path.last() else {
                panic!("text and bytes must enter the bounded journal");
            };
            assert!(Arc::ptr_eq(batch.writes.get_key_value("0").unwrap().0, &canonical));
            let before = pending.snapshot_value();
            match (&leaf(&before)["0"], &expected) {
                (Value::Str(actual), Value::Str(expected)) => assert_eq!(actual, expected),
                (Value::Bytes(actual), Value::Bytes(expected)) => assert_eq!(actual, expected),
                _ => panic!("payload type changed"),
            }
            let mut next = Value::array(vec![Value::Int(9)]);
            assert!(pending.try_record_owned_with_policy(&path(1), &mut next, policy(2)));
            pending.publish(Some(&mut owner));
            assert!(matches!(&leaf(&owner)["1"], Value::Array(v) if matches!(v[0], Value::Int(9))));
            assert!(matches!(&leaf(&before)["1"], Value::Int(_)));
        }
    }

    #[test]
    fn initial_collection_insertion_requires_policy_unique_owners_and_budget() {
        let owner = root();
        let mut value = Value::array(vec![Value::Int(7)]);
        for blocked in [
            Default::default(),
            policy(1),
            super::super::nested_spill_proof::SpillPolicy {
                allow_initial_insertion: false,
                ..policy(2)
            },
            policy(u16::MAX),
        ] {
            assert!(!NestedPublication::can_prepare_with_policy(
                &owner.clone(),
                &path(200),
                &value,
                blocked
            ));
            assert!(NestedPublication::prepare_owned_with_policy(
                "state".into(),
                owner.clone(),
                path(200),
                &mut value,
                blocked
            )
            .is_err());
            assert!(matches!(&value,Value::Array(a) if matches!(a[0],Value::Int(7))));
        }
        let alias = leaf(&owner).clone();
        assert!(!NestedPublication::can_prepare_with_policy(
            &owner.clone(),
            &path(200),
            &value,
            policy(2)
        ));
        drop(alias);
        assert!(NestedPublication::can_prepare_with_policy(
            &owner.clone(),
            &path(200),
            &value,
            policy(2)
        ));
    }
    #[test]
    fn same_key_dictionary_changes_keep_compact_storage() {
        let owner = root();
        let mut initial = Value::array(vec![Value::Int(1)]);
        let mut pending =
            NestedPublication::prepare_owned("state".into(), owner.clone(), path(0), &mut initial)
                .unwrap();
        let mut scalar = Value::Int(7);
        assert!(pending.try_record_owned_with_policy(&path(0), &mut scalar, policy(2)));
        assert!(matches!(pending.path.last(), Some(Access::DeferredDictionary(_))));
        let mut array = Value::array(vec![Value::Int(8)]);
        assert!(pending.try_record_owned_with_policy(&path(0), &mut array, policy(2)));
        assert!(matches!(pending.path.last(), Some(Access::DeferredCollection(_))));
        assert!(matches!(array, Value::Null));
        let saved = pending.snapshot_value();
        assert!(
            matches!(&leaf(&saved)["0"],Value::Array(items) if matches!(items[0],Value::Int(8)))
        );
        assert!(matches!(&leaf(&owner)["0"], Value::Int(0)));
    }
}
