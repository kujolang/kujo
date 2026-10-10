#[path = "support/native_values.rs"]
mod native_values;
use kujo::interpreter::{IntDictMap, Value};
use native_values::{array, call, json};
use std::sync::Arc;

fn representations() -> Vec<Value> {
    let mut map = IntDictMap::default();
    map.insert(0, Value::Int(7));
    map.insert(1, Value::Int(8));
    vec![
        json(r#"{"0":7,"1":8}"#),
        Value::FixedDict {
            keys: Arc::new(vec![Arc::from("1"), Arc::from("0")]),
            values: vec![Value::Int(8), Value::Int(7)],
        },
        Value::IntDict(Arc::new(map)),
        Value::DenseIntDict(Arc::new(vec![Value::Int(7), Value::Int(8)])),
        Value::DenseIntDictInt(Arc::new(vec![Some(7), Some(8)])),
        Value::DenseIntDictIntFull(Arc::new(vec![7, 8])),
    ]
}

#[test]
fn all_dictionary_representations_meet_collection_contracts() {
    for d in representations() {
        for (name, rest, expected) in [
            ("len", vec![], Value::Int(2)),
            ("keys", vec![], json(r#"["0","1"]"#)),
            ("values", vec![], json("[7,8]")),
            ("items", vec![], json(r#"[["0",7],["1",8]]"#)),
            ("get", vec![json(r#""1""#)], Value::Int(8)),
            ("has_key", vec![json(r#""1""#)], Value::Int(1)),
            ("get_default", vec![json(r#""missing""#), Value::Int(99)], Value::Int(99)),
            ("invert", vec![], json(r#"{"7":"0","8":"1"}"#)),
            ("clear", vec![], json("{}")),
        ] {
            let mut args = vec![d.clone()];
            args.extend(rest);
            let actual = call(name, &args);
            assert!(Value::equals(&actual, &expected), "{name}({d:?}): {actual:?}");
        }
    }
}

#[test]
fn every_dictionary_pair_merges_and_updates_without_losing_entries() {
    for left in representations() {
        for right in representations() {
            for operation in ["merge", "update"] {
                let actual = call(operation, &[left.clone(), right.clone()]);
                assert!(
                    Value::equals(&actual, &left),
                    "{operation}({left:?},{right:?}): {actual:?}"
                );
            }
        }
    }
}

#[test]
fn all_dictionary_representations_serialize_and_validate() {
    for d in representations() {
        for format in ["json", "json_pretty", "toml", "yaml"] {
            let actual = call(&format!("to_{format}"), std::slice::from_ref(&d));
            assert!(matches!(actual, Value::Str(_)), "{format}({d:?}): {actual:?}");
            let parser = if format == "json_pretty" { "json" } else { format };
            let decoded = call(&format!("parse_{parser}"), &[actual]);
            assert!(Value::equals(&decoded, &d), "{format}: {decoded:?}");
        }
        let result = call(
            "json_schema_validate",
            &[
                d.clone(),
                json(
                    r#"{"type":"object","required":["0","1"],"properties":{"1":{"const":8}},"additionalProperties":true}"#,
                ),
            ],
        );
        assert!(
            Value::equals(&result, &json(r#"{"valid":true,"errors":[]}"#)),
            "{d:?}: {result:?}"
        );
        assert!(Value::equals(
            &call("unique", &[array(vec![d.clone(), json(r#"{"0":7,"1":8}"#)])]),
            &array(vec![d])
        ));
    }
}

#[test]
fn specialized_schema_constants_compare_nested_numbers_exactly() {
    for d in representations() {
        let mut schema = kujo::interpreter::DictMap::default();
        schema.insert("const".into(), array(vec![d.clone()]));
        let expected = json(r#"{"valid":true,"errors":[]}"#);
        let result = call(
            "json_schema_validate",
            &[array(vec![json(r#"{"0":7.0,"1":8.0}"#)]), Value::Dict(Arc::new(schema))],
        );
        assert!(Value::equals(&result, &expected), "{d:?}: {result:?}");
    }
}

#[test]
fn specialized_schema_comparison_retains_pending_node_budget() {
    for d in representations() {
        let mut instance = vec![Value::Null; 99_998];
        let mut constant = instance.clone();
        instance[99_997] = d;
        constant[99_997] = json(r#"{"0":99,"1":99}"#);
        let mut schema = kujo::interpreter::DictMap::default();
        schema.insert("const".into(), array(constant));
        let actual =
            call("json_schema_validate", &[array(instance), Value::Dict(Arc::new(schema))]);
        assert!(matches!(actual,Value::Error(ref e) if e.contains("node limit")), "{actual:?}");
    }
}

#[test]
fn empty_specialized_schemas_are_valid_empty_objects() {
    for schema in [
        Value::IntDict(Arc::new(IntDictMap::default())),
        Value::DenseIntDict(Arc::new(vec![])),
        Value::DenseIntDictInt(Arc::new(vec![])),
        Value::DenseIntDictIntFull(Arc::new(vec![])),
    ] {
        let actual = call("json_schema_validate", &[Value::Null, schema]);
        assert!(Value::equals(&actual, &json(r#"{"valid":true,"errors":[]}"#)), "{actual:?}");
    }
}
