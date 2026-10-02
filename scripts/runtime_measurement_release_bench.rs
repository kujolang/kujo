//! Optimized-build campaign, independent of the Kujo crate/dependency graph.
//! rustc -O scripts/runtime_measurement_release_bench.rs -o /tmp/kujo-release-bench
//! kujo-release-bench BASELINE CANDIDATE > release-overhead.json
use std::{env, fs, process::Command, time::Instant};

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3, "provide immutable baseline and candidate binaries");
    let root = env::temp_dir().join(format!("kujo-release-bench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let workloads = [
        ("calls", 10000, "func plus(n) { return n + 1 } mut n := 0 while n < LIMIT { n = plus(n) } print(n)"),
        ("captures_generators", 1000, "func factory(n) { func get() { return n } return get } func* numbers() { yield 1 yield 2 } mut i := 0 mut total := 0 while i < LIMIT { let f := factory(i) total += f() let g := numbers() for v in g { total += v } i += 1 } print(total)"),
        ("tasks", 100, "async func work(n) { return n + 1 } mut n := 0 while n < LIMIT { n = await work(n) } print(n)"),
        ("calls_sustained", 200000, "func plus(n) { return n + 1 } mut n := 0 while n < LIMIT { n = plus(n) } print(n)"),
        ("captures_generators_sustained", 10000, "func factory(n) { func get() { return n } return get } func* numbers() { yield 1 yield 2 } mut i := 0 mut total := 0 while i < LIMIT { let f := factory(i) total += f() let g := numbers() for v in g { total += v } i += 1 } print(total)"),
        ("tasks_sustained", 1000, "async func work(n) { return n + 1 } mut n := 0 while n < LIMIT { n = await work(n) } print(n)"),
        ("language_collections_structs", 5000, "struct Pair { left: int, right: int } mut i := 0 mut total := 0 while i < LIMIT { let pair := Pair { left: i, right: i + 1 } let values := [pair.left, pair.right] total += values[0] + values[1] i += 1 } print(total)"),
        ("closures_retained", 250, "func factory(n) { func get() { return n } return get } mut closures := [] mut i := 0 while i < LIMIT { closures = push(closures, factory(i)) i += 1 } print(closures[LIMIT - 1]())"),
        ("generator_nested_alias", 5000, "func* inner(n) { yield n yield n + 1 } func* outer(n) { for value in inner(n) { yield value } } mut i := 0 mut total := 0 while i < LIMIT { let generator := outer(i) let alias := generator for value in alias { total += value } i += 1 } print(total)"),
        ("promise_observers", 1000, "async func work(n) { return n + 1 } mut i := 0 mut total := 0 while i < LIMIT { let promise := work(i) total += await promise total += await promise i += 1 } print(total)"),
        ("scheduler_channels", 1000, "let queue := channel() async func work(value) { queue.send(value) return queue.receive() } mut i := 0 mut total := 0 while i < LIMIT { total += await work(i) i += 1 } print(total)"),
        ("task_admission_rejection", 100, "async func recurse(n) { if n == 0 { return 0 } return await recurse(n - 1) } mut caught := false try { await recurse(LIMIT) } except err { caught = true } print(caught)"),
        ("task_cancellation", 40, "shared_set(\"started\", 0) func work() { shared_add_int(\"started\", 1) loop {} } mut i := 0 while i < LIMIT { let handle := spawn_task(work) while (shared_get(\"started\") <= i) {} cancel_task(handle) try { await await_task(handle) } except err {} i += 1 } print(shared_get(\"started\"))"),
        ("ai_native_hash", 100, "let messages := [ai_message(\"user\", \"offline benchmark\")] let options := {\"endpoint\": \"https://example.test/v1/chat\", \"model\": \"fixture\"} mut i := 0 mut latest := \"\" while i < LIMIT { latest = ai_request_hash(messages, options) i += 1 } print(len(latest))"),
    ];
    println!("{{\"schema\":\"kujo.measurement-benchmark/v1\",\"samples_per_mode\":11,\"discarded_warmups_per_mode\":2,\"process_inclusive\":true,\"mode_order\":\"(round + offset) % 3, round 0..12\",\"workloads\":[");
    for (index, (name, limit, template)) in workloads.iter().enumerate() {
        let source = template.replace("LIMIT", &limit.to_string());
        let script = root.join("workload.kujo");
        fs::write(&script, &source).unwrap();
        let mut samples = [Vec::new(), Vec::new(), Vec::new()];
        let mut warmups = [Vec::new(), Vec::new(), Vec::new()];
        let mut expected = None;
        let mut enabled_reports = Vec::new();
        for round in 0..13 {
            for offset in 0..3 {
                let mode = (round + offset) % 3;
                let report = root.join(format!("{index}-{round}.json"));
                let mut command = Command::new(if mode == 0 { &args[1] } else { &args[2] });
                command.arg("run");
                if mode == 2 {
                    command.arg("--measurements").arg(&report);
                }
                command.arg(&script);
                let start = Instant::now();
                let output = command.output().unwrap();
                let elapsed = start.elapsed().as_nanos();
                assert!(output.status.success(), "{}: {:?}", name, output);
                let observed = (output.stdout, output.stderr);
                match &expected {
                    Some(expected) => assert_eq!(&observed, expected),
                    None => expected = Some(observed),
                }
                if mode == 2 && round >= 2 {
                    enabled_reports.push(fs::read_to_string(&report).unwrap());
                }
                if round < 2 {
                    warmups[mode].push(elapsed);
                } else {
                    samples[mode].push(elapsed);
                }
            }
        }
        if index > 0 {
            println!(",");
        }
        println!("{{\"name\":\"{name}\",\"iterations\":{limit},\"source\":{},\"warmups_ns\":{warmups:?},\"baseline_ns\":{:?},\"disabled_ns\":{:?},\"enabled_ns\":{:?},\"enabled_reports\":[{}]}}", json_string(&source), samples[0], samples[1], samples[2], enabled_reports.join(","));
    }
    println!("]}}");
    fs::remove_dir_all(root).unwrap();
}
