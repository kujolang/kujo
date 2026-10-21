#[path = "support/native_api.rs"]
mod support;

#[test]
fn cached_string_access_preserves_unicode_indices_and_mutated_aliases() {
    support::success(
        r#"
        mut text := repeat("a", 100)
        assert_equal(len(text), 100)
        let alias := text
        text += "é🙂"
        assert_equal(len(text), 102)
        assert_equal(count_chars(text), 102)
        assert_equal(text[100], "é")
        assert_equal(text[-1], "🙂")
        assert_equal(text[100.9], "é")
        assert_equal(substring(text, 99, 999), "aé🙂")
        assert_equal(substr(text, 100, 102), "é🙂")
        assert_equal(substring(text, 999, 1000), "")
        assert_equal(alias[-1], "a")
        assert_equal(len(alias), 100)
        mut other := repeat("x", 100)
        assert_equal(len(other), 100)
        other += "é"
        assert_equal(len(other), 101)
        assert_equal(other[-1], "é")
        assert_equal(substring(alias, 20, 22), "aa")
        assert_equal(len("é🙂"), 3)
        assert_equal("é🙂"[1], "́")
    "#,
    );
}

#[test]
fn cached_string_bounds_remain_checked() {
    support::failure("let s := repeat(\"a\", 100)\nprint(s[-101])", "Index out of bounds");
    support::failure("let s := repeat(\"é\", 100)\nprint(s[100])", "Index out of bounds");
    support::failure("substring(repeat(\"x\", 100), 3, 2)", "end must be greater");
}
