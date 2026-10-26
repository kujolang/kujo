use std::process::Command;

#[test]
fn byte_materialization_preserves_inputs_validation_and_snapshots_in_all_runtimes() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("bytes.kujo");
    std::fs::write(
        &source,
        r#"
        let source := [0, 127, 255]
        mut result := bytes(source)
        let snapshot := result
        result[0] = 42
        assert_equal(source, [0, 127, 255])
        assert_equal(snapshot, bytes([0, 127, 255]))
        assert_equal(result, bytes([42, 127, 255]))
        assert_equal(byte_length(bytes([])), 0)
        mut caught := 0
        try { bytes([0, -1]) } except e { caught += 1 }
        try { bytes([0, 256]) } except e { caught += 1 }
        try { bytes([0, 1.5]) } except e { caught += 1 }
        try { bytes([0, true]) } except e { caught += 1 }
        try { bytes([0, "1"]) } except e { caught += 1 }
        try { bytes("hello") } except e { caught += 1 }
        assert_equal(caught, 6)
        assert_equal(source, [0, 127, 255])
        assert_equal(utf8_decode(utf8_encode("é☃")), "é☃")
    "#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&source).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    }
}
