use std::process::Command;

#[test]
fn imported_setup_preserves_permissions_state_recursion_and_callbacks_in_all_runtimes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("setup.kujo"),
        r#"
        mut count := 0
        const locked := [7]
        export func bump() { count += 1 return count }
        export func recurse(n) { if n == 0 { return locked[0] } return recurse(n - 1) }
        export func reject() { locked[0] = 9 }
        export func invoke(callback) { return callback(locked) }
        export func factory(value) {
            mut items := [value]
            let saved := func() { return items[0] }
            items[0] = 99
            return saved
        }
    "#,
    )
    .unwrap();
    let main = dir.path().join("main.kujo");
    std::fs::write(
        &main,
        r#"
        from setup import bump
        from setup import recurse
        from setup import reject
        from setup import invoke
        from setup import factory
        const count := 900
        func edit(items) { items[0] = 88 return items[0] }
        mut i := 0
        while i < 16 {
            assert_equal(bump(), i + 1)
            assert_equal(recurse(5), 7)
            mut caught := false
            try { reject() } except error { caught = true }
            assert(caught)
            assert_equal(invoke(edit), 88)
            assert_equal(recurse(0), 7)
            let saved := factory(i)
            assert_equal(saved(), i)
            i += 1
        }
        assert_equal(count, 900)
    "#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&main).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    }
}
