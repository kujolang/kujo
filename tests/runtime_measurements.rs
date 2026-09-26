use serde_json::Value;
use std::fs;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn bounded(command: &mut Command) -> Output {
    let mut child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("measurement fixture timed out: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}

fn run(source: &str, extra: &[&str]) -> (Output, Value) {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("program.kujo");
    let report = dir.path().join("measurements.json");
    fs::write(&script, source).unwrap();
    let plain =
        bounded(Command::new(env!("CARGO_BIN_EXE_kujo")).arg("run").arg(&script).args(extra));
    let measured = bounded(
        Command::new(env!("CARGO_BIN_EXE_kujo"))
            .arg("run")
            .arg("--measurements")
            .arg(&report)
            .arg(&script)
            .args(extra),
    );
    assert_eq!(plain.status.code(), measured.status.code());
    assert_eq!(plain.stdout, measured.stdout);
    assert_eq!(plain.stderr, measured.stderr);
    let bytes = fs::read(&report).unwrap();
    assert!(bytes.len() < 8192);
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["schema"], "kujo.runtime-measurements/v1");
    assert_eq!(value["scope"], "process");
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/runtime-measurements-v1.schema.json"))
            .unwrap();
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .unwrap();
    assert!(validator.is_valid(&value), "invalid report: {value}");
    let mut invalid = value.clone();
    invalid["counters"]["vm_entries"] = serde_json::json!(-1);
    assert!(!validator.is_valid(&invalid));
    for count in value["counters"].as_object().unwrap().values() {
        assert!(count.is_u64());
    }
    (measured, value)
}

#[test]
fn closure_and_generator_measurements_preserve_output_and_omit_payloads() {
    let (output, report) = run(
        "func factory() { let secret_label := \"sensitive-sentinel\" func get() { return secret_label } return get } let f := factory() print(f()) func* values() { yield 7 yield 8 } let g := values() for v in g { print(v) }",
        &["--untrusted"],
    );
    assert!(output.status.success(), "{:?}", output);
    assert!(!report.to_string().contains("sensitive-sentinel"));
    let c = &report["counters"];
    assert!(c["vm_call_opcodes"].as_u64().unwrap() >= 3);
    assert!(c["vm_return_opcodes"].as_u64().unwrap() >= 2);
    assert!(c["vm_capture_cells_created"].as_u64().unwrap() >= 1);
    assert_eq!(c["vm_generator_states_created"], 1);
    assert_eq!(c["vm_generator_state_drops"], 1);
    assert_eq!(c["vm_generator_resume_attempts"], 3);
    assert_eq!(report["outcome"], "success");
}

#[test]
fn async_worker_and_shared_completion_are_measured() {
    let (output, report) =
        run("async func work() { return 42 } let p := work() print(await p) print(await p)", &[]);
    assert!(output.status.success(), "{:?}", output);
    let c = &report["counters"];
    assert_eq!(c["tasks_admitted"], 1);
    assert_eq!(c["tasks_started"], 1);
    assert_eq!(c["task_completions_published"], 1);
    assert!(c["promise_ready_polls"].as_u64().unwrap() >= 1);
}

#[test]
fn runtime_failure_keeps_diagnostics_and_exit_code() {
    let (output, report) =
        run("print(missing_measurement_fixture_name)", &["--json-runtime-diagnostics"]);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(report["outcome"], "runtime_error");
    assert!(report["counters"]["vm_entries"].as_u64().unwrap() >= 1);
}

#[test]
fn output_is_exclusive_and_interpreter_combination_rejects() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("program.kujo");
    fs::write(&script, "print(42)").unwrap();
    let output = bounded(
        Command::new(env!("CARGO_BIN_EXE_kujo"))
            .arg("run")
            .arg("--measurements")
            .arg(&script)
            .arg(&script),
    );
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_to_string(&script).unwrap(), "print(42)");
    let output = bounded(
        Command::new(env!("CARGO_BIN_EXE_kujo"))
            .args(["run", "--interpreter", "--measurements", "unused.json"])
            .arg(&script),
    );
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn detached_spawn_does_not_acquire_a_join_requirement() {
    let (output, report) = run("spawn { loop {} } print(42)", &[]);
    assert!(output.status.success());
    assert_eq!(report["counters"]["detached_tasks_observed"], 1);
    assert_eq!(report["waited_for_detached"], false);
}

#[test]
fn jit_opt_in_keeps_results_and_reports_compile_work() {
    let (output, report) = run("42", &["--jit"]);
    assert!(output.status.success(), "{:?}", output);
    #[cfg(feature = "runtime-jit")]
    assert!(report["counters"]["jit_compile_entries"].as_u64().unwrap() > 0);
    #[cfg(not(feature = "runtime-jit"))]
    assert_eq!(report["counters"]["jit_compile_entries"], 0);
}

#[test]
fn measurement_option_is_not_a_program_argument() {
    let (output, _) = run("print(to_json(args()))", &[]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "[]");
    let (output, _) = run("print(to_json(args()))", &["--", "--measurements", "literal"]);
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("literal"));
}

#[test]
fn admission_failure_and_restricted_worker_errors_remain_conservative() {
    let (output, report) = run("async func recurse(n) { if n == 0 { return 0 } return await recurse(n - 1) } mut caught := false try { await recurse(100) } except err { caught = true } print(caught)", &[]);
    assert!(output.status.success());
    assert_eq!(report["counters"]["tasks_admitted"], 16);
    assert_eq!(report["counters"]["tasks_admission_rejected"], 1);
    let (output, report) = run("async func forbidden() { return read_file(\"must-not-be-opened\") } let p := forbidden() mut caught := false try { await p } except err { caught = true } print(caught)", &["--untrusted"]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "true");
    assert_eq!(report["counters"]["task_completions_published"], 1);
}
