#[path = "support/native_api.rs"]
mod support;

#[test]
fn borrowed_serialization_preserves_output_and_snapshots_in_both_runtimes() {
    support::success(
        r#"
        let original := {"z": [1, true, null], "a": "café\n\""}
        let snapshot := original
        assert_equal(to_json(original), "{\"a\":\"café\\n\\\"\",\"z\":[1,true,null]}")
        assert_equal(parse_json(to_json_pretty(original)), original)
        assert_equal(original, snapshot)
        assert_equal(to_json(secret("not-for-output")), "\"***\"")
        mut numeric := {}
        numeric[2] = 2
        numeric[10] = 10
        assert_equal(to_json(numeric), "{\"10\":10,\"2\":2}")
        mut nested := 1
        mut depth := 0
        while depth < 64 { nested = [nested] depth += 1 }
        assert_equal(parse_json(to_json(nested)), nested)
    "#,
    );
}

#[test]
fn borrowed_serialization_keeps_errors_in_both_runtimes() {
    support::failure(
        r#"
        mut nested := 1
        mut depth := 0
        while depth < 65 { nested = [nested] depth += 1 }
        to_json(nested)
    "#,
        "JSON value exceeds maximum nesting depth of 64",
    );
    support::failure("to_json(bytes([0, 255]))", "Cannot convert");
}
