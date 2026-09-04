// File: src/interpreter/native_functions/json.rs
//
// JSON encoding/decoding native functions

use crate::builtins;
use crate::interpreter::Value;
use std::sync::Arc;

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "parse_json" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("parse_json requires a string argument".to_string()));
            }

            if let Some(Value::Str(json_str)) = arg_values.first() {
                match builtins::parse_json(json_str.as_ref()) {
                    Ok(value) => value,
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("parse_json requires a string argument".to_string())
            }
        }

        "to_json" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("to_json requires a value argument".to_string()));
            }

            if let Some(value) = arg_values.first() {
                match builtins::to_json(value) {
                    Ok(json_str) => Value::Str(Arc::new(json_str)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("to_json requires a value argument".to_string())
            }
        }

        "to_json_pretty" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("to_json_pretty requires a value argument".to_string()));
            }

            if let Some(value) = arg_values.first() {
                match builtins::to_json_pretty(value) {
                    Ok(json_str) => Value::Str(Arc::new(json_str)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("to_json_pretty requires a value argument".to_string())
            }
        }

        "parse_toml" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("parse_toml requires a string argument".to_string()));
            }

            if let Some(Value::Str(toml_str)) = arg_values.first() {
                match builtins::parse_toml(toml_str.as_ref()) {
                    Ok(value) => value,
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("parse_toml requires a string argument".to_string())
            }
        }

        "to_toml" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("to_toml requires a value argument".to_string()));
            }

            if let Some(value) = arg_values.first() {
                match builtins::to_toml(value) {
                    Ok(toml_str) => Value::Str(Arc::new(toml_str)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("to_toml requires a value argument".to_string())
            }
        }

        "parse_yaml" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("parse_yaml requires a string argument".to_string()));
            }

            if let Some(Value::Str(yaml_str)) = arg_values.first() {
                match builtins::parse_yaml(yaml_str.as_ref()) {
                    Ok(value) => value,
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("parse_yaml requires a string argument".to_string())
            }
        }

        "to_yaml" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("to_yaml requires a value argument".to_string()));
            }

            if let Some(value) = arg_values.first() {
                match builtins::to_yaml(value) {
                    Ok(yaml_str) => Value::Str(Arc::new(yaml_str)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("to_yaml requires a value argument".to_string())
            }
        }

        "parse_csv" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("parse_csv requires a string argument".to_string()));
            }

            if let Some(Value::Str(csv_str)) = arg_values.first() {
                match builtins::parse_csv(csv_str.as_ref()) {
                    Ok(value) => value,
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("parse_csv requires a string argument".to_string())
            }
        }

        "to_csv" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("to_csv requires an array argument".to_string()));
            }

            if let Some(value) = arg_values.first() {
                match builtins::to_csv(value) {
                    Ok(csv_str) => Value::Str(Arc::new(csv_str)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("to_csv requires an array argument".to_string())
            }
        }

        "encode_base64" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "encode_base64 requires a bytes or string argument".to_string(),
                ));
            }

            match arg_values.first() {
                Some(Value::Bytes(bytes)) => Value::Str(Arc::new(builtins::encode_base64(bytes))),
                Some(Value::Str(s)) => {
                    Value::Str(Arc::new(builtins::encode_base64(s.as_ref().as_bytes())))
                }
                _ => Value::Error("encode_base64 requires a bytes or string argument".to_string()),
            }
        }

        "encode_uri_component" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "encode_uri_component requires one string argument".to_string(),
                ));
            }
            if let Some(Value::Str(value)) = arg_values.first() {
                Value::Str(Arc::new(builtins::encode_uri_component(value.as_ref())))
            } else {
                Value::Error("encode_uri_component requires one string argument".to_string())
            }
        }

        "decode_base64" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("decode_base64 requires a string argument".to_string()));
            }

            if let Some(Value::Str(s)) = arg_values.first() {
                match builtins::decode_base64(s.as_ref()) {
                    Ok(bytes) => Value::Bytes(bytes),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("decode_base64 requires a string argument".to_string())
            }
        }

        "decode_base64_utf8" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "decode_base64_utf8 requires a string argument".to_string(),
                ));
            }

            if let Some(Value::Str(s)) = arg_values.first() {
                match builtins::decode_base64_utf8(s.as_ref()) {
                    Ok(text) => Value::Str(Arc::new(text)),
                    Err(error) => Value::Error(error),
                }
            } else {
                Value::Error("decode_base64_utf8 requires a string argument".to_string())
            }
        }

        "decode_charset" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(
                    "decode_charset requires bytes, charset label, and maximum output bytes"
                        .to_string(),
                ));
            }
            match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                (Value::Bytes(bytes), Value::Str(label), Value::Int(max_output_bytes)) => {
                    match builtins::decode_charset(bytes, label.as_ref(), *max_output_bytes) {
                        Ok(text) => Value::Str(Arc::new(text)),
                        Err(error) => Value::Error(error),
                    }
                }
                _ => Value::Error(
                    "decode_charset requires bytes, charset label, and maximum output bytes"
                        .to_string(),
                ),
            }
        }

        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::interpreter::{DictMap, Value};
    use std::sync::Arc;

    fn string_value(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    #[test]
    fn test_parse_json_and_to_json_round_trip() {
        let parse_result =
            handle("parse_json", &[string_value("{\"name\":\"kujo\",\"n\":2}")]).unwrap();
        match parse_result {
            Value::Dict(map) => {
                assert!(map.contains_key("name"));
                assert!(map.contains_key("n"));
            }
            other => panic!("Expected Value::Dict from parse_json, got {:?}", other),
        }

        let mut dict = DictMap::default();
        dict.insert(Arc::<str>::from("ok"), Value::Bool(true));
        let to_json_result = handle("to_json", &[Value::Dict(Arc::new(dict))]).unwrap();
        match to_json_result {
            Value::Str(json) => assert!(json.contains("\"ok\":true")),
            other => panic!("Expected Value::Str from to_json, got {:?}", other),
        }

        let mut pretty_dict = DictMap::default();
        pretty_dict.insert(Arc::<str>::from("ok"), Value::Bool(true));
        let to_json_pretty_result =
            handle("to_json_pretty", &[Value::Dict(Arc::new(pretty_dict))]).unwrap();
        match to_json_pretty_result {
            Value::Str(json) => {
                assert!(json.contains("\"ok\": true"));
                assert!(json.contains("\n"));
            }
            other => panic!("Expected Value::Str from to_json_pretty, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_toml_and_to_toml() {
        let parse_result = handle("parse_toml", &[string_value("title = \"Kujo\"")]).unwrap();
        match parse_result {
            Value::Dict(map) => {
                assert!(map.contains_key("title"));
            }
            other => panic!("Expected Value::Dict from parse_toml, got {:?}", other),
        }

        let mut dict = DictMap::default();
        dict.insert(Arc::<str>::from("title"), string_value("Kujo"));
        let to_toml_result = handle("to_toml", &[Value::Dict(Arc::new(dict))]).unwrap();
        match to_toml_result {
            Value::Str(toml) => assert!(toml.contains("title = \"Kujo\"")),
            other => panic!("Expected Value::Str from to_toml, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_yaml_and_to_yaml() {
        let parse_result = handle("parse_yaml", &[string_value("name: Kujo")]).unwrap();
        match parse_result {
            Value::Dict(map) => {
                assert!(map.contains_key("name"));
            }
            other => panic!("Expected Value::Dict from parse_yaml, got {:?}", other),
        }

        let mut dict = DictMap::default();
        dict.insert(Arc::<str>::from("name"), string_value("Kujo"));
        let to_yaml_result = handle("to_yaml", &[Value::Dict(Arc::new(dict))]).unwrap();
        match to_yaml_result {
            Value::Str(yaml) => assert!(yaml.contains("name")),
            other => panic!("Expected Value::Str from to_yaml, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_csv_and_to_csv() {
        let parse_result = handle("parse_csv", &[string_value("name,age\nKujo,2")]).unwrap();
        match parse_result {
            Value::Array(rows) => assert_eq!(rows.len(), 1),
            other => panic!("Expected Value::Array from parse_csv, got {:?}", other),
        }

        let mut row = DictMap::default();
        row.insert(Arc::<str>::from("name"), string_value("Kujo"));
        row.insert(Arc::<str>::from("age"), Value::Int(2));
        let rows = Value::Array(Arc::new(vec![Value::Dict(Arc::new(row))]));
        let to_csv_result = handle("to_csv", &[rows]).unwrap();
        match to_csv_result {
            Value::Str(csv) => {
                assert!(csv.contains("name"));
                assert!(csv.contains("Kujo"));
            }
            other => panic!("Expected Value::Str from to_csv, got {:?}", other),
        }
    }

    #[test]
    fn test_base64_encode_decode() {
        let encode_from_string = handle("encode_base64", &[string_value("kujo")]).unwrap();
        match encode_from_string {
            Value::Str(encoded) => {
                let decode_result = handle("decode_base64", &[Value::Str(encoded)]).unwrap();
                match decode_result {
                    Value::Bytes(bytes) => assert_eq!(bytes, b"kujo"),
                    other => panic!("Expected Value::Bytes from decode_base64, got {:?}", other),
                }
            }
            other => panic!("Expected Value::Str from encode_base64, got {:?}", other),
        }
    }

    #[test]
    fn test_base64_utf8_decode_is_strict() {
        let decoded = handle("decode_base64_utf8", &[string_value("a3VqbyDimIM=")]).unwrap();
        assert!(matches!(decoded, Value::Str(value) if value.as_ref() == "kujo ☃"));

        let invalid_utf8 = handle("decode_base64_utf8", &[string_value("/w==")]).unwrap();
        assert!(
            matches!(invalid_utf8, Value::Error(message) if message == "Base64 decoded value is not valid UTF-8")
        );

        let malformed = handle("decode_base64_utf8", &[string_value("%%")]).unwrap();
        assert!(
            matches!(malformed, Value::Error(message) if message.starts_with("Base64 decode error:"))
        );
    }

    #[test]
    fn test_charset_decode_is_strict_bounded_and_label_explicit() {
        let latin1 = handle(
            "decode_charset",
            &[Value::Bytes(b"caf\xe9".to_vec()), string_value("iso-8859-1"), Value::Int(16)],
        )
        .unwrap();
        assert!(matches!(latin1, Value::Str(value) if value.as_ref() == "café"));

        let windows = handle(
            "decode_charset",
            &[
                Value::Bytes(b"smart \x91quotes\x92".to_vec()),
                string_value("windows-1252"),
                Value::Int(64),
            ],
        )
        .unwrap();
        assert!(matches!(windows, Value::Str(value) if value.as_ref() == "smart ‘quotes’"));

        let invalid = handle(
            "decode_charset",
            &[Value::Bytes(vec![0xff]), string_value("us-ascii"), Value::Int(8)],
        )
        .unwrap();
        assert!(
            matches!(invalid, Value::Error(message) if message.contains("invalid for the selected charset"))
        );

        let bounded = handle(
            "decode_charset",
            &[Value::Bytes(b"caf\xe9".to_vec()), string_value("iso-8859-1"), Value::Int(4)],
        )
        .unwrap();
        assert!(
            matches!(bounded, Value::Error(message) if message.contains("output exceeds configured limit"))
        );
    }

    #[test]
    fn test_encode_uri_component_rfc3986_and_utf8() {
        let cases = [
            ("AZaz09-._~", "AZaz09-._~"),
            ("a b+c&d=e?f#:/", "a%20b%2Bc%26d%3De%3Ff%23%3A%2F"),
            ("café", "caf%C3%A9"),
            ("東京", "%E6%9D%B1%E4%BA%AC"),
            ("💡", "%F0%9F%92%A1"),
            ("%2F", "%252F"),
        ];
        for (input, expected) in cases {
            let result = handle("encode_uri_component", &[string_value(input)]).unwrap();
            assert!(matches!(result, Value::Str(value) if value.as_ref() == expected));
        }
    }

    #[test]
    fn test_data_format_argument_validation_errors() {
        let parse_json_error = handle("parse_json", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(parse_json_error, Value::Error(message) if message.contains("parse_json requires a string argument"))
        );

        let decode_base64_error = handle("decode_base64", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(decode_base64_error, Value::Error(message) if message.contains("decode_base64 requires a string argument"))
        );

        let decode_base64_utf8_error = handle("decode_base64_utf8", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(decode_base64_utf8_error, Value::Error(message) if message.contains("decode_base64_utf8 requires a string argument"))
        );

        let encode_base64_error = handle("encode_base64", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(encode_base64_error, Value::Error(message) if message.contains("encode_base64 requires a bytes or string argument"))
        );
    }

    #[test]
    fn test_data_format_and_base64_strict_arity_rejects_extra_arguments() {
        let parse_json_extra = handle("parse_json", &[string_value("{}"), Value::Int(1)]).unwrap();
        assert!(
            matches!(parse_json_extra, Value::Error(message) if message.contains("parse_json requires a string argument"))
        );

        let to_json_extra = handle("to_json", &[Value::Bool(true), Value::Int(1)]).unwrap();
        assert!(
            matches!(to_json_extra, Value::Error(message) if message.contains("to_json requires a value argument"))
        );

        let to_json_pretty_extra =
            handle("to_json_pretty", &[Value::Bool(true), Value::Int(1)]).unwrap();
        assert!(
            matches!(to_json_pretty_extra, Value::Error(message) if message.contains("to_json_pretty requires a value argument"))
        );

        let parse_toml_extra =
            handle("parse_toml", &[string_value("title='x'"), Value::Int(1)]).unwrap();
        assert!(
            matches!(parse_toml_extra, Value::Error(message) if message.contains("parse_toml requires a string argument"))
        );

        let to_toml_extra = handle("to_toml", &[Value::Bool(true), Value::Int(1)]).unwrap();
        assert!(
            matches!(to_toml_extra, Value::Error(message) if message.contains("to_toml requires a value argument"))
        );

        let parse_yaml_extra =
            handle("parse_yaml", &[string_value("name: x"), Value::Int(1)]).unwrap();
        assert!(
            matches!(parse_yaml_extra, Value::Error(message) if message.contains("parse_yaml requires a string argument"))
        );

        let to_yaml_extra = handle("to_yaml", &[Value::Bool(true), Value::Int(1)]).unwrap();
        assert!(
            matches!(to_yaml_extra, Value::Error(message) if message.contains("to_yaml requires a value argument"))
        );

        let parse_csv_extra =
            handle("parse_csv", &[string_value("a,b\n1,2"), Value::Int(1)]).unwrap();
        assert!(
            matches!(parse_csv_extra, Value::Error(message) if message.contains("parse_csv requires a string argument"))
        );

        let to_csv_extra =
            handle("to_csv", &[Value::Array(Arc::new(vec![])), Value::Int(1)]).unwrap();
        assert!(
            matches!(to_csv_extra, Value::Error(message) if message.contains("to_csv requires an array argument"))
        );

        let encode_base64_extra =
            handle("encode_base64", &[string_value("kujo"), Value::Int(1)]).unwrap();
        assert!(
            matches!(encode_base64_extra, Value::Error(message) if message.contains("encode_base64 requires a bytes or string argument"))
        );

        let decode_base64_extra =
            handle("decode_base64", &[string_value("cnVmZg=="), Value::Int(1)]).unwrap();
        assert!(
            matches!(decode_base64_extra, Value::Error(message) if message.contains("decode_base64 requires a string argument"))
        );
    }
}
