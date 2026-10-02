use kujo::native_aot::{compile_file_to_c, compile_file_to_executable};
use std::fs;
use std::process::Command;

#[test]
fn compiles_scalar_program_to_standalone_native_executable() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let source = workspace.path().join("sum.kujo");
    let executable = workspace.path().join("sum");
    fs::write(
        &source,
        r#"
func sum_to(limit) {
    mut total := 0
    mut current := 1
    while current <= limit {
        total = total + current
        current = current + 1
    }
    return total
}

print("sum=" + to_string(sum_to(100)))
"#,
    )
    .expect("write fixture");

    compile_file_to_executable(&source, &executable, None).expect("compile fixture");
    let output = Command::new(&executable).output().expect("run native fixture");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "sum=5050\n");
}

#[test]
fn rejects_unsupported_heap_values_instead_of_falling_back() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let source = workspace.path().join("array.kujo");
    fs::write(&source, "let values = [1, 2, 3]\nprint(values)\n").expect("write fixture");

    let error = compile_file_to_c(&source).expect_err("array compilation must fail");
    assert!(error.contains("native AOT"), "unexpected error: {error}");
}

#[test]
fn preserves_checked_integer_overflow() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let source = workspace.path().join("overflow.kujo");
    let executable = workspace.path().join("overflow");
    fs::write(&source, "print(9223372036854775807 + 1)\n").expect("write fixture");

    compile_file_to_executable(&source, &executable, None).expect("compile fixture");
    let output = Command::new(&executable).output().expect("run native fixture");
    assert!(!output.status.success());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "integer overflow\n");
}
