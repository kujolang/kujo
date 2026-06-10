// High-precision timing utilities for benchmarks

use std::time::{Duration, Instant};

pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }

    pub fn start() -> Self {
        Self::new()
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Reset timer to current time (for multi-phase benchmarks)
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a function multiple times and collect timing samples
/// Infrastructure for programmatic benchmarking
#[allow(dead_code)]
pub fn benchmark<F>(_name: &str, iterations: usize, mut f: F) -> Vec<Duration>
where
    F: FnMut(),
{
    let mut samples = Vec::with_capacity(iterations);

    // Warmup run (not timed)
    f();

    // Collect samples
    for _ in 0..iterations {
        let timer = Timer::start();
        f();
        samples.push(timer.elapsed());
    }

    samples
}

/// Time a single execution
/// Infrastructure for one-off performance measurements
#[allow(dead_code)]
pub fn time_once<F>(f: F) -> Duration
where
    F: FnOnce(),
{
    let timer = Timer::start();
    f();
    timer.elapsed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_timer() {
        let timer = Timer::start();
        thread::sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(50));
    }

    #[test]
    fn test_benchmark() {
        let samples = benchmark("test", 5, || {
            thread::sleep(Duration::from_millis(1));
        });
        assert_eq!(samples.len(), 5);
        for sample in samples {
            assert!(sample >= Duration::from_millis(1));
        }
    }

    #[test]
    fn test_time_once() {
        let duration = time_once(|| {
            thread::sleep(Duration::from_millis(5));
        });
        assert!(duration >= Duration::from_millis(5));
    }
}
