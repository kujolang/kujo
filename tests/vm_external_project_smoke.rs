use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_{}_{}", prefix, nanos));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn kujo_binary() -> String {
    env!("CARGO_BIN_EXE_kujo").to_string()
}

fn run_kujo(args: &[&str], current_dir: &Path) -> Output {
    Command::new(kujo_binary())
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("failed to execute kujo binary")
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

fn write_external_project_module(project_root: &Path) {
    let module_path = project_root.join("math_helper.kujo");
    fs::write(
        module_path,
        r#"
export answer := 42
export func add_one(x) {
    return x + 1
}
"#,
    )
    .expect("failed to write helper module");
}

#[test]
fn vm_external_project_smoke_from_import_symbol_call() {
    let project_root = unique_temp_dir("vm_external_from_import_call");
    write_external_project_module(&project_root);

    let script_path = project_root.join("from_import_flow.kujo");
    fs::write(
        &script_path,
        "from math_helper import add_one\nprint(\"FROM_CALL=\" + to_string(add_one(41)))\n",
    )
    .expect("failed to write from-import script");

    let output =
        run_kujo(&["run", script_path.to_str().expect("path should be utf-8")], &project_root);

    assert!(
        output.status.success(),
        "expected VM from-import call flow to succeed, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("FROM_CALL=42"),
        "expected from-import call marker in stdout, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn vm_external_project_smoke_import_module_symbol_call() {
    let project_root = unique_temp_dir("vm_external_module_symbol_call");
    write_external_project_module(&project_root);

    let script_path = project_root.join("module_symbol_flow.kujo");
    fs::write(
        &script_path,
        "import math_helper\nprint(\"MODULE_CALL=\" + to_string(math_helper.add_one(41)))\n",
    )
    .expect("failed to write module-symbol script");

    let output =
        run_kujo(&["run", script_path.to_str().expect("path should be utf-8")], &project_root);

    assert!(
        output.status.success(),
        "expected VM module-symbol call flow to succeed, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("MODULE_CALL=42"),
        "expected module-symbol call marker in stdout, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}
