//! Private binding metadata: ordinary maps stay unchanged; oversized invocation
//! membership packs owned names into one slab and an indexed table.
use super::BindingKind;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
struct Entry {
    start: u32,
    len: u32,
    kind: BindingKind,
}
#[derive(Clone, Debug)]
pub(crate) struct Compact {
    names: Box<str>,
    entries: Box<[Entry]>,
    buckets: Box<[u32]>,
    hash_builder: ahash::RandomState,
}
impl Compact {
    fn hash(&self, name: &str) -> usize {
        self.hash_builder.hash_one(name) as usize
    }
    fn name(&self, e: &Entry) -> &str {
        &self.names[e.start as usize..e.start as usize + e.len as usize]
    }
    fn find_slot(&self, name: &str) -> Result<usize, usize> {
        let mut slot = self.hash(name) & (self.buckets.len() - 1);
        loop {
            let entry = self.buckets[slot];
            if entry == 0 {
                return Err(slot);
            }
            let index = entry as usize - 1;
            if self.name(&self.entries[index]) == name {
                return Ok(index);
            }
            slot = (slot + 1) & (self.buckets.len() - 1);
        }
    }
    fn find(&self, name: &str) -> Option<usize> {
        self.find_slot(name).ok()
    }
    fn build<'a, I>(source: I) -> Option<Self>
    where
        I: Iterator<Item = (&'a str, BindingKind)> + Clone,
    {
        let mut bytes = 0usize;
        let mut count = 0usize;
        for (name, _) in source.clone() {
            bytes = bytes.checked_add(name.len())?;
            count = count.checked_add(1)?;
        }
        if bytes > u32::MAX as usize || count >= u32::MAX as usize {
            return None;
        }
        let buckets = count.max(1).checked_mul(2)?.checked_next_power_of_two()?;
        let mut names = String::with_capacity(bytes);
        let mut entries: Vec<Entry> = Vec::with_capacity(count);
        let mut table = vec![0u32; buckets];
        let hash_builder = ahash::RandomState::new();
        for (name, kind) in source {
            let mut slot = hash_builder.hash_one(name) as usize & (table.len() - 1);
            loop {
                let entry = table[slot];
                if entry == 0 {
                    entries.push(Entry { start: names.len() as u32, len: name.len() as u32, kind });
                    names.push_str(name);
                    table[slot] = entries.len() as u32;
                    break;
                }
                let existing = &mut entries[entry as usize - 1];
                if &names[existing.start as usize..existing.start as usize + existing.len as usize]
                    == name
                {
                    existing.kind = kind;
                    break;
                }
                slot = (slot + 1) & (table.len() - 1);
            }
        }
        Some(Self {
            names: names.into_boxed_str(),
            entries: entries.into_boxed_slice(),
            buckets: table.into_boxed_slice(),
            hash_builder,
        })
    }
}
#[derive(Clone, Debug)]
pub(crate) enum BindingMap {
    Owned(HashMap<String, BindingKind>),
    Compact(Arc<Compact>),
}
impl Default for BindingMap {
    fn default() -> Self {
        Self::Owned(HashMap::new())
    }
}
#[derive(Clone)]
enum IterInner<'a> {
    Owned(std::collections::hash_map::Iter<'a, String, BindingKind>),
    Compact(&'a Compact, std::slice::Iter<'a, Entry>),
}
#[derive(Clone)]
pub(crate) struct Iter<'a>(IterInner<'a>);
impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, &'a BindingKind);
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterInner::Owned(i) => i.next().map(|(k, v)| (k.as_str(), v)),
            IterInner::Compact(c, i) => i.next().map(|e| (c.name(e), &e.kind)),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IterInner::Owned(i) => i.size_hint(),
            IterInner::Compact(_, i) => i.size_hint(),
        }
    }
}
impl ExactSizeIterator for Iter<'_> {}
impl BindingMap {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn compact_from<'a, I>(source: I) -> Option<Self>
    where
        I: Iterator<Item = (&'a str, BindingKind)> + Clone,
    {
        Compact::build(source).map(|c| Self::Compact(Arc::new(c)))
    }
    /// Validate exact membership, exploiting the unchanged owner's iteration order.
    /// On the first difference, validate the remaining names by ordinary lookup:
    /// equal lengths alone cannot detect host-side name replacement.
    pub(crate) fn matches_names<T>(&self, names: &HashMap<String, T>) -> bool {
        if names.len() != self.len() {
            return false;
        }
        let mut actual = names.keys();
        for expected in self.keys() {
            let name = actual.next().expect("equal binding and scope lengths");
            if name != expected {
                return self.contains_key(name) && actual.all(|name| self.contains_key(name));
            }
        }
        true
    }

    pub(crate) fn is_compact(&self) -> bool {
        matches!(self, Self::Compact(_))
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Owned(m) => m.len(),
            Self::Compact(c) => c.entries.len(),
        }
    }
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub(crate) fn capacity(&self) -> usize {
        match self {
            Self::Owned(m) => m.capacity(),
            Self::Compact(c) => c.buckets.len(),
        }
    }
    pub(crate) fn key_capacity_bytes(&self) -> usize {
        match self {
            Self::Owned(m) => m.keys().map(String::capacity).fold(0usize, usize::saturating_add),
            Self::Compact(c) => c.names.len(),
        }
    }
    pub(crate) fn iter(&self) -> Iter<'_> {
        match self {
            Self::Owned(m) => Iter(IterInner::Owned(m.iter())),
            Self::Compact(c) => Iter(IterInner::Compact(c, c.entries.iter())),
        }
    }
    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> + Clone {
        self.iter().map(|(k, _)| k)
    }
    pub(crate) fn values(&self) -> impl Iterator<Item = &BindingKind> + Clone {
        self.iter().map(|(_, v)| v)
    }
    pub(crate) fn get(&self, name: &str) -> Option<&BindingKind> {
        match self {
            Self::Owned(m) => m.get(name),
            Self::Compact(c) => c.find(name).map(|i| &c.entries[i].kind),
        }
    }
    pub(crate) fn get_mut(&mut self, name: &str) -> Option<&mut BindingKind> {
        match self {
            Self::Owned(m) => m.get_mut(name),
            Self::Compact(c) => {
                let index = c.find(name)?;
                Some(&mut Arc::make_mut(c).entries[index].kind)
            }
        }
    }
    pub(crate) fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
    fn materialize(&mut self) -> &mut HashMap<String, BindingKind> {
        if self.is_compact() {
            *self = Self::Owned(self.iter().map(|(k, v)| (k.to_owned(), *v)).collect());
        }
        let Self::Owned(m) = self else { unreachable!() };
        m
    }
    pub(crate) fn insert(&mut self, name: String, value: BindingKind) -> Option<BindingKind> {
        if let Self::Owned(map) = self {
            return map.insert(name, value);
        }
        if let Some(slot) = self.get_mut(&name) {
            return Some(std::mem::replace(slot, value));
        }
        self.materialize().insert(name, value)
    }
    pub(crate) fn remove(&mut self, name: &str) -> Option<BindingKind> {
        if let Self::Owned(map) = self {
            return map.remove(name);
        }
        if !self.contains_key(name) {
            return None;
        }
        self.materialize().remove(name)
    }
    pub(crate) fn reserve(&mut self, additional: usize) {
        if additional != 0 {
            self.materialize().reserve(additional);
        }
    }
    pub(crate) fn clear(&mut self) {
        match self {
            Self::Owned(m) => m.clear(),
            Self::Compact(_) => *self = Self::new(),
        }
    }
    pub(crate) fn retain(&mut self, mut predicate: impl FnMut(&str, &mut BindingKind) -> bool) {
        self.materialize().retain(|k, v| predicate(k, v));
    }
}
impl FromIterator<(String, BindingKind)> for BindingMap {
    fn from_iter<I: IntoIterator<Item = (String, BindingKind)>>(i: I) -> Self {
        Self::Owned(i.into_iter().collect())
    }
}
impl Extend<(String, BindingKind)> for BindingMap {
    fn extend<I: IntoIterator<Item = (String, BindingKind)>>(&mut self, i: I) {
        for (k, v) in i {
            self.insert(k, v);
        }
    }
}
impl std::ops::Index<&str> for BindingMap {
    type Output = BindingKind;
    fn index(&self, name: &str) -> &Self::Output {
        self.get(name).expect("missing binding kind")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_compact_metadata_detaches_only_for_existing_kind_mutation() {
        let mut original = BindingMap::compact_from(
            [("mutable", BindingKind::Mutable), ("fixed", BindingKind::Const)].into_iter(),
        )
        .unwrap();
        let mut snapshot = original.clone();
        let (BindingMap::Compact(a), BindingMap::Compact(b)) = (&original, &snapshot) else {
            panic!("expected compact metadata");
        };
        assert!(Arc::ptr_eq(a, b));
        assert!(snapshot.get_mut("missing").is_none());
        *original.get_mut("mutable").unwrap() = BindingKind::LetImmutable;
        assert_eq!(snapshot["mutable"], BindingKind::Mutable);
        assert_eq!(original["mutable"], BindingKind::LetImmutable);
        assert_eq!(snapshot["fixed"], BindingKind::Const);
        snapshot.remove("fixed");
        assert_eq!(original["fixed"], BindingKind::Const);
    }

    #[test]
    fn layout_and_pool_boundary() {
        use std::{
            collections::HashMap,
            mem::{align_of, size_of},
        };
        assert_eq!(size_of::<BindingMap>(), size_of::<HashMap<String, BindingKind>>());
        assert_eq!(align_of::<BindingMap>(), align_of::<HashMap<String, BindingKind>>());
        let names: Vec<_> = (0..8192).map(|i| format!("name-{i}")).collect();
        let map =
            BindingMap::compact_from(names.iter().map(|n| (n.as_str(), BindingKind::Mutable)))
                .unwrap();
        assert_eq!(map.len(), 8192);
        assert!(map.capacity() > 8192);
        assert!(map.key_capacity_bytes() > 0);
    }
    #[test]
    fn mutable_operations_match_owned_map_and_preserve_snapshots() {
        use std::collections::HashMap;
        for compact in [false, true] {
            let mut expected: HashMap<String, BindingKind> =
                (0..256).map(|i| (format!("key-{i}"), BindingKind::Mutable)).collect();
            let mut actual = if compact {
                BindingMap::compact_from(expected.iter().map(|(k, v)| (k.as_str(), *v))).unwrap()
            } else {
                expected.clone().into_iter().collect()
            };
            let snapshot = actual.clone();
            for i in 0..512 {
                let key = format!("key-{}", (i * 73) % 384);
                let kind = if i % 2 == 0 { BindingKind::Const } else { BindingKind::LetImmutable };
                match i % 3 {
                    0 => assert_eq!(actual.insert(key.clone(), kind), expected.insert(key, kind)),
                    1 => assert_eq!(actual.remove(&key), expected.remove(&key)),
                    _ => {
                        if let Some(v) = actual.get_mut(&key) {
                            *v = kind;
                        }
                        if let Some(v) = expected.get_mut(&key) {
                            *v = kind;
                        }
                    }
                }
                assert_eq!(actual.len(), expected.len());
                for (k, v) in &expected {
                    assert_eq!(actual.get(k), Some(v));
                }
                assert_eq!(actual.iter().count(), expected.len());
            }
            assert_eq!(snapshot.len(), 256);
            assert!(snapshot.values().all(|v| *v == BindingKind::Mutable));
            actual.retain(|k, v| {
                *v = BindingKind::Const;
                k.ends_with('0')
            });
            expected.retain(|k, v| {
                *v = BindingKind::Const;
                k.ends_with('0')
            });
            assert_eq!(actual.len(), expected.len());
            for (k, v) in &expected {
                assert_eq!(actual.get(k), Some(v));
            }
            actual.reserve(10);
            actual.extend([("extra".into(), BindingKind::Const)]);
            assert_eq!(actual["extra"], BindingKind::Const);
            actual.clear();
            assert!(actual.is_empty());
            assert_eq!(actual.keys().count(), 0);
        }
    }
    #[test]
    fn compact_kind_updates_do_not_materialize_and_names_are_owned() {
        let names = vec!["".to_owned(), "日本語".to_owned(), "alpha".to_owned()];
        let mut map =
            BindingMap::compact_from(names.iter().map(|s| (s.as_str(), BindingKind::Mutable)))
                .unwrap();
        drop(names);
        assert_eq!(map.insert("alpha".into(), BindingKind::Const), Some(BindingKind::Mutable));
        assert!(map.is_compact());
        assert_eq!(map.remove("absent"), None);
        map.reserve(0);
        assert!(map.is_compact());
        assert_eq!(map["日本語"], BindingKind::Mutable);
        assert_eq!(map[""], BindingKind::Mutable);
        let duplicate = BindingMap::compact_from(
            [("same", BindingKind::Mutable), ("same", BindingKind::Const)].into_iter(),
        )
        .unwrap();
        assert_eq!(duplicate.len(), 1);
        assert_eq!(duplicate["same"], BindingKind::Const);
        let empty = BindingMap::compact_from(std::iter::empty()).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.get("missing"), None);
    }
}

#[cfg(test)]
mod membership_tests {
    use super::*;

    #[test]
    fn exact_membership_survives_reordering_and_rejects_same_size_replacements() {
        let names: HashMap<String, usize> = (0..8192).map(|i| (format!("name_{i}"), i)).collect();
        let metadata =
            BindingMap::compact_from(names.keys().map(|key| (key.as_str(), BindingKind::Const)))
                .unwrap();
        assert!(metadata.matches_names(&names));
        let mut reversed: Vec<_> = names.keys().collect();
        reversed.reverse();
        let reordered =
            BindingMap::compact_from(reversed.iter().map(|key| (key.as_str(), BindingKind::Const)))
                .unwrap();
        assert!(reordered.matches_names(&names));
        let keys: Vec<_> = names.keys().cloned().collect();
        for key in [&keys[0], &keys[keys.len() / 2], &keys[keys.len() - 1]] {
            let mut changed = names.clone();
            changed.remove(key);
            changed.insert("host_replacement".into(), 0);
            assert!(!metadata.matches_names(&changed));
            assert!(!reordered.matches_names(&changed));
        }
        let mut changed_values = names.clone();
        changed_values.values_mut().for_each(|value| *value = 0);
        assert!(metadata.matches_names(&changed_values));
        changed_values.remove(&keys[0]);
        assert!(!metadata.matches_names(&changed_values));
        assert!(metadata.matches_names(&names));
        assert_eq!(metadata[&keys[0]], BindingKind::Const);
    }
}
