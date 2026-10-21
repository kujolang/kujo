#[path = "support/native_api.rs"]
mod support;

#[test]
fn finite_float_read_indices_match_integer_truncation_in_both_runtimes() {
    support::success(
        r#"
        let data := [10, 20, 30]
        assert_equal(data[max(0, 1 - 1)], 10)
        assert_equal(data[1.9], 20)
        assert_equal(data[-1.9], 30)
        assert_equal(data[-0.9], 10)
        assert_equal(bytes([10, 20, 30])[-1.9], 30)
        assert_equal("aé雪"[1.9], "é")
        assert_equal("aé雪"[-1.9], "雪")
        mut caught := 0
        for index in [3.1, -4.1, to_float("1e30")] {
            try { data[index] } except e { caught += 1 }
        }
        assert_equal(caught, 3)
    "#,
    );
    support::failure("let x := [1][parse_float(\"NaN\")]", "Invalid index operation");
    support::failure("let x := [1][parse_float(\"inf\")]", "Invalid index operation");
}

#[test]
fn float_indices_do_not_relax_dictionary_or_write_contracts() {
    support::failure("let x := {\"0\": 1}[0.0]", "Invalid index operation");
    support::failure("mut x := [1] x[0.0] = 2", "index");
    support::failure("mut x := {\"0\": 1} x[0.0] = 2", "index");
    support::failure("mut x := {\"0\": 1} x[true] = 2", "index");
    support::success(
        r#"
        mut array := [1]
        mut map := {"0": 1}
        let old_array := array
        let old_map := map
        mut caught := 0
        try { array[0.5] = 9 } except e { caught += 1 }
        try { map[0.5] = 9 } except e { caught += 1 }
        assert_equal(caught, 2)
        assert_equal(array, old_array)
        assert_equal(map, old_map)
    "#,
    );
}

#[test]
fn type_checker_accepts_numeric_string_read_indices() {
    let mut parser =
        kujo::parser::Parser::new(kujo::lexer::tokenize("let x := \"aé雪\"[1.9]").unwrap());
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty());
    kujo::type_checker::TypeChecker::new().check(&parsed.stmts).unwrap();
}
