use std::process::Command;

#[test]
fn scalar_field_reads_preserve_tags_owned_results_and_legacy_parser_precedence() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fields.kujo");
    std::fs::write(
        &path,
        r#"
struct Holder { value: any }
struct ArgParser { help: any }
func read(item) {
    mut i := 0
    while i < 2 { i += 1 }
    return item.value
}
func read_help(item) {
    mut i := 0
    while i < 2 { i += 1 }
    return item.help
}
assert_equal(read(Holder { value: 0 }),0)
assert_equal(read(Holder { value: -17 }),-17)
assert_equal(read(Holder { value: true }),true)
assert_equal(read(Holder { value: false }),false)
assert_equal(read(Holder { value: null }),null)
assert_equal(read(Holder { value: -2.5 }),-2.5)
let original := Holder { value: [7,8] }
mut returned := read(original)
returned[0] = 99
assert_equal(original.value,[7,8])
assert_equal(returned,[99,8])
mut caught := 0
try { read(7) } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(read(Holder { value: 9 }),9)
print(type(read_help(ArgParser { help: 7 })))
print("PASS")
"#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let metrics = directory.path().join("metrics.json");
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").env_remove("DISABLE_FUNCTION_JIT");
        match mode {
            "interpreter" => {
                command.arg("--interpreter");
            }
            "jit" => {
                command.arg("--jit").arg("--measurements").arg(&metrics);
            }
            _ => {}
        }
        let output = command.arg(&path).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode}: {stderr}");
        // Preserve this existing divergence during the performance change:
        // VM/JIT prioritize legacy ArgParser methods; interpreter reads fields.
        let expected = if mode == "interpreter" { "int\nPASS\n" } else { "function\nPASS\n" };
        assert_eq!(output.stdout, expected.as_bytes(), "{mode}: {stderr}");
        if mode == "jit" {
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 9);
        }
    }
}
