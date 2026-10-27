//! Captured read projection must preserve ownership and fallback boundaries.
fn verify(mode: &str) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("read_state.kujo"),
        r#"
        mut state := [7, 8]
        mut scalar := 10
        export func read(i) { return state[i] }
        export func snapshot() { return state }
        export func computed(i) { return scalar + i }
        export func copied(i) { let local := state return local[i] }
        export func bump() { state[0] += 1 scalar += 2 }
        export func reentry() { state[0] += 3 return read(0) }
        export func missing() { return only_caller }
    "#,
    )
    .unwrap();
    let path = dir.path().join("main.kujo");
    std::fs::write(
        &path,
        r#"
        from read_state import read, snapshot, computed, copied, bump, reentry, missing
        let only_caller := 900
        let old := snapshot()
        assert_equal(read(0), 7)
        assert_equal(copied(1), 8)
        assert_equal(computed(4), 14)
        bump()
        assert_equal(old[0], 7)
        assert_equal(read(0), 8)
        assert_equal(computed(4), 16)
        assert_equal(reentry(), 11)
        assert_equal(read(0), 11)
        mut caught := 0
        try { missing() } except e { caught += 1 }
        try { read(99) } except e { caught += 1 }
        assert_equal(caught, 2)
        assert_equal(read(0), 11)
        assert_equal(old[0], 7)
        print("READ PROJECTION PASS")
    "#,
    )
    .unwrap();
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.arg("run").arg(path);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "READ PROJECTION PASS\n", "{mode}");
}
#[test]
fn vm_read_projection_preserves_capture_boundaries() {
    verify("vm");
}
#[test]
fn interpreter_read_projection_preserves_capture_boundaries() {
    verify("interpreter");
}
#[test]
fn jit_read_projection_preserves_capture_boundaries() {
    verify("jit");
}
