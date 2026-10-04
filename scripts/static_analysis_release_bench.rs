//! Process-inclusive startup, static-check and LSP diagnostic campaign.
//! rustc -O scripts/static_analysis_release_bench.rs -o /tmp/kujo-static-bench
//! kujo-static-bench BASELINE CANDIDATE > static-analysis.json
use std::{env, fs, process::Command, time::Instant};

fn run(binary: &str, arguments: &[&str]) -> u128 {
    let start = Instant::now();
    let output = Command::new(binary).args(arguments).output().unwrap();
    let elapsed = start.elapsed().as_nanos();
    assert!(output.status.success(), "{:?}", output);
    elapsed
}

fn samples(
    baseline: &str,
    candidate: &str,
    arguments: &[&str],
) -> (Vec<u128>, Vec<u128>, Vec<u128>, Vec<u128>) {
    let mut baseline_warmups = Vec::new();
    let mut candidate_warmups = Vec::new();
    let mut baseline_measured = Vec::new();
    let mut candidate_measured = Vec::new();
    for round in 0..23 {
        let (baseline_elapsed, candidate_elapsed) = if round % 2 == 0 {
            (run(baseline, arguments), run(candidate, arguments))
        } else {
            let candidate_elapsed = run(candidate, arguments);
            let baseline_elapsed = run(baseline, arguments);
            (baseline_elapsed, candidate_elapsed)
        };
        if round < 2 {
            baseline_warmups.push(baseline_elapsed);
            candidate_warmups.push(candidate_elapsed);
        } else {
            baseline_measured.push(baseline_elapsed);
            candidate_measured.push(candidate_elapsed);
        }
    }
    (
        baseline_warmups,
        candidate_warmups,
        baseline_measured,
        candidate_measured,
    )
}

fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3, "provide immutable baseline and candidate binaries");
    let root = env::temp_dir().join(format!("kujo-static-bench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let helper = root.join("helper.kujo");
    fs::write(
        &helper,
        "export const label: string := \"ready\"\nexport struct Profile { score: int }\nexport func make_profile(score: int) { return Profile { score: score } }\n",
    )
    .unwrap();
    let entry = root.join("entry.kujo");
    let mut source = String::from("import helper\n");
    for index in 0..200 {
        source.push_str(&format!(
            "let profile_{index} := helper.make_profile({index})\nlet score_{index}: int := profile_{index}.score\n"
        ));
    }
    fs::write(&entry, source).unwrap();
    let entry_text = entry.to_string_lossy().into_owned();
    let workloads = [
        ("startup", vec!["--version"]),
        ("project_check", vec!["check", entry_text.as_str()]),
        ("lsp_diagnostics", vec!["lsp-diagnostics", entry_text.as_str(), "--json"]),
    ];
    println!(
        "{{\"schema\":\"kujo.static-analysis-benchmark/v1\",\"samples_per_binary\":21,\"discarded_warmups_per_binary\":2,\"process_inclusive\":true,\"workloads\":["
    );
    for (index, (name, arguments)) in workloads.iter().enumerate() {
        let (baseline_warmups, candidate_warmups, baseline, candidate) =
            samples(&args[1], &args[2], arguments);
        if index > 0 {
            println!(",");
        }
        println!(
            "{{\"name\":\"{name}\",\"arguments\":{arguments:?},\"baseline_warmups_ns\":{baseline_warmups:?},\"candidate_warmups_ns\":{candidate_warmups:?},\"baseline_ns\":{baseline:?},\"candidate_ns\":{candidate:?}}}"
        );
    }
    println!("]}}");
    fs::remove_dir_all(root).unwrap();
}
