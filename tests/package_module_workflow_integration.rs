use serde_json::Value;
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

fn parse_stdout_json(output: &Output) -> Value {
    let stdout = stdout_text(output);
    serde_json::from_str(&stdout).expect("stdout should be valid json")
}

#[test]
fn package_module_workflow_end_to_end_contract() {
    let project_root = unique_temp_dir("package_module_workflow");
    let project_root_str = project_root.to_str().expect("path should be utf-8");

    let init =
        run_kujo(&["init", "--dir", project_root_str, "--name", "workflow_demo"], &project_root);
    assert!(
        init.status.success(),
        "kujo init failed: stdout={} stderr={}",
        stdout_text(&init),
        stderr_text(&init)
    );

    let manifest_path = project_root.join("kujo.toml");
    let lockfile_path = project_root.join("kujo.lock");
    let manifest_path_str = manifest_path.to_str().expect("path should be utf-8");
    let lockfile_path_str = lockfile_path.to_str().expect("path should be utf-8");
    assert!(manifest_path.exists(), "expected init to create kujo.toml");
    assert!(project_root.join("src/main.kujo").exists(), "expected init to create src/main.kujo");

    let package_add = run_kujo(
        &["package-add", "example_dep", "--version", "1.2.3", "--manifest", manifest_path_str],
        &project_root,
    );
    assert!(
        package_add.status.success(),
        "kujo package-add failed: stdout={} stderr={}",
        stdout_text(&package_add),
        stderr_text(&package_add)
    );

    let package_install = run_kujo(
        &["package-install", "--manifest", manifest_path_str, "--lockfile", lockfile_path_str],
        &project_root,
    );
    assert!(
        package_install.status.success(),
        "kujo package-install failed: stdout={} stderr={}",
        stdout_text(&package_install),
        stderr_text(&package_install)
    );
    assert!(lockfile_path.exists(), "expected package-install to create lockfile");

    let lockfile_contents_first =
        fs::read_to_string(&lockfile_path).expect("lockfile contents should be readable");
    assert!(
        stdout_text(&package_install).contains("lockfile written"),
        "expected package-install output to include lockfile write marker, got: {}",
        stdout_text(&package_install)
    );
    assert!(
        stdout_text(&package_install).contains("install\texample_dep\t1.2.3"),
        "expected package-install output to include declared dependency, got: {}",
        stdout_text(&package_install)
    );

    let package_install_second = run_kujo(
        &["package-install", "--manifest", manifest_path_str, "--lockfile", lockfile_path_str],
        &project_root,
    );
    assert!(
        package_install_second.status.success(),
        "second package-install failed: stdout={} stderr={}",
        stdout_text(&package_install_second),
        stderr_text(&package_install_second)
    );
    let lockfile_contents_second =
        fs::read_to_string(&lockfile_path).expect("second lockfile read should succeed");
    assert_eq!(
        lockfile_contents_first, lockfile_contents_second,
        "lockfile content should be deterministic across repeated installs"
    );

    let module_path = project_root.join("math_helper.kujo");
    fs::write(&module_path, "export answer := 42\n").expect("failed to write module file");

    let workflow_path = project_root.join("workflow.kujo");
    fs::write(&workflow_path, "from math_helper import answer\nprint(answer)\n")
        .expect("failed to write workflow script");

    let run_output = run_kujo(
        &["run", workflow_path.to_str().expect("path should be utf-8"), "--interpreter"],
        &project_root,
    );
    assert!(
        run_output.status.success(),
        "kujo run workflow failed: stdout={} stderr={}",
        stdout_text(&run_output),
        stderr_text(&run_output)
    );
    assert!(
        stdout_text(&run_output).contains("42"),
        "expected import/export workflow to print imported value, got stdout={} stderr={}",
        stdout_text(&run_output),
        stderr_text(&run_output)
    );

    let format_output = run_kujo(
        &["format", workflow_path.to_str().expect("path should be utf-8"), "--json"],
        &project_root,
    );
    assert!(
        format_output.status.success(),
        "kujo format --json failed: stdout={} stderr={}",
        stdout_text(&format_output),
        stderr_text(&format_output)
    );
    let format_json = parse_stdout_json(&format_output);
    assert_eq!(format_json["command"], "format");

    let lint_output = run_kujo(
        &["lint", workflow_path.to_str().expect("path should be utf-8"), "--json"],
        &project_root,
    );
    assert!(
        lint_output.status.success(),
        "kujo lint --json failed: stdout={} stderr={}",
        stdout_text(&lint_output),
        stderr_text(&lint_output)
    );
    let lint_json = parse_stdout_json(&lint_output);
    assert!(lint_json.is_array());

    let docs_input = project_root.join("docs_input.kujo");
    fs::write(&docs_input, "/// Adds one\nfunc add_one(value) {\n\treturn value + 1\n}\n")
        .expect("failed to write doc input file");
    let docs_out_dir = project_root.join("generated_docs");

    let docgen_output = run_kujo(
        &[
            "docgen",
            docs_input.to_str().expect("path should be utf-8"),
            "--out-dir",
            docs_out_dir.to_str().expect("path should be utf-8"),
            "--json",
        ],
        &project_root,
    );
    assert!(
        docgen_output.status.success(),
        "kujo docgen --json failed: stdout={} stderr={}",
        stdout_text(&docgen_output),
        stderr_text(&docgen_output)
    );
    let docgen_json = parse_stdout_json(&docgen_output);
    assert_eq!(docgen_json["command"], "docgen");
    assert!(docgen_json["item_count"].as_u64().unwrap_or(0) >= 1);
}

#[test]
fn package_module_workflow_vm_default_supports_nested_layout_and_frozen_lockfile() {
    let project_root = unique_temp_dir("package_module_vm_default_nested_layout");
    let project_root_str = project_root.to_str().expect("path should be utf-8");

    let init =
        run_kujo(&["init", "--dir", project_root_str, "--name", "vm_default_demo"], &project_root);
    assert!(
        init.status.success(),
        "kujo init failed: stdout={} stderr={}",
        stdout_text(&init),
        stderr_text(&init)
    );

    let manifest_path = project_root.join("kujo.toml");
    let lockfile_path = project_root.join("kujo.lock");
    let manifest_path_str = manifest_path.to_str().expect("manifest path should be utf-8");
    let lockfile_path_str = lockfile_path.to_str().expect("lockfile path should be utf-8");

    let package_add = run_kujo(
        &["package-add", "nested_dep", "--version", "2.3.4", "--manifest", manifest_path_str],
        &project_root,
    );
    assert!(
        package_add.status.success(),
        "package-add failed: stdout={} stderr={}",
        stdout_text(&package_add),
        stderr_text(&package_add)
    );

    let package_install = run_kujo(
        &["package-install", "--manifest", manifest_path_str, "--lockfile", lockfile_path_str],
        &project_root,
    );
    assert!(
        package_install.status.success(),
        "package-install failed: stdout={} stderr={}",
        stdout_text(&package_install),
        stderr_text(&package_install)
    );
    assert!(
        stdout_text(&package_install).contains("lockfile written"),
        "expected package-install output to include lockfile write marker, got: {}",
        stdout_text(&package_install)
    );

    let frozen_verify = run_kujo(
        &[
            "package-install",
            "--manifest",
            manifest_path_str,
            "--lockfile",
            lockfile_path_str,
            "--frozen",
        ],
        &project_root,
    );
    assert!(
        frozen_verify.status.success(),
        "frozen verification failed: stdout={} stderr={}",
        stdout_text(&frozen_verify),
        stderr_text(&frozen_verify)
    );
    assert!(
        stdout_text(&frozen_verify).contains("lockfile verified"),
        "expected frozen verification output to include lockfile verification marker, got: {}",
        stdout_text(&frozen_verify)
    );

    let src_core_dir = project_root.join("src").join("core");
    let src_rag_dir = project_root.join("src").join("rag");
    fs::create_dir_all(&src_core_dir).expect("failed to create src/core directory");
    fs::create_dir_all(&src_rag_dir).expect("failed to create src/rag directory");

    fs::write(
        src_core_dir.join("math.kujo"),
        "func add(left, right) {\n    return left + right\n}\nexport add := add\n",
    )
    .expect("failed to write src/core/math module");
    fs::write(src_rag_dir.join("config.kujo"), "export base := 40\n")
        .expect("failed to write src/rag/config module");
    fs::write(
        src_rag_dir.join("pipeline.kujo"),
        "from src.core.math import add\nfrom src.rag.config import base\nexport answer := add(base, 2)\n",
    )
    .expect("failed to write src/rag/pipeline module");

    let nested_workflow = project_root.join("nested_vm_default_workflow.kujo");
    fs::write(&nested_workflow, "from src.rag.pipeline import answer\nprint(answer)\n")
        .expect("failed to write nested VM-default workflow");

    let output =
        run_kujo(&["run", nested_workflow.to_str().expect("path should be utf-8")], &project_root);
    assert!(
        output.status.success(),
        "VM-default nested workflow failed: stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    assert!(
        stdout_text(&output).contains("42"),
        "expected VM-default nested workflow to print 42, got stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
}

#[test]
fn package_install_frozen_detects_lockfile_drift_and_verifies_after_regeneration() {
    let project_root = unique_temp_dir("package_install_frozen_lockfile");
    let project_root_str = project_root.to_str().expect("path should be utf-8");
    let init = run_kujo(&["init", "--dir", project_root_str, "--name", "lock_demo"], &project_root);
    assert!(
        init.status.success(),
        "kujo init failed: stdout={} stderr={}",
        stdout_text(&init),
        stderr_text(&init)
    );

    let manifest_path = project_root.join("kujo.toml");
    let lockfile_path = project_root.join("kujo.lock");
    let manifest_path_str = manifest_path.to_str().expect("manifest path should be utf-8");
    let lockfile_path_str = lockfile_path.to_str().expect("lockfile path should be utf-8");

    let add_first_dep = run_kujo(
        &["package-add", "alpha", "--version", "1.0.0", "--manifest", manifest_path_str],
        &project_root,
    );
    assert!(
        add_first_dep.status.success(),
        "first package-add failed: stdout={} stderr={}",
        stdout_text(&add_first_dep),
        stderr_text(&add_first_dep)
    );

    let install = run_kujo(
        &["package-install", "--manifest", manifest_path_str, "--lockfile", lockfile_path_str],
        &project_root,
    );
    assert!(
        install.status.success(),
        "package-install failed: stdout={} stderr={}",
        stdout_text(&install),
        stderr_text(&install)
    );

    let add_second_dep = run_kujo(
        &["package-add", "beta", "--version", "2.0.0", "--manifest", manifest_path_str],
        &project_root,
    );
    assert!(
        add_second_dep.status.success(),
        "second package-add failed: stdout={} stderr={}",
        stdout_text(&add_second_dep),
        stderr_text(&add_second_dep)
    );

    let frozen_fail = run_kujo(
        &[
            "package-install",
            "--manifest",
            manifest_path_str,
            "--lockfile",
            lockfile_path_str,
            "--frozen",
        ],
        &project_root,
    );
    assert!(
        !frozen_fail.status.success(),
        "expected frozen lockfile verification to fail after manifest drift"
    );
    assert!(
        stderr_text(&frozen_fail).contains("dependencies are out of date"),
        "expected deterministic lockfile drift message, got stderr={}",
        stderr_text(&frozen_fail)
    );

    let regenerate = run_kujo(
        &["package-install", "--manifest", manifest_path_str, "--lockfile", lockfile_path_str],
        &project_root,
    );
    assert!(
        regenerate.status.success(),
        "lockfile regeneration failed: stdout={} stderr={}",
        stdout_text(&regenerate),
        stderr_text(&regenerate)
    );

    let frozen_pass = run_kujo(
        &[
            "package-install",
            "--manifest",
            manifest_path_str,
            "--lockfile",
            lockfile_path_str,
            "--frozen",
        ],
        &project_root,
    );
    assert!(
        frozen_pass.status.success(),
        "expected frozen verification to pass after regeneration: stdout={} stderr={}",
        stdout_text(&frozen_pass),
        stderr_text(&frozen_pass)
    );
    assert!(
        stdout_text(&frozen_pass).contains("lockfile verified"),
        "expected frozen verification marker, got stdout={}",
        stdout_text(&frozen_pass)
    );
}

#[test]
fn package_module_cycle_error_reports_import_chain() {
    let project_root = unique_temp_dir("package_module_cycle_error");

    let module_a = "cycle_a";
    let module_b = "cycle_b";
    let module_a_path = project_root.join(format!("{}.kujo", module_a));
    let module_b_path = project_root.join(format!("{}.kujo", module_b));
    let workflow_path = project_root.join("cycle_workflow.kujo");

    fs::write(&module_a_path, format!("import {}\nexport a := 1\n", module_b))
        .expect("failed to write module A");
    fs::write(&module_b_path, format!("import {}\nexport b := 2\n", module_a))
        .expect("failed to write module B");
    fs::write(&workflow_path, format!("import {}\nprint(\"unreachable\")\n", module_a))
        .expect("failed to write cycle workflow script");

    let run_output = run_kujo(
        &["run", workflow_path.to_str().expect("path should be utf-8"), "--interpreter"],
        &project_root,
    );

    assert!(
        !run_output.status.success(),
        "expected circular import run to fail, stdout={} stderr={}",
        stdout_text(&run_output),
        stderr_text(&run_output)
    );

    let stderr = stderr_text(&run_output);
    assert!(
        stderr.contains("Circular import detected: cycle_a -> cycle_b -> cycle_a"),
        "expected circular import chain in stderr, got: {}",
        stderr
    );
}

#[test]
fn package_module_run_refreshes_after_module_file_change() {
    let project_root = unique_temp_dir("package_module_refresh");

    let module_path = project_root.join("math_helper.kujo");
    let workflow_path = project_root.join("refresh_workflow.kujo");

    fs::write(&module_path, "export answer := 10\n").expect("failed to write initial module");
    fs::write(&workflow_path, "from math_helper import answer\nprint(answer)\n")
        .expect("failed to write workflow script");

    let first_run = run_kujo(
        &["run", workflow_path.to_str().expect("path should be utf-8"), "--interpreter"],
        &project_root,
    );
    assert!(
        first_run.status.success(),
        "first run failed: stdout={} stderr={}",
        stdout_text(&first_run),
        stderr_text(&first_run)
    );
    assert!(
        stdout_text(&first_run).contains("10"),
        "expected first run output to include 10, got stdout={} stderr={}",
        stdout_text(&first_run),
        stderr_text(&first_run)
    );

    fs::write(&module_path, "export answer := 25\n").expect("failed to update module export value");

    let second_run = run_kujo(
        &["run", workflow_path.to_str().expect("path should be utf-8"), "--interpreter"],
        &project_root,
    );
    assert!(
        second_run.status.success(),
        "second run failed: stdout={} stderr={}",
        stdout_text(&second_run),
        stderr_text(&second_run)
    );
    assert!(
        stdout_text(&second_run).contains("25"),
        "expected second run output to include updated value 25, got stdout={} stderr={}",
        stdout_text(&second_run),
        stderr_text(&second_run)
    );
}

#[test]
fn package_module_workflow_supports_dotted_from_imports_for_nested_source_layout() {
    let project_root = unique_temp_dir("package_module_dotted_imports");
    let nested_dir = project_root.join("src").join("core");
    fs::create_dir_all(&nested_dir).expect("failed to create nested source layout");

    fs::write(
        nested_dir.join("math.kujo"),
        "func add(left, right) {\n    return left + right\n}\nexport add := add\n",
    )
    .expect("failed to write nested math module");
    fs::write(project_root.join("src").join("util.kujo"), "export value := 40\n")
        .expect("failed to write util module");

    let workflow_path = project_root.join("nested_workflow.kujo");
    fs::write(
        &workflow_path,
        "from src.core.math import add\nfrom src.util import value\nprint(add(value, 2))\n",
    )
    .expect("failed to write dotted import workflow");

    let run_output = run_kujo(
        &["run", workflow_path.to_str().expect("path should be utf-8"), "--interpreter"],
        &project_root,
    );
    assert!(
        run_output.status.success(),
        "dotted import workflow failed: stdout={} stderr={}",
        stdout_text(&run_output),
        stderr_text(&run_output)
    );
    assert!(
        stdout_text(&run_output).contains("42"),
        "expected dotted import workflow to print 42, got stdout={} stderr={}",
        stdout_text(&run_output),
        stderr_text(&run_output)
    );
}

#[test]
fn package_module_workflow_nested_layout_is_runtime_mode_consistent_and_keeps_flat_imports() {
    let project_root = unique_temp_dir("package_module_nested_runtime_consistency");

    let src_core_dir = project_root.join("src").join("core");
    let src_rag_dir = project_root.join("src").join("rag");
    fs::create_dir_all(&src_core_dir).expect("failed to create src/core directory");
    fs::create_dir_all(&src_rag_dir).expect("failed to create src/rag directory");

    fs::write(
        src_core_dir.join("math.kujo"),
        "func add(left, right) {\n    return left + right\n}\nexport add := add\n",
    )
    .expect("failed to write src/core/math module");
    fs::write(src_rag_dir.join("config.kujo"), "export base := 40\n")
        .expect("failed to write src/rag/config module");
    fs::write(
        src_rag_dir.join("pipeline.kujo"),
        "from src.core.math import add\nfrom src.rag.config import base\nexport answer := add(base, 2)\n",
    )
    .expect("failed to write src/rag/pipeline module");

    let nested_workflow = project_root.join("nested_runtime_workflow.kujo");
    fs::write(&nested_workflow, "from src.rag.pipeline import answer\nprint(answer)\n")
        .expect("failed to write nested runtime workflow");

    for args in [
        vec!["run", nested_workflow.to_str().expect("path should be utf-8")],
        vec!["run", "--interpreter", nested_workflow.to_str().expect("path should be utf-8")],
    ] {
        let output = run_kujo(&args, &project_root);
        assert!(
            output.status.success(),
            "nested workflow failed: args={:?} stdout={} stderr={}",
            args,
            stdout_text(&output),
            stderr_text(&output)
        );
        assert!(
            stdout_text(&output).contains("42"),
            "expected nested workflow to print 42: args={:?} stdout={} stderr={}",
            args,
            stdout_text(&output),
            stderr_text(&output)
        );
    }

    fs::write(project_root.join("math_helper.kujo"), "export answer := 7\n")
        .expect("failed to write flat module");
    let flat_workflow = project_root.join("flat_runtime_workflow.kujo");
    fs::write(&flat_workflow, "from math_helper import answer\nprint(answer)\n")
        .expect("failed to write flat workflow");

    for args in [
        vec!["run", flat_workflow.to_str().expect("path should be utf-8")],
        vec!["run", "--interpreter", flat_workflow.to_str().expect("path should be utf-8")],
    ] {
        let output = run_kujo(&args, &project_root);
        assert!(
            output.status.success(),
            "flat workflow failed: args={:?} stdout={} stderr={}",
            args,
            stdout_text(&output),
            stderr_text(&output)
        );
        assert!(
            stdout_text(&output).contains("7"),
            "expected flat workflow to print 7: args={:?} stdout={} stderr={}",
            args,
            stdout_text(&output),
            stderr_text(&output)
        );
    }
}
