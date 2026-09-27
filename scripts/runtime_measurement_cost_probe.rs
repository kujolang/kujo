//! Focused component costs, not an additive model of total runtime overhead.
//! Link against the same optimized candidate library and serde_json artifacts.
use kujo::benchmarks::profiler::runtime;
use std::{
    fs,
    hint::black_box,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

fn samples(mut operation: impl FnMut(), iterations: usize) -> Vec<f64> {
    let mut values = Vec::new();
    for round in 0..23 {
        let started = Instant::now();
        for _ in 0..iterations {
            operation();
        }
        let ns = started.elapsed().as_nanos() as f64 / iterations as f64;
        if round >= 2 {
            values.push(ns);
        }
    }
    values
}
fn main() {
    runtime::start().unwrap();
    let report = runtime::snapshot("success");
    let counter = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!("kujo-cost-probe-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("report.json");
    let mut results = serde_json::Map::new();
    results.insert(
        "empty_black_box".into(),
        serde_json::json!(samples(
            || {
                black_box(());
            },
            1_000_000
        )),
    );
    // Same saturating relaxed CAS operation as runtime::add; uncontended only.
    results.insert(
        "saturating_atomic_increment".into(),
        serde_json::json!(samples(
            || {
                black_box(counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                    Some(v.saturating_add(black_box(1)))
                }))
                .unwrap();
            },
            1_000_000
        )),
    );
    results.insert(
        "monotonic_now_elapsed".into(),
        serde_json::json!(samples(
            || {
                black_box(black_box(Instant::now()).elapsed());
            },
            100_000
        )),
    );
    results.insert(
        "actual_snapshot_including_process_usage".into(),
        serde_json::json!(samples(
            || {
                black_box(runtime::snapshot("success"));
            },
            1000
        )),
    );
    results.insert(
        "serialize_report_to_vec".into(),
        serde_json::json!(samples(
            || {
                black_box(serde_json::to_vec(&report).unwrap());
            },
            1000
        )),
    );
    for (name, write, sync) in [
        ("exclusive_create", false, false),
        ("unbuffered_json_file", true, false),
        ("unbuffered_json_file_sync", true, true),
    ] {
        let mut values = Vec::new();
        for round in 0..23 {
            let started = Instant::now();
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&path).unwrap();
            if write {
                serde_json::to_writer(&mut file, &report).unwrap();
                file.write_all(b"\n").unwrap();
            }
            if sync {
                file.sync_all().unwrap();
            }
            drop(file);
            let ns = started.elapsed().as_nanos() as f64;
            fs::remove_file(&path).unwrap(); // cleanup is outside the timed interval
            if round >= 2 {
                values.push(ns);
            }
        }
        results.insert(name.into(), serde_json::json!(values));
    }
    println!(
        "{}",
        serde_json::json!({"schema":"kujo.measurement-cost-probe/v1", "units":"nanoseconds_per_operation", "samples":results, "discarded_warmups":2, "samples_per_operation":21, "note":"component probes are not additive attribution; no contention; actual snapshot, serialization and export APIs"})
    );
    fs::remove_dir(&root).unwrap();
}
