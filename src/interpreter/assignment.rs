//! Value-semantic write-back for a resolved, binding-rooted assignment path.
use super::Value;
use crate::ast::Expr;
use std::sync::Arc;

pub(crate) enum AccessExpr<'a> {
    Index(&'a Expr),
    Field(&'a str),
}

/// Resolve syntax without evaluating it, in root-to-leaf order.
pub(crate) fn split_target(mut target: &Expr) -> Result<(&str, Vec<AccessExpr<'_>>), String> {
    let mut path = Vec::new();
    loop {
        match target {
            Expr::IndexAccess { object, index } => {
                path.push(AccessExpr::Index(index));
                target = object;
            }
            Expr::FieldAccess { object, field } => {
                path.push(AccessExpr::Field(field));
                target = object;
            }
            Expr::Identifier(name) => {
                path.reverse();
                return Ok((name, path));
            }
            _ => return Err("Assignment path must be rooted in a binding".to_string()),
        }
    }
}

#[cfg_attr(test, derive(Clone, Debug))]
pub(crate) enum Access {
    Index(Value),
    Field(String),
    #[cfg(test)]
    DeferredRow(super::nested_publication::RowWrite),
}

// Borrow string keys and format integer keys on the stack. Only insertion of a
// previously absent key needs owned storage; traversing an assignment does not.
fn with_dictionary_key<T>(
    index: &Value,
    access: impl FnOnce(&str) -> Result<T, String>,
) -> Result<T, String> {
    match index {
        Value::Str(key) => access(key.as_str()),
        Value::Int(key) => super::dictionary_publication::with_integer_key(*key, access),
        _ => Err("Invalid index assignment".into()),
    }
}

/// Work on an owned snapshot: callers publish the root only after success.
/// Copy-on-write containers retain alias isolation, including on failed writes.
pub(crate) fn update(mut root: Value, path: &[Access], value: Value) -> Result<Value, String> {
    write(&mut root, path, value)?;
    Ok(root)
}

/// Validate the complete path before touching any container. Afterwards writes
/// cannot fail, so unique array rows can be edited without a rollback snapshot.
pub(crate) fn validate(root: &Value, path: &[Access], value: &Value) -> Result<(), String> {
    let Some((access, rest)) = path.split_first() else { return Ok(()) };
    match (root, access) {
        (Value::Array(items), Access::Index(Value::Int(index))) => {
            let resolved = if *index < 0 { items.len() as i64 + index } else { *index };
            let child = if resolved >= 0 { items.get(resolved as usize) } else { None };
            validate(child.ok_or_else(|| format!("Index out of bounds: {index}"))?, rest, value)
        }
        (Value::Bytes(bytes), Access::Index(index)) if rest.is_empty() => {
            validate_byte(bytes, index, value).map(|_| ())
        }
        (Value::Struct { fields, .. }, Access::Field(field)) => {
            if rest.is_empty() {
                Ok(())
            } else {
                validate(
                    fields.get(field).ok_or_else(|| format!("Field not found: {field}"))?,
                    rest,
                    value,
                )
            }
        }
        (Value::Dict(map), Access::Index(index)) => with_dictionary_key(index, |key| {
            if rest.is_empty() {
                Ok(())
            } else {
                validate(
                    map.get(key).ok_or_else(|| format!("Missing map key: {key}"))?,
                    rest,
                    value,
                )
            }
        }),
        (Value::FixedDict { .. }, _) => {
            let entries = Value::map_entries(root).expect("fixed dictionary");
            validate(
                &Value::Dict(Arc::new(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())),
                path,
                value,
            )
        }
        (_, _) if root.integer_dictionary_as_dict().is_some() => {
            validate(&root.integer_dictionary_as_dict().unwrap(), path, value)
        }
        (_, Access::Field(_)) => Err("Cannot set field on non-struct value".into()),
        _ => Err("Invalid index assignment".into()),
    }
}

pub(crate) fn update_in_place(
    root: &mut Value,
    path: &[Access],
    value: Value,
) -> Result<(), String> {
    validate(root, path, &value)?;
    write(root, path, value)
}

fn write(root: &mut Value, path: &[Access], value: Value) -> Result<(), String> {
    let Some((access, rest)) = path.split_first() else {
        *root = value;
        return Ok(());
    };

    // Specialized maps are a VM storage detail, not a different language value.
    if matches!(root, Value::FixedDict { .. }) {
        let entries = Value::map_entries(root).expect("fixed dictionary");
        *root = Value::Dict(Arc::new(entries.into_iter().map(|(k, v)| (k.into(), v)).collect()));
    } else if let Some(map) = root.integer_dictionary_as_dict() {
        *root = map;
    }

    match (root, access) {
        (Value::Bytes(bytes), Access::Index(index)) if rest.is_empty() => {
            write_byte(bytes, index, &value)
        }
        (Value::Array(items), Access::Index(Value::Int(index))) => {
            let resolved = if *index < 0 { items.len() as i64 + index } else { *index };
            if resolved < 0 || resolved as usize >= items.len() {
                return Err(format!("Index out of bounds: {index}"));
            }
            write(&mut Arc::make_mut(items)[resolved as usize], rest, value)
        }
        (Value::Struct { fields, .. }, Access::Field(field)) => {
            if rest.is_empty() {
                Arc::make_mut(fields).insert(field.clone(), value);
                Ok(())
            } else {
                let child = Arc::make_mut(fields)
                    .get_mut(field)
                    .ok_or_else(|| format!("Field not found: {field}"))?;
                write(child, rest, value)
            }
        }
        (Value::Dict(map), Access::Index(index)) => with_dictionary_key(index, |key| {
            let map = Arc::make_mut(map);
            if rest.is_empty() {
                if let Some(destination) = map.get_mut(key) {
                    *destination = value;
                } else {
                    map.insert(key.into(), value);
                }
                Ok(())
            } else {
                let child = map.get_mut(key).ok_or_else(|| format!("Missing map key: {key}"))?;
                write(child, rest, value)
            }
        }),
        (_, Access::Field(_)) => Err("Cannot set field on non-struct value".to_string()),
        _ => Err("Invalid index assignment".to_string()),
    }
}

/// Validate before copy-on-write so rejected writes never change storage or content.
pub(crate) fn write_byte(
    bytes: &mut Arc<Vec<u8>>,
    index: &Value,
    value: &Value,
) -> Result<(), String> {
    let (index, byte) = validate_byte(bytes, index, value)?;
    Arc::make_mut(bytes)[index] = byte;
    Ok(())
}

fn validate_byte(bytes: &[u8], index: &Value, value: &Value) -> Result<(usize, u8), String> {
    let Value::Int(index) = index else {
        return Err("Byte index assignment requires an integer index".into());
    };
    let resolved = if *index < 0 { bytes.len() as i64 + index } else { *index };
    if resolved < 0 || resolved as usize >= bytes.len() {
        return Err(format!("Index out of bounds: {index}"));
    }
    let Value::Int(byte @ 0..=255) = value else {
        return Err("Byte index assignment requires an integer in range 0-255".into());
    };
    Ok((resolved as usize, *byte as u8))
}

#[cfg(test)]
mod ownership_tests {
    use super::*;

    #[test]
    fn unique_nested_rows_are_not_copied_on_writes() {
        let mut root = Value::array(vec![Value::array(vec![Value::Int(0); 10_000])]);
        fn row_pointer(root: &Value) -> *const Vec<Value> {
            let Value::Array(outer) = root else { panic!() };
            let Value::Array(row) = &outer[0] else { panic!() };
            Arc::as_ptr(row)
        }
        let before = row_pointer(&root);
        let path = [Access::Index(Value::Int(0)), Access::Index(Value::Int(7))];
        for i in 0..100 {
            update_in_place(&mut root, &path, Value::Int(i)).unwrap();
        }
        assert_eq!(before, row_pointer(&root));
        let saved = root.clone();
        update_in_place(&mut root, &path, Value::Int(999)).unwrap();
        assert_ne!(row_pointer(&saved), row_pointer(&root));
        let detached = row_pointer(&root);
        update_in_place(&mut root, &path, Value::Int(1000)).unwrap();
        assert_eq!(detached, row_pointer(&root));
        let invalid = [Access::Index(Value::Int(0)), Access::Index(Value::Int(10_000))];
        assert!(update_in_place(&mut root, &invalid, Value::Int(1)).is_err());
        assert_eq!(detached, row_pointer(&root));
    }
}

/// Release the already-evaluated first argument only after proving that the
/// destination still holds that exact value. Other aliases continue to force COW.
pub(crate) fn append_rebinding(
    root: &mut Value,
    original: &mut Option<Value>,
    item: &mut Option<Value>,
) -> bool {
    let Value::Array(destination) = root else { return false };
    let Some(Value::Array(source)) = original.as_ref() else { return false };
    if !Arc::ptr_eq(destination, source) {
        return false;
    }
    drop(original.take());
    Arc::make_mut(destination).push(item.take().expect("evaluated append item"));
    true
}

#[cfg(test)]
mod append_ownership_tests {
    use super::*;
    #[test]
    fn rebinding_releases_only_its_temporary_argument_snapshot() {
        let mut items = Vec::with_capacity(128);
        items.push(Value::Int(0));
        let mut root = Value::array(items);
        let Value::Array(storage) = &root else { unreachable!() };
        let pointer = Arc::as_ptr(storage);
        for i in 1..100 {
            let mut original = Some(root.clone());
            assert!(append_rebinding(&mut root, &mut original, &mut Some(Value::Int(i))));
            assert!(original.is_none());
        }
        let Value::Array(storage) = &root else { unreachable!() };
        assert_eq!(pointer, Arc::as_ptr(storage));
        let saved = root.clone();
        let mut original = Some(root.clone());
        assert!(append_rebinding(&mut root, &mut original, &mut Some(Value::Int(100))));
        let Value::Array(storage) = &root else { unreachable!() };
        assert_ne!(pointer, Arc::as_ptr(storage));
        let Value::Array(old) = saved else { unreachable!() };
        assert_eq!(old.len(), 100);
    }
    #[test]
    fn replaced_destination_preserves_both_call_arguments_for_fallback() {
        let mut root = Value::array(vec![Value::Int(9)]);
        let mut original = Some(Value::array(vec![Value::Int(1)]));
        let mut item = Some(Value::Int(2));
        assert!(!append_rebinding(&mut root, &mut original, &mut item));
        assert!(original.is_some() && item.is_some());
        let Value::Array(items) = root else { unreachable!() };
        assert!(matches!(items[0], Value::Int(9)));
    }
}

// Weak identities invalidate on every Arc::make_mut, including writes outside
// this module. Bounds may overestimate depth after deletion; only the exact
// fallback below can reject a write. No cached entry retains a collection payload.
#[derive(Default)]
pub(crate) struct DepthCache {
    structs: std::collections::HashMap<
        usize,
        (std::sync::Weak<std::collections::HashMap<String, Value>>, usize),
    >,
    arrays: std::collections::HashMap<usize, (std::sync::Weak<Vec<Value>>, usize)>,
    dictionaries: std::collections::HashMap<usize, (std::sync::Weak<super::DictMap>, usize)>,
    integer_dictionaries:
        std::collections::HashMap<usize, (std::sync::Weak<super::IntDictMap>, usize)>,
    #[cfg(test)]
    scans: usize,
}
impl DepthCache {
    #[cfg(test)]
    pub(crate) fn scan_count(&self) -> usize {
        self.scans
    }

    // A Weak reference makes Arc::make_mut allocate even with one strong owner.
    // Discard only the identities about to be mutated, after reading their depth
    // bounds. Other cached identities retain automatic external-write invalidation.
    fn release_path(&mut self, root: &Value, path: &[Access]) {
        let mut cursor = root;
        for index in 0..=path.len() {
            match cursor {
                Value::Struct { fields, .. } => {
                    self.structs.remove(&(Arc::as_ptr(fields) as usize));
                }
                Value::Array(items)
                | Value::DenseIntDict(items)
                | Value::FixedDict { values: items, .. } => {
                    self.arrays.remove(&(Arc::as_ptr(items) as usize));
                }
                Value::Dict(items) => {
                    self.dictionaries.remove(&(Arc::as_ptr(items) as usize));
                }
                Value::IntDict(items) => {
                    self.integer_dictionaries.remove(&(Arc::as_ptr(items) as usize));
                }
                _ => {}
            }
            let Some(access) = path.get(index) else { break };
            let Some(child) = collection_child(cursor, access) else { break };
            cursor = child;
        }
    }

    fn remember(&mut self, value: &Value, bound: usize) {
        match value {
            Value::Struct { fields, .. } => {
                self.structs.insert(Arc::as_ptr(fields) as usize, (Arc::downgrade(fields), bound));
            }
            Value::Array(items)
            | Value::DenseIntDict(items)
            | Value::FixedDict { values: items, .. } => {
                self.arrays.insert(Arc::as_ptr(items) as usize, (Arc::downgrade(items), bound));
            }
            Value::Dict(items) => {
                self.dictionaries
                    .insert(Arc::as_ptr(items) as usize, (Arc::downgrade(items), bound));
            }
            Value::IntDict(items) => {
                self.integer_dictionaries
                    .insert(Arc::as_ptr(items) as usize, (Arc::downgrade(items), bound));
            }
            _ => {}
        }
    }
    fn cached_depth(&self, root: &Value) -> Option<usize> {
        match root {
            Value::Struct { fields, .. } => {
                self.structs.get(&(Arc::as_ptr(fields) as usize)).map(|e| e.1)
            }
            Value::Array(items)
            | Value::DenseIntDict(items)
            | Value::FixedDict { values: items, .. } => {
                self.arrays.get(&(Arc::as_ptr(items) as usize)).map(|e| e.1)
            }
            Value::Dict(items) => {
                self.dictionaries.get(&(Arc::as_ptr(items) as usize)).map(|e| e.1)
            }
            Value::IntDict(items) => {
                self.integer_dictionaries.get(&(Arc::as_ptr(items) as usize)).map(|e| e.1)
            }
            _ => None,
        }
    }
    fn depth(&mut self, root: &Value) -> usize {
        if let Some(bound) = self.cached_depth(root) {
            return bound;
        }
        if !matches!(root, Value::FixedDict { .. }) {
            #[cfg(test)]
            {
                self.scans += 1;
            }
            let bound = Value::nesting_depth(root);
            self.remember(root, bound);
            return bound;
        }
        // Fresh fixed wrappers must still reuse validated child depths. Shared
        // fixed subgraphs now need per-walk identities to avoid DAG expansion.
        let mut work = vec![(root, 0usize)];
        let mut seen = std::collections::HashMap::<usize, usize>::new();
        let mut depth = 0;
        while let Some((value, offset)) = work.pop() {
            depth = depth.max(offset);
            if let Some(bound) = self.cached_depth(value) {
                depth = depth.max(offset + bound);
            } else if let Value::FixedDict { values, .. } = value {
                let identity = Arc::as_ptr(values) as usize;
                if seen.get(&identity).is_some_and(|previous| *previous >= offset) {
                    continue;
                }
                seen.insert(identity, offset);
                work.extend(values.iter().map(|child| (child, offset + 1)));
            } else {
                #[cfg(test)]
                {
                    self.scans += 1;
                }
                let bound = Value::nesting_depth(value);
                self.remember(value, bound);
                depth = depth.max(offset + bound);
            }
        }
        self.remember(root, depth);
        depth
    }
    fn remember_path(&mut self, root: &Value, path: &[Access], bound: usize) {
        let mut cursor = root;
        let mut bound = bound;
        self.remember(cursor, bound);
        for access in path {
            let Some(child) = collection_child(cursor, access) else { break };
            cursor = child;
            bound = bound.saturating_sub(1);
            self.remember(cursor, bound);
        }
        if self.structs.len()
            + self.arrays.len()
            + self.dictionaries.len()
            + self.integer_dictionaries.len()
            >= 1024
        {
            self.structs.retain(|_, (weak, _)| weak.strong_count() > 0);
            self.arrays.retain(|_, (weak, _)| weak.strong_count() > 0);
            self.dictionaries.retain(|_, (weak, _)| weak.strong_count() > 0);
            self.integer_dictionaries.retain(|_, (weak, _)| weak.strong_count() > 0);
            if self.structs.len()
                + self.arrays.len()
                + self.dictionaries.len()
                + self.integer_dictionaries.len()
                >= 1024
            {
                self.structs.clear();
                self.arrays.clear();
                self.dictionaries.clear();
                self.integer_dictionaries.clear();
            }
        }
    }
}

fn child<'a>(value: &'a Value, access: &Access) -> Option<&'a Value> {
    match (value, access) {
        (Value::Array(a), Access::Index(Value::Int(i))) => {
            let i = if *i < 0 { a.len() as i64 + i } else { *i };
            if i < 0 {
                None
            } else {
                a.get(i as usize)
            }
        }
        (Value::Struct { fields, .. }, Access::Field(field)) => fields.get(field),
        (Value::Dict(d), Access::Index(Value::Str(key))) => d.get(key.as_str()),
        (Value::Dict(d), Access::Index(Value::Int(key))) => d.get(key.to_string().as_str()),
        _ => None,
    }
}

pub(crate) fn update_checked(
    root: &mut Value,
    path: &[Access],
    value: Value,
    cache: &mut DepthCache,
) -> Result<(), String> {
    validate(root, path, &value)?;
    let bound = cache.depth(root).max(Value::nesting_depth(&value) + path.len());
    let limit = crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH;
    let bound = if bound <= limit {
        cache.release_path(root, path);
        update_in_place(root, path, value)?;
        bound
    } else {
        // Host/native inputs can already be too deep. Check the proposed result
        // exactly, allowing a write that repairs them and never partially writing.
        let updated = update(root.clone(), path, value)?;
        let exact = Value::nesting_depth(&updated);
        if exact > limit {
            return Err(format!(
                "Maximum data nesting depth of {limit} exceeded while updating nested assignment"
            ));
        }
        *root = updated;
        exact
    };
    cache.remember_path(root, path, bound);
    Ok(())
}

#[cfg(test)]
mod state_depth_tests {
    use super::*;
    #[test]
    fn cached_unique_assignment_preserves_arc_allocations() {
        let mut root = Value::array(vec![Value::array(vec![Value::Int(0); 100])]);
        let path = [Access::Index(Value::Int(0)), Access::Index(Value::Int(-1))];
        let identities = |root: &Value| {
            let Value::Array(outer) = root else { panic!() };
            let Value::Array(inner) = &outer[0] else { panic!() };
            (Arc::as_ptr(outer), Arc::as_ptr(inner))
        };
        let mut cache = DepthCache::default();
        update_checked(&mut root, &path, Value::Int(1), &mut cache).unwrap();
        let before = identities(&root);
        for value in 2..102 {
            update_checked(&mut root, &path, Value::Int(value), &mut cache).unwrap();
            assert_eq!(identities(&root), before, "unique assignment allocated a new Arc");
        }
    }

    #[test]
    fn released_depth_identities_still_detect_external_growth_and_allow_repair() {
        // Match the existing over-depth fixture's stack allowance for teardown
        // of native-created values deeper than the language permits.
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let mut root = Value::array(vec![Value::Int(0)]);
                let path = [Access::Index(Value::Int(0))];
                let mut cache = DepthCache::default();
                update_checked(&mut root, &path, Value::Int(1), &mut cache).unwrap();
                let mut deep = Value::Int(0);
                for _ in 0..crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH {
                    deep = Value::array(vec![deep]);
                }
                // Model a native write outside this cache: make_mut must still invalidate
                // the remembered root identity, otherwise the next write accepts it.
                let Value::Array(items) = &mut root else { panic!() };
                Arc::make_mut(items).push(deep);
                let snapshot = root.clone();
                assert!(update_checked(&mut root, &path, Value::Int(2), &mut cache).is_err());
                let (Value::Array(current), Value::Array(saved)) = (&root, &snapshot) else {
                    panic!()
                };
                assert!(Arc::ptr_eq(current, saved), "rejected write changed the root");
                update_checked(
                    &mut root,
                    &[Access::Index(Value::Int(1))],
                    Value::Int(3),
                    &mut cache,
                )
                .unwrap();
                assert!(Value::equals(&root, &Value::array(vec![Value::Int(1), Value::Int(3)])));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn state_container_writes_reuse_rows_and_cached_depths() {
        for dictionary in [false, true] {
            let rows = Value::array(vec![Value::array(vec![Value::Int(0); 10_000])]);
            let mut root = if dictionary {
                Value::Dict(Arc::new([("rows".into(), rows)].into_iter().collect()))
            } else {
                Value::Struct {
                    name: "State".into(),
                    fields: [("rows".into(), rows)]
                        .into_iter()
                        .collect::<std::collections::HashMap<_, _>>()
                        .into(),
                }
            };
            let path = [
                if dictionary {
                    Access::Index(Value::str_ref("rows"))
                } else {
                    Access::Field("rows".into())
                },
                Access::Index(Value::Int(0)),
                Access::Index(Value::Int(0)),
            ];
            let buffer = |root: &Value| {
                let rows = child(root, &path[0]).unwrap();
                let Value::Array(row) = child(rows, &path[1]).unwrap() else { panic!() };
                row.as_ptr()
            };
            let mut cache = DepthCache::default();
            update_checked(&mut root, &path, Value::Int(1), &mut cache).unwrap();
            let ptr = buffer(&root);
            let scans = cache.scans;
            for _ in 0..100 {
                update_checked(&mut root, &path, Value::Int(2), &mut cache).unwrap();
                assert_eq!(buffer(&root), ptr);
            }
            assert_eq!(cache.scans, scans, "unchanged collection was rescanned");
            let alias = root.clone();
            update_checked(&mut root, &path, Value::Int(3), &mut cache).unwrap();
            assert_ne!(buffer(&alias), buffer(&root));
        }
    }
}

fn child_mut<'a>(root: &'a mut Value, access: &Access) -> Option<&'a mut Value> {
    if matches!(root, Value::FixedDict { .. }) {
        let entries = Value::map_entries(root)?;
        *root = Value::Dict(Arc::new(entries.into_iter().map(|(k, v)| (k.into(), v)).collect()));
    } else if let Some(map) = root.integer_dictionary_as_dict() {
        *root = map;
    }
    match (root, access) {
        (Value::Array(a), Access::Index(Value::Int(i))) => {
            let i = if *i < 0 { a.len() as i64 + i } else { *i };
            if i < 0 {
                None
            } else {
                Arc::make_mut(a).get_mut(i as usize)
            }
        }
        (Value::Struct { fields, .. }, Access::Field(field)) => {
            Arc::make_mut(fields).get_mut(field)
        }
        (Value::Dict(d), Access::Index(Value::Str(key))) => Arc::make_mut(d).get_mut(key.as_str()),
        (Value::Dict(d), Access::Index(Value::Int(key))) => {
            Arc::make_mut(d).get_mut(key.to_string().as_str())
        }
        _ => None,
    }
}

pub(crate) fn append_path(
    root: &mut Value,
    path: &[Access],
    original: Value,
    item: Value,
    mut cache: Option<&mut DepthCache>,
) -> Result<(), String> {
    // Single dictionary field assignments predate nested assignment and accept
    // dot syntax. Preserve that existing surface without broadening nested paths.
    let normalized;
    let path = if path.len() == 1 && !matches!(root, Value::Struct { .. }) {
        if let Access::Field(field) = &path[0] {
            normalized = vec![Access::Index(Value::str_ref(field))];
            &normalized[..]
        } else {
            path
        }
    } else {
        path
    };
    validate(root, path, &original)?;
    let bound = cache.as_mut().map(|cache| {
        cache
            .depth(root)
            .max(path.len() + cache.depth(&original).max(1 + Value::nesting_depth(&item)))
    });
    let mut original = Some(original);
    let mut item = Some(item);
    let mut destination = &*root;
    let mut same = true;
    for access in path {
        match child(destination, access) {
            Some(value) => destination = value,
            None => {
                same = false;
                break;
            }
        }
    }
    same &= matches!((destination, original.as_ref()), (Value::Array(a), Some(Value::Array(b))) if Arc::ptr_eq(a,b));
    if same && bound.is_none_or(|n| n <= crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH) {
        // Drop only the cache weak identities on the mutation path. Otherwise
        // Arc::make_mut relocates unique nodes even without a retained value alias.
        if let Some(cache) = cache.as_mut() {
            cache.release_path(root, path);
        }
        let mut destination = &mut *root;
        for access in path {
            destination = child_mut(destination, access).expect("validated append path");
        }
        // Parent detachment may retain the source through a real alias. Only
        // release our temporary argument; ordinary COW handles all other owners.
        assert!(append_rebinding(destination, &mut original, &mut item));
        if let Some(cache) = cache {
            cache.remember_path(root, path, bound.unwrap());
        }
        Ok(())
    } else {
        let Value::Array(mut array) = original.take().unwrap() else {
            unreachable!("guarded native array append")
        };
        Arc::make_mut(&mut array).push(item.take().unwrap());
        match cache {
            Some(cache) => update_checked(root, path, Value::Array(array), cache),
            None => update_in_place(root, path, Value::Array(array)),
        }
    }
}

#[cfg(test)]
mod field_append_tests {
    use super::*;
    #[test]
    fn field_append_preserves_reserved_buffer_and_snapshot() {
        for checked in [false, true] {
            let array = Value::array(Vec::with_capacity(1000));
            let mut root = Value::Struct {
                name: "S".into(),
                fields: [("items".into(), array)]
                    .into_iter()
                    .collect::<std::collections::HashMap<_, _>>()
                    .into(),
            };
            let path = [Access::Field("items".into())];
            let buffer = |v: &Value| {
                let Value::Array(a) = child(v, &path[0]).unwrap() else { panic!() };
                a.as_ptr()
            };
            let ptr = buffer(&root);
            let mut cache = DepthCache::default();
            for i in 0..500 {
                let original = child(&root, &path[0]).unwrap().clone();
                append_path(
                    &mut root,
                    &path,
                    original,
                    Value::Int(i),
                    if checked { Some(&mut cache) } else { None },
                )
                .unwrap();
                assert_eq!(ptr, buffer(&root));
            }
            let alias = root.clone();
            let original = child(&root, &path[0]).unwrap().clone();
            append_path(&mut root, &path, original, Value::Int(501), None).unwrap();
            assert_ne!(buffer(&alias), buffer(&root));
        }
    }
}

/// Recognize adjacent `let pair = f(collection, ...); collection = pair[0]`. There is no
/// observable execution between these statements; all other uses keep ordinary
/// functional pop/remove semantics. The runtime verifies the actual native callee.
pub(crate) fn pop_rebind_pair<'a>(
    stmt: &'a crate::ast::Stmt,
    next: Option<&crate::ast::Stmt>,
) -> Option<(&'a str, &'a str, &'a Expr, &'a [Expr], bool)> {
    use crate::ast::{Pattern, Stmt};
    let Stmt::Let {
        pattern: Pattern::Identifier(pair),
        value: Expr::Call { function, args },
        mutable,
        ..
    } = stmt
    else {
        return None;
    };
    if !(1..=2).contains(&args.len()) || !matches!(function.as_ref(), Expr::Identifier(_)) {
        return None;
    }
    let (root, _) = split_target(&args[0]).ok()?;
    let Some(Stmt::Assign { target, value: Expr::IndexAccess { object, index } }) = next else {
        return None;
    };
    if root != pair
        && same_stable_place(&args[0], target, pair)
        && matches!(object.as_ref(), Expr::Identifier(name) if name == pair)
        && matches!(index.as_ref(), Expr::Int(0))
    {
        Some((root, pair, function, args, *mutable))
    } else {
        None
    }
}

// Only paths whose re-evaluation cannot run user code are fused. The pair's
// declaration must not shadow an index binding needed by the following write.
fn same_stable_place(a: &Expr, b: &Expr, pair: &str) -> bool {
    match (a, b) {
        (Expr::Identifier(a), Expr::Identifier(b)) => a == b && a != pair,
        (Expr::Int(a), Expr::Int(b)) => a == b,
        (Expr::String(a), Expr::String(b)) => a == b,
        (
            Expr::FieldAccess { object: a, field: af },
            Expr::FieldAccess { object: b, field: bf },
        ) => af == bf && same_stable_place(a, b, pair),
        (
            Expr::IndexAccess { object: a, index: ai },
            Expr::IndexAccess { object: b, index: bi },
        ) => same_stable_place(a, b, pair) && same_stable_index(ai, bi, pair),
        _ => false,
    }
}
fn same_stable_index(a: &Expr, b: &Expr, pair: &str) -> bool {
    match (a, b) {
        (Expr::Identifier(_), Expr::Identifier(_))
        | (Expr::Int(_), Expr::Int(_))
        | (Expr::String(_), Expr::String(_)) => same_stable_place(a, b, pair),
        (Expr::UnaryOp { op: aop, operand: a }, Expr::UnaryOp { op: bop, operand: b })
            if aop == "-"
                && bop == "-"
                && matches!(a.as_ref(), Expr::Int(_))
                && matches!(b.as_ref(), Expr::Int(_)) =>
        {
            same_stable_place(a, b, pair)
        }
        _ => false,
    }
}

fn collection_index(access: &Access) -> Option<i64> {
    match access {
        Access::Index(Value::Int(i)) => Some(*i),
        Access::Index(Value::Str(s)) => Value::dict_index(s).ok(),
        _ => None,
    }
}
fn fixed_collection_index(keys: &[Arc<str>], access: &Access) -> Option<usize> {
    let key = match access {
        Access::Index(Value::Str(s)) => s.as_ref().clone(),
        Access::Index(Value::Int(i)) => i.to_string(),
        _ => return None,
    };
    keys.iter().position(|k| k.as_ref() == key)
}
fn collection_child<'a>(root: &'a Value, access: &Access) -> Option<&'a Value> {
    match root {
        Value::FixedDict { keys, values } => values.get(fixed_collection_index(keys, access)?),
        Value::IntDict(map) => map.get(&collection_index(access)?),
        Value::DenseIntDict(values) => values.get(usize::try_from(collection_index(access)?).ok()?),
        _ => child(root, access),
    }
}
fn collection_child_mut<'a>(root: &'a mut Value, access: &Access) -> Option<&'a mut Value> {
    match root {
        Value::FixedDict { keys, values } => {
            Arc::make_mut(values).get_mut(fixed_collection_index(keys, access)?)
        }
        Value::IntDict(map) => Arc::make_mut(map).get_mut(&collection_index(access)?),
        Value::DenseIntDict(values) => {
            Arc::make_mut(values).get_mut(usize::try_from(collection_index(access)?).ok()?)
        }
        _ => child_mut(root, access),
    }
}

pub(crate) fn collection_rebinding_path_checked(
    root: &mut Value,
    path: &[Access],
    original: &mut Option<Value>,
    key: Option<&Value>,
    cache: &mut DepthCache,
) -> Option<Value> {
    let bound = cache.depth(root);
    if bound > crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH {
        return None;
    }
    // Cache weak references must not force Arc relocation along an otherwise
    // unique path. Dropping these observations does not change fallback values.
    cache.release_path(root, path);
    let pair = collection_rebinding_path(root, path, original, key)?;
    // Shrinking cannot increase the validated bound. Real aliases still detach;
    // refresh identities so the next operation does not rescan payloads.
    cache.remember_path(root, path, bound);
    Some(pair)
}

pub(crate) fn collection_rebinding_path(
    root: &mut Value,
    path: &[Access],
    original: &mut Option<Value>,
    key: Option<&Value>,
) -> Option<Value> {
    let mut destination = &*root;
    for access in path {
        destination = collection_child(destination, access)?;
    }
    let same = match (destination, original.as_ref()?, key) {
        (Value::Array(a), Value::Array(b), None) => Arc::ptr_eq(a, b),
        (Value::Dict(a), Value::Dict(b), Some(Value::Str(_) | Value::Int(_))) => Arc::ptr_eq(a, b),
        (Value::IntDict(a), Value::IntDict(b), Some(_)) => Arc::ptr_eq(a, b),
        _ => false,
    };
    if !same {
        return None;
    }
    // Detach retained parent snapshots before mutating the leaf. Releasing only
    // the call's argument leaves all real aliases visible to copy-on-write.
    let mut destination = root;
    for access in path {
        destination = collection_child_mut(destination, access).expect("validated collection path");
    }
    if let Some(key) = key {
        remove_rebinding(destination, original, key)
    } else {
        pop_rebinding(destination, original)
    }
}

pub(crate) fn pop_rebinding(root: &mut Value, original: &mut Option<Value>) -> Option<Value> {
    let Value::Array(destination) = root else { return None };
    let Some(Value::Array(source)) = original.as_ref() else { return None };
    if !Arc::ptr_eq(destination, source) {
        return None;
    }
    drop(original.take());
    let popped = Arc::make_mut(destination).pop().unwrap_or(Value::Int(0));
    Some(Value::array(vec![Value::Array(destination.clone()), popped]))
}

pub(crate) fn remove_rebinding(
    root: &mut Value,
    original: &mut Option<Value>,
    key: &Value,
) -> Option<Value> {
    let removed = match (root, original.as_ref()?, key) {
        (Value::Dict(destination), Value::Dict(source), key)
            if Arc::ptr_eq(destination, source) =>
        {
            let key = match key {
                Value::Str(s) => s.as_ref().clone(),
                Value::Int(i) => i.to_string(),
                _ => return None,
            };
            drop(original.take());
            let removed = Arc::make_mut(destination).remove(key.as_str()).unwrap_or(Value::Int(0));
            (Value::Dict(destination.clone()), removed)
        }
        (Value::IntDict(destination), Value::IntDict(source), key)
            if Arc::ptr_eq(destination, source) =>
        {
            let key = match key {
                Value::Int(i) => Some(*i),
                Value::Str(s) => Value::dict_index(s).ok(),
                _ => None,
            };
            drop(original.take());
            let removed =
                key.and_then(|k| Arc::make_mut(destination).remove(&k)).unwrap_or(Value::Int(0));
            (Value::IntDict(destination.clone()), removed)
        }
        _ => return None,
    };
    Some(Value::array(vec![removed.0, removed.1]))
}

#[cfg(test)]
mod pop_tests {
    use super::*;
    #[test]
    fn pop_rebinding_reuses_unique_buffer_and_detaches_aliases() {
        let mut root = Value::array((0..10_000).map(Value::Int).collect());
        let buffer = |v: &Value| {
            let Value::Array(a) = v else { panic!() };
            a.as_ptr()
        };
        let ptr = buffer(&root);
        for _ in 0..100 {
            let mut argument = Some(root.clone());
            let pair = pop_rebinding(&mut root, &mut argument).unwrap();
            assert_eq!(buffer(&root), ptr);
            drop(pair);
        }
        let alias = root.clone();
        let mut argument = Some(root.clone());
        pop_rebinding(&mut root, &mut argument).unwrap();
        assert_ne!(buffer(&alias), buffer(&root));
    }
}

/// Only storage-backed values can be released by allocation identity. Retained
/// aliases keep their own strong references and therefore still force COW.
pub(crate) fn same_storage(a: &Value, b: &Value) -> bool {
    // Scalars alone do not benefit from release. Struct field maps are owned,
    // but their storage-backed children can be released without walking payloads.
    match (a, b) {
        (Value::Array(a), Value::Array(b)) => return Arc::ptr_eq(a, b),
        (Value::Bytes(a), Value::Bytes(b)) => return Arc::ptr_eq(a, b),
        (Value::Dict(a), Value::Dict(b)) => return Arc::ptr_eq(a, b),
        (Value::Struct { .. }, Value::Struct { .. }) => {}
        _ => return false,
    }
    let mut pending = vec![(a, b)];
    while let Some((a, b)) = pending.pop() {
        let same = match (a, b) {
            (Value::Array(a), Value::Array(b)) => Arc::ptr_eq(a, b),
            (Value::Bytes(a), Value::Bytes(b)) => Arc::ptr_eq(a, b),
            (Value::Dict(a), Value::Dict(b)) => Arc::ptr_eq(a, b),
            (Value::Str(a), Value::Str(b)) => Arc::ptr_eq(a, b),
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (
                Value::Struct { name: a_name, fields: a },
                Value::Struct { name: b_name, fields: b },
            ) if a_name == b_name && a.len() == b.len() => {
                if Arc::ptr_eq(a, b) {
                    continue;
                }
                for (key, value) in a.iter() {
                    let Some(other) = b.get(key) else { return false };
                    pending.push((value, other));
                }
                true
            }
            _ => false,
        };
        if !same {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod storage_identity_tests {
    use super::*;

    #[test]
    fn storage_identity_distinguishes_aliases_equal_payloads_and_scalars() {
        let pairs = [
            (Value::array(vec![Value::Int(7)]), Value::array(vec![Value::Int(7)])),
            (Value::Bytes(Arc::new(vec![0, 255])), Value::Bytes(Arc::new(vec![0, 255]))),
            (Value::Dict(Arc::new(Default::default())), Value::Dict(Arc::new(Default::default()))),
        ];
        for (root, equal_payload) in &pairs {
            assert!(same_storage(root, &root.clone()));
            assert!(!same_storage(root, equal_payload));
            assert!(!same_storage(root, &Value::Null));
            for (other, _) in &pairs {
                if std::mem::discriminant(root) != std::mem::discriminant(other) {
                    assert!(!same_storage(root, other));
                }
            }
        }
        for scalar in [
            Value::Int(7),
            Value::Float(0.0),
            Value::Bool(true),
            Value::Null,
            Value::Str(Arc::new("same".into())),
            Value::IntDict(Arc::new(Default::default())),
        ] {
            assert!(!same_storage(&scalar, &scalar.clone()));
        }
    }

    #[test]
    fn storage_identity_preserves_nested_struct_and_float_bit_rules() {
        let array = Value::array(vec![Value::Int(7)]);
        let structure = |name: &str, child: Value, scalar: f64| Value::Struct {
            name: name.into(),
            fields: Arc::new(
                [("child".into(), child), ("scalar".into(), Value::Float(scalar))]
                    .into_iter()
                    .collect(),
            ),
        };
        let root = structure("State", array.clone(), 0.0);
        assert!(same_storage(&root, &root.clone()));
        assert!(same_storage(&root, &structure("State", array.clone(), 0.0)));
        assert!(!same_storage(&root, &structure("State", array.clone(), -0.0)));
        assert!(!same_storage(&root, &structure("Other", array.clone(), 0.0)));
        assert!(!same_storage(&root, &structure("State", Value::array(vec![Value::Int(7)]), 0.0)));
        assert!(!same_storage(&root, &array));
        let nan = f64::from_bits(0x7ff8000000000001);
        assert!(same_storage(
            &structure("State", array.clone(), nan),
            &structure("State", array.clone(), nan)
        ));
        assert!(!same_storage(
            &structure("State", array.clone(), nan),
            &structure("State", array.clone(), f64::from_bits(0x7ff8000000000002))
        ));
        let mut left = array.clone();
        let mut right = array;
        for _ in 0..128 {
            left = structure("Node", left, 0.0);
            right = structure("Node", right, 0.0);
        }
        assert!(same_storage(&left, &right));
    }
}

/// A function's initial local alias can consume a parameter that is never read
/// again. Use the compiler's lexical resolver, not a textual name search; on
/// unsupported syntax keep the original value. The binding itself stays defined.
pub(crate) fn initial_parameter_alias<'a>(body: &'a [crate::ast::Stmt]) -> Option<&'a str> {
    use crate::ast::{Pattern, Stmt};
    let Stmt::Let { pattern: Pattern::Identifier(target), value: Expr::Identifier(source), .. } =
        body.first()?
    else {
        return None;
    };
    if target == source {
        return None;
    }
    let free =
        crate::compiler::Compiler::spawn_free_bindings(&body[1..], std::iter::once(source.clone()))
            .ok()?;
    (!free.contains(source)).then_some(source.as_str())
}

#[cfg(test)]
mod fixed_dictionary_depth_cache_tests {
    use super::*;

    #[test]
    fn fresh_fixed_wrappers_reuse_cached_child_depth() {
        let row = Value::array((0..10_000).map(Value::Int).collect());
        let mut cache = DepthCache::default();
        assert_eq!(cache.depth(&row), 1);
        for _ in 0..32 {
            let root = Value::FixedDict {
                keys: Arc::new(vec!["row".into()]),
                values: vec![row.clone()].into(),
            };
            assert_eq!(cache.depth(&root), 2);
        }
        assert_eq!(cache.scan_count(), 1, "fresh wrappers rescanned an unchanged child");
    }

    #[test]
    fn shared_fixed_depth_cache_walk_does_not_expand_the_dag() {
        let mut root = Value::Int(1);
        for _ in 0..16 {
            root = Value::FixedDict {
                keys: Arc::new(vec!["left".into(), "right".into()]),
                values: vec![root.clone(), root].into(),
            };
        }
        let mut cache = DepthCache::default();
        assert_eq!(cache.depth(&root), 16);
        let scans = cache.scan_count();
        assert!(scans <= 2, "shared fixed subtree rescanned {scans} times");
        assert_eq!(cache.depth(&root), 16);
        assert_eq!(cache.scan_count(), scans);
    }
}

#[cfg(test)]
mod removal_tests {
    use super::*;
    #[test]
    fn deletion_keeps_unique_map_allocation_and_detaches_snapshots() {
        let mut root =
            Value::dict((0..10_000).map(|i| (i.to_string().into(), Value::Int(i))).collect());
        let address = |v: &Value| {
            let Value::Dict(d) = v else { panic!() };
            Arc::as_ptr(d)
        };
        let original_address = address(&root);
        for i in 0..100 {
            let mut original = Some(root.clone());
            let pair = remove_rebinding(&mut root, &mut original, &Value::str_ref(&i.to_string()))
                .unwrap();
            assert_eq!(address(&root), original_address);
            drop(pair);
        }
        let alias = root.clone();
        let mut original = Some(root.clone());
        remove_rebinding(&mut root, &mut original, &Value::str_ref("100")).unwrap();
        assert_ne!(address(&root), address(&alias));
        assert!(alias.dictionary_get("100").is_some());
        assert!(root.dictionary_get("100").is_none());
    }
}

#[cfg(test)]
mod path_removal_tests {
    use super::*;
    #[test]
    fn nested_pop_keeps_leaf_buffer_and_detaches_parent_aliases() {
        let row = Value::array((0..10_000).map(Value::Int).collect());
        let mut root = Value::Struct {
            name: "State".into(),
            fields: [("rows".into(), Value::array(vec![row]))]
                .into_iter()
                .collect::<std::collections::HashMap<_, _>>()
                .into(),
        };
        let path = [Access::Field("rows".into()), Access::Index(Value::Int(0))];
        let leaf = |v: &Value| {
            let Value::Array(a) = child(child(v, &path[0]).unwrap(), &path[1]).unwrap() else {
                panic!()
            };
            a.as_ptr()
        };
        let address = leaf(&root);
        let mut cache = DepthCache::default();
        for _ in 0..100 {
            let mut original =
                Some(child(child(&root, &path[0]).unwrap(), &path[1]).unwrap().clone());
            let pair = collection_rebinding_path_checked(
                &mut root,
                &path,
                &mut original,
                None,
                &mut cache,
            )
            .unwrap();
            assert_eq!(leaf(&root), address);
            drop(pair);
        }
        assert_eq!(cache.scans, 1, "shrinking path rescanned payloads");
        let alias = root.clone();
        let mut original = Some(child(child(&root, &path[0]).unwrap(), &path[1]).unwrap().clone());
        collection_rebinding_path(&mut root, &path, &mut original, None).unwrap();
        assert_ne!(leaf(&root), leaf(&alias));
    }
}

#[cfg(test)]
mod specialized_path_removal_tests {
    use super::*;
    #[test]
    fn specialized_parent_maps_reuse_unique_leaf_storage() {
        for kind in 0..3 {
            let row = Value::array((0..10_000).map(Value::Int).collect());
            let mut root = match kind {
                0 => {
                    Value::FixedDict { keys: Arc::new(vec!["0".into()]), values: vec![row].into() }
                }
                1 => Value::IntDict(Arc::new([(0, row)].into_iter().collect())),
                _ => Value::DenseIntDict(Arc::new(vec![row])),
            };
            let path = [Access::Index(Value::Int(0))];
            let address = |v: &Value| {
                let Value::Array(a) = collection_child(v, &path[0]).unwrap() else { panic!() };
                a.as_ptr()
            };
            let initial = address(&root);
            let mut cache = DepthCache::default();
            for _ in 0..64 {
                let mut original = Some(collection_child(&root, &path[0]).unwrap().clone());
                collection_rebinding_path_checked(
                    &mut root,
                    &path,
                    &mut original,
                    None,
                    &mut cache,
                )
                .unwrap();
                assert_eq!(address(&root), initial);
            }
            assert_eq!(cache.scans, 1, "specialized parent rescanned payloads");
            let alias = root.clone();
            let mut original = Some(collection_child(&root, &path[0]).unwrap().clone());
            collection_rebinding_path(&mut root, &path, &mut original, None).unwrap();
            assert_ne!(address(&root), address(&alias));
        }
    }
}

#[cfg(test)]
mod removal_matcher_tests {
    use super::*;
    #[test]
    fn pair_shadowing_an_index_keeps_ordinary_evaluation() {
        let statements = crate::parser::Parser::new(
            crate::lexer::tokenize("let slot := pop(rows[slot]) rows[slot] = slot[0]").unwrap(),
        )
        .parse();
        assert!(pop_rebind_pair(&statements[0], statements.get(1)).is_none());
    }
}

#[cfg(test)]
mod shared_struct_depth_tests {
    use super::*;
    #[test]
    fn repeated_shared_struct_updates_use_depth_cache_and_detach() {
        let mut root = Value::Null;
        for _ in 0..40 {
            root = Value::Struct {
                name: "Node".into(),
                fields: Arc::new(
                    [("left".into(), root.clone()), ("right".into(), root)].into_iter().collect(),
                ),
            };
        }
        let alias = root.clone();
        let mut cache = DepthCache::default();
        for i in 0..64 {
            update_checked(&mut root, &[Access::Field("right".into())], Value::Int(i), &mut cache)
                .unwrap();
        }
        assert_eq!(cache.scans, 1);
        assert!(matches!(
            child(&alias, &Access::Field("right".into())),
            Some(Value::Struct { .. })
        ));
    }
}

/// Consume only the evaluated byte argument; retained values still trigger COW.
pub(crate) fn concat_bytes_rebinding(
    root: &mut Value,
    original: &mut Option<Value>,
    suffix: &Value,
) -> bool {
    let (Value::Bytes(destination), Some(Value::Bytes(source)), Value::Bytes(suffix)) =
        (root, original.as_ref(), suffix)
    else {
        return false;
    };
    if !Arc::ptr_eq(destination, source) || destination.len().checked_add(suffix.len()).is_none() {
        return false;
    }
    drop(original.take());
    Arc::make_mut(destination).extend_from_slice(suffix);
    true
}

/// Reuse a string only when it is still the evaluated left operand. RHS calls
/// may reassign the destination; retained aliases must continue to force COW.
pub(crate) fn concat_string_rebinding(
    root: &mut Value,
    original: &mut Option<Value>,
    suffix: &Value,
) -> bool {
    let (Value::Str(destination), Some(Value::Str(source)), Value::Str(suffix)) =
        (root, original.as_ref(), suffix)
    else {
        return false;
    };
    if !Arc::ptr_eq(destination, source) || destination.len().checked_add(suffix.len()).is_none() {
        return false;
    }
    drop(original.take());
    Arc::make_mut(destination).push_str(suffix);
    true
}

pub(crate) fn concat_bytes_path(
    root: &mut Value,
    path: &[Access],
    original: Value,
    suffix: Value,
    mut cache: Option<&mut DepthCache>,
) -> Result<(), String> {
    let (Value::Bytes(source), Value::Bytes(tail)) = (&original, &suffix) else {
        return Err("concat() requires two arrays or two bytes values".into());
    };
    source.len().checked_add(tail.len()).ok_or("concat() byte length overflow")?;
    let normalized;
    let path = if path.len() == 1 && !matches!(root, Value::Struct { .. }) {
        if let Access::Field(field) = &path[0] {
            normalized = vec![Access::Index(Value::str_ref(field))];
            &normalized[..]
        } else {
            path
        }
    } else {
        path
    };
    let bound = cache.as_mut().map(|c| c.depth(root).max(path.len()));
    let mut destination = &*root;
    let mut same = true;
    for access in path {
        match collection_child(destination, access) {
            Some(child) => destination = child,
            None => {
                same = false;
                break;
            }
        }
    }
    same &= matches!(destination, Value::Bytes(bytes) if Arc::ptr_eq(bytes, source));
    let mut original = Some(original);
    if same && bound.is_none_or(|n| n <= crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH) {
        // Drop only the cache weak identities on the mutation path. Otherwise
        // Arc::make_mut relocates unique nodes even without a retained value alias.
        if let Some(cache) = cache.as_mut() {
            cache.release_path(root, path);
        }
        let mut destination = &mut *root;
        for access in path {
            destination = collection_child_mut(destination, access).expect("validated byte path");
        }
        assert!(concat_bytes_rebinding(destination, &mut original, &suffix));
        if let Some(cache) = cache {
            cache.remember_path(root, path, bound.unwrap());
        }
        Ok(())
    } else {
        let Value::Bytes(mut bytes) = original.take().unwrap() else { unreachable!() };
        Arc::make_mut(&mut bytes).extend_from_slice(tail);
        let result = Value::Bytes(bytes);
        if let Some(cache) = cache {
            update_checked(root, path, result, cache)
        } else {
            update_in_place(root, path, result)
        }
    }
}

#[cfg(test)]
mod byte_concat_storage_tests {
    use super::*;
    #[test]
    fn byte_growth_reuses_capacity_and_detaches_real_aliases() {
        let mut bytes = Vec::with_capacity(1024);
        bytes.push(1);
        let mut root = Value::bytes(bytes);
        let pointer = |v: &Value| {
            let Value::Bytes(b) = v else { panic!() };
            b.as_ptr()
        };
        let original_pointer = pointer(&root);
        let suffix = Value::bytes(vec![2]);
        for _ in 0..100 {
            let mut original = Some(root.clone());
            assert!(concat_bytes_rebinding(&mut root, &mut original, &suffix));
            assert_eq!(pointer(&root), original_pointer);
        }
        let saved = root.clone();
        let mut original = Some(root.clone());
        assert!(concat_bytes_rebinding(&mut root, &mut original, &suffix));
        assert_ne!(pointer(&root), pointer(&saved));
        let Value::Bytes(b) = saved else { panic!() };
        assert_eq!(b.len(), 101);
    }
}

#[cfg(test)]
mod byte_concat_depth_tests {
    use super::*;
    #[test]
    fn rejected_byte_growth_does_not_modify_an_overdeep_root() {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let mut deep = Value::Null;
                for _ in 0..crate::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH {
                    deep = Value::array(vec![deep]);
                }
                let mut root = Value::dict(
                    [("bad".into(), deep), ("safe".into(), Value::bytes(vec![1]))]
                        .into_iter()
                        .collect(),
                );
                let path = [Access::Index(Value::str_ref("safe"))];
                let original = collection_child(&root, &path[0]).unwrap().clone();
                let mut cache = DepthCache::default();
                let error = concat_bytes_path(
                    &mut root,
                    &path,
                    original,
                    Value::bytes(vec![2]),
                    Some(&mut cache),
                )
                .unwrap_err();
                assert!(error.contains("nesting depth"));
                let Value::Bytes(bytes) = collection_child(&root, &path[0]).unwrap() else {
                    panic!()
                };
                assert_eq!(bytes.as_slice(), &[1]);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(test)]
mod string_rebinding_tests {
    use super::*;

    #[test]
    fn append_reuses_capacity_and_retained_aliases_force_copying() {
        let mut text = String::with_capacity(4096);
        text.push_str("a");
        let pointer = text.as_ptr();
        let mut root = Value::Str(Arc::new(text));
        for _ in 0..100 {
            let mut original = Some(root.clone());
            assert!(concat_string_rebinding(&mut root, &mut original, &Value::str_ref("b")));
            let Value::Str(value) = &root else { panic!() };
            assert_eq!(value.as_ptr(), pointer);
        }
        let alias = root.clone();
        let mut original = Some(root.clone());
        assert!(concat_string_rebinding(&mut root, &mut original, &Value::str_ref("c")));
        let (Value::Str(old), Value::Str(new)) = (&alias, &root) else { panic!() };
        assert!(!Arc::ptr_eq(old, new));
        assert_eq!(old.len(), 101);
        assert_eq!(new.len(), 102);
        let mut stale = Some(alias);
        assert!(!concat_string_rebinding(&mut root, &mut stale, &Value::str_ref("d")));
        assert!(stale.is_some());
    }
}

#[cfg(test)]
mod checked_append_node_storage_tests {
    use super::*;

    fn nodes(root: &Value, path: &[Access]) -> (usize, usize) {
        let parent = match root {
            Value::Struct { fields, .. } => Arc::as_ptr(fields) as usize,
            Value::Dict(map) => Arc::as_ptr(map) as usize,
            Value::Array(items) => Arc::as_ptr(items) as usize,
            _ => unreachable!(),
        };
        let child = match collection_child(root, &path[0]).unwrap() {
            Value::Array(items) => Arc::as_ptr(items) as usize,
            Value::Bytes(bytes) => Arc::as_ptr(bytes) as usize,
            _ => unreachable!(),
        };
        (parent, child)
    }

    fn length(root: &Value, path: &[Access]) -> usize {
        match collection_child(root, &path[0]).unwrap() {
            Value::Array(items) => items.len(),
            Value::Bytes(bytes) => bytes.len(),
            _ => unreachable!(),
        }
    }

    fn grow(root: &mut Value, path: &[Access], bytes: bool, cache: &mut DepthCache) {
        let original = collection_child(root, &path[0]).unwrap().clone();
        if bytes {
            concat_bytes_path(root, path, original, Value::bytes(vec![255]), Some(cache)).unwrap();
        } else {
            append_path(root, path, original, Value::Int(7), Some(cache)).unwrap();
        }
    }

    #[test]
    fn checked_appends_preserve_unique_arc_nodes_and_detach_real_snapshots() {
        let mut measurements = Vec::new();
        for repetitions in [8, 128] {
            for bytes in [false, true] {
                for container in 0..3 {
                    let leaf = if bytes {
                        Value::bytes(Vec::with_capacity(256))
                    } else {
                        Value::array(Vec::with_capacity(256))
                    };
                    let (mut root, path) = match container {
                        0 => (
                            Value::Struct {
                                name: "State".into(),
                                fields: Arc::new([("items".into(), leaf)].into_iter().collect()),
                            },
                            vec![Access::Field("items".into())],
                        ),
                        1 => (
                            Value::dict([(Arc::from("items"), leaf)].into_iter().collect()),
                            vec![Access::Index(Value::str_ref("items"))],
                        ),
                        _ => (Value::array(vec![leaf]), vec![Access::Index(Value::Int(0))]),
                    };
                    let mut cache = DepthCache::default();
                    let mut changes = (0, 0);
                    for _ in 0..repetitions {
                        let before = nodes(&root, &path);
                        grow(&mut root, &path, bytes, &mut cache);
                        let after = nodes(&root, &path);
                        changes.0 += usize::from(before.0 != after.0);
                        changes.1 += usize::from(before.1 != after.1);
                    }
                    measurements.push((repetitions, bytes, container, changes));
                    assert_eq!(length(&root, &path), repetitions);
                    let snapshot = root.clone();
                    grow(&mut root, &path, bytes, &mut cache);
                    assert_eq!(length(&snapshot, &path), repetitions);
                    assert_eq!(length(&root, &path), repetitions + 1);
                    assert_ne!(nodes(&root, &path), nodes(&snapshot, &path));
                }
            }
        }
        eprintln!("checked append Arc-node relocation measurements: {measurements:?}");
        assert!(measurements.iter().all(|row| row.3 == (0, 0)), "{measurements:?}");
    }
}

#[cfg(test)]
mod checked_removal_node_storage_tests {
    use super::*;

    fn nodes(root: &Value, path: &[Access]) -> (usize, usize) {
        let parent = match root {
            Value::Struct { fields, .. } => Arc::as_ptr(fields) as usize,
            Value::Dict(map) => Arc::as_ptr(map) as usize,
            Value::Array(items) => Arc::as_ptr(items) as usize,
            _ => unreachable!(),
        };
        let leaf = match collection_child(root, &path[0]).unwrap() {
            Value::Array(items) => Arc::as_ptr(items) as usize,
            Value::Dict(map) => Arc::as_ptr(map) as usize,
            _ => unreachable!(),
        };
        (parent, leaf)
    }

    fn length(root: &Value, path: &[Access]) -> usize {
        match collection_child(root, &path[0]).unwrap() {
            Value::Array(items) => items.len(),
            Value::Dict(map) => map.len(),
            _ => unreachable!(),
        }
    }

    fn remove(
        root: &mut Value,
        path: &[Access],
        dictionary: bool,
        index: usize,
        cache: &mut DepthCache,
    ) {
        let mut original = Some(collection_child(root, &path[0]).unwrap().clone());
        let key = dictionary.then(|| Value::str_ref(&format!("key{index}")));
        let pair =
            collection_rebinding_path_checked(root, path, &mut original, key.as_ref(), cache)
                .unwrap();
        assert!(original.is_none());
        drop(pair);
    }

    #[test]
    fn checked_removals_preserve_unique_arc_nodes_and_detach_real_snapshots() {
        let mut measurements = Vec::new();
        for repetitions in [8, 128] {
            for dictionary in [false, true] {
                for container in 0..3 {
                    let leaf = if dictionary {
                        Value::dict(
                            (0..repetitions + 2)
                                .map(|i| (Arc::from(format!("key{i}")), Value::Int(i as i64)))
                                .collect(),
                        )
                    } else {
                        Value::array((0..repetitions + 2).map(|i| Value::Int(i as i64)).collect())
                    };
                    let (mut root, path) = match container {
                        0 => (
                            Value::Struct {
                                name: "State".into(),
                                fields: Arc::new([("items".into(), leaf)].into_iter().collect()),
                            },
                            vec![Access::Field("items".into())],
                        ),
                        1 => (
                            Value::dict([(Arc::from("items"), leaf)].into_iter().collect()),
                            vec![Access::Index(Value::str_ref("items"))],
                        ),
                        _ => (Value::array(vec![leaf]), vec![Access::Index(Value::Int(0))]),
                    };
                    let mut cache = DepthCache::default();
                    let mut changes = (0, 0);
                    for index in 0..repetitions {
                        let before = nodes(&root, &path);
                        remove(&mut root, &path, dictionary, index, &mut cache);
                        let after = nodes(&root, &path);
                        changes.0 += usize::from(before.0 != after.0);
                        changes.1 += usize::from(before.1 != after.1);
                    }
                    measurements.push((repetitions, dictionary, container, changes));
                    assert_eq!(length(&root, &path), 2);
                    let snapshot = root.clone();
                    remove(&mut root, &path, dictionary, repetitions, &mut cache);
                    assert_eq!(length(&snapshot, &path), 2);
                    assert_eq!(length(&root, &path), 1);
                    assert_ne!(nodes(&root, &path), nodes(&snapshot, &path));
                }
            }
        }
        eprintln!("checked removal Arc-node relocation measurements: {measurements:?}");
        assert!(measurements.iter().all(|row| row.3 == (0, 0)), "{measurements:?}");
    }
}
