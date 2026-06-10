// Statistical analysis for benchmark results

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Statistics {
    pub mean: Duration,
    pub median: Duration,
    pub min: Duration,
    pub max: Duration,
    pub stddev: Duration,
    pub samples: usize,
}

impl Statistics {
    pub fn from_samples(samples: &[Duration]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }

        let mean = calculate_mean(samples);
        let median = calculate_median(samples);
        let min = samples.iter().min().copied().unwrap();
        let max = samples.iter().max().copied().unwrap();
        let stddev = calculate_stddev(samples, mean);

        Some(Self { mean, median, min, max, stddev, samples: samples.len() })
    }

    pub fn format_duration(duration: Duration) -> String {
        let nanos = duration.as_nanos();
        if nanos < 1_000 {
            format!("{} ns", nanos)
        } else if nanos < 1_000_000 {
            format!("{:.2} µs", nanos as f64 / 1_000.0)
        } else if nanos < 1_000_000_000 {
            format!("{:.2} ms", nanos as f64 / 1_000_000.0)
        } else {
            format!("{:.2} s", nanos as f64 / 1_000_000_000.0)
        }
    }
}

fn calculate_mean(samples: &[Duration]) -> Duration {
    let total: Duration = samples.iter().sum();
    total / samples.len() as u32
}

fn calculate_median(samples: &[Duration]) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort();
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2
    } else {
        sorted[mid]
    }
}

fn calculate_stddev(samples: &[Duration], mean: Duration) -> Duration {
    let mean_nanos = mean.as_nanos() as f64;
    let variance: f64 = samples
        .iter()
        .map(|s| {
            let diff = s.as_nanos() as f64 - mean_nanos;
            diff * diff
        })
        .sum::<f64>()
        / samples.len() as f64;

    Duration::from_nanos(variance.sqrt() as u64)
}

/// Compare speedup between baseline and optimized implementations
/// Infrastructure for performance regression detection
#[allow(dead_code)]
pub fn compare_speedup(baseline: Duration, optimized: Duration) -> f64 {
    if optimized.as_nanos() == 0 {
        return 0.0;
    }
    baseline.as_nanos() as f64 / optimized.as_nanos() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics() {
        let samples = vec![
            Duration::from_millis(10),
            Duration::from_millis(12),
            Duration::from_millis(11),
            Duration::from_millis(13),
            Duration::from_millis(10),
        ];

        let stats = Statistics::from_samples(&samples).unwrap();
        assert!(stats.mean >= Duration::from_millis(10));
        assert!(stats.mean <= Duration::from_millis(13));
        assert_eq!(stats.median, Duration::from_millis(11));
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(13));
        assert_eq!(stats.samples, 5);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(Statistics::format_duration(Duration::from_nanos(500)), "500 ns");
        assert_eq!(Statistics::format_duration(Duration::from_micros(500)), "500.00 µs");
        assert_eq!(Statistics::format_duration(Duration::from_millis(500)), "500.00 ms");
        assert_eq!(Statistics::format_duration(Duration::from_secs(5)), "5.00 s");
    }

    #[test]
    fn test_compare_speedup() {
        let baseline = Duration::from_millis(100);
        let optimized = Duration::from_millis(10);
        let speedup = compare_speedup(baseline, optimized);
        assert!((speedup - 10.0).abs() < 0.01);
    }
}
