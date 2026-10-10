//! Representation matrix: the same scripts must meet independent expectations in both engines.
use std::process::Command;

fn run(source: &str, interpreter: bool, flags: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("matrix.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(path).args(flags);
    if interpreter {
        command.arg("--interpreter");
    }
    command.output().unwrap()
}

fn success(source: &str, expected: &str) {
    for interpreter in [false, true] {
        let output = run(source, interpreter, &["--untrusted"]);
        assert!(
            output.status.success(),
            "interpreter={interpreter}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            expected,
            "interpreter={interpreter}"
        );
    }
}

#[test]
fn dictionary_lengths_include_fixed_literals_and_native_aliases() {
    success(
        r#"let size := len
print(size({"a":1,"b":2}))
print(len(parse_json("{}")))"#,
        "2\n0",
    );
}

#[test]
fn specialized_dictionary_serializers_and_schema_preserve_nested_values() {
    for setup in [
        "mut d := {}; d[0] = 7; d[1] = 8",
        "mut d := {}; d[0] = 7; d[1] = null",
        "mut d := {}; d[-1] = 7",
    ] {
        for format in ["json", "yaml", "toml"] {
            // TOML's existing null-to-empty-string conversion is intentional.
            let comparison = if format == "toml" {
                "to_toml(decoded) == encoded"
            } else {
                "decoded == {\"nested\":d}"
            };
            success(&format!("{setup}\nlet encoded := to_{format}({{\"nested\":d}})\nlet decoded := parse_{format}(encoded)\nprint({comparison})"), "true");
        }
        success(&format!("{setup}\nlet checked := json_schema_validate({{\"nested\":d}}, {{\"type\":\"object\",\"properties\":{{\"nested\":{{\"type\":\"object\"}}}}}})\nprint(checked[\"valid\"])"), "true");
    }
}

#[test]
fn merge_update_clear_accept_mixed_dictionary_representations_without_mutation() {
    for operation in ["merge", "update"] {
        success(
            &format!(
                r#"mut d := {{}}
d[0] = 7
d[1] = 8
let before := d
let merged := {operation}(d, {{"1":9,"extra":10}})
print(merged == {{"0":7,"1":9,"extra":10}})
print(d == before)
print(clear(d) == {{}})
print(len(d))"#
            ),
            "true\ntrue\ntrue\n2",
        );
    }
}

#[test]
fn sparse_integer_writes_do_not_create_phantom_entries() {
    for body in [
        "mut d := {}; d[2] = 7",
        "mut d := {}; d[0] = 1; d[2] = 7; d = remove(d, \"0\")[0]",
        "mut d := {}; d[2] = null; d[2] = 7",
    ] {
        let assertions = r#"print(len(d)); print(keys(d)); print(values(d)); print(has_key(d,"0")); print(get(d,"0",99)); print(d == {"2":7})"#;
        for source in [
            format!("{body}\n{assertions}"),
            format!("func probe() {{ {body}\n{assertions} }}\nprobe()"),
        ] {
            success(&source, "1\n[2]\n[7]\n0\n99\ntrue");
        }
    }
}

#[test]
fn null_dictionary_entries_are_present_and_participate_in_equality() {
    success(
        r#"mut d := {}; d[0] = null
print(d == {"0":null})
print(d == {})
print(unique([d, {"0":null}, {}]) == [{"0":null}, {}])"#,
        "true\nfalse\ntrue",
    );
}

#[test]
fn sparse_optimized_fill_retains_only_written_keys() {
    success(
        r#"func probe() {
mut d := {}
mut i := 2
let stop := 4
while i < stop { d[i] = i * 2; i += 1 }
print(len(d))
print(keys(d))
print(values(d))
}
probe()"#,
        "2\n[2, 3]\n[4, 6]",
    );
}

#[test]
fn arrays_bytes_secrets_and_numeric_boundaries_preserve_contracts() {
    success(
        r#"let bytes := decode_base64("AP9B")
print(byte_length(bytes))
print(encode_base64(slice(bytes,1,3)))
print(to_json({"nested":[secret("DO_NOT_LEAK"),9223372036854775807]}))
print(sort([9007199254740993,9007199254740992.0])[1] == 9007199254740993)
print(json_schema_validate(9007199254740993,{"const":9007199254740992.0})["valid"])
print(unique([[1],[2],[1]]))"#,
        "3\n/0E=\n{\"nested\":[\"***\",9223372036854775807]}\ntrue\nfalse\n[[1], [2]]",
    );
}

#[test]
fn errors_and_capability_denial_keep_runtime_exit_class() {
    for (source, message) in [
        ("to_json(decode_base64(\"AA==\"))", "Cannot convert"),
        ("sum([9223372036854775807,1])", "sum() integer overflow"),
        ("json_schema_validate(null,{\"minimum\":\"bad\"})", "json_schema_validate()"),
        ("read_file(\"missing-parity-file\")", "Capability denied"),
        ("mut d := {}; d[0] = 1; let frozen := d; frozen[0] = 2", "immutable"),
    ] {
        for interpreter in [false, true] {
            let output = run(source, interpreter, &["--untrusted"]);
            assert_eq!(
                output.status.code(),
                Some(4),
                "{source}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(message),
                "{source}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
#[cfg(feature = "runtime-network")]
fn ai_construction_hashes_and_token_estimates_accept_literal_and_parsed_objects() {
    success(
        r#"let literal := [{"role":"user","content":"hello","name":"dev"}]
let parsed := parse_json(to_json(literal))
let options := {"model":"gpt-test","endpoint":"https://example.test/v1/chat/completions"}
print(ai_request_hash(literal,options) == ai_request_hash(parsed,parse_json(to_json(options))))
print(ai_count_tokens(literal,options) == ai_count_tokens(parsed,options))
print(ai_fit_context(literal,100,options) == ai_fit_context(parsed,100,options))
print(ai_message("user",[ai_text("hello")]) == {"role":"user","content":[{"type":"text","text":"hello"}]})"#,
        "true\ntrue\ntrue\ntrue",
    );
}

#[test]
fn integer_dictionary_contains_has_boolean_result() {
    success(
        r#"mut d := {}; d[0] = 7
print(contains(d,"0"))
print(contains(d,"missing"))
print(type(contains(d,"0")))"#,
        "true\nfalse\nbool",
    );
}

#[test]
fn collection_callbacks_and_array_operations_share_behavior() {
    success(r#"func twice(x) { return x * 2 }
func large(x) { return x > 2 }
func add(a,b) { return a + b }
print(map([1,2,3],twice))
print(filter([1,2,3],large))
print(reduce([1,2,3],0,add))
print(find([1,2,3],large))
print(any([1,2,3],large))
print(all([1,2,3],large))
print(concat(take([1,2,3],2),skip([1,2,3],2)))
print(flatten(chunk([1,2,3],2)))
print(reverse([1,2,3]))
print(zip([1,2],[3,4]))
print(enumerate(["a","b"]))"#, "[2, 4, 6]\n[3]\n6\n3\ntrue\nfalse\n[1, 2, 3]\n[1, 2, 3]\n[3, 2, 1]\n[[1, 3], [2, 4]]\n[[0, a], [1, b]]");
}

#[test]
fn numeric_looking_string_keys_retain_their_exact_identity() {
    success(
        r#"mut d := {}; d[0] = 7
print(has_key(d,"00"))
print(get(d,"+0",99))
d["00"] = 8
print(d[0])
print(d["00"])
print(len(d))"#,
        "0\n99\n7\n8\n2",
    );
}

#[test]
fn explicit_null_slots_are_readable_by_integer_and_string_key() {
    success(
        r#"mut d := {}; d[0] = null
print(d[0]); print(d["0"])
print(get(d,"0",99)); print(has_key(d,"0"))"#,
        "null\nnull\nnull\n1",
    );
}

#[test]
fn equal_dictionaries_deduplicate_across_representations() {
    success(
        r#"mut d := {}; d[0] = 7
print(len(unique([d,{"0":7}])))
print(len(unique([[d],[{"0":7}]])))"#,
        "1\n1",
    );
}

#[test]
fn huge_sparse_keys_do_not_require_dense_allocation() {
    // The small-gap test is the safe baseline reproduction. This regression
    // exercises the same repaired boundary without attempting a baseline OOM.
    success(
        r#"mut d := {}; d[9223372036854775807] = 7
print(len(d)); print(d[9223372036854775807])
func probe() { mut d := {}; d[0] = 1; d[9223372036854775807] = 7; print(len(d)) }
probe()"#,
        "1\n7\n2",
    );
}

#[test]
fn absent_string_keys_keep_the_missing_key_error() {
    for key in ["00", "+0", "-0", "missing"] {
        for interpreter in [false, true] {
            let source = format!("mut d := {{}}; d[0] = 7\nprint(d[\"{key}\"])");
            let output = run(&source, interpreter, &["--untrusted"]);
            assert_eq!(output.status.code(), Some(4));
            assert!(String::from_utf8_lossy(&output.stderr)
                .contains(&format!("Missing map key: \"{key}\"")));
        }
    }
}

#[test]
fn numeric_helper_keys_are_supported_without_type_warnings_in_both_engines() {
    for interpreter in [false, true] {
        let output = run(
            r#"mut d := {}; d[0] = 7
print(get(d,0,99)); print(has_key(d,0)); print(get_default(d,0,99))
print(remove(d,0))
print(get({"0":8},0,99)); print(get_default(d,1,99))"#,
            interpreter,
            &["--untrusted"],
        );
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "7\n1\n7\n[{}, 7]\n8\n99");
        assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn schema_properties_and_refs_accept_integer_keyed_schema_maps() {
    success(
        r##"mut properties := {}; properties[0] = {"type":"integer"}
let schema := {"type":"object","properties":properties,"additionalProperties":false}
print(json_schema_validate({"0":7},schema)["valid"])
print(json_schema_validate({"0":"bad"},schema)["valid"])
print(json_schema_validate(7,{"$defs":properties,"$ref":"#/$defs/0"})["valid"])
print(json_schema_validate(null,clear(properties))["valid"])
print(ai_count_tokens("abcd",clear(properties)))"##,
        "true\nfalse\ntrue\ntrue\n1",
    );
}
