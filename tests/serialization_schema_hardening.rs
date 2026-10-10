#[path = "support/native_values.rs"]
mod native_values;
use kujo::{builtins, interpreter::Value};
use native_values::{array, call, json};
use std::sync::Arc;

#[test]
fn csv_accepts_fixed_dictionary_rows() {
    let row = Value::FixedDict {
        keys: Arc::new(vec![Arc::from("name"), Arc::from("age")]),
        values: vec![Value::Str(Arc::new("Kujo".into())), Value::Int(2)],
    };
    let output = builtins::to_csv(&array(vec![row])).unwrap();
    assert!(Value::equals(
        &builtins::parse_csv(&output).unwrap(),
        &json(r#"[{"name":"Kujo","age":2}]"#)
    ));
}

#[test]
fn csv_rejects_duplicate_headers_instead_of_overwriting_cells() {
    assert!(builtins::parse_csv("name,name\na,b\n").is_err());
    assert!(builtins::parse_csv("name,name\n").is_err());
}

#[test]
fn csv_rejects_unrepresentable_rows_instead_of_dropping_data() {
    for rows in [r#"[{"a":1},{"a":2,"b":3}]"#, r#"[{"a":[1]}]"#, r#"[{"a":{"b":1}}]"#] {
        assert!(builtins::to_csv(&json(rows)).is_err(), "{rows}");
    }
    let rows = json(r#"[{"a":1,"b":null},{"a":2}]"#);
    assert!(builtins::to_csv(&rows).is_ok());
}

#[test]
fn schema_rejects_malformed_keywords_even_for_other_instance_types() {
    for schema in [
        r#"{"minimum":"bad"}"#,
        r#"{"maxLength":-1}"#,
        r#"{"pattern":"["}"#,
        r#"{"required":42}"#,
        r#"{"items":42}"#,
        r#"{"additionalProperties":42}"#,
    ] {
        let result = call("json_schema_validate", &[Value::Null, json(schema)]);
        assert!(matches!(result, Value::Error(_)), "{schema}: {result:?}");
    }
}

#[test]
fn schema_rejects_invalid_unvisited_children() {
    for schema in [
        r#"{"properties":{"absent":{"unknown":1}}}"#,
        r#"{"items":{"pattern":"["}}"#,
        r#"{"$defs":{"unused":{"minimum":"bad"}}}"#,
    ] {
        let result = call("json_schema_validate", &[json("[]"), json(schema)]);
        assert!(matches!(result, Value::Error(_)), "{schema}: {result:?}");
    }
}

#[test]
fn schema_rejects_empty_constraint_arrays() {
    for keyword in ["type", "allOf", "anyOf", "oneOf"] {
        let result =
            call("json_schema_validate", &[Value::Null, json(&format!(r#"{{"{keyword}":[]}}"#))]);
        assert!(matches!(result, Value::Error(_)), "{keyword}: {result:?}");
    }
}

#[test]
fn csv_columns_are_deterministic() {
    let output = builtins::to_csv(&json(r#"[{"z":1,"a":2,"m":3}]"#)).unwrap();
    assert_eq!(output, "a,m,z\n2,3,1\n");
}

#[test]
fn toml_diagnostics_do_not_echo_whole_long_lines() {
    let text = format!("a = {}!", "x".repeat(8192));
    let error = builtins::parse_toml(&text).unwrap_err();
    eprintln!("TOML error bytes={}", error.len());
    assert!(error.len() < 1024, "error bytes={}", error.len());
    assert!(error.contains("line 1") || error.contains("byte"));
}

#[test]
fn csv_preserves_scalars_escaping_missing_cells_and_redaction() {
    let mut row = kujo::interpreter::DictMap::default();
    row.insert("a".into(), Value::Str(Arc::new("comma,quote\"\n雪".into())));
    row.insert("b".into(), Value::Bool(true));
    row.insert("c".into(), Value::Null);
    let text = builtins::to_csv(&array(vec![Value::Dict(Arc::new(row)), json(r#"{"a":"other"}"#)]))
        .unwrap();
    let parsed = builtins::parse_csv(&text).unwrap();
    assert!(Value::equals(
        &parsed,
        &json(r#"[{"a":"comma,quote\"\n雪","b":"true","c":""},{"a":"other","b":"","c":""}]"#)
    ));
    let secret = call("secret", &[Value::Str(Arc::new("hidden".into()))]);
    let row = Value::FixedDict { keys: Arc::new(vec![Arc::from("token")]), values: vec![secret] };
    assert_eq!(builtins::to_csv(&array(vec![row])).unwrap(), "token\n***\n");
}

#[test]
fn schema_preflight_does_not_apply_constraints_to_other_types() {
    let schema = json(
        r#"{"minimum":4,"maxLength":2,"pattern":"^[a-z]+$","required":["x"],"items":{"type":"string"},"additionalProperties":false}"#,
    );
    let result = call("json_schema_validate", &[Value::Null, schema]);
    assert!(
        matches!(result, Value::Dict(ref d) if matches!(d.get("valid"), Some(Value::Bool(true))))
    );
}

#[test]
fn toml_short_diagnostics_are_unchanged_and_long_unicode_locations_survive() {
    let text = "a = !";
    let expected =
        format!("TOML parse error: {}", toml::from_str::<toml::Value>(text).unwrap_err());
    assert_eq!(builtins::parse_toml(text).unwrap_err(), expected);
    let text = format!("a = \"{}\"\nb = !", "雪".repeat(1000));
    let error = toml::from_str::<toml::Value>(&text).unwrap_err();
    let span = error.span().unwrap();
    let actual = builtins::parse_toml(&text).unwrap_err();
    assert!(actual.contains(&format!("bytes {}..{}", span.start, span.end)));
    assert!(actual.contains(error.message()));
    assert!(actual.len() < 1024);
}

#[test]
fn literal_csv_rows_work_in_vm_and_interpreter() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("csv.kujo");
    std::fs::write(&source, r#"print(to_csv([{"name":"Kujo","age":2}]))"#).unwrap();
    for interpreter in [false, true] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&source);
        if interpreter {
            command.arg("--interpreter");
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "age,name\n2,Kujo");
    }
}

#[test]
fn unused_schema_depth_is_bounded_but_annotations_are_not_schemas() {
    let mut schema = json("{}");
    for _ in 0..66 {
        let mut map = kujo::interpreter::DictMap::default();
        map.insert("items".into(), schema);
        schema = Value::Dict(Arc::new(map));
    }
    assert!(
        matches!(call("json_schema_validate", &[Value::Null, schema]), Value::Error(e) if e.contains("depth limit"))
    );
    let result = call(
        "json_schema_validate",
        &[Value::Null, json(r#"{"default":{"unknown":"data"},"examples":[{"not":"a schema"}]}"#)],
    );
    assert!(
        matches!(result, Value::Dict(ref d) if matches!(d.get("valid"), Some(Value::Bool(true))))
    );
}

#[test]
fn empty_enum_remains_a_valid_schema_that_matches_nothing() {
    let result = call("json_schema_validate", &[Value::Null, json(r#"{"enum":[]}"#)]);
    assert!(
        matches!(result, Value::Dict(ref d) if matches!(d.get("valid"), Some(Value::Bool(false))))
    );
}
