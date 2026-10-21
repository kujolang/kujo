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
    integer_dictionaries:
        std::collections::HashMap<usize, (std::sync::Weak<super::IntDictMap>, usize)>,
    #[cfg(test)]
    scans: usize,
}
impl DepthCache {
    fn remember(&mut self, value: &Value, bound: usize) {
        match value {
            Value::Array(items) | Value::DenseIntDict(items) => {
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
    fn depth(&mut self, root: &Value) -> usize {
        let mut work = vec![(root, 0)];
        let mut depth = 0;
        while let Some((value, offset)) = work.pop() {
            depth = depth.max(offset);
            let cached = match value {
                Value::Array(a) | Value::DenseIntDict(a) => {
                    self.arrays.get(&(Arc::as_ptr(a) as usize)).map(|e| e.1)
                }
                Value::Dict(d) => self.dictionaries.get(&(Arc::as_ptr(d) as usize)).map(|e| e.1),
                Value::IntDict(d) => {
                    self.integer_dictionaries.get(&(Arc::as_ptr(d) as usize)).map(|e| e.1)
                }
                _ => None,
            };
            if let Some(bound) = cached {
                depth = depth.max(offset + bound);
            } else if let Value::Struct { fields, .. } = value {
                work.extend(fields.values().map(|v| (v, offset + 1)));
            } else if let Value::FixedDict { values, .. } = value {
                work.extend(values.iter().map(|v| (v, offset + 1)));
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
            let Some(child) = collection_child(cursor, access) else { break };
            cursor = child;
            bound = bound.saturating_sub(1);
            self.remember(cursor, bound);
        }
        if self.arrays.len() + self.dictionaries.len() + self.integer_dictionaries.len() >= 1024 {
            self.arrays.retain(|_, (weak, _)| weak.strong_count() > 0);
            self.dictionaries.retain(|_, (weak, _)| weak.strong_count() > 0);
            self.integer_dictionaries.retain(|_, (weak, _)| weak.strong_count() > 0);
            if self.arrays.len() + self.dictionaries.len() + self.integer_dictionaries.len() >= 1024
            {
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
        Value::FixedDict { keys, values } => values.get_mut(fixed_collection_index(keys, access)?),
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
    let pair = collection_rebinding_path(root, path, original, key)?;
    // Shrinking cannot increase the validated bound. Refresh identities after
    // COW/weak-pointer detachment so the next operation does not rescan payloads.
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
    if !matches!(a, Value::Array(_) | Value::Bytes(_) | Value::Dict(_) | Value::Struct { .. }) {
        return false;
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
                for (key, value) in a {
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
            fields: [("rows".into(), Value::array(vec![row]))].into_iter().collect(),
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
                0 => Value::FixedDict { keys: Arc::new(vec!["0".into()]), values: vec![row] },
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
