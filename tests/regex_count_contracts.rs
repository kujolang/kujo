#[path = "support/native_api.rs"]
mod support;

#[test]
fn count_matches_without_materializing_match_strings() {
    support::success(
        r#"
        assert_equal(regex_count("a12 b34", "[0-9]+"), 2)
        assert_equal(regex_count("é🙂", ""), 3)
        assert_equal(regex_count("", ""), 1)
        assert_equal(regex_count("none", "[0-9]+"), 0)
        assert_equal(regex_count("CAFÉ café", "(?i)café"), 2)
        assert_equal(regex_count("aaa", "aa"), 1)
        let count := regex_count
        assert_equal(count("a1 b22", "[0-9]+"), 2)
        mut i := 0
        while i < 32 {
            let pattern := "a{" + str(i) + "}"
            assert_equal(count("aaaa", pattern), len(regex_find_all("aaaa", pattern)))
            i += 1
        }
    "#,
    );
}

#[test]
fn regex_count_validates_arguments_and_registration() {
    for source in ["regex_count(1, \"a\")", "regex_count(\"a\", 1)", "regex_count(\"a\", \"[\")"] {
        support::failure(source, "regex_count");
    }
    support::failure("regex_count(\"a\")", "argument");
    support::failure("regex_count(\"a\", \"a\", 1)", "argument");
    assert!(kujo::interpreter::Interpreter::is_builtin_name("regex_count"));
    let arity = kujo::interpreter::Interpreter::native_function_arity("regex_count").unwrap();
    assert!(arity.validate(2).is_ok());
    assert!(arity.validate(3).is_err());
}
