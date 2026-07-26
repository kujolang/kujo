use kujo::module::ModuleLoader;
use kujo::runtime_limits;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_{prefix}_{nanos}"));
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

fn write_script(project_root: &Path, filename: &str, source: &str) -> PathBuf {
    let script_path = project_root.join(filename);
    fs::write(&script_path, source).expect("failed to write Kujo script");
    script_path
}

#[test]
fn runtime_security_rejects_invalid_escape_sequences() {
    let project_root = unique_temp_dir("runtime_security_invalid_escape");
    let script_path = write_script(&project_root, "invalid_escape.kujo", "print(\"\\q\")\n");

    let output =
        run_kujo(&["run", script_path.to_str().expect("path should be utf-8")], &project_root);

    assert_eq!(
        output.status.code(),
        Some(3),
        "expected lexer diagnostic exit code for invalid escape, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stderr_text(&output).contains("Invalid escape sequence"),
        "expected invalid escape diagnostic, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn runtime_security_rejects_oversized_string_literals() {
    let project_root = unique_temp_dir("runtime_security_string_limit");
    let oversized = "a".repeat(runtime_limits::DEFAULT_MAX_STRING_LITERAL_LENGTH + 1);
    let script_source = format!("print(\"{oversized}\")\n");
    let script_path = write_script(&project_root, "oversized_string.kujo", &script_source);

    let output =
        run_kujo(&["run", script_path.to_str().expect("path should be utf-8")], &project_root);

    assert_eq!(
        output.status.code(),
        Some(3),
        "expected parse/lexer diagnostics for oversized string literal, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stderr_text(&output).contains("String literal exceeds max length"),
        "expected oversized string diagnostic, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn runtime_security_rejects_oversized_collection_literals() {
    let project_root = unique_temp_dir("runtime_security_collection_limit");
    let item_count = runtime_limits::DEFAULT_MAX_COLLECTION_LITERAL_ITEMS + 1;
    let elements: Vec<String> = (0..item_count).map(|_| "1".to_string()).collect();
    let script_source = format!("let values := [{}]\nprint(len(values))\n", elements.join(", "));
    let script_path = write_script(&project_root, "oversized_collection.kujo", &script_source);

    let output =
        run_kujo(&["run", script_path.to_str().expect("path should be utf-8")], &project_root);

    assert_eq!(
        output.status.code(),
        Some(3),
        "expected parse diagnostics for oversized collection literal, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stderr_text(&output).contains("Array literal exceeds maximum element count"),
        "expected collection literal limit diagnostic, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn runtime_security_break_and_continue_outside_loops_are_rejected() {
    let project_root = unique_temp_dir("runtime_security_control_flow");
    let break_script = write_script(&project_root, "break_outside_loop.kujo", "break\n");
    let continue_script = write_script(&project_root, "continue_outside_loop.kujo", "continue\n");

    let break_output = run_kujo(
        &["run", "--interpreter", break_script.to_str().expect("path should be utf-8")],
        &project_root,
    );
    assert_eq!(
        break_output.status.code(),
        Some(4),
        "expected runtime error for break outside loop, stdout={} stderr={}",
        stdout_text(&break_output),
        stderr_text(&break_output)
    );
    assert!(
        stderr_text(&break_output).contains("break can only be used inside a loop"),
        "expected deterministic break diagnostic, stdout={} stderr={}",
        stdout_text(&break_output),
        stderr_text(&break_output)
    );

    let continue_output = run_kujo(
        &["run", "--interpreter", continue_script.to_str().expect("path should be utf-8")],
        &project_root,
    );
    assert_eq!(
        continue_output.status.code(),
        Some(4),
        "expected runtime error for continue outside loop, stdout={} stderr={}",
        stdout_text(&continue_output),
        stderr_text(&continue_output)
    );
    assert!(
        stderr_text(&continue_output).contains("continue can only be used inside a loop"),
        "expected deterministic continue diagnostic, stdout={} stderr={}",
        stdout_text(&continue_output),
        stderr_text(&continue_output)
    );
}

#[test]
fn runtime_security_enforces_interpreter_call_depth_limit() {
    let project_root = unique_temp_dir("runtime_security_call_depth");
    let depth = runtime_limits::DEFAULT_MAX_INTERPRETER_CALL_DEPTH + 8;
    let script_source = format!(
        "func dive(n) {{\n    if n <= 0 {{ return 0 }}\n    return dive(n - 1)\n}}\nprint(dive({depth}))\n"
    );
    let script_path = write_script(&project_root, "call_depth_limit.kujo", &script_source);

    let output = run_kujo(
        &["run", "--interpreter", script_path.to_str().expect("path should be utf-8")],
        &project_root,
    );
    assert_eq!(
        output.status.code(),
        Some(4),
        "expected runtime call-depth failure, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stderr_text(&output).contains("Maximum call stack depth of"),
        "expected call-depth boundary error, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn runtime_security_reports_module_cycle_import_chain() {
    let project_root = unique_temp_dir("runtime_security_module_cycle");
    let module_a = project_root.join("cycle_a.kujo");
    let module_b = project_root.join("cycle_b.kujo");
    let workflow = project_root.join("cycle_main.kujo");

    fs::write(&module_a, "import cycle_b\nexport a := 1\n").expect("failed to write module A");
    fs::write(&module_b, "import cycle_a\nexport b := 2\n").expect("failed to write module B");
    fs::write(&workflow, "import cycle_a\nprint(\"unreachable\")\n")
        .expect("failed to write workflow");

    let output = run_kujo(
        &["run", "--interpreter", workflow.to_str().expect("path should be utf-8")],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected circular import to fail at runtime, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stderr_text(&output).contains("Circular import detected: cycle_a -> cycle_b -> cycle_a"),
        "expected deterministic module-cycle chain, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn runtime_security_vm_resolves_sibling_module_for_relative_entry_script_path() {
    let project_root = unique_temp_dir("runtime_security_entry_script_relative_vm");
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).expect("failed to create src directory");

    let common_module = src_dir.join("common.kujo");
    fs::write(
        &common_module,
        "export func dict_get(payload, key, fallback) {\n    if has_key(payload, key) { return payload[key] }\n    return fallback\n}\n",
    )
    .expect("failed to write common module");

    let render_script = src_dir.join("render.kujo");
    fs::write(
        &render_script,
        "from src.common import dict_get\npayload := {\"name\": \"kujo\"}\nprint(dict_get(payload, \"name\", \"missing\"))\n",
    )
    .expect("failed to write render script");

    let output = run_kujo(&["run", "src/render.kujo"], &project_root);
    assert!(
        output.status.success(),
        "expected VM run to resolve sibling module imports, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("kujo"),
        "expected imported helper output, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn runtime_security_entry_script_parent_search_path_works_cross_cwd_vm_and_interpreter() {
    let project_root = unique_temp_dir("runtime_security_entry_script_cross_cwd");
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).expect("failed to create src directory");

    let common_module = src_dir.join("common.kujo");
    fs::write(
        &common_module,
        "export func dict_get(payload, key, fallback) {\n    if has_key(payload, key) { return payload[key] }\n    return fallback\n}\n",
    )
    .expect("failed to write common module");

    let render_script = src_dir.join("render.kujo");
    fs::write(
        &render_script,
        "from src.common import dict_get\npayload := {\"name\": \"kujo\"}\nprint(dict_get(payload, \"name\", \"missing\"))\n",
    )
    .expect("failed to write render script");

    let isolated_cwd = unique_temp_dir("runtime_security_entry_script_cross_cwd_runner");
    let render_abs = render_script.to_str().expect("render path should be utf-8");

    let vm_output = run_kujo(&["run", render_abs], &isolated_cwd);
    assert!(
        vm_output.status.success(),
        "expected cross-cwd VM run to resolve entry-script parent imports, stdout={} stderr={}",
        stdout_text(&vm_output),
        stderr_text(&vm_output)
    );
    assert!(
        stdout_text(&vm_output).contains("kujo"),
        "expected VM imported helper output, stdout={} stderr={}",
        stdout_text(&vm_output),
        stderr_text(&vm_output)
    );

    let interpreter_output = run_kujo(&["run", render_abs, "--interpreter"], &isolated_cwd);
    assert!(
        interpreter_output.status.success(),
        "expected cross-cwd interpreter run to resolve entry-script parent imports, stdout={} stderr={}",
        stdout_text(&interpreter_output),
        stderr_text(&interpreter_output)
    );
    assert!(
        stdout_text(&interpreter_output).contains("kujo"),
        "expected interpreter imported helper output, stdout={} stderr={}",
        stdout_text(&interpreter_output),
        stderr_text(&interpreter_output)
    );
}

#[test]
fn runtime_security_module_loader_rejects_parent_traversal_import_name_cross_platform() {
    let project_root = unique_temp_dir("runtime_security_module_traversal_guard");
    let mut loader = ModuleLoader::new();
    loader.add_search_path(&project_root);

    let err = loader
        .get_all_exports("../outside")
        .expect_err("expected module traversal import name to fail");

    assert!(
        err.message.contains("Unsafe module import '../outside'"),
        "expected unsafe traversal error, got: {}",
        err.message
    );
}

#[cfg(unix)]
#[test]
fn runtime_security_rejects_module_symlink_escape() {
    let project_root = unique_temp_dir("runtime_security_module_symlink_root");
    let outside_root = unique_temp_dir("runtime_security_module_symlink_outside");
    let module_name = "escaped_module";
    let outside_module_path = outside_root.join(format!("{module_name}.kujo"));
    fs::write(&outside_module_path, "export escaped := 99\n")
        .expect("failed to write outside module file");

    let symlink_path = project_root.join(format!("{module_name}.kujo"));
    unix_fs::symlink(&outside_module_path, &symlink_path)
        .expect("failed to create module symlink escape");

    let workflow = project_root.join("escape_main.kujo");
    fs::write(&workflow, format!("import {module_name}\nprint(\"unreachable\")\n"))
        .expect("failed to write workflow");

    let output = run_kujo(
        &["run", "--interpreter", workflow.to_str().expect("path should be utf-8")],
        &project_root,
    );

    assert_eq!(
        output.status.code(),
        Some(4),
        "expected symlink escape import to fail, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stderr_text(&output).contains("escapes module search root"),
        "expected symlink-escape rejection, stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[cfg(unix)]
#[test]
fn runtime_security_rejects_dotted_module_symlink_escape_in_vm_and_interpreter() {
    let project_root = unique_temp_dir("runtime_security_dotted_symlink_root");
    let outside_root = unique_temp_dir("runtime_security_dotted_symlink_outside");
    let root_module = "src";
    let outside_module_path = outside_root.join("math.kujo");
    fs::write(&outside_module_path, "export answer := 99\n")
        .expect("failed to write outside dotted module file");

    let nested_dir = project_root.join(root_module).join("core");
    fs::create_dir_all(&nested_dir).expect("failed to create nested module directory");
    let symlink_path = nested_dir.join("math.kujo");
    unix_fs::symlink(&outside_module_path, &symlink_path)
        .expect("failed to create dotted module symlink escape");

    let workflow = project_root.join("escape_dotted_main.kujo");
    fs::write(&workflow, "from src.core.math import answer\nprint(answer)\n")
        .expect("failed to write dotted symlink workflow");

    for args in [
        vec!["run", workflow.to_str().expect("path should be utf-8")],
        vec!["run", "--interpreter", workflow.to_str().expect("path should be utf-8")],
    ] {
        let output = run_kujo(&args, &project_root);
        assert_eq!(
            output.status.code(),
            Some(4),
            "expected dotted symlink escape import to fail, args={:?} stdout={} stderr={}",
            args,
            stdout_text(&output),
            stderr_text(&output)
        );
        assert!(
            stderr_text(&output).contains("escapes module search root"),
            "expected symlink-escape rejection for dotted import, args={:?} stdout={} stderr={}",
            args,
            stdout_text(&output),
            stderr_text(&output)
        );
    }
}
