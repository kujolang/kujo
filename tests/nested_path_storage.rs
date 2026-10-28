use std::process::Command;

fn verify(mode: &str) {
    let dir = tempfile::tempdir().unwrap();
    let mut source = String::new();
    for depth in 2..=8 {
        let mut seed = "[1, 2]".to_string();
        for _ in 1..depth {
            seed = format!("[{seed}]");
        }
        let path = "[-1]".repeat(depth);
        let prefix = "[-1]".repeat(depth - 1);
        source.push_str(&format!(
            r#"
func update_{depth}(input) {{
    mut state := input
    mut i := 0
    while i < 240 {{
        let before := state
        state{path} = i
        if i == 0 {{ assert_equal(before{path}, 2) }}
        else {{ assert_equal(before{path}, i - 1) }}
        i += 1
    }}
    let before_error := state
    mut caught := false
    try {{ state{prefix}["bad"] = 17 }} except err {{ caught = true }}
    assert(caught)
    assert_equal(state, before_error)
    return state
}}
let original_{depth} := {seed}
let changed_{depth} := update_{depth}(original_{depth})
assert_equal(original_{depth}, {seed})
assert_equal(changed_{depth}{path}, 239)
"#
        ));
    }
    source.push_str(
        r#"
struct Cell { values: array }
mut mixed := {"outer": [Cell { values: [3, 4] }]}
let mixed_saved := mixed
mixed["outer"][0].values[-1] = 9
assert_equal(mixed_saved["outer"][0].values, [3, 4])
assert_equal(mixed["outer"][0].values, [3, 9])
mut deep := [mixed]
let deep_saved := deep
deep[0]["outer"][0].values[-1] = 10
assert_equal(deep_saved[0]["outer"][0].values, [3, 9])
assert_equal(deep[0]["outer"][0].values, [3, 10])
mut order := ""
mut rows := [[0, 1]]
func rhs() { order += "r" return 7 }
func outer() { order += "o" rows = [[3, 4]] return 0 }
func inner() { order += "i" return -1 }
rows[outer()][inner()] = rhs()
assert_equal(order, "roi")
assert_equal(rows, [[3, 7]])
from owner import rewrite
let imported_original := {"row": [[1, 2]]}
let imported_result := rewrite(imported_original)
assert_equal(imported_original, {"row": [[1, 2]]})
assert_equal(imported_result, {"row": [[1, 42]]})
print("NESTED PATH STORAGE PASS")
"#,
    );
    std::fs::write(
        dir.path().join("owner.kujo"),
        "export func rewrite(input) { mut data := input data[\"row\"][0][-1] = 42 return data }",
    )
    .unwrap();
    let path = dir.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--untrusted"]);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.arg(path).output().unwrap();
    assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"NESTED PATH STORAGE PASS\n");
}

#[test]
fn vm_nested_path_storage() {
    verify("vm");
}
#[test]
fn interpreter_nested_path_storage() {
    verify("interpreter");
}
#[test]
fn jit_nested_path_storage() {
    verify("jit");
}
