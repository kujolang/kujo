// Appended inside an exact source-extracted DepthCache module by the builder.
// Measures helper mechanics, not end-to-end runtime speed or an alternative cache.
fn owner(kind: usize, i: usize) -> Value {
    match kind {
        0 => Value::array(vec![Value::Int(i as i64)]),
        1 => Value::dict([(format!("k{i}").into(), Value::Int(i as i64))].into_iter().collect()),
        2 => Value::IntDict(Arc::new([(i as i64, Value::Int(i as i64))].into_iter().collect())),
        _ => Value::Struct { name: "State".into(), fields: Arc::new([("item".into(), Value::Int(i as i64))].into_iter().collect()) },
    }
}
fn slots(cache: &DepthCache) -> usize {
    cache.arrays.len() + cache.dictionaries.len() + cache.integer_dictionaries.len() + cache.structs.len()
}
pub fn run() {
    use std::{hint::black_box, time::Instant};
    // Host mutation must detach from the weak identity without changing snapshots.
    let mut cache = DepthCache::default();
    let mut root = Value::array(vec![Value::Int(1)]);
    cache.remember_path(&root, &[], 1);
    assert_eq!(cache.cached_depth(&root), Some(1));
    let snapshot = root.clone();
    if let Value::Array(items) = &mut root {
        Arc::make_mut(items)[0] = Value::array(vec![Value::Int(2)]);
    }
    assert_eq!(cache.cached_depth(&root), None);
    assert_eq!(cache.cached_depth(&snapshot), Some(1));
    // Cache must retain weak identities, not payload owners, and enforce its cap.
    let owners: Vec<_> = (0..1024).map(|i| owner(i % 4, i)).collect();
    let mut bounded = DepthCache::default();
    for value in &owners { bounded.remember_path(value, &[], 1); }
    assert_eq!(slots(&bounded), 0);
    let mut invalidated = DepthCache::default();
    for i in 0..2048 {
        let mut value = owner(0, i);
        invalidated.remember_path(&value, &[], 1);
        assert_eq!(invalidated.cached_depth(&value), Some(1));
        invalidated.release_path(&value, &[]);
        if let Value::Array(items) = &mut value {
            assert_eq!(Arc::weak_count(items), 0);
            Arc::make_mut(items)[0] = Value::array(vec![Value::Int(2)]);
        }
        invalidated.remember_path(&value, &[], 2);
        assert_eq!(invalidated.cached_depth(&value), Some(2));
    }
    println!("semantic_controls=passed");
    for round in 0..3 {
        for kind in 0..4 {
            for live in [1usize, 64, 512] {
                for mode in ["lookup", "release_remember"] {
                    let owners: Vec<_> = (0..live).map(|i| owner(kind, i)).collect();
                    let mut cache = DepthCache::default();
                    for value in &owners { cache.remember_path(value, &[], 1); }
                    let iterations = 100_000usize;
                    let started = Instant::now();
                    let mut sum = 0usize;
                    for i in 0..iterations {
                        let value = black_box(&owners[i % live]);
                        if mode == "release_remember" {
                            cache.release_path(value, &[]);
                            cache.remember_path(value, &[], 1);
                        }
                        sum += black_box(&mut cache).cached_depth(value).expect("cached owner");
                    }
                    let ns = started.elapsed().as_nanos();
                    assert_eq!(sum, iterations);
                    assert_eq!(slots(&cache), live);
                    println!("{round},{kind},{live},{mode},{iterations},{sum},{ns}");
                }
            }
        }
    }
}
