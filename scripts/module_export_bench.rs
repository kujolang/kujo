//! Local cached-import microbenchmark; compile against the baseline and candidate
//! libkujo with identical rustc flags. Timings are observations, not CI thresholds.
use kujo::module::ModuleLoader;
use std::{fs, hint::black_box, time::Instant};

fn main() {
    let root = std::env::temp_dir().join(format!("kujo-export-bench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    for count in [100, 1000, 5000] {
        let source: String = (0..count).map(|i| format!("export value_{i} := {i}\n")).collect();
        fs::write(root.join("payload.kujo"), source).unwrap();
        let mut loader = ModuleLoader::new();
        loader.add_search_path(&root);
        assert_eq!(loader.get_all_exports("payload").unwrap().len(), count);
        let mut samples = Vec::new();
        for round in 0..9 {
            let start = Instant::now();
            for _ in 0..100 {
                let exports = loader.get_all_exports("payload").unwrap();
                assert_eq!(exports.len(), count);
                black_box(exports);
            }
            if round >= 2 {
                samples.push(start.elapsed().as_nanos());
            }
        }
        samples.sort_unstable();
        println!("exports={count} iterations=100 median_ns={} samples_ns={samples:?}", samples[3]);
    }
    fs::remove_dir_all(root).unwrap();
}
