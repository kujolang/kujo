//! Private nested scalar writes retained in the already allocated assignment path.
use super::assignment::Access;
use super::dictionary_publication::with_integer_key;
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
/// The canonical key is shared with the map; admission never copies key bytes.
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
                map.insert(key.clone(), self.value(slot));
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
            map.insert(key.clone(), value.value());
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

impl NestedPublication {
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
    ) -> Option<(Arc<str>, Scalar)> {
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
            _ => None,
        }?;
        Some((key, value))
    }

    pub(super) fn can_prepare(root: &Value, path: &[Access], value: &Value) -> bool {
        Self::admission(root, path, value).is_some()
            || Self::dictionary_admission(root, path, value).is_some()
    }

    pub(super) fn prepare(
        name: String,
        root: Value,
        mut path: Vec<Access>,
        value: &Value,
    ) -> Result<Self, (String, Value, Vec<Access>)> {
        let dictionary = Self::dictionary_admission(&root, &path, value);
        let row = if dictionary.is_none() { Self::admission(&root, &path, value) } else { None };
        if dictionary.is_none() && row.is_none() {
            return Err((name, root, path));
        }
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
        let write = if let Some((key, value)) = dictionary {
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

    pub(super) fn try_record_with_batching(
        &mut self,
        path: &[Access],
        value: &Value,
        spill_hint: u16,
    ) -> bool {
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

    fn apply_path(prefix: &[Access], write: &Access, root: &mut Value) {
        if let Some((first, rest)) = prefix.split_first() {
            let child = match (first, root) {
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
            Self::apply_path(rest, write, child);
            return;
        }
        match write {
            Access::DeferredRow(write) => write.apply(root),
            Access::DeferredRowMany(write) => write.apply(root),
            Access::DeferredDictionary(write) => write.apply(root),
            Access::DeferredDictionaryMany(write) => write.apply(root),
            _ => unreachable!("nested publication write"),
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
                Self::apply_path(&self.path, &write, owner);
                return (self.name, owner.clone());
            }
        }
        Self::apply_path(&self.path, &write, &mut root);
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
        assert!(NestedPublication::prepare(
            "state".into(),
            owner.clone(),
            path(Value::str_ref("missing")),
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
