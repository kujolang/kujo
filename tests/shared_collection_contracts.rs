#[allow(dead_code)]
#[path = "support/native_api.rs"]
mod support;

#[test]
fn shared_collections_preserve_values_and_copy_isolation_in_both_runtimes() {
    support::success(
        r#"
        mut tree := [7]
        mut i := 0
        while i < 40 { tree = [tree, tree] i += 1 }
        mut cursor := tree
        i = 0
        while i < 40 { cursor = cursor[0] i += 1 }
        assert_equal(cursor[0], 7)
        cursor[0] = 9
        assert_equal(cursor[0], 9)
        cursor = tree
        i = 0
        while i < 40 { cursor = cursor[1] i += 1 }
        assert_equal(cursor[0], 7)
    "#,
    );
}
