#[path = "support/native_values.rs"]
mod native_values;
use kujo::interpreter::{DictMap, Value};
use native_values::{array, call, json};
use std::sync::Arc;

#[test]
fn vectors_preserve_direction_at_large_and_small_finite_scales() {
    for scale in [1e200, 1e-200, f64::from_bits(1), f64::MAX] {
        let v = array(vec![Value::Float(scale), Value::Float(0.0)]);
        let norm = call("vec_norm", std::slice::from_ref(&v));
        assert!(matches!(norm, Value::Float(n) if n == scale), "{norm:?}");
        let unit = call("vec_normalize", std::slice::from_ref(&v));
        assert!(Value::equals(&unit, &json("[1.0,0.0]")), "{unit:?}");
        let cosine = call("vec_cosine", &[v.clone(), v.clone()]);
        assert!(matches!(cosine, Value::Float(n) if (n - 1.0).abs() < 1e-12), "{cosine:?}");
        let top = call("vec_top_k", &[v.clone(), array(vec![json("[0,1]"), v]), Value::Int(1)]);
        assert!(Value::equals(&top, &json(r#"[{"index":1,"score":1.0}]"#)), "{top:?}");
    }
}

#[test]
fn schema_const_and_enum_use_exact_json_numeric_equality() {
    for (value, expected) in [
        (Value::Int(9_007_199_254_740_993), Value::Float(9_007_199_254_740_992.0)),
        (Value::Float(1e-20), Value::Float(0.0)),
        (json("[9007199254740993]"), json("[9007199254740992.0]")),
    ] {
        for keyword in ["const", "enum"] {
            let mut schema = DictMap::default();
            schema.insert(
                keyword.into(),
                if keyword == "enum" { array(vec![expected.clone()]) } else { expected.clone() },
            );
            let result =
                call("json_schema_validate", &[value.clone(), Value::Dict(Arc::new(schema))]);
            assert!(
                matches!(result, Value::Dict(ref d) if matches!(d.get("valid"), Some(Value::Bool(false)))),
                "{result:?}"
            );
        }
    }
    let result = call("json_schema_validate", &[Value::Int(1), json(r#"{"const":1.0}"#)]);
    assert!(
        matches!(result, Value::Dict(ref d) if matches!(d.get("valid"), Some(Value::Bool(true))))
    );
}

#[test]
fn schema_local_json_pointers_can_address_array_elements() {
    let schema = json(r##"{"allOf":[{"type":"integer"}],"$ref":"#/allOf/0"}"##);
    let result = call("json_schema_validate", &[Value::Int(1), schema]);
    assert!(
        matches!(result, Value::Dict(ref d) if matches!(d.get("valid"), Some(Value::Bool(true)))),
        "{result:?}"
    );
}

#[test]
fn numeric_conversion_errors_bound_model_visible_input_echoes() {
    for name in ["parse_int", "parse_float", "to_int", "to_float"] {
        for text in ["x".repeat(8192), "界".repeat(8192)] {
            let Value::Error(message) = call(name, &[Value::Str(Arc::new(text.clone()))]) else {
                panic!("expected conversion error")
            };
            assert!(message.len() < 600, "{name}: {} error bytes", message.len());
            assert!(message.contains(&format!("{} bytes", text.len())));
        }
    }
    assert!(
        matches!(call("parse_int", &[Value::Str(Arc::new("bad".into()))]), Value::Error(s) if s == "Cannot parse 'bad' as integer")
    );
}

#[test]
fn corrected_native_behaviors_are_visible_in_both_runtimes() {
    let cases = [
        ("print(sum([9223372036854775807, 1]))", 4, "overflow"),
        ("print(parse_int(repeat(\"x\", 8192)))", 4, "8192 bytes"),
        ("print(len(unique([secret(\"one\"), secret(\"two\"), secret(\"one\")])))", 0, "2"),
        ("print(vec_cosine([parse_float(\"1e200\")], [parse_float(\"1e200\")]))", 0, "1"),
        ("print(json_schema_validate(9007199254740993, {\"const\": 9007199254740992.0})[\"valid\"])", 0, "false"),
    ];
    let dir = tempfile::tempdir().unwrap();
    for interpreter in [false, true] {
        for (index, (source, exit, expected)) in cases.iter().enumerate() {
            let path = dir.path().join(format!("case_{index}.kujo"));
            std::fs::write(&path, source).unwrap();
            let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(&path).env("NO_COLOR", "1");
            if interpreter {
                command.arg("--interpreter");
            }
            let output = command.output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(
                output.status.code(),
                Some(*exit),
                "case {index}, interpreter={interpreter}: {stdout} {stderr}"
            );
            if *exit == 0 {
                assert_eq!(stdout.trim(), *expected);
            } else {
                assert!(stderr.contains(expected), "{stderr}");
            }
        }
    }
}
