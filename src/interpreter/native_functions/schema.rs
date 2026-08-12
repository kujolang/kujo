// File: src/interpreter/native_functions/schema.rs
//
// Native JSON Schema subset validation.

use crate::interpreter::{DictMap, Value};
use regex::Regex;
use std::sync::Arc;

const MAX_SCHEMA_DEPTH: usize = 64;
const MAX_VALIDATION_NODES: usize = 100_000;
const MAX_PATTERN_BYTES: usize = 1024;
const MAX_INSTANCE_ARRAY_ITEMS: usize = 100_000;

const ALLOWED_SCHEMA_KEYS: &[&str] = &[
    "$comment",
    "$defs",
    "$id",
    "$ref",
    "$schema",
    "additionalProperties",
    "allOf",
    "anyOf",
    "const",
    "default",
    "definitions",
    "description",
    "deprecated",
    "enum",
    "examples",
    "exclusiveMaximum",
    "exclusiveMinimum",
    "format",
    "items",
    "maximum",
    "maxItems",
    "maxLength",
    "minimum",
    "minItems",
    "minLength",
    "oneOf",
    "pattern",
    "properties",
    "required",
    "readOnly",
    "title",
    "type",
    "writeOnly",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidationError {
    path: String,
    message: String,
    keyword: String,
}

#[derive(Default)]
struct ValidationState {
    visited_nodes: usize,
    ref_stack: Vec<String>,
}

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    match name {
        "json_schema_validate" => Some(handle_json_schema_validate(arg_values)),
        _ => None,
    }
}

fn handle_json_schema_validate(arg_values: &[Value]) -> Value {
    if arg_values.len() != 2 {
        return Value::Error(format!(
            "json_schema_validate() expects 2 arguments (value, schema), got {}",
            arg_values.len()
        ));
    }

    match json_schema_validate(&arg_values[0], &arg_values[1]) {
        Ok(errors) => validation_result(errors),
        Err(message) => Value::Error(format!("json_schema_validate() {}", message)),
    }
}

fn json_schema_validate(value: &Value, schema: &Value) -> Result<Vec<ValidationError>, String> {
    let mut state = ValidationState::default();
    let mut errors = Vec::new();
    validate_schema(value, schema, schema, "", 0, &mut state, &mut errors)?;
    Ok(errors)
}

fn validation_result(errors: Vec<ValidationError>) -> Value {
    let mut result = DictMap::default();
    result.insert(Arc::<str>::from("valid"), Value::Bool(errors.is_empty()));
    result.insert(
        Arc::<str>::from("errors"),
        Value::Array(Arc::new(errors.into_iter().map(validation_error_value).collect())),
    );
    Value::Dict(Arc::new(result))
}

fn validation_error_value(error: ValidationError) -> Value {
    let mut map = DictMap::default();
    map.insert(Arc::<str>::from("path"), Value::Str(Arc::new(error.path)));
    map.insert(Arc::<str>::from("message"), Value::Str(Arc::new(error.message)));
    map.insert(Arc::<str>::from("keyword"), Value::Str(Arc::new(error.keyword)));
    Value::Dict(Arc::new(map))
}

fn validate_schema(
    value: &Value,
    schema: &Value,
    root_schema: &Value,
    instance_path: &str,
    depth: usize,
    state: &mut ValidationState,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(format!("exceeded schema recursion depth limit ({MAX_SCHEMA_DEPTH})"));
    }
    state.visited_nodes += 1;
    if state.visited_nodes > MAX_VALIDATION_NODES {
        return Err(format!("exceeded validation node limit ({MAX_VALIDATION_NODES})"));
    }

    let Some(schema_entries) = object_entries(schema) else {
        return Err("requires schema to be a dictionary".to_string());
    };

    reject_unknown_keywords(&schema_entries)?;

    if let Some(reference) = get_key(schema, "$ref") {
        let reference = string_schema_value(reference, "$ref")?;
        validate_ref(value, reference, root_schema, instance_path, depth, state, errors)?;
    }

    if let Some(type_schema) = get_key(schema, "type") {
        validate_type_keyword(value, type_schema, instance_path, errors)?;
    }

    if let Some(enum_schema) = get_key(schema, "enum") {
        validate_enum_keyword(value, enum_schema, instance_path, errors)?;
    }

    if let Some(const_schema) = get_key(schema, "const") {
        if !Value::equals(value, const_schema) {
            push_error(errors, instance_path, "const", "value does not match const");
        }
    }

    validate_number_keywords(value, schema, instance_path, errors)?;
    validate_string_keywords(value, schema, instance_path, errors)?;
    validate_array_keywords(value, schema, root_schema, instance_path, depth, state, errors)?;
    validate_object_keywords(value, schema, root_schema, instance_path, depth, state, errors)?;
    validate_combinators(value, schema, root_schema, instance_path, depth, state, errors)?;

    Ok(())
}

fn validate_ref(
    value: &Value,
    reference: &str,
    root_schema: &Value,
    instance_path: &str,
    depth: usize,
    state: &mut ValidationState,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    if !reference.starts_with('#') {
        return Err(format!("unsupported remote $ref '{reference}'"));
    }
    if state.ref_stack.iter().any(|entry| entry == reference) {
        return Err(format!("detected cyclic $ref '{reference}'"));
    }

    let referenced_schema = resolve_local_ref(root_schema, reference)
        .ok_or_else(|| format!("could not resolve local $ref '{reference}'"))?;

    state.ref_stack.push(reference.to_string());
    let result = validate_schema(
        value,
        referenced_schema,
        root_schema,
        instance_path,
        depth + 1,
        state,
        errors,
    );
    state.ref_stack.pop();
    result
}

fn validate_type_keyword(
    value: &Value,
    type_schema: &Value,
    instance_path: &str,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    let accepted_types = match type_schema {
        Value::Str(expected) => vec![expected.as_ref().clone()],
        Value::Array(values) => {
            let mut types = Vec::with_capacity(values.len());
            for item in values.iter() {
                types.push(string_schema_value(item, "type")?.to_string());
            }
            types
        }
        _ => return Err("requires 'type' to be a string or array of strings".to_string()),
    };

    for expected in &accepted_types {
        if !is_supported_type_name(expected) {
            return Err(format!("unsupported type '{expected}'"));
        }
    }

    if !accepted_types.iter().any(|expected| value_matches_type(value, expected)) {
        push_error(
            errors,
            instance_path,
            "type",
            &format!("expected type {}", accepted_types.join(" or ")),
        );
    }

    Ok(())
}

fn validate_enum_keyword(
    value: &Value,
    enum_schema: &Value,
    instance_path: &str,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    let Value::Array(cases) = enum_schema else {
        return Err("requires 'enum' to be an array".to_string());
    };

    if !cases.iter().any(|case| Value::equals(value, case)) {
        push_error(errors, instance_path, "enum", "value is not one of the allowed enum cases");
    }

    Ok(())
}

fn validate_number_keywords(
    value: &Value,
    schema: &Value,
    instance_path: &str,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    let Some(number) = numeric_value(value) else {
        return Ok(());
    };

    if let Some(minimum) = get_key(schema, "minimum") {
        let minimum = numeric_schema_value(minimum, "minimum")?;
        if number < minimum {
            push_error(errors, instance_path, "minimum", &format!("number is less than {minimum}"));
        }
    }

    if let Some(maximum) = get_key(schema, "maximum") {
        let maximum = numeric_schema_value(maximum, "maximum")?;
        if number > maximum {
            push_error(
                errors,
                instance_path,
                "maximum",
                &format!("number is greater than {maximum}"),
            );
        }
    }

    if let Some(minimum) = get_key(schema, "exclusiveMinimum") {
        let minimum = numeric_schema_value(minimum, "exclusiveMinimum")?;
        if number <= minimum {
            push_error(
                errors,
                instance_path,
                "exclusiveMinimum",
                &format!("number is not greater than {minimum}"),
            );
        }
    }

    if let Some(maximum) = get_key(schema, "exclusiveMaximum") {
        let maximum = numeric_schema_value(maximum, "exclusiveMaximum")?;
        if number >= maximum {
            push_error(
                errors,
                instance_path,
                "exclusiveMaximum",
                &format!("number is not less than {maximum}"),
            );
        }
    }

    Ok(())
}

fn validate_string_keywords(
    value: &Value,
    schema: &Value,
    instance_path: &str,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    let Value::Str(text) = value else {
        return Ok(());
    };
    let char_count = text.chars().count() as i64;

    if let Some(min_length) = get_key(schema, "minLength") {
        let min_length = non_negative_i64_schema_value(min_length, "minLength")?;
        if char_count < min_length {
            push_error(
                errors,
                instance_path,
                "minLength",
                &format!("string length is less than {min_length}"),
            );
        }
    }

    if let Some(max_length) = get_key(schema, "maxLength") {
        let max_length = non_negative_i64_schema_value(max_length, "maxLength")?;
        if char_count > max_length {
            push_error(
                errors,
                instance_path,
                "maxLength",
                &format!("string length is greater than {max_length}"),
            );
        }
    }

    if let Some(pattern) = get_key(schema, "pattern") {
        let pattern = string_schema_value(pattern, "pattern")?;
        if pattern.len() > MAX_PATTERN_BYTES {
            return Err(format!("pattern exceeds {MAX_PATTERN_BYTES} bytes"));
        }
        let regex = Regex::new(pattern)
            .map_err(|error| format!("invalid pattern for 'pattern': {error}"))?;
        if !regex.is_match(text.as_ref()) {
            push_error(errors, instance_path, "pattern", "string does not match pattern");
        }
    }

    Ok(())
}

fn validate_array_keywords(
    value: &Value,
    schema: &Value,
    root_schema: &Value,
    instance_path: &str,
    depth: usize,
    state: &mut ValidationState,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    let Value::Array(items) = value else {
        return Ok(());
    };

    if items.len() > MAX_INSTANCE_ARRAY_ITEMS {
        return Err(format!("array exceeds item limit ({MAX_INSTANCE_ARRAY_ITEMS})"));
    }

    if let Some(min_items) = get_key(schema, "minItems") {
        let min_items = non_negative_i64_schema_value(min_items, "minItems")? as usize;
        if items.len() < min_items {
            push_error(
                errors,
                instance_path,
                "minItems",
                &format!("array has fewer than {min_items} items"),
            );
        }
    }

    if let Some(max_items) = get_key(schema, "maxItems") {
        let max_items = non_negative_i64_schema_value(max_items, "maxItems")? as usize;
        if items.len() > max_items {
            push_error(
                errors,
                instance_path,
                "maxItems",
                &format!("array has more than {max_items} items"),
            );
        }
    }

    if let Some(item_schema) = get_key(schema, "items") {
        if !is_object_like(item_schema) {
            return Err("requires 'items' to be a dictionary schema".to_string());
        }

        for (index, item) in items.iter().enumerate() {
            let child_path = join_path(instance_path, &index.to_string());
            validate_schema(item, item_schema, root_schema, &child_path, depth + 1, state, errors)?;
        }
    }

    Ok(())
}

fn validate_object_keywords(
    value: &Value,
    schema: &Value,
    root_schema: &Value,
    instance_path: &str,
    depth: usize,
    state: &mut ValidationState,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    let Some(value_entries) = object_entries(value) else {
        return Ok(());
    };

    if let Some(required) = get_key(schema, "required") {
        let Value::Array(required_names) = required else {
            return Err("requires 'required' to be an array of strings".to_string());
        };
        for required_name in required_names.iter() {
            let required_name = string_schema_value(required_name, "required")?;
            if get_key(value, required_name).is_none() {
                push_error(
                    errors,
                    &join_path(instance_path, required_name),
                    "required",
                    &format!("required property '{required_name}' is missing"),
                );
            }
        }
    }

    let properties = match get_key(schema, "properties") {
        Some(properties) => {
            if !is_object_like(properties) {
                return Err("requires 'properties' to be a dictionary".to_string());
            }
            Some(properties)
        }
        None => None,
    };

    if let Some(properties) = properties {
        for (property_name, property_schema) in object_entries(properties).unwrap() {
            if !is_object_like(property_schema) {
                return Err(format!(
                    "requires schema for property '{property_name}' to be a dictionary"
                ));
            }
            if let Some(child_value) = get_key(value, &property_name) {
                let child_path = join_path(instance_path, &property_name);
                validate_schema(
                    child_value,
                    property_schema,
                    root_schema,
                    &child_path,
                    depth + 1,
                    state,
                    errors,
                )?;
            }
        }
    }

    if let Some(additional) = get_key(schema, "additionalProperties") {
        match additional {
            Value::Bool(true) => {}
            Value::Bool(false) => {
                let property_names = properties.map(object_property_names).unwrap_or_default();
                for (key, _) in value_entries {
                    if !property_names.iter().any(|known| known == &key) {
                        push_error(
                            errors,
                            &join_path(instance_path, &key),
                            "additionalProperties",
                            &format!("additional property '{key}' is not allowed"),
                        );
                    }
                }
            }
            schema if is_object_like(schema) => {
                let property_names = properties.map(object_property_names).unwrap_or_default();
                for (key, child_value) in value_entries {
                    if !property_names.iter().any(|known| known == &key) {
                        let child_path = join_path(instance_path, &key);
                        validate_schema(
                            child_value,
                            schema,
                            root_schema,
                            &child_path,
                            depth + 1,
                            state,
                            errors,
                        )?;
                    }
                }
            }
            _ => {
                return Err("requires 'additionalProperties' to be boolean or a dictionary schema"
                    .to_string())
            }
        }
    }

    Ok(())
}

fn validate_combinators(
    value: &Value,
    schema: &Value,
    root_schema: &Value,
    instance_path: &str,
    depth: usize,
    state: &mut ValidationState,
    errors: &mut Vec<ValidationError>,
) -> Result<(), String> {
    if let Some(all_of) = get_key(schema, "allOf") {
        for sub_schema in schema_array(all_of, "allOf")? {
            validate_schema(
                value,
                sub_schema,
                root_schema,
                instance_path,
                depth + 1,
                state,
                errors,
            )?;
        }
    }

    if let Some(any_of) = get_key(schema, "anyOf") {
        let sub_schemas = schema_array(any_of, "anyOf")?;
        let mut matches = 0;
        for sub_schema in sub_schemas {
            if validate_for_match(value, sub_schema, root_schema, instance_path, depth, state)? {
                matches += 1;
            }
        }
        if matches == 0 {
            push_error(errors, instance_path, "anyOf", "value does not match any allowed schema");
        }
    }

    if let Some(one_of) = get_key(schema, "oneOf") {
        let sub_schemas = schema_array(one_of, "oneOf")?;
        let mut matches = 0;
        for sub_schema in sub_schemas {
            if validate_for_match(value, sub_schema, root_schema, instance_path, depth, state)? {
                matches += 1;
            }
        }
        if matches != 1 {
            push_error(
                errors,
                instance_path,
                "oneOf",
                &format!("value matches {matches} schemas; expected exactly one"),
            );
        }
    }

    Ok(())
}

fn validate_for_match(
    value: &Value,
    schema: &Value,
    root_schema: &Value,
    instance_path: &str,
    depth: usize,
    state: &mut ValidationState,
) -> Result<bool, String> {
    let mut nested_errors = Vec::new();
    validate_schema(
        value,
        schema,
        root_schema,
        instance_path,
        depth + 1,
        state,
        &mut nested_errors,
    )?;
    Ok(nested_errors.is_empty())
}

fn reject_unknown_keywords(schema_entries: &[(String, &Value)]) -> Result<(), String> {
    for (key, _) in schema_entries {
        if !ALLOWED_SCHEMA_KEYS.contains(&key.as_str()) {
            return Err(format!("unsupported schema keyword '{key}'"));
        }
    }
    Ok(())
}

fn schema_array<'a>(schema: &'a Value, keyword: &str) -> Result<&'a [Value], String> {
    match schema {
        Value::Array(items) => {
            for item in items.iter() {
                if !is_object_like(item) {
                    return Err(format!("requires '{keyword}' entries to be dictionary schemas"));
                }
            }
            Ok(items.as_slice())
        }
        _ => Err(format!("requires '{keyword}' to be an array of schemas")),
    }
}

fn resolve_local_ref<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    if reference == "#" {
        return Some(root);
    }

    let pointer = reference.strip_prefix("#/")?;
    let mut current = root;
    for raw_segment in pointer.split('/') {
        let segment = unescape_json_pointer_segment(raw_segment);
        current = get_key(current, &segment)?;
    }
    Some(current)
}

fn unescape_json_pointer_segment(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

fn object_entries(value: &Value) -> Option<Vec<(String, &Value)>> {
    match value {
        Value::Dict(map) => {
            Some(map.iter().map(|(key, value)| (key.as_ref().to_string(), value)).collect())
        }
        Value::FixedDict { keys, values } => Some(
            keys.iter()
                .zip(values.iter())
                .map(|(key, value)| (key.as_ref().to_string(), value))
                .collect(),
        ),
        _ => None,
    }
}

fn object_property_names(value: &Value) -> Vec<String> {
    object_entries(value).unwrap_or_default().into_iter().map(|(key, _)| key).collect()
}

fn get_key<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Dict(map) => map.get(key),
        Value::FixedDict { keys, values } => keys
            .iter()
            .position(|candidate| candidate.as_ref() == key)
            .and_then(|index| values.get(index)),
        _ => None,
    }
}

fn is_object_like(value: &Value) -> bool {
    matches!(value, Value::Dict(_) | Value::FixedDict { .. })
}

fn value_matches_type(value: &Value, expected: &str) -> bool {
    match expected {
        "null" => matches!(value, Value::Null),
        "boolean" => matches!(value, Value::Bool(_)),
        "object" => is_object_like(value),
        "array" => matches!(value, Value::Array(_)),
        "number" => numeric_value(value).is_some(),
        "integer" => {
            matches!(value, Value::Int(_))
                || matches!(value, Value::Float(n) if n.is_finite() && n.fract() == 0.0)
        }
        "string" => matches!(value, Value::Str(_)),
        _ => false,
    }
}

fn is_supported_type_name(name: &str) -> bool {
    matches!(name, "null" | "boolean" | "object" | "array" | "number" | "integer" | "string")
}

fn numeric_value(value: &Value) -> Option<f64> {
    match value {
        Value::Int(n) => Some(*n as f64),
        Value::Float(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}

fn numeric_schema_value(value: &Value, keyword: &str) -> Result<f64, String> {
    match numeric_value(value) {
        Some(number) => Ok(number),
        None => Err(format!("requires '{keyword}' to be a finite number")),
    }
}

fn non_negative_i64_schema_value(value: &Value, keyword: &str) -> Result<i64, String> {
    let number = numeric_schema_value(value, keyword)?;
    if number < 0.0 || number.fract() != 0.0 {
        return Err(format!("requires '{keyword}' to be a non-negative integer"));
    }
    Ok(number as i64)
}

fn string_schema_value<'a>(value: &'a Value, keyword: &str) -> Result<&'a str, String> {
    match value {
        Value::Str(text) => Ok(text.as_ref()),
        _ => Err(format!("requires '{keyword}' to be a string")),
    }
}

fn join_path(parent: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    if parent.is_empty() {
        format!("/{escaped}")
    } else {
        format!("{parent}/{escaped}")
    }
}

fn push_error(errors: &mut Vec<ValidationError>, path: &str, keyword: &str, message: &str) {
    errors.push(ValidationError {
        path: if path.is_empty() { String::new() } else { path.to_string() },
        keyword: keyword.to_string(),
        message: message.to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::{handle, json_schema_validate};
    use crate::interpreter::{DictMap, Value};
    use std::sync::Arc;

    fn s(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    fn a(values: Vec<Value>) -> Value {
        Value::Array(Arc::new(values))
    }

    fn d(values: Vec<(&str, Value)>) -> Value {
        let mut map = DictMap::default();
        for (key, value) in values {
            map.insert(Arc::<str>::from(key), value);
        }
        Value::Dict(Arc::new(map))
    }

    fn assert_valid(value: Value, schema: Value) {
        let errors = json_schema_validate(&value, &schema).expect("schema should be supported");
        assert!(errors.is_empty(), "expected valid value, got errors: {:?}", errors);
    }

    fn assert_invalid(value: Value, schema: Value, expected_path: &str, expected_keyword: &str) {
        let errors = json_schema_validate(&value, &schema).expect("schema should be supported");
        assert!(
            errors
                .iter()
                .any(|error| { error.path == expected_path && error.keyword == expected_keyword }),
            "expected {expected_keyword} at {expected_path}, got {errors:?}"
        );
    }

    #[test]
    fn validates_type_required_properties_and_additional_properties() {
        let schema = d(vec![
            ("type", s("object")),
            ("required", a(vec![s("name"), s("age")])),
            (
                "properties",
                d(vec![
                    ("name", d(vec![("type", s("string")), ("minLength", Value::Int(2))])),
                    ("age", d(vec![("type", s("integer")), ("minimum", Value::Int(0))])),
                ]),
            ),
            ("additionalProperties", Value::Bool(false)),
        ]);

        assert_valid(d(vec![("name", s("Kujo")), ("age", Value::Int(1))]), schema.clone());
        assert_invalid(
            d(vec![("name", s("K")), ("extra", Value::Bool(true))]),
            schema.clone(),
            "/age",
            "required",
        );
        assert_invalid(
            d(vec![("name", s("K")), ("age", Value::Int(1))]),
            schema,
            "/name",
            "minLength",
        );
    }

    #[test]
    fn validates_items_enum_const_numbers_strings_and_array_bounds() {
        let schema = d(vec![
            ("type", s("array")),
            ("minItems", Value::Int(2)),
            ("maxItems", Value::Int(3)),
            (
                "items",
                d(vec![
                    ("type", s("string")),
                    ("enum", a(vec![s("red"), s("blue")])),
                    ("pattern", s("^[a-z]+$")),
                    ("maxLength", Value::Int(4)),
                ]),
            ),
        ]);

        assert_valid(a(vec![s("red"), s("blue")]), schema.clone());
        assert_invalid(a(vec![s("red"), s("green")]), schema.clone(), "/1", "enum");
        assert_invalid(a(vec![s("red"), s("BLUE")]), schema.clone(), "/1", "pattern");
        assert_invalid(a(vec![s("red"), s("blue"), s("red"), s("blue")]), schema, "", "maxItems");

        let number_schema = d(vec![
            ("type", s("number")),
            ("minimum", Value::Float(1.5)),
            ("maximum", Value::Int(2)),
        ]);
        assert_valid(Value::Int(2), number_schema.clone());
        assert_invalid(Value::Int(3), number_schema, "", "maximum");
        assert_valid(Value::Float(2.0), d(vec![("type", s("integer"))]));

        let const_schema = d(vec![("const", d(vec![("ok", Value::Bool(true))]))]);
        assert_invalid(d(vec![("ok", Value::Bool(false))]), const_schema, "", "const");
    }

    #[test]
    fn validates_any_one_all_of_and_local_refs() {
        let schema = d(vec![
            (
                "$defs",
                d(vec![("name", d(vec![("type", s("string")), ("minLength", Value::Int(2))]))]),
            ),
            (
                "allOf",
                a(vec![d(vec![("type", s("object"))]), d(vec![("required", a(vec![s("name")]))])]),
            ),
            ("properties", d(vec![("name", d(vec![("$ref", s("#/$defs/name"))]))])),
            (
                "anyOf",
                a(vec![
                    d(vec![("properties", d(vec![("mode", d(vec![("const", s("a"))]))]))]),
                    d(vec![("properties", d(vec![("mode", d(vec![("const", s("b"))]))]))]),
                ]),
            ),
        ]);

        assert_valid(d(vec![("name", s("Kujo")), ("mode", s("a"))]), schema.clone());
        assert_invalid(
            d(vec![("name", s("K")), ("mode", s("a"))]),
            schema.clone(),
            "/name",
            "minLength",
        );
        assert_invalid(d(vec![("name", s("Kujo")), ("mode", s("c"))]), schema, "", "anyOf");

        let one_of = d(vec![(
            "oneOf",
            a(vec![d(vec![("type", s("number"))]), d(vec![("type", s("integer"))])]),
        )]);
        assert_invalid(Value::Int(1), one_of, "", "oneOf");
    }

    #[test]
    fn accepts_standard_annotation_keywords_and_rejects_unknown_keywords() {
        let annotations = d(vec![
            ("$schema", s("https://json-schema.org/draft/2020-12/schema")),
            ("$id", s("https://example.com/schema.json")),
            ("$comment", s("fixture")),
            ("title", s("Fixture")),
            ("description", s("Annotation coverage")),
            ("default", Value::Int(0)),
            ("examples", a(vec![Value::Int(1)])),
            ("format", s("uri")),
            ("deprecated", Value::Bool(false)),
            ("readOnly", Value::Bool(true)),
            ("writeOnly", Value::Bool(false)),
            ("type", s("integer")),
            ("exclusiveMinimum", Value::Int(0)),
            ("exclusiveMaximum", Value::Int(2)),
        ]);
        assert!(json_schema_validate(&Value::Int(1), &annotations).is_ok());
        assert_invalid(Value::Int(0), annotations.clone(), "", "exclusiveMinimum");
        assert_invalid(Value::Int(2), annotations, "", "exclusiveMaximum");

        let unknown = json_schema_validate(&Value::Int(1), &d(vec![("unknownKeyword", s("x"))]))
            .expect_err("unsupported keyword should reject schema");
        assert!(unknown.contains("unsupported schema keyword 'unknownKeyword'"));
    }

    #[test]
    fn rejects_malformed_schemas_and_ref_cycles() {
        let bad_pattern = json_schema_validate(&s("x"), &d(vec![("pattern", s("["))])).unwrap_err();
        assert!(bad_pattern.contains("invalid pattern"));

        let cycle_schema = d(vec![("$ref", s("#"))]);
        let cycle = json_schema_validate(&Value::Int(1), &cycle_schema).unwrap_err();
        assert!(cycle.contains("cyclic $ref") || cycle.contains("recursion depth"));
    }

    #[test]
    fn guards_large_arrays_and_returns_result_shape() {
        let values = (0..=super::MAX_INSTANCE_ARRAY_ITEMS).map(|n| Value::Int(n as i64)).collect();
        let too_large = json_schema_validate(
            &Value::Array(Arc::new(values)),
            &d(vec![("items", d(vec![("type", s("integer"))]))]),
        )
        .unwrap_err();
        assert!(too_large.contains("array exceeds item limit"));

        let result =
            handle("json_schema_validate", &[Value::Int(1), d(vec![("type", s("string"))])])
                .expect("handler should return value");
        match result {
            Value::Dict(map) => {
                assert!(matches!(map.get("valid"), Some(Value::Bool(false))));
                match map.get("errors") {
                    Some(Value::Array(errors)) => assert_eq!(errors.len(), 1),
                    other => panic!("expected errors array, got {:?}", other),
                }
            }
            other => panic!("expected validation result dict, got {:?}", other),
        }
    }
}
