//! Potentially blocking concurrency contracts run in bounded child processes.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn both(source: &str, restricted: bool) {
    for interpreter in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let script = directory.path().join("fixture.kujo");
        let stdout = directory.path().join("stdout");
        let stderr = directory.path().join("stderr");
        fs::write(&script, source).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&script);
        if interpreter {
            command.arg("--interpreter");
        }
        if restricted {
            command.arg("--untrusted");
        }
        let mut child = command
            .stdout(Stdio::from(File::create(&stdout).unwrap()))
            .stderr(Stdio::from(File::create(&stderr).unwrap()))
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "concurrency fixture timed out (interpreter={interpreter}): {source}\n{}",
                    fs::read_to_string(&stderr).unwrap()
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(
            status.success(),
            "interpreter={interpreter}: {}",
            fs::read_to_string(&stderr).unwrap()
        );
        assert_eq!(
            fs::read_to_string(&stdout).unwrap().trim(),
            "true",
            "interpreter={interpreter}"
        );
    }
}

#[test]
fn async_submission_returns_before_body_completes() {
    both("let ch := channel() async func work() { let ready := ch.receive() return ready + 1 } let p := work() ch.send(41) let result := await p print(result == 42)", false);
}

#[test]
fn detached_spawn_runs_with_snapshot_capture_after_factory_return() {
    both("shared_set(\"done\", 0) mut n := 3 func launch() { let n := 7 spawn { shared_set(\"done\", n) } } launch() while shared_get(\"done\") == 0 {} print(n == 3 && shared_get(\"done\") == 7)", false);
}

#[test]
fn detached_spawn_keeps_mutability_and_does_not_mutate_parent_copy() {
    both("shared_set(\"done\", 0) mut n := 3 let fixed := 7 spawn { n = 8 try { fixed = 9 } except err { shared_set(\"done\", n) } } while shared_get(\"done\") == 0 {} print(n == 3 && fixed == 7 && shared_get(\"done\") == 8)", false);
}

#[test]
fn unsupported_inner_spawn_capture_never_falls_back_to_outer_value() {
    both("let item := 7 func launch() { let item := channel() spawn { print(item) } } mut caught := false try { launch() } except err { caught = true } print(caught)", false);
}

#[test]
fn unrelated_unsupported_values_do_not_block_spawn() {
    both("let unused := channel() shared_set(\"done\", 0) spawn { let unused := 7 shared_set(\"done\", unused) } while shared_get(\"done\") == 0 {} print(shared_get(\"done\") == 7)", false);
}

#[test]
fn cancellation_is_repeatable_and_cooperatively_stops_language_loops() {
    both("shared_set(\"started\", 0) func work() { shared_set(\"started\", 1) loop {} } let h := spawn_task(work) while shared_get(\"started\") == 0 {} let cancelled := cancel_task(h) mut failures := 0 try { await await_task(h) } except err { failures += 1 } try { await await_task(h) } except err { failures += 1 } print(cancelled && failures == 2)", false);
}

#[test]
fn recursive_async_admission_fails_boundedly() {
    both("async func recurse(n) { if n == 0 { return 0 } return await recurse(n - 1) } mut caught := false try { await recurse(100) } except err { caught = true } print(caught)", false);
}

#[test]
fn tasks_preserve_restricted_capabilities() {
    both("async func forbidden() { return read_file(\"must-not-be-opened\") } let p := forbidden() mut caught := false try { await p } except err { caught = true } print(caught)", true);
}

#[test]
fn detached_work_does_not_implicitly_join_at_process_exit() {
    both("spawn { loop {} } print(true)", false);
}

#[test]
fn overlapping_async_calls_preserve_changes_to_different_captured_bindings() {
    both("shared_set(\"started\", 0) let gate := channel() func factory() { mut a := 0 mut b := 0 async func apply_change(key) { if key == 0 { a = 1 } if key == 1 { b = 1 } if key < 2 { shared_add_int(\"started\", 1) gate.receive() } return a + b } return apply_change } let apply_change := factory() let a := apply_change(0) let b := apply_change(1) while shared_get(\"started\") < 2 {} gate.send(1) gate.send(1) await a await b let result := await apply_change(2) print(result == 2)", false);
}

#[test]
fn ordinary_generators_work_inside_async_calls() {
    both("async func sum() { func* numbers() { yield 1 while false { yield 99 } yield 2 } mut total := 0 for n in numbers() { total += n } return total } let result := await sum() print(result == 3)", false);
}

#[test]
fn generator_can_await_an_ordinary_async_call() {
    both("async func value() { return 21 } func* numbers() { let n := await value() yield n yield n } mut total := 0 for n in numbers() { total += n } print(total == 42)", false);
}

#[test]
fn cancellation_releases_execution_capacity_after_language_body_exits() {
    both("shared_set(\"started\", 0) func work() { shared_add_int(\"started\", 1) loop {} } mut i := 0 while i < 40 { let h := spawn_task(work) while (shared_get(\"started\") <= i) {} cancel_task(h) try { await await_task(h) } except err {} i += 1 } print(shared_get(\"started\") == 40)", false);
}

#[test]
fn yield_in_loop_condition_restarts_after_continue_and_finishes() {
    both("func* values() { mut n := 0 while (yield n < 2) { n += 1 continue } yield 42 } mut count := 0 mut total := 0 for item in values() { count += 1 if item == 42 { total += item } } print(count == 4 && total == 42)", false);
}

#[test]
fn task_callbacks_and_detached_workers_inherit_all_host_denials() {
    let body = r#"
        mut denied := 0
        try { read_file("absent") } except err { if contains(err.message, "Capability denied") { denied += 1 } }
        try { spawn_process(["must-not-execute"]) } except err { if contains(err.message, "Capability denied") { denied += 1 } }
        try { env("HOME") } except err { if contains(err.message, "Capability denied") { denied += 1 } }
        try { http_get("http://127.0.0.1:1") } except err { if contains(err.message, "Capability denied") { denied += 1 } }
        try { db_connect("sqlite", ":memory:") } except err { if contains(err.message, "Capability denied") { denied += 1 } }
    "#;
    both(&format!("async func work() {{ {body} return denied }} let result := await work() shared_set(\"done\", -1) spawn {{ {body} shared_set(\"done\", denied) }} while shared_get(\"done\") < 0 {{}} print(result == 5 && shared_get(\"done\") == 5)"), true);
}

#[test]
fn cross_feature_task_spawn_and_generator_captures_keep_their_owners() {
    both("shared_set(\"done\", 0) async func outer() { let n := 21 spawn { async func inner() { return n * 2 } let v := await inner() shared_set(\"done\", v) } return 1 } let started := await outer() while shared_get(\"done\") == 0 {} print(started == 1 && shared_get(\"done\") == 42)", false);
}
