#[path = "support/native_api.rs"]
mod support;

#[test]
fn string_rebinding_preserves_aliases_unicode_and_rhs_effects() {
    support::success(
        r#"
        func build() {
            mut text := "start"
            let alias := text
            let suffix := "é🙂"
            mut i := 0
            while i < 1000 { text += suffix i += 1 }
            assert_equal(alias, "start")
            assert_equal(len(text), 2005)
            let snapshot := text
            text += text
            assert_equal(text, snapshot + snapshot)
            mut small := "old"
            func change() { small = "replacement" return "!" }
            small += change()
            assert_equal(small, "old!")
            func append() { small += suffix return small }
            assert_equal(append(), "old!é🙂")
            assert_equal(small, "old!")
            let saved := small
            try { small += assert(false, "stop") } except err { }
            assert_equal(small, saved)
            mut number := 1
            let step := 2
            number += step
            assert_equal(number, 3)
        }
        build()
        mut changed := "before"
        func replace_target() { changed = "replacement" return "!" }
        changed += replace_target()
        assert_equal(changed, "before!")
        mut global := "a"
        let retained := global
        let tail := "b"
        global += tail
        assert_equal(global, "ab")
        assert_equal(retained, "a")
    "#,
    );
}

#[test]
fn string_append_does_not_bypass_immutable_binding_checks() {
    support::failure("let text := \"a\"\nlet suffix := \"b\"\ntext += suffix", "immutable");
    support::failure("const text := \"a\"\ntext += \"b\"", "const");
}

#[test]
fn dynamic_nonstring_suffix_preserves_type_errors() {
    support::failure(
        "mut text := \"a\"\nlet suffix := 1\ntext += suffix",
        "Invalid binary operation",
    );
}
