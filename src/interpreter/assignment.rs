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
