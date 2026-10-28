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

    fn admission(root: &Value, path: &[Access], value: &Value) -> Option<(usize, Scalar)> {
        let [first, Access::Index(Value::Int(index))] = path else { return None };
        let shared_only_with_owner = match root {
            Value::Struct { fields, .. } => Arc::strong_count(fields) == 2,
            Value::Dict(map) => Arc::strong_count(map) == 2,
            Value::Array(rows) => Arc::strong_count(rows) == 2,
            _ => false,
        };
        if !shared_only_with_owner {
            return None;
        }
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
        let [first, Access::Index(index)] = path else { return None };
        let owner_count = match root {
            Value::Struct { fields, .. } => Arc::strong_count(fields),
            Value::Dict(map) => Arc::strong_count(map),
            Value::Array(rows) => Arc::strong_count(rows),
            _ => return None,
        };
        if owner_count != 2 {
            return None;
        }
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
        if let Some((key, value)) = Self::dictionary_admission(&root, &path, value) {
            let selector = match std::mem::replace(&mut path[0], Access::Index(root)) {
                Access::Field(field) => Selector::Field(field),
                Access::Index(Value::Str(key)) => Selector::String(key),
                Access::Index(Value::Int(key)) => Selector::Integer(key),
                _ => unreachable!("admitted dictionary owner"),
            };
            path[1] = Access::DeferredDictionary(DictionaryWrite::new(selector, key, value));
            return Ok(Self { name, path });
        }
        let admission = Self::admission(&root, &path, value);

        let Some((index, value)) = admission else { return Err((name, root, path)) };
        let selector = match std::mem::replace(&mut path[0], Access::Index(root)) {
            Access::Field(field) => Selector::Field(field),
            Access::Index(Value::Str(key)) => Selector::String(key),
            Access::Index(Value::Int(key)) => Selector::Integer(key),
            _ => unreachable!("admitted selector"),
        };
        path[1] = Access::DeferredRow(RowWrite {
            selector,
            entries: [Some(RowEntry { index, value }), None, None, None],
        });
        Ok(Self { name, path })
    }

    fn parts(&self) -> (&Value, &RowWrite) {
        let [Access::Index(root), Access::DeferredRow(write)] = self.path.as_slice() else {
            unreachable!("nested publication storage")
        };
        (root, write)
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn read(&self, key: RowKey<'_>, index: i64) -> Option<Value> {
        if let [Access::Index(root), Access::DeferredDictionary(write)] = self.path.as_slice() {
            return with_integer_key(index, |index| write.read(root, key, index));
        }
        let (root, write) = self.parts();
        if !write.selector.key().same(key, root) {
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

    pub(super) fn read_string(&self, owner: RowKey<'_>, key: &str) -> Option<Value> {
        let [Access::Index(root), Access::DeferredDictionary(write)] = self.path.as_slice() else {
            return None;
        };
        write.read(root, owner, key)
    }

    pub(super) fn try_record(&mut self, path: &[Access], value: &Value) -> bool {
        if let [Access::Index(root), Access::DeferredDictionary(write)] = self.path.as_mut_slice() {
            let [first, Access::Index(index)] = path else { return false };
            let Some(owner) = RowKey::from_access(first) else { return false };
            let Some(value) = Scalar::from_value(value) else { return false };
            if !write.selector.key().same(owner, root) {
                return false;
            }
            return match index {
                Value::Str(key) => write.record(root, key, value),
                Value::Int(index) => with_integer_key(*index, |key| write.record(root, key, value)),
                _ => false,
            };
        }

        let [first, Access::Index(Value::Int(index))] = path else { return false };
        let Some(key) = RowKey::from_access(first) else { return false };
        let Some(value) = Scalar::from_value(value) else { return false };
        let (root, write) = self.parts();
        let row = write.selector.key().row(root).expect("validated row");
        if !write.selector.key().same(key, root) {
            return false;
        }
        let Some(index) = Self::resolve(*index, row.len()) else { return false };
        // Coalesce aliases before looking for an unused slot. A full journal
        // must still accept repeated writes; rejection leaves it untouched.
        let slot = write
            .entries
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.index == index))
            .or_else(|| write.entries.iter().position(Option::is_none));
        let Some(slot) = slot else { return false };
        let Access::DeferredRow(write) = &mut self.path[1] else { unreachable!() };
        write.entries[slot] = Some(RowEntry { index, value });
        true
    }

    pub(super) fn snapshot_value(&self) -> Value {
        if let [Access::Index(root), Access::DeferredDictionary(write)] = self.path.as_slice() {
            let mut root = root.clone();
            write.apply(&mut root);
            return root;
        }

        let (root, write) = self.parts();
        let mut root = root.clone();
        write.apply(&mut root);
        root
    }

    pub(super) fn publish(mut self, owner: Option<&mut Value>) -> (String, Value) {
        let write = self.path.pop().expect("nested write");
        let Some(Access::Index(mut root)) = self.path.pop() else { unreachable!() };
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
                Self::apply_write(&write, owner);
                return (self.name, owner.clone());
            }
        }
        Self::apply_write(&write, &mut root);
        (self.name, root)
    }

    fn apply_write(write: &Access, root: &mut Value) {
        match write {
            Access::DeferredRow(write) => write.apply(root),
            Access::DeferredDictionary(write) => write.apply(root),
            _ => unreachable!("nested publication write"),
        }
    }

    pub(super) fn discard(mut self) -> (String, Value) {
        self.path.pop();
        let Some(Access::Index(root)) = self.path.pop() else { unreachable!() };
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
