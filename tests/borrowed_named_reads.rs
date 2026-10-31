use std::process::Command;

fn program(name: &str) -> String {
    r#"
mut __LONG__ := [1, 2]
func snapshot() { return __LONG__ }
func update(n) { __LONG__[0] = n return __LONG__ }
func sum_reads(n) {
    mut result := 0
    mut i := 0
    while i < n { result += __LONG__[0] i += 1 }
    return result
}
let original := __LONG__
let first := update(9)
assert_equal(sum_reads(64), 576)
update(12)
assert_equal(original, [1, 2])
assert_equal(first, [9, 2])
assert_equal(__LONG__, [12, 2])
let builtin := len
assert_equal(builtin(first), 2)
func length(data) { return len(data) }
assert_equal(length([1, 2, 3]), 3)
func call_named(callback) { return callback() }
let alias := snapshot
assert_equal(call_named(alias), [12, 2])
func factory(value) {
    mut local := value
    func read() { return local }
    return read
}
let left := factory(17)
let right := factory(23)
assert_equal(left(), 17)
assert_equal(right(), 23)
func map_read(value) { return value + __LONG__[0] }
assert_equal(map([1, 2], map_read), [13, 14])
assert_equal(original, [1, 2])
print("NAMED READ PASS")
"#
    .replace("__LONG__", name)
}

fn run(mode: &str, source: &str) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    command.arg(path).output().unwrap()
}

fn verify(mode: &str) {
    for length in [8, 256] {
        let name = format!("v{}", "x".repeat(length - 1));
        let output = run(mode, &program(&name));
        assert!(
            output.status.success(),
            "{mode}/{length}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"NAMED READ PASS\n");
    }
    let missing = format!("missing_{}", "x".repeat(120));
    for read in [missing.clone(), format!("func read() {{ return {missing} }}\nread()")] {
        let source = format!("print(\"BEFORE MISSING\")\n{read}\nprint(\"UNREACHABLE\")\n");
        let output = run(mode, &source);
        assert_eq!(output.status.code(), Some(4), "{mode}: expected runtime error");
        assert_eq!(output.stdout, b"BEFORE MISSING\n");
        assert!(String::from_utf8_lossy(&output.stderr).contains(&missing));
    }
}

#[test]
fn vm_named_reads_preserve_resolution_values_and_errors() {
    verify("vm");
}
#[test]
fn interpreter_named_reads_preserve_resolution_values_and_errors() {
    verify("interpreter");
}
#[test]
fn jit_named_reads_preserve_resolution_values_and_errors() {
    verify("jit");
}
