use std::process::Command;

#[test]
fn owned_local_stores_preserve_recursive_snapshots_branches_and_errors() {
    let source = r#"
func recur(input, depth) {
    mut state := input
    if depth == 0 { return state }
    let before := state
    state = recur(state, depth - 1)
    state[0][0] += 1
    assert_equal(before[0][0], 2)
    return state
}
func exercise(input, choose) {
    mut current := input
    let snapshot := current
    mut i := 0
    while i < 12 {
        if choose { current = [[i], [9]] }
        else { current = [[i + 1], [8]] }
        let old := current
        current[0][0] += 1
        assert_equal(old[0][0], i + (1 - choose))
        i += 1
    }
    assert_equal(snapshot, [[2], [3]])
    return current
}
func fail_after_store(input) {
    mut data := input
    data = [[99]]
    return data[999]
}
let original := [[2], [3]]
mut n := 0
while n < 160 {
    assert_equal(recur(original, 5), [[7], [3]])
    assert_equal(exercise(original, 1), [[12], [9]])
    assert_equal(exercise(original, 0), [[13], [8]])
    mut caught := 0
    try { fail_after_store(original) } except error { caught = 1 }
    assert_equal(caught, 1)
    assert_equal(original, [[2], [3]])
    n += 1
}
print("local-store-transfer: OK")
"#;
    let path = std::env::temp_dir().join(format!("kujo-local-store-{}.kujo", std::process::id()));
    std::fs::write(&path, source).unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "local-store-transfer: OK");
    }
    std::fs::remove_file(path).unwrap();
}
