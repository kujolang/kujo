// Isolate lookup cost only: excludes marshalling, diagnostics and VM callbacks.
// This single-threaded process mutates only its own environment between runs.
use std::{hint::black_box, time::Instant};
const KEY: &str = "DEBUG_JIT_CALL";
fn set(mode: &str) {
    match mode {
        "absent" => std::env::remove_var(KEY),
        "empty" => std::env::set_var(KEY, ""),
        "enabled" => std::env::set_var(KEY, "1"),
        "unicode" => std::env::set_var(KEY, "λ"),
        #[cfg(unix)]
        "invalid" => {
            use std::os::unix::ffi::OsStringExt;
            std::env::set_var(KEY, std::ffi::OsString::from_vec(vec![255]));
        }
        _ => panic!("unknown mode"),
    }
}
#[inline(never)]
fn checks(reuse: bool, callback: impl FnOnce()) -> u64 {
    let entry = std::env::var(black_box(KEY)).is_ok();
    let before_call = if reuse { entry } else { std::env::var(black_box(KEY)).is_ok() };
    callback();
    let returned = std::env::var(black_box(KEY)).is_ok();
    u64::from(entry) + 2 * u64::from(before_call) + 4 * u64::from(returned)
}
fn main() {
    let modes = if cfg!(unix) {
        vec!["absent", "empty", "enabled", "unicode", "invalid"]
    } else {
        vec!["absent", "empty", "enabled", "unicode"]
    };
    // Every ordered environment transition, including invalid Unicode, retains
    // the post-callback read. This does not model concurrent environment writers.
    for from in &modes {
        for to in &modes {
            let enabled = |s: &str| s != "absent" && s != "invalid";
            let expected = 3 * u64::from(enabled(from)) + 4 * u64::from(enabled(to));
            for reuse in [false, true] {
                set(from);
                assert_eq!(checks(reuse, || set(to)), expected);
            }
        }
    }
    println!("controls={}", modes.len() * modes.len() * 2);
    let iterations = 200_000u64;
    for pair in 0..7 {
        for mode in &modes {
            set(mode);
            for reuse in if pair % 2 == 0 { [false, true] } else { [true, false] } {
                for _ in 0..1000 {
                    black_box(checks(reuse, || {}));
                }
                let started = Instant::now();
                let mut sum = 0;
                for _ in 0..iterations {
                    sum += black_box(checks(reuse, || {}));
                }
                let ns = started.elapsed().as_nanos();
                assert_eq!(
                    sum,
                    if *mode == "absent" || *mode == "invalid" { 0 } else { iterations * 7 }
                );
                println!("pair={pair} mode={mode} reuse={reuse} iterations={iterations} ns={ns} sum={sum}");
            }
        }
    }
}
