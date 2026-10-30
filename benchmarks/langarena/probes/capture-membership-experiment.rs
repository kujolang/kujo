//! Isolated exact-membership experiment; no runtime integration or speed claim.
//! Include the real binding representation so the lookup control stays faithful.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    Mutable,
    LetImmutable,
    Const,
}
#[path = "../../../src/interpreter/binding_map.rs"]
mod binding_map;
use binding_map::BindingMap;
use std::{collections::HashMap, hint::black_box, time::Instant};

fn current(names: &HashMap<String, ()>, metadata: &BindingMap) -> bool {
    names.len() == metadata.len() && names.keys().all(|name| metadata.contains_key(name))
}
fn ordered(names: &HashMap<String, ()>, metadata: &BindingMap) -> bool {
    metadata.matches_names(names)
}
fn lookup_counts(names: &HashMap<String, ()>, metadata: &BindingMap) -> (usize, usize) {
    let mut before = 0;
    let mut after = 0;
    if names.len() == metadata.len() {
        names.keys().all(|name| {
            before += 1;
            metadata.contains_key(name)
        });
        let mut actual = names.keys();
        for expected in metadata.keys() {
            let name = actual.next().unwrap();
            if name != expected {
                after += 1;
                if metadata.contains_key(name) {
                    actual.all(|name| {
                        after += 1;
                        metadata.contains_key(name)
                    });
                }
                break;
            }
        }
    }
    (before, after)
}
fn measure(
    f: fn(&HashMap<String, ()>, &BindingMap) -> bool,
    names: &HashMap<String, ()>,
    metadata: &BindingMap,
    calls: usize,
) -> u128 {
    let start = Instant::now();
    for _ in 0..calls {
        black_box(f(black_box(names), black_box(metadata)));
    }
    start.elapsed().as_nanos()
}
fn main() {
    println!("width,case,round,expected,before_lookups,after_lookups,before_ns,after_ns");
    for width in [0, 1, 128, 8192, 16384] {
        let original: HashMap<String, ()> =
            (0..width).map(|i| (format!("extra_{i}"), ())).collect();
        let metadata = BindingMap::compact_from(
            original.keys().map(|name| (name.as_str(), BindingKind::Mutable)),
        )
        .unwrap();
        let reordered: HashMap<String, ()> =
            original.keys().map(|name| (name.clone(), ())).collect();
        let mut inserted = original.clone();
        inserted.insert("new_member".into(), ());
        let mut cases = vec![
            ("unchanged", original.clone(), true),
            ("reordered", reordered, true),
            ("inserted", inserted, false),
        ];
        if width > 0 {
            let keys: Vec<_> = original.keys().cloned().collect();
            for (label, key) in [("first_replaced", &keys[0]), ("last_replaced", &keys[width - 1])]
            {
                let mut replaced = original.clone();
                replaced.remove(key);
                replaced.insert("new_member".into(), ());
                cases.push((label, replaced, false));
            }
            let mut removed = original.clone();
            removed.remove(&keys[0]);
            cases.push(("removed", removed, false));
        }
        for (name, map, expected) in cases {
            assert_eq!(current(&map, &metadata), expected, "current {width} {name}");
            assert_eq!(ordered(&map, &metadata), expected, "ordered {width} {name}");
            let (before_count, after_count) = lookup_counts(&map, &metadata);
            if name == "unchanged" {
                assert_eq!((before_count, after_count), (width, 0));
            }
            for round in 0..5 {
                let calls = 128;
                let (before, after) = if round % 2 == 0 {
                    (
                        measure(current, &map, &metadata, calls),
                        measure(ordered, &map, &metadata, calls),
                    )
                } else {
                    let after = measure(ordered, &map, &metadata, calls);
                    (measure(current, &map, &metadata, calls), after)
                };
                println!("{width},{name},{round},{expected},{before_count},{after_count},{before},{after}");
            }
        }
    }
}
