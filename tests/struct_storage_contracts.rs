#[path = "support/native_api.rs"]
mod support;

#[test]
fn struct_reads_and_nested_updates_preserve_value_snapshots() {
    support::success(
        r#"
        struct Node { left: any, right: any }
        func make(depth) {
            if depth == 0 { return Node { left: null, right: null } }
            return Node { left: make(depth-1), right: make(depth-1) }
        }
        func count(node) {
            if node.left == null { return 1 }
            return 1 + count(node.left) + count(node.right)
        }
        func edit(node) { mut changed := node changed.left.right = null return changed }
        func work() {
        mut root := make(8)
        let original := root
        let snapshot := func() { return root }
        root = edit(root)
        assert_equal(count(original), 511)
        assert_equal(count(snapshot()), 511)
        assert_equal(root.left.right, null)
        assert(original.left.right != null)
        mut rows := [original]
        rows[0].left.left = null
        assert(original.left.left != null)
        let frozen := original
        mut caught := false
        try { frozen.left.left = null } except err { caught = true }
        assert(caught)
        assert_equal(count(frozen), 511)
        }
        work()
    "#,
    );
}

#[test]
fn imported_struct_updates_detach_shared_fields() {
    let module = r#"
        export func edit(node) {
            mut result := node
            result.left[0] = 9
            return result
        }
    "#;
    let source = r#"
        from tree import edit
        struct Node { left: any, right: any }
        mut root := Node { left: [1, 2], right: null }
        let old := root
        root = edit(root)
        assert_equal(root.left, [9, 2])
        assert_equal(old.left, [1, 2])
    "#;
    for interpreter in [false, true] {
        let out = support::run_modules(source, interpreter, &[("tree.kujo", module)]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
}

#[test]
#[ignore = "manual depth-20 resource probe; up to ten minutes per runtime"]
fn full_depth_struct_tree_resource_probe() {
    let source = include_str!("fixtures/collection_profiles/object_tree.txt")
        .replace("make(12)", "make(20)")
        .replace("8191", "2097151");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tree.kujo");
    std::fs::write(&path, source).unwrap();
    for interpreter in [false, true] {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_kujo"));
        cmd.arg("run").arg(&path).arg("--allow-clock").arg("--scheduler-timeout-ms").arg("600000");
        if interpreter {
            cmd.arg("--interpreter");
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        println!(
            "interpreter={interpreter}: construction/traversal ns {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}
