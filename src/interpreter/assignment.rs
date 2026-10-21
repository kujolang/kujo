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

pub(crate) enum Access {
    Index(Value),
    Field(String),
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
        (Value::Dict(map), Access::Index(index)) => {
            let key = match index {
                Value::Str(key) => key.as_ref().clone(),
                Value::Int(key) => key.to_string(),
                _ => return Err("Invalid index assignment".into()),
            };
            if rest.is_empty() {
                Ok(())
            } else {
                validate(
                    map.get(key.as_str()).ok_or_else(|| format!("Missing map key: {key}"))?,
                    rest,
                    value,
                )
            }
        }
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
                fields.insert(field.clone(), value);
                Ok(())
            } else {
                let child =
                    fields.get_mut(field).ok_or_else(|| format!("Field not found: {field}"))?;
                write(child, rest, value)
            }
        }
        (Value::Dict(map), Access::Index(index)) => {
            let key = match index {
                Value::Str(key) => key.as_ref().clone(),
                Value::Int(key) => key.to_string(),
                _ => return Err("Invalid index assignment".to_string()),
            };
            let map = Arc::make_mut(map);
            if rest.is_empty() {
                map.insert(key.into(), value);
                Ok(())
            } else {
                let child =
                    map.get_mut(key.as_str()).ok_or_else(|| format!("Missing map key: {key}"))?;
                write(child, rest, value)
            }
        }
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
    arrays: std::collections::HashMap<usize, (std::sync::Weak<Vec<Value>>, usize)>,
    dictionaries: std::collections::HashMap<usize, (std::sync::Weak<super::DictMap>, usize)>,
    #[cfg(test)]
    scans: usize,
}
impl DepthCache {
    fn remember(&mut self, value: &Value, bound: usize) {
        match value {
            Value::Array(items) => {
                self.arrays.insert(Arc::as_ptr(items) as usize, (Arc::downgrade(items), bound));
            }
            Value::Dict(items) => {
                self.dictionaries
                    .insert(Arc::as_ptr(items) as usize, (Arc::downgrade(items), bound));
            }
            _ => {}
        }
    }
    fn depth(&mut self, root: &Value) -> usize {
        let mut work = vec![(root, 0)];
        let mut depth = 0;
        while let Some((value, offset)) = work.pop() {
            depth = depth.max(offset);
            let cached = match value {
                Value::Array(a) => self.arrays.get(&(Arc::as_ptr(a) as usize)).map(|e| e.1),
                Value::Dict(d) => self.dictionaries.get(&(Arc::as_ptr(d) as usize)).map(|e| e.1),
                _ => None,
            };
            if let Some(bound) = cached {
                depth = depth.max(offset + bound);
            } else if let Value::Struct { fields, .. } = value {
                work.extend(fields.values().map(|v| (v, offset + 1)));
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
        depth
    }
    fn remember_path(&mut self, root: &Value, path: &[Access], bound: usize) {
        let mut cursor = root;
        let mut bound = bound;
        self.remember(cursor, bound);
        for access in path {
            let Some(child) = child(cursor, access) else { break };
            cursor = child;
            bound = bound.saturating_sub(1);
            self.remember(cursor, bound);
        }
        if self.arrays.len() + self.dictionaries.len() >= 1024 {
            self.arrays.retain(|_, (weak, _)| weak.strong_count() > 0);
            self.dictionaries.retain(|_, (weak, _)| weak.strong_count() > 0);
            if self.arrays.len() + self.dictionaries.len() >= 1024 {
                self.arrays.clear();
                self.dictionaries.clear();
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
    fn state_container_writes_reuse_rows_and_cached_depths() {
        for dictionary in [false, true] {
            let rows = Value::array(vec![Value::array(vec![Value::Int(0); 10_000])]);
            let mut root = if dictionary {
                Value::Dict(Arc::new([("rows".into(), rows)].into_iter().collect()))
            } else {
                Value::Struct {
                    name: "State".into(),
                    fields: [("rows".into(), rows)].into_iter().collect(),
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
        (Value::Struct { fields, .. }, Access::Field(field)) => fields.get_mut(field),
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
                fields: [("items".into(), array)].into_iter().collect(),
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
