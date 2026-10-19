//! Distinct imports sharing one directory. Compile against baseline/candidate
//! libkujo with identical flags; no wall-clock assertion belongs in CI.
use kujo::{interpreter::Value, module::ModuleLoader};
use std::{fs, time::Instant};

fn main() {
    let root = std::env::temp_dir().join(format!("kujo-module-load-bench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    for count in [100, 500] {
        for i in 0..count {
            fs::write(root.join(format!("load_{i}.kujo")), format!("export value := {i}\n"))
                .unwrap();
        }
        let mut samples = Vec::new();
        for round in 0..9 {
            let mut loader = ModuleLoader::new();
            loader.add_search_path(&root);
            let start = Instant::now();
            for i in 0..count {
                assert!(matches!(
                    loader.get_symbol(&format!("load_{i}"), "value").unwrap(),
                    Value::Int(value) if value == i
                ));
            }
            if round >= 2 {
                samples.push(start.elapsed().as_nanos());
            }
        }
        samples.sort_unstable();
        println!("modules={count} median_ns={} samples_ns={samples:?}", samples[3]);
    }
    fs::remove_dir_all(root).unwrap();
}
