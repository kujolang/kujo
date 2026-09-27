//! Optimized-build campaign, independent of the Kujo crate/dependency graph.
//! rustc -O scripts/runtime_measurement_release_bench.rs -o /tmp/kujo-release-bench
//! kujo-release-bench BASELINE CANDIDATE > release-overhead.json
use std::{env, fs, process::Command, time::Instant};

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
    ];
    println!("{{\"schema\":\"kujo.measurement-benchmark/v1\",\"samples_per_mode\":21,\"discarded_warmups_per_mode\":2,\"process_inclusive\":true,\"mode_order\":\"(round + offset) % 3, round 0..22\",\"workloads\":[");
    for (index, (name, limit, template)) in workloads.iter().enumerate() {
        let source = template.replace("LIMIT", &limit.to_string());
        let script = root.join("workload.kujo");
        fs::write(&script, &source).unwrap();
        let mut samples = [Vec::new(), Vec::new(), Vec::new()];
        let mut warmups = [Vec::new(), Vec::new(), Vec::new()];
        let mut expected = None;
        let mut enabled_reports = Vec::new();
        for round in 0..23 {
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
        // Sources are fixed ASCII without JSON-sensitive quotes or newlines.
        println!("{{\"name\":\"{name}\",\"iterations\":{limit},\"source\":\"{source}\",\"warmups_ns\":{warmups:?},\"baseline_ns\":{:?},\"disabled_ns\":{:?},\"enabled_ns\":{:?},\"enabled_reports\":[{}]}}", samples[0], samples[1], samples[2], enabled_reports.join(","));
    }
    println!("]}}");
    fs::remove_dir_all(root).unwrap();
}
