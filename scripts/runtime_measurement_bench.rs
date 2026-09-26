//! rustc -O scripts/runtime_measurement_bench.rs -o /tmp/kujo-measure-bench
//! /tmp/kujo-measure-bench BASELINE_KUJO CANDIDATE_KUJO > results.json
//! Process-inclusive paired samples; same profile/toolchain for both executables.
use std::{env, fs, process::Command, time::Instant};

fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3, "provide baseline and candidate executables");
    let root = env::temp_dir().join(format!("kujo-measure-bench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let workloads = [
        ("calls", "func plus(n) { return n + 1 } mut n := 0 while n < 10000 { n = plus(n) } print(n)"),
        ("captures_generators", "func factory(n) { func get() { return n } return get } func* numbers() { yield 1 yield 2 } mut i := 0 mut total := 0 while i < 1000 { let f := factory(i) total += f() let g := numbers() for v in g { total += v } i += 1 } print(total)"),
        ("tasks", "async func work(n) { return n + 1 } mut n := 0 while n < 100 { n = await work(n) } print(n)"),
    ];
    println!("{{\"schema\":\"kujo.measurement-benchmark/v1\",\"samples_per_mode\":11,\"process_inclusive\":true,\"workloads\":[");
    for (index, (name, source)) in workloads.iter().enumerate() {
        let script = root.join("workload.kujo");
        fs::write(&script, source).unwrap();
        let mut samples = [Vec::new(), Vec::new(), Vec::new()];
        let mut expected = None;
        // One discarded warmup triplet, then rotate order to reduce drift bias.
        for round in 0..12 {
            for order in 0..3 {
                let mode = (round + order) % 3;
                let report = root.join(format!("{index}-{round}.json"));
                let mut command = Command::new(if mode == 0 { &args[1] } else { &args[2] });
                command.arg("run");
                if mode == 2 { command.arg("--measurements").arg(&report); }
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
                if round > 0 { samples[mode].push(elapsed); }
            }
        }
        if index > 0 { println!(","); }
        println!("{{\"name\":\"{name}\",\"baseline_ns\":{:?},\"disabled_ns\":{:?},\"enabled_ns\":{:?}}}", samples[0], samples[1], samples[2]);
    }
    println!("]}}");
    fs::remove_dir_all(root).unwrap();
}
