use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kujo_{prefix}_{nanos}"));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn write_fixture(path: &Path, content: &str) {
    fs::write(path, content).expect("failed to write fixture file");
}

fn run_kujo(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kujo"))
        .args(args)
        .output()
        .expect("failed to execute kujo binary")
}

fn run_kujo_with_env(args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("failed to execute kujo binary")
}

#[test]
fn cli_run_without_jit_opt_in_executes_program_normally() {
    let dir = unique_temp_dir("jit_contract_default_off");
    let file = dir.join("array_program.kujo");
    write_fixture(
        &file,
        r#"
values := [1, 2, 3]
assert(values[1] == 2, "unexpected value")
"#,
    );

    let output = run_kujo(&["run", file.to_str().expect("path should be utf-8")]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(!stderr.contains("JIT opt-in requested"));
}

#[test]
fn cli_run_with_jit_opt_in_reports_unsupported_regions_without_disabling_jit() {
    let dir = unique_temp_dir("jit_contract_unsupported_surface");
    let file = dir.join("unsupported_jit_surface.kujo");
    write_fixture(
        &file,
        r#"
values := [1, 2, 3]
assert(values[1] == 2, "unexpected value")
"#,
    );

    let output = run_kujo(&["run", "--jit", file.to_str().expect("path should be utf-8")]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected regional fallback success, stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("JIT opt-in requested"));
    assert!(stderr.contains("unsupported opcode"));
    assert!(stderr.contains("unsupported bytecode regions will use the VM"));
    assert!(!stderr.contains("without JIT"));
}

#[test]
fn cli_run_with_jit_opt_in_accepts_supported_surface_without_warning() {
    let dir = unique_temp_dir("jit_contract_supported_surface");
    let file = dir.join("supported_jit_surface.kujo");
    write_fixture(
        &file,
        r#"
assert((2 + 3) * 4 == 20, "arithmetic mismatch")
"#,
    );

    let output = run_kujo(&["run", "--jit", file.to_str().expect("path should be utf-8")]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(!stderr.contains("not JIT-compatible"));
}

#[test]
fn cli_jit_compiles_function_local_control_flow_inside_mixed_programs() {
    let dir = unique_temp_dir("jit_contract_regional_function");
    let file = dir.join("regional_function.kujo");
    write_fixture(
        &file,
        r#"
func sum_odds(limit) {
    mut total := 0
    mut n := 0
    while n < limit {
        if n % 2 != 0 {
            total += n
        }
        n += 1
    }
    return total
}

assert(sum_odds(10000) == 25000000, "hot-loop JIT result mismatch")
"#,
    );

    let output = run_kujo_with_env(
        &["run", "--jit", file.to_str().expect("path should be utf-8")],
        &[("DEBUG_JIT", "1")],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("unsupported bytecode regions will use the VM"));
    assert!(stderr.contains("Successfully compiled function 'sum_odds'"));
    assert!(stderr.contains("Successfully compiled hot loop"));
}
