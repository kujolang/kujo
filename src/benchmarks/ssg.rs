use std::path::{Path, PathBuf};
use std::process::Command;

pub const SSG_VARIABILITY_WARNING_THRESHOLD_PERCENT: f64 = 5.0;
pub const SSG_TREND_WARNING_THRESHOLD_PERCENT: f64 = 10.0;
pub const SSG_MEAN_MEDIAN_DRIFT_WARNING_THRESHOLD_PERCENT: f64 = 7.5;
pub const SSG_RANGE_SPREAD_WARNING_THRESHOLD_PERCENT: f64 = 42.0;
const SSG_PROFILE_ASYNC_ENV: &str = "KUJO_BENCH_SSG_PROFILE_ASYNC";

#[derive(Debug, Clone, Copy)]
pub struct SsgWarningThresholds {
    pub variability_percent: f64,
    pub trend_percent: f64,
    pub mean_median_drift_percent: f64,
    pub range_spread_percent: f64,
}

impl Default for SsgWarningThresholds {
    fn default() -> Self {
        Self {
            variability_percent: SSG_VARIABILITY_WARNING_THRESHOLD_PERCENT,
            trend_percent: SSG_TREND_WARNING_THRESHOLD_PERCENT,
            mean_median_drift_percent: SSG_MEAN_MEDIAN_DRIFT_WARNING_THRESHOLD_PERCENT,
            range_spread_percent: SSG_RANGE_SPREAD_WARNING_THRESHOLD_PERCENT,
        }
    }
}

pub fn format_ssg_measurement_warning_header(thresholds: SsgWarningThresholds) -> String {
    format!(
        "Measurement quality warnings (CV >= {:.2}%, mean/median drift >= {:.2}%, range spread >= {:.2}%):",
        thresholds.variability_percent.max(0.0),
        thresholds.mean_median_drift_percent.max(0.0),
        thresholds.range_spread_percent.max(0.0)
    )
}

pub fn format_ssg_trend_warning_header(thresholds: SsgWarningThresholds) -> String {
    format!("Trend stability warnings (drift >= {:.2}%):", thresholds.trend_percent.max(0.0))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SsgThroughputGateStatus {
    pub threshold_ms: f64,
    pub observed_median_ms: f64,
    pub margin_ms: f64,
    pub margin_percent: f64,
    pub passed: bool,
}

pub fn evaluate_ssg_throughput_gate(
    observed_median_ms: f64,
    threshold_ms: f64,
) -> Result<SsgThroughputGateStatus, String> {
    if !observed_median_ms.is_finite() || observed_median_ms < 0.0 {
        return Err(format!(
            "SSG throughput gate observed median must be finite and >= 0.0 ms, got {}",
            observed_median_ms
        ));
    }

    if !threshold_ms.is_finite() || threshold_ms <= 0.0 {
        return Err(format!(
            "SSG throughput gate threshold must be finite and > 0.0 ms, got {}",
            threshold_ms
        ));
    }

    let margin_ms = threshold_ms - observed_median_ms;
    let margin_percent = (margin_ms / threshold_ms) * 100.0;
    let passed = observed_median_ms <= threshold_ms;

    Ok(SsgThroughputGateStatus {
        threshold_ms,
        observed_median_ms,
        margin_ms,
        margin_percent,
        passed,
    })
}

pub fn format_ssg_throughput_gate_summary(gate: SsgThroughputGateStatus) -> String {
    let status_label = if gate.passed { "PASS" } else { "FAIL" };
    let comparator = if gate.passed { "<=" } else { ">" };

    format!(
        "Throughput gate [{}]: Kujo median build {:.3} ms {} target {:.3} ms (margin {:+.3} ms, {:+.2}%)",
        status_label,
        gate.observed_median_ms,
        comparator,
        gate.threshold_ms,
        gate.margin_ms,
        gate.margin_percent
    )
}

pub fn collect_ssg_warning_operator_hints(thresholds: SsgWarningThresholds) -> Vec<String> {
    vec![
        "Tune CV sensitivity with --variability-warning-threshold <PERCENT>".to_string(),
        "Tune trend sensitivity with --trend-warning-threshold <PERCENT>".to_string(),
        "Tune skew sensitivity with --mean-median-drift-warning-threshold <PERCENT>".to_string(),
        "Tune range-spread sensitivity with --range-spread-warning-threshold <PERCENT>".to_string(),
        format!(
            "Current thresholds: CV {:.2}%, trend {:.2}%, mean/median {:.2}%, range spread {:.2}%",
            thresholds.variability_percent.max(0.0),
            thresholds.trend_percent.max(0.0),
            thresholds.mean_median_drift_percent.max(0.0),
            thresholds.range_spread_percent.max(0.0)
        ),
    ]
}

#[derive(Debug, Clone)]
pub struct SsgRunStatistics {
    pub runs: usize,
    pub mean: f64,
    pub median: f64,
    pub p90: f64,
    pub p95: f64,
    pub min: f64,
    pub max: f64,
    pub stddev: f64,
}

impl SsgRunStatistics {
    fn percentile(sorted_samples: &[f64], percentile: f64) -> f64 {
        if sorted_samples.is_empty() {
            return 0.0;
        }

        if sorted_samples.len() == 1 {
            return sorted_samples[0];
        }

        let clamped_percentile = percentile.clamp(0.0, 100.0);
        let rank = (clamped_percentile / 100.0) * (sorted_samples.len() - 1) as f64;
        let lower_index = rank.floor() as usize;
        let upper_index = rank.ceil() as usize;

        if lower_index == upper_index {
            return sorted_samples[lower_index];
        }

        let interpolation_weight = rank - lower_index as f64;
        let lower_value = sorted_samples[lower_index];
        let upper_value = sorted_samples[upper_index];
        lower_value + ((upper_value - lower_value) * interpolation_weight)
    }

    pub fn from_samples(samples: &[f64]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }

        let runs = samples.len();
        let sum: f64 = samples.iter().sum();
        let mean = sum / runs as f64;

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));

        let median = if runs % 2 == 0 {
            let right = runs / 2;
            let left = right - 1;
            (sorted[left] + sorted[right]) / 2.0
        } else {
            sorted[runs / 2]
        };

        let p90 = Self::percentile(&sorted, 90.0);
        let p95 = Self::percentile(&sorted, 95.0);

        let min = *sorted.first().unwrap_or(&0.0);
        let max = *sorted.last().unwrap_or(&0.0);

        let variance = samples
            .iter()
            .map(|value| {
                let diff = *value - mean;
                diff * diff
            })
            .sum::<f64>()
            / runs as f64;

        let stddev = variance.sqrt();

        Some(SsgRunStatistics { runs, mean, median, p90, p95, min, max, stddev })
    }

    pub fn coefficient_of_variation_percent(&self) -> Option<f64> {
        let mean_abs = self.mean.abs();
        if mean_abs <= f64::EPSILON {
            return None;
        }

        Some((self.stddev / mean_abs) * 100.0)
    }

    pub fn is_high_variability(&self, threshold_percent: f64) -> bool {
        if self.runs < 3 {
            return false;
        }

        self.coefficient_of_variation_percent().is_some_and(|cv| cv >= threshold_percent.max(0.0))
    }

    pub fn mean_median_drift_percent(&self) -> Option<f64> {
        let median_abs = self.median.abs();
        if median_abs <= f64::EPSILON {
            return None;
        }

        Some(((self.mean - self.median).abs() / median_abs) * 100.0)
    }

    pub fn is_high_mean_median_drift(&self, threshold_percent: f64) -> bool {
        if self.runs < 3 {
            return false;
        }

        self.mean_median_drift_percent().is_some_and(|drift| drift >= threshold_percent.max(0.0))
    }

    pub fn range_spread_percent(&self) -> Option<(f64, f64)> {
        let median_abs = self.median.abs();
        if median_abs <= f64::EPSILON {
            return None;
        }

        let lower_percent = ((self.median - self.min).abs() / median_abs) * 100.0;
        let upper_percent = ((self.max - self.median).abs() / median_abs) * 100.0;
        Some((lower_percent, upper_percent))
    }

    pub fn is_high_range_spread(&self, threshold_percent: f64) -> bool {
        if self.runs < 3 {
            return false;
        }

        self.range_spread_percent().is_some_and(|(lower, upper)| {
            let threshold = threshold_percent.max(0.0);
            lower >= threshold || upper >= threshold
        })
    }
}

#[derive(Debug, Clone)]
pub struct SsgStageProfileStatistics {
    pub read_ms: SsgRunStatistics,
    pub render_write_ms: SsgRunStatistics,
}

#[derive(Debug, Clone)]
pub struct SsgBenchmarkAggregateResult {
    pub files: usize,
    pub kujo_checksum: i128,
    pub kujo_build_ms: SsgRunStatistics,
    pub kujo_files_per_sec: SsgRunStatistics,
    pub kujo_stage_profile: Option<SsgStageProfileStatistics>,
    pub python_build_ms: Option<SsgRunStatistics>,
    pub python_files_per_sec: Option<SsgRunStatistics>,
    pub python_stage_profile: Option<SsgStageProfileStatistics>,
    pub kujo_vs_python_speedup: Option<SsgRunStatistics>,
}

#[derive(Debug, Clone)]
pub struct SsgStageProfile {
    pub read_ms: f64,
    pub render_write_ms: f64,
}

impl SsgStageProfile {
    pub fn total_profiled_ms(&self) -> f64 {
        self.read_ms + self.render_write_ms
    }

    pub fn bottleneck_stage(&self) -> Option<(&'static str, f64, f64)> {
        let total = self.total_profiled_ms();
        if total <= 0.0 {
            return None;
        }

        if self.read_ms >= self.render_write_ms {
            Some(("read", self.read_ms, (self.read_ms / total) * 100.0))
        } else {
            Some(("render/write", self.render_write_ms, (self.render_write_ms / total) * 100.0))
        }
    }
}

#[derive(Debug, Clone)]
pub struct SsgBenchmarkResult {
    pub files: usize,
    pub kujo_build_ms: f64,
    pub kujo_files_per_sec: f64,
    pub kujo_checksum: i128,
    pub kujo_stage_profile: Option<SsgStageProfile>,
    pub python_build_ms: Option<f64>,
    pub python_files_per_sec: Option<f64>,
    pub python_stage_profile: Option<SsgStageProfile>,
}

#[derive(Debug, Clone)]
pub struct SsgTrendMetric {
    pub first: f64,
    pub last: f64,
    pub absolute_delta: f64,
    pub percent_delta: Option<f64>,
}

impl SsgTrendMetric {
    fn from_first_last(first: f64, last: f64) -> Self {
        let absolute_delta = last - first;
        let percent_delta = if first.abs() <= f64::EPSILON {
            None
        } else {
            Some((absolute_delta / first.abs()) * 100.0)
        };

        SsgTrendMetric { first, last, absolute_delta, percent_delta }
    }
}

#[derive(Debug, Clone)]
pub struct SsgBenchmarkTrendReport {
    pub measured_runs: usize,
    pub kujo_build_ms: SsgTrendMetric,
    pub kujo_files_per_sec: SsgTrendMetric,
    pub python_build_ms: Option<SsgTrendMetric>,
    pub python_files_per_sec: Option<SsgTrendMetric>,
    pub kujo_vs_python_speedup: Option<SsgTrendMetric>,
}

impl SsgBenchmarkResult {
    pub fn kujo_vs_python_speedup(&self) -> Option<f64> {
        self.python_build_ms.map(|python_ms| {
            if self.kujo_build_ms <= 0.0 {
                0.0
            } else {
                python_ms / self.kujo_build_ms
            }
        })
    }
}

pub fn analyze_ssg_benchmark_trends(
    run_results: &[SsgBenchmarkResult],
) -> Result<Option<SsgBenchmarkTrendReport>, String> {
    if run_results.len() < 2 {
        return Ok(None);
    }

    let first = run_results
        .first()
        .ok_or_else(|| "Cannot analyze trends: no benchmark runs provided".to_string())?;
    let last = run_results
        .last()
        .ok_or_else(|| "Cannot analyze trends: no benchmark runs provided".to_string())?;

    let all_have_python = run_results
        .iter()
        .all(|result| result.python_build_ms.is_some() && result.python_files_per_sec.is_some());
    let none_have_python = run_results
        .iter()
        .all(|result| result.python_build_ms.is_none() && result.python_files_per_sec.is_none());

    if !all_have_python && !none_have_python {
        return Err(
            "Cannot analyze SSG benchmark trends: inconsistent Python comparison presence across runs"
                .to_string(),
        );
    }

    let python_build_ms = if all_have_python {
        Some(SsgTrendMetric::from_first_last(
            first.python_build_ms.ok_or_else(|| {
                "Cannot analyze trends: first run missing Python build metric".to_string()
            })?,
            last.python_build_ms.ok_or_else(|| {
                "Cannot analyze trends: last run missing Python build metric".to_string()
            })?,
        ))
    } else {
        None
    };

    let python_files_per_sec = if all_have_python {
        Some(SsgTrendMetric::from_first_last(
            first.python_files_per_sec.ok_or_else(|| {
                "Cannot analyze trends: first run missing Python throughput metric".to_string()
            })?,
            last.python_files_per_sec.ok_or_else(|| {
                "Cannot analyze trends: last run missing Python throughput metric".to_string()
            })?,
        ))
    } else {
        None
    };

    let kujo_vs_python_speedup = if all_have_python {
        Some(SsgTrendMetric::from_first_last(
            first.kujo_vs_python_speedup().ok_or_else(|| {
                "Cannot analyze trends: first run missing Kujo/Python speedup metric".to_string()
            })?,
            last.kujo_vs_python_speedup().ok_or_else(|| {
                "Cannot analyze trends: last run missing Kujo/Python speedup metric".to_string()
            })?,
        ))
    } else {
        None
    };

    Ok(Some(SsgBenchmarkTrendReport {
        measured_runs: run_results.len(),
        kujo_build_ms: SsgTrendMetric::from_first_last(first.kujo_build_ms, last.kujo_build_ms),
        kujo_files_per_sec: SsgTrendMetric::from_first_last(
            first.kujo_files_per_sec,
            last.kujo_files_per_sec,
        ),
        python_build_ms,
        python_files_per_sec,
        kujo_vs_python_speedup,
    }))
}

pub fn aggregate_ssg_results(
    run_results: &[SsgBenchmarkResult],
) -> Result<SsgBenchmarkAggregateResult, String> {
    if run_results.is_empty() {
        return Err("Cannot aggregate SSG benchmark results: no runs provided".to_string());
    }

    let baseline = &run_results[0];
    let files = baseline.files;
    let checksum = baseline.kujo_checksum;

    let mut kujo_build_samples = Vec::with_capacity(run_results.len());
    let mut kujo_throughput_samples = Vec::with_capacity(run_results.len());
    let mut kujo_read_samples = Vec::new();
    let mut kujo_render_write_samples = Vec::new();

    let mut python_build_samples = Vec::new();
    let mut python_throughput_samples = Vec::new();
    let mut python_read_samples = Vec::new();
    let mut python_render_write_samples = Vec::new();
    let mut speedup_samples = Vec::new();

    let all_have_python = run_results
        .iter()
        .all(|result| result.python_build_ms.is_some() && result.python_files_per_sec.is_some());
    let none_have_python = run_results
        .iter()
        .all(|result| result.python_build_ms.is_none() && result.python_files_per_sec.is_none());

    if !all_have_python && !none_have_python {
        return Err(
            "Cannot aggregate SSG benchmark results: inconsistent Python comparison presence across runs"
                .to_string(),
        );
    }

    for result in run_results {
        if result.files != files {
            return Err(format!(
                "Cannot aggregate SSG benchmark results: file count mismatch across runs (expected {}, got {})",
                files, result.files
            ));
        }

        if result.kujo_checksum != checksum {
            return Err(format!(
                "Cannot aggregate SSG benchmark results: Kujo checksum mismatch across runs (expected {}, got {})",
                checksum, result.kujo_checksum
            ));
        }

        kujo_build_samples.push(result.kujo_build_ms);
        kujo_throughput_samples.push(result.kujo_files_per_sec);

        if let Some(profile) = result.kujo_stage_profile.as_ref() {
            kujo_read_samples.push(profile.read_ms);
            kujo_render_write_samples.push(profile.render_write_ms);
        }

        if all_have_python {
            let python_build_ms = result.python_build_ms.ok_or_else(|| {
                "Cannot aggregate SSG benchmark results: missing Python build metric".to_string()
            })?;
            let python_files_per_sec = result.python_files_per_sec.ok_or_else(|| {
                "Cannot aggregate SSG benchmark results: missing Python throughput metric"
                    .to_string()
            })?;

            python_build_samples.push(python_build_ms);
            python_throughput_samples.push(python_files_per_sec);

            if let Some(profile) = result.python_stage_profile.as_ref() {
                python_read_samples.push(profile.read_ms);
                python_render_write_samples.push(profile.render_write_ms);
            }

            if let Some(speedup) = result.kujo_vs_python_speedup() {
                speedup_samples.push(speedup);
            } else {
                return Err(
                    "Cannot aggregate SSG benchmark results: missing speedup metric for Python comparison"
                        .to_string(),
                );
            }
        }
    }

    let kujo_stage_profile = if kujo_read_samples.len() == run_results.len()
        && kujo_render_write_samples.len() == run_results.len()
    {
        Some(SsgStageProfileStatistics {
            read_ms: SsgRunStatistics::from_samples(&kujo_read_samples)
                .ok_or_else(|| "Cannot aggregate Kujo stage profile read metrics".to_string())?,
            render_write_ms: SsgRunStatistics::from_samples(&kujo_render_write_samples)
                .ok_or_else(|| {
                    "Cannot aggregate Kujo stage profile render/write metrics".to_string()
                })?,
        })
    } else {
        None
    };

    let python_stage_profile = if all_have_python
        && python_read_samples.len() == run_results.len()
        && python_render_write_samples.len() == run_results.len()
    {
        Some(SsgStageProfileStatistics {
            read_ms: SsgRunStatistics::from_samples(&python_read_samples)
                .ok_or_else(|| "Cannot aggregate Python stage profile read metrics".to_string())?,
            render_write_ms: SsgRunStatistics::from_samples(&python_render_write_samples)
                .ok_or_else(|| {
                    "Cannot aggregate Python stage profile render/write metrics".to_string()
                })?,
        })
    } else {
        None
    };

    let python_build_ms = if all_have_python {
        Some(
            SsgRunStatistics::from_samples(&python_build_samples)
                .ok_or_else(|| "Cannot aggregate Python build metrics".to_string())?,
        )
    } else {
        None
    };

    let python_files_per_sec = if all_have_python {
        Some(
            SsgRunStatistics::from_samples(&python_throughput_samples)
                .ok_or_else(|| "Cannot aggregate Python throughput metrics".to_string())?,
        )
    } else {
        None
    };

    let kujo_vs_python_speedup = if all_have_python {
        Some(
            SsgRunStatistics::from_samples(&speedup_samples)
                .ok_or_else(|| "Cannot aggregate Kujo vs Python speedup metrics".to_string())?,
        )
    } else {
        None
    };

    Ok(SsgBenchmarkAggregateResult {
        files,
        kujo_checksum: checksum,
        kujo_build_ms: SsgRunStatistics::from_samples(&kujo_build_samples)
            .ok_or_else(|| "Cannot aggregate Kujo build metrics".to_string())?,
        kujo_files_per_sec: SsgRunStatistics::from_samples(&kujo_throughput_samples)
            .ok_or_else(|| "Cannot aggregate Kujo throughput metrics".to_string())?,
        kujo_stage_profile,
        python_build_ms,
        python_files_per_sec,
        python_stage_profile,
        kujo_vs_python_speedup,
    })
}

fn collect_variability_warning(
    warnings: &mut Vec<String>,
    label: &str,
    stats: &SsgRunStatistics,
    threshold_percent: f64,
) {
    if stats.is_high_variability(threshold_percent) {
        if let Some(cv) = stats.coefficient_of_variation_percent() {
            warnings.push(format!(
                "{} variability is high (CV {:.2}% >= {:.2}%; median {:.3}, mean {:.3}, stddev {:.3}, runs {})",
                label,
                cv,
                threshold_percent,
                stats.median,
                stats.mean,
                stats.stddev,
                stats.runs
            ));
        }
    }
}

pub fn collect_ssg_variability_warnings_with_threshold(
    summary: &SsgBenchmarkAggregateResult,
    threshold_percent: f64,
) -> Vec<String> {
    let threshold = threshold_percent.max(0.0);
    let mut warnings = Vec::new();

    collect_variability_warning(
        &mut warnings,
        "Kujo build time",
        &summary.kujo_build_ms,
        threshold,
    );
    collect_variability_warning(
        &mut warnings,
        "Kujo throughput",
        &summary.kujo_files_per_sec,
        threshold,
    );

    if let Some(profile) = summary.kujo_stage_profile.as_ref() {
        collect_variability_warning(&mut warnings, "Kujo read stage", &profile.read_ms, threshold);
        collect_variability_warning(
            &mut warnings,
            "Kujo render/write stage",
            &profile.render_write_ms,
            threshold,
        );
    }

    if let Some(python_build_ms) = summary.python_build_ms.as_ref() {
        collect_variability_warning(&mut warnings, "Python build time", python_build_ms, threshold);
    }

    if let Some(python_files_per_sec) = summary.python_files_per_sec.as_ref() {
        collect_variability_warning(
            &mut warnings,
            "Python throughput",
            python_files_per_sec,
            threshold,
        );
    }

    if let Some(speedup) = summary.kujo_vs_python_speedup.as_ref() {
        collect_variability_warning(&mut warnings, "Kujo vs Python speedup", speedup, threshold);
    }

    warnings
}

fn collect_mean_median_drift_warning(
    warnings: &mut Vec<String>,
    label: &str,
    stats: &SsgRunStatistics,
    threshold_percent: f64,
) {
    if stats.is_high_mean_median_drift(threshold_percent) {
        if let Some(drift_percent) = stats.mean_median_drift_percent() {
            warnings.push(format!(
                "{} mean/median drift is high ({:.2}% >= {:.2}%; median {:.3}, mean {:.3}, runs {})",
                label,
                drift_percent,
                threshold_percent,
                stats.median,
                stats.mean,
                stats.runs
            ));
        }
    }
}

pub fn collect_ssg_mean_median_drift_warnings_with_threshold(
    summary: &SsgBenchmarkAggregateResult,
    threshold_percent: f64,
) -> Vec<String> {
    let threshold = threshold_percent.max(0.0);
    let mut warnings = Vec::new();

    collect_mean_median_drift_warning(
        &mut warnings,
        "Kujo build time",
        &summary.kujo_build_ms,
        threshold,
    );
    collect_mean_median_drift_warning(
        &mut warnings,
        "Kujo throughput",
        &summary.kujo_files_per_sec,
        threshold,
    );

    if let Some(profile) = summary.kujo_stage_profile.as_ref() {
        collect_mean_median_drift_warning(
            &mut warnings,
            "Kujo read stage",
            &profile.read_ms,
            threshold,
        );
        collect_mean_median_drift_warning(
            &mut warnings,
            "Kujo render/write stage",
            &profile.render_write_ms,
            threshold,
        );
    }

    if let Some(python_build_ms) = summary.python_build_ms.as_ref() {
        collect_mean_median_drift_warning(
            &mut warnings,
            "Python build time",
            python_build_ms,
            threshold,
        );
    }

    if let Some(python_files_per_sec) = summary.python_files_per_sec.as_ref() {
        collect_mean_median_drift_warning(
            &mut warnings,
            "Python throughput",
            python_files_per_sec,
            threshold,
        );
    }

    if let Some(speedup) = summary.kujo_vs_python_speedup.as_ref() {
        collect_mean_median_drift_warning(
            &mut warnings,
            "Kujo vs Python speedup",
            speedup,
            threshold,
        );
    }

    warnings
}

fn collect_range_spread_warning(
    warnings: &mut Vec<String>,
    label: &str,
    stats: &SsgRunStatistics,
    threshold_percent: f64,
) {
    if stats.is_high_range_spread(threshold_percent) {
        if let Some((lower_percent, upper_percent)) = stats.range_spread_percent() {
            warnings.push(format!(
                "{} range spread is high (lower {:.2}%, upper {:.2}% >= {:.2}%; min {:.3}, median {:.3}, max {:.3}, runs {})",
                label,
                lower_percent,
                upper_percent,
                threshold_percent,
                stats.min,
                stats.median,
                stats.max,
                stats.runs
            ));
        }
    }
}

pub fn collect_ssg_range_spread_warnings_with_threshold(
    summary: &SsgBenchmarkAggregateResult,
    threshold_percent: f64,
) -> Vec<String> {
    let threshold = threshold_percent.max(0.0);
    let mut warnings = Vec::new();

    collect_range_spread_warning(
        &mut warnings,
        "Kujo build time",
        &summary.kujo_build_ms,
        threshold,
    );
    collect_range_spread_warning(
        &mut warnings,
        "Kujo throughput",
        &summary.kujo_files_per_sec,
        threshold,
    );

    if let Some(profile) = summary.kujo_stage_profile.as_ref() {
        collect_range_spread_warning(&mut warnings, "Kujo read stage", &profile.read_ms, threshold);
        collect_range_spread_warning(
            &mut warnings,
            "Kujo render/write stage",
            &profile.render_write_ms,
            threshold,
        );
    }

    if let Some(python_build_ms) = summary.python_build_ms.as_ref() {
        collect_range_spread_warning(
            &mut warnings,
            "Python build time",
            python_build_ms,
            threshold,
        );
    }

    if let Some(python_files_per_sec) = summary.python_files_per_sec.as_ref() {
        collect_range_spread_warning(
            &mut warnings,
            "Python throughput",
            python_files_per_sec,
            threshold,
        );
    }

    if let Some(speedup) = summary.kujo_vs_python_speedup.as_ref() {
        collect_range_spread_warning(&mut warnings, "Kujo vs Python speedup", speedup, threshold);
    }

    warnings
}

fn collect_trend_warning(
    warnings: &mut Vec<String>,
    label: &str,
    metric: &SsgTrendMetric,
    threshold_percent: f64,
) {
    if let Some(percent_delta) = metric.percent_delta {
        let threshold = threshold_percent.max(0.0);
        if percent_delta.abs() >= threshold {
            warnings.push(format!(
                "{} trend drift is high ({:+.2}% >= {:.2}%; first {:.3}, last {:.3}, delta {:+.3})",
                label, percent_delta, threshold, metric.first, metric.last, metric.absolute_delta
            ));
        }
    }
}

pub fn collect_ssg_trend_warnings_with_threshold(
    trends: &SsgBenchmarkTrendReport,
    threshold_percent: f64,
) -> Vec<String> {
    if trends.measured_runs < 3 {
        return Vec::new();
    }

    let threshold = threshold_percent.max(0.0);
    let mut warnings = Vec::new();

    collect_trend_warning(&mut warnings, "Kujo build time", &trends.kujo_build_ms, threshold);
    collect_trend_warning(&mut warnings, "Kujo throughput", &trends.kujo_files_per_sec, threshold);

    if let Some(metric) = trends.python_build_ms.as_ref() {
        collect_trend_warning(&mut warnings, "Python build time", metric, threshold);
    }

    if let Some(metric) = trends.python_files_per_sec.as_ref() {
        collect_trend_warning(&mut warnings, "Python throughput", metric, threshold);
    }

    if let Some(metric) = trends.kujo_vs_python_speedup.as_ref() {
        collect_trend_warning(&mut warnings, "Kujo vs Python speedup", metric, threshold);
    }

    warnings
}

fn format_ssg_benchmark_stats_line(label: &str, stats: &SsgRunStatistics, unit: &str) -> String {
    format!(
        "{} (median): {:.3} {} [mean {:.3}, p90 {:.3}, p95 {:.3}, min {:.3}, max {:.3}, stddev {:.3}]",
        label, stats.median, unit, stats.mean, stats.p90, stats.p95, stats.min, stats.max, stats.stddev
    )
}

fn format_ssg_throughput_stats_line(label: &str, stats: &SsgRunStatistics) -> String {
    format!(
        "{} (median): {:.2} files/sec [mean {:.2}, p90 {:.2}, p95 {:.2}, min {:.2}, max {:.2}, stddev {:.2}]",
        label, stats.median, stats.mean, stats.p90, stats.p95, stats.min, stats.max, stats.stddev
    )
}

fn format_ssg_speedup_stats_line(label: &str, stats: &SsgRunStatistics) -> String {
    format!(
        "{} (median): {:.2}x [mean {:.2}x, p90 {:.2}x, p95 {:.2}x, min {:.2}x, max {:.2}x, stddev {:.2}]",
        label, stats.median, stats.mean, stats.p90, stats.p95, stats.min, stats.max, stats.stddev
    )
}

fn append_ssg_stage_profile_lines(
    lines: &mut Vec<String>,
    label: &str,
    profile: Option<&SsgStageProfileStatistics>,
) {
    if let Some(profile) = profile {
        lines.push(format!("{} stage breakdown (median):", label));
        lines.push(crate::cli_output::format_kv(
            "read stage",
            format!("{:.3} ms", profile.read_ms.median),
        ));
        lines.push(crate::cli_output::format_kv(
            "render/write stage",
            format!("{:.3} ms", profile.render_write_ms.median),
        ));
        let stage_profile = SsgStageProfile {
            read_ms: profile.read_ms.median,
            render_write_ms: profile.render_write_ms.median,
        };
        if let Some((stage_name, stage_ms, stage_percent)) = stage_profile.bottleneck_stage() {
            lines.push(format!(
                "  bottleneck: {} ({:.3} ms, {:.2}% of profiled median)",
                stage_name, stage_ms, stage_percent
            ));
        }
    } else {
        lines.push(format!(
            "{} stage breakdown: unavailable (metrics not emitted by script)",
            label
        ));
    }
}

fn format_ssg_trend_delta(metric: &SsgTrendMetric, unit_suffix: &str) -> String {
    let absolute = if metric.absolute_delta >= 0.0 {
        format!("+{:.3}{}", metric.absolute_delta, unit_suffix)
    } else {
        format!("{:.3}{}", metric.absolute_delta, unit_suffix)
    };

    match metric.percent_delta {
        Some(percent) if percent >= 0.0 => format!("{} (+{:.2}%)", absolute, percent),
        Some(percent) => format!("{} ({:.2}%)", absolute, percent),
        None => format!("{} (n/a %)", absolute),
    }
}

/// Render the human SSG benchmark report.
pub fn render_ssg_benchmark_report(
    summary: &SsgBenchmarkAggregateResult,
    trend_report: Option<&SsgBenchmarkTrendReport>,
    warmup_runs: usize,
    throughput_gate: Option<SsgThroughputGateStatus>,
    profile_async: bool,
    warning_thresholds: SsgWarningThresholds,
) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push("Kujo async SSG benchmark".to_string());
    lines.push("------------------------".to_string());
    lines.push(format!("Warmup runs: {}", warmup_runs));
    lines.push(format!("Runs: {}", summary.kujo_build_ms.runs));
    lines.push(format!("Files rendered: {}", summary.files));
    lines.push(format!("Kujo checksum: {}", summary.kujo_checksum));
    lines.push(format_ssg_benchmark_stats_line("Kujo build time", &summary.kujo_build_ms, "ms"));
    lines.push(format_ssg_throughput_stats_line("Kujo throughput", &summary.kujo_files_per_sec));

    if let Some(gate) = throughput_gate {
        lines.push(format_ssg_throughput_gate_summary(gate));
    }

    if profile_async {
        append_ssg_stage_profile_lines(&mut lines, "Kujo", summary.kujo_stage_profile.as_ref());
    }

    if let (Some(python_build_ms), Some(python_files_per_sec)) =
        (summary.python_build_ms.as_ref(), summary.python_files_per_sec.as_ref())
    {
        lines.push(format_ssg_benchmark_stats_line("Python build time", python_build_ms, "ms"));
        lines.push(format_ssg_throughput_stats_line("Python throughput", python_files_per_sec));

        if let Some(speedup) = summary.kujo_vs_python_speedup.as_ref() {
            lines.push(format_ssg_speedup_stats_line("Kujo speedup vs Python", speedup));
        }

        if profile_async {
            append_ssg_stage_profile_lines(
                &mut lines,
                "Python",
                summary.python_stage_profile.as_ref(),
            );
        }
    }

    if let Some(trends) = trend_report {
        lines.push(format!("Measured trend (first→last across {} runs):", trends.measured_runs));
        lines.push(format!(
            "  Kujo build time: {:.3} ms → {:.3} ms [{}]",
            trends.kujo_build_ms.first,
            trends.kujo_build_ms.last,
            format_ssg_trend_delta(&trends.kujo_build_ms, " ms")
        ));
        lines.push(format!(
            "  Kujo throughput: {:.2} files/sec → {:.2} files/sec [{}]",
            trends.kujo_files_per_sec.first,
            trends.kujo_files_per_sec.last,
            format_ssg_trend_delta(&trends.kujo_files_per_sec, " files/sec")
        ));

        if let Some(metric) = trends.python_build_ms.as_ref() {
            lines.push(format!(
                "  Python build time: {:.3} ms → {:.3} ms [{}]",
                metric.first,
                metric.last,
                format_ssg_trend_delta(metric, " ms")
            ));
        }

        if let Some(metric) = trends.python_files_per_sec.as_ref() {
            lines.push(format!(
                "  Python throughput: {:.2} files/sec → {:.2} files/sec [{}]",
                metric.first,
                metric.last,
                format_ssg_trend_delta(metric, " files/sec")
            ));
        }

        if let Some(metric) = trends.kujo_vs_python_speedup.as_ref() {
            lines.push(format!(
                "  Kujo/Python speedup: {:.2}x → {:.2}x [{}]",
                metric.first,
                metric.last,
                format_ssg_trend_delta(metric, "x")
            ));
        }

        let trend_warnings =
            collect_ssg_trend_warnings_with_threshold(trends, warning_thresholds.trend_percent);
        if !trend_warnings.is_empty() {
            lines.push(format_ssg_trend_warning_header(warning_thresholds));
            for warning in trend_warnings {
                lines.push(crate::cli_output::format_list_item(warning));
            }
            for hint in collect_ssg_warning_operator_hints(warning_thresholds) {
                lines.push(crate::cli_output::format_list_item(format!("hint: {}", hint)));
            }
        }
    }

    let variability_warnings = collect_ssg_variability_warnings_with_threshold(
        summary,
        warning_thresholds.variability_percent,
    );
    let mean_median_drift_warnings = collect_ssg_mean_median_drift_warnings_with_threshold(
        summary,
        warning_thresholds.mean_median_drift_percent,
    );
    let range_spread_warnings = collect_ssg_range_spread_warnings_with_threshold(
        summary,
        warning_thresholds.range_spread_percent,
    );
    if !variability_warnings.is_empty()
        || !mean_median_drift_warnings.is_empty()
        || !range_spread_warnings.is_empty()
    {
        lines.push(format_ssg_measurement_warning_header(warning_thresholds));
        for warning in variability_warnings {
            lines.push(crate::cli_output::format_list_item(warning));
        }
        for warning in mean_median_drift_warnings {
            lines.push(crate::cli_output::format_list_item(warning));
        }
        for warning in range_spread_warnings {
            lines.push(crate::cli_output::format_list_item(warning));
        }
        for hint in collect_ssg_warning_operator_hints(warning_thresholds) {
            lines.push(crate::cli_output::format_list_item(format!("hint: {}", hint)));
        }
    }

    lines
}

fn parse_metric_value(output: &str, metric_key: &str) -> Result<f64, String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == metric_key {
                let parsed = value.trim().parse::<f64>().map_err(|e| {
                    format!("Metric '{}' had invalid numeric value '{}': {}", metric_key, value, e)
                })?;
                return Ok(parsed);
            }
        }
    }

    Err(format!("Metric '{}' not found in output", metric_key))
}

fn parse_metric_value_optional(output: &str, metric_key: &str) -> Result<Option<f64>, String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == metric_key {
                let parsed = value.trim().parse::<f64>().map_err(|e| {
                    format!("Metric '{}' had invalid numeric value '{}': {}", metric_key, value, e)
                })?;
                return Ok(Some(parsed));
            }
        }
    }

    Ok(None)
}

fn parse_metric_usize(output: &str, metric_key: &str) -> Result<usize, String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == metric_key {
                let parsed = value.trim().parse::<usize>().map_err(|e| {
                    format!("Metric '{}' had invalid integer value '{}': {}", metric_key, value, e)
                })?;
                return Ok(parsed);
            }
        }
    }

    Err(format!("Metric '{}' not found in output", metric_key))
}

fn parse_checksum(output: &str, metric_key: &str) -> Result<i128, String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == metric_key {
                let parsed = value.trim().parse::<i128>().map_err(|e| {
                    format!(
                        "Checksum '{}' had invalid integer value '{}': {}",
                        metric_key, value, e
                    )
                })?;
                return Ok(parsed);
            }
        }
    }

    Err(format!("Checksum '{}' not found in output", metric_key))
}

fn run_and_capture_with_optional_tmp_dir(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    tmp_dir_override: Option<&str>,
    profile_async: bool,
) -> Result<String, String> {
    let mut command = Command::new(program);
    command.args(args).current_dir(working_dir);
    if let Some(tmp_dir) = tmp_dir_override {
        command.env("KUJO_BENCH_SSG_TMP_DIR", tmp_dir);
    }
    command.env(SSG_PROFILE_ASYNC_ENV, if profile_async { "1" } else { "0" });

    let output = command.output().map_err(|e| format!("Failed to run '{}': {}", program, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined =
        if stderr.trim().is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) };

    if !output.status.success() {
        return Err(format!(
            "Command '{}' failed with status {:?}:\n{}",
            program,
            output.status.code(),
            combined
        ));
    }

    Ok(combined)
}

fn resolve_tmp_dir_override(tmp_dir: Option<&Path>) -> Result<Option<String>, String> {
    match tmp_dir {
        Some(path) => {
            let path_str = path.to_str().ok_or_else(|| {
                format!("Invalid tmp dir path (must be valid UTF-8): {}", path.display())
            })?;
            Ok(Some(path_str.to_string()))
        }
        None => Ok(None),
    }
}

fn determine_workspace_root(script_path: &Path) -> PathBuf {
    let absolute_script_path = if script_path.is_absolute() {
        script_path.to_path_buf()
    } else {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        current_dir.join(script_path)
    };

    let mut current = absolute_script_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    loop {
        if current.join("Cargo.toml").exists() || current.join("tmp").exists() {
            return current;
        }

        if !current.pop() {
            break;
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn run_ssg_benchmark(
    kujo_binary: &Path,
    kujo_script: &Path,
    python_binary: Option<&str>,
    python_script: Option<&Path>,
    tmp_dir: Option<&Path>,
    profile_async: bool,
) -> Result<SsgBenchmarkResult, String> {
    if !kujo_script.exists() {
        return Err(format!("Kujo SSG benchmark script not found: {}", kujo_script.display()));
    }

    if let Some(script) = python_script {
        if !script.exists() {
            return Err(format!("Python SSG benchmark script not found: {}", script.display()));
        }
    }

    let kujo_binary_str = kujo_binary
        .to_str()
        .ok_or_else(|| format!("Invalid Kujo binary path: {}", kujo_binary.display()))?;
    let kujo_script_str = kujo_script
        .to_str()
        .ok_or_else(|| format!("Invalid Kujo script path: {}", kujo_script.display()))?;
    let working_dir = determine_workspace_root(kujo_script);
    let tmp_dir_override = resolve_tmp_dir_override(tmp_dir)?;
    let tmp_dir_override_str = tmp_dir_override.as_deref();

    let kujo_output = run_and_capture_with_optional_tmp_dir(
        kujo_binary_str,
        &["run", kujo_script_str],
        &working_dir,
        tmp_dir_override_str,
        profile_async,
    )?;

    let files = parse_metric_usize(&kujo_output, "KUJO_SSG_FILES")?;
    let kujo_build_ms = parse_metric_value(&kujo_output, "KUJO_SSG_BUILD_MS")?;
    let kujo_files_per_sec = parse_metric_value(&kujo_output, "KUJO_SSG_FILES_PER_SEC")?;
    let kujo_checksum = parse_checksum(&kujo_output, "KUJO_SSG_CHECKSUM")?;
    let kujo_read_ms = parse_metric_value_optional(&kujo_output, "KUJO_SSG_READ_MS")?;
    let kujo_render_write_ms =
        parse_metric_value_optional(&kujo_output, "KUJO_SSG_RENDER_WRITE_MS")?;

    let kujo_stage_profile = match (kujo_read_ms, kujo_render_write_ms) {
        (Some(read_ms), Some(render_write_ms)) => {
            Some(SsgStageProfile { read_ms, render_write_ms })
        }
        _ => None,
    };

    let mut result = SsgBenchmarkResult {
        files,
        kujo_build_ms,
        kujo_files_per_sec,
        kujo_checksum,
        kujo_stage_profile,
        python_build_ms: None,
        python_files_per_sec: None,
        python_stage_profile: None,
    };

    if let (Some(python_binary), Some(python_script)) = (python_binary, python_script) {
        let python_script_str = python_script
            .to_str()
            .ok_or_else(|| format!("Invalid Python script path: {}", python_script.display()))?;

        let python_output = run_and_capture_with_optional_tmp_dir(
            python_binary,
            &[python_script_str],
            &working_dir,
            tmp_dir_override_str,
            profile_async,
        )?;
        let python_files = parse_metric_usize(&python_output, "PYTHON_SSG_FILES")?;
        let python_build_ms = parse_metric_value(&python_output, "PYTHON_SSG_BUILD_MS")?;
        let python_files_per_sec = parse_metric_value(&python_output, "PYTHON_SSG_FILES_PER_SEC")?;
        let python_checksum = parse_checksum(&python_output, "PYTHON_SSG_CHECKSUM")?;
        let python_read_ms = parse_metric_value_optional(&python_output, "PYTHON_SSG_READ_MS")?;
        let python_render_write_ms =
            parse_metric_value_optional(&python_output, "PYTHON_SSG_RENDER_WRITE_MS")?;

        let python_stage_profile = match (python_read_ms, python_render_write_ms) {
            (Some(read_ms), Some(render_write_ms)) => {
                Some(SsgStageProfile { read_ms, render_write_ms })
            }
            _ => None,
        };

        if python_files != result.files {
            return Err(format!(
                "File count mismatch: Kujo={} Python={} (benchmarks must use identical workload)",
                result.files, python_files
            ));
        }

        if python_checksum != result.kujo_checksum {
            return Err(format!(
                "Checksum mismatch: Kujo={} Python={} (outputs are not equivalent)",
                result.kujo_checksum, python_checksum
            ));
        }

        result.python_build_ms = Some(python_build_ms);
        result.python_files_per_sec = Some(python_files_per_sec);
        result.python_stage_profile = python_stage_profile;
    }

    Ok(result)
}

pub fn run_ssg_benchmark_series(
    kujo_binary: &Path,
    kujo_script: &Path,
    python_binary: Option<&str>,
    python_script: Option<&Path>,
    tmp_dir: Option<&Path>,
    warmup_runs: usize,
    measured_runs: usize,
    profile_async: bool,
) -> Result<Vec<SsgBenchmarkResult>, String> {
    if measured_runs == 0 {
        return Err("SSG benchmark runs must be >= 1".to_string());
    }

    for warmup_index in 0..warmup_runs {
        if let Err(err) = run_ssg_benchmark(
            kujo_binary,
            kujo_script,
            python_binary,
            python_script,
            tmp_dir,
            profile_async,
        ) {
            return Err(format!("Warmup run {}/{} failed: {}", warmup_index + 1, warmup_runs, err));
        }
    }

    let mut measured_results = Vec::with_capacity(measured_runs);
    for run_index in 0..measured_runs {
        let result = run_ssg_benchmark(
            kujo_binary,
            kujo_script,
            python_binary,
            python_script,
            tmp_dir,
            profile_async,
        )
        .map_err(|err| {
            format!("Measured run {}/{} failed: {}", run_index + 1, measured_runs, err)
        })?;
        measured_results.push(result);
    }

    Ok(measured_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::sync::atomic::{AtomicU64, Ordering};
    #[cfg(unix)]
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[cfg(unix)]
    fn unique_test_dir(prefix: &str) -> PathBuf {
        // The test suite exercises os_chdir in parallel. Anchor fixtures to the
        // manifest directory so another test cannot place and remove them under
        // its temporary working directory.
        let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.push("tmp");
        base.push("bench_ssg_harness_tests");
        fs::create_dir_all(&base).expect("bench-ssg test root should be created");

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        base.push(format!("{}_{}_{}", prefix, timestamp, counter));
        fs::create_dir_all(&base).expect("bench-ssg test dir should be created");
        base
    }

    #[cfg(unix)]
    fn write_stub_executable(path: &Path, script_body: &str) {
        fs::write(path, script_body).expect("stub executable should be written");
        let mut perms = fs::metadata(path).expect("stub metadata should resolve").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("stub executable should be chmod +x");
    }

    fn stats(
        runs: usize,
        mean: f64,
        median: f64,
        p90: f64,
        p95: f64,
        min: f64,
        max: f64,
        stddev: f64,
    ) -> SsgRunStatistics {
        SsgRunStatistics { runs, mean, median, p90, p95, min, max, stddev }
    }

    #[test]
    fn test_render_ssg_benchmark_report_snapshot() {
        let summary = SsgBenchmarkAggregateResult {
            files: 12,
            kujo_checksum: 99,
            kujo_build_ms: stats(3, 110.0, 100.0, 130.0, 140.0, 90.0, 150.0, 20.0),
            kujo_files_per_sec: stats(3, 22.0, 20.0, 26.0, 28.0, 18.0, 30.0, 4.0),
            kujo_stage_profile: Some(SsgStageProfileStatistics {
                read_ms: stats(3, 32.0, 30.0, 35.0, 36.0, 28.0, 40.0, 3.0),
                render_write_ms: stats(3, 75.0, 70.0, 85.0, 90.0, 65.0, 95.0, 8.0),
            }),
            python_build_ms: Some(stats(3, 210.0, 200.0, 230.0, 240.0, 190.0, 250.0, 30.0)),
            python_files_per_sec: Some(stats(3, 11.0, 10.0, 13.0, 14.0, 9.0, 15.0, 2.0)),
            python_stage_profile: Some(SsgStageProfileStatistics {
                read_ms: stats(3, 125.0, 120.0, 140.0, 145.0, 115.0, 150.0, 10.0),
                render_write_ms: stats(3, 85.0, 80.0, 95.0, 100.0, 75.0, 105.0, 8.0),
            }),
            kujo_vs_python_speedup: Some(stats(3, 2.1, 2.0, 2.3, 2.4, 1.9, 2.5, 0.3)),
        };
        let trends = SsgBenchmarkTrendReport {
            measured_runs: 3,
            kujo_build_ms: SsgTrendMetric {
                first: 100.0,
                last: 112.0,
                absolute_delta: 12.0,
                percent_delta: Some(12.0),
            },
            kujo_files_per_sec: SsgTrendMetric {
                first: 20.0,
                last: 18.0,
                absolute_delta: -2.0,
                percent_delta: Some(-10.0),
            },
            python_build_ms: Some(SsgTrendMetric {
                first: 200.0,
                last: 190.0,
                absolute_delta: -10.0,
                percent_delta: Some(-5.0),
            }),
            python_files_per_sec: Some(SsgTrendMetric {
                first: 10.0,
                last: 11.0,
                absolute_delta: 1.0,
                percent_delta: Some(10.0),
            }),
            kujo_vs_python_speedup: Some(SsgTrendMetric {
                first: 2.0,
                last: 1.8,
                absolute_delta: -0.2,
                percent_delta: Some(-10.0),
            }),
        };
        let gate = SsgThroughputGateStatus {
            threshold_ms: 150.0,
            observed_median_ms: 100.0,
            margin_ms: 50.0,
            margin_percent: 33.333333,
            passed: true,
        };
        let thresholds = SsgWarningThresholds {
            variability_percent: 1000.0,
            trend_percent: 10.0,
            mean_median_drift_percent: 1000.0,
            range_spread_percent: 1000.0,
        };

        let rendered =
            render_ssg_benchmark_report(&summary, Some(&trends), 1, Some(gate), true, thresholds)
                .join("\n");

        let expected = [
            "Kujo async SSG benchmark",
            "------------------------",
            "Warmup runs: 1",
            "Runs: 3",
            "Files rendered: 12",
            "Kujo checksum: 99",
            "Kujo build time (median): 100.000 ms [mean 110.000, p90 130.000, p95 140.000, min 90.000, max 150.000, stddev 20.000]",
            "Kujo throughput (median): 20.00 files/sec [mean 22.00, p90 26.00, p95 28.00, min 18.00, max 30.00, stddev 4.00]",
            "Throughput gate [PASS]: Kujo median build 100.000 ms <= target 150.000 ms (margin +50.000 ms, +33.33%)",
            "Kujo stage breakdown (median):",
            "  read stage: 30.000 ms",
            "  render/write stage: 70.000 ms",
            "  bottleneck: render/write (70.000 ms, 70.00% of profiled median)",
            "Python build time (median): 200.000 ms [mean 210.000, p90 230.000, p95 240.000, min 190.000, max 250.000, stddev 30.000]",
            "Python throughput (median): 10.00 files/sec [mean 11.00, p90 13.00, p95 14.00, min 9.00, max 15.00, stddev 2.00]",
            "Kujo speedup vs Python (median): 2.00x [mean 2.10x, p90 2.30x, p95 2.40x, min 1.90x, max 2.50x, stddev 0.30]",
            "Python stage breakdown (median):",
            "  read stage: 120.000 ms",
            "  render/write stage: 80.000 ms",
            "  bottleneck: read (120.000 ms, 60.00% of profiled median)",
            "Measured trend (first→last across 3 runs):",
            "  Kujo build time: 100.000 ms → 112.000 ms [+12.000 ms (+12.00%)]",
            "  Kujo throughput: 20.00 files/sec → 18.00 files/sec [-2.000 files/sec (-10.00%)]",
            "  Python build time: 200.000 ms → 190.000 ms [-10.000 ms (-5.00%)]",
            "  Python throughput: 10.00 files/sec → 11.00 files/sec [+1.000 files/sec (+10.00%)]",
            "  Kujo/Python speedup: 2.00x → 1.80x [-0.200x (-10.00%)]",
            "Trend stability warnings (drift >= 10.00%):",
            "  - Kujo build time trend drift is high (+12.00% >= 10.00%; first 100.000, last 112.000, delta +12.000)",
            "  - Kujo throughput trend drift is high (-10.00% >= 10.00%; first 20.000, last 18.000, delta -2.000)",
            "  - Python throughput trend drift is high (+10.00% >= 10.00%; first 10.000, last 11.000, delta +1.000)",
            "  - Kujo vs Python speedup trend drift is high (-10.00% >= 10.00%; first 2.000, last 1.800, delta -0.200)",
            "  - hint: Tune CV sensitivity with --variability-warning-threshold <PERCENT>",
            "  - hint: Tune trend sensitivity with --trend-warning-threshold <PERCENT>",
            "  - hint: Tune skew sensitivity with --mean-median-drift-warning-threshold <PERCENT>",
            "  - hint: Tune range-spread sensitivity with --range-spread-warning-threshold <PERCENT>",
            "  - hint: Current thresholds: CV 1000.00%, trend 10.00%, mean/median 1000.00%, range spread 1000.00%",
        ]
        .join("\n");
        assert_eq!(rendered, expected);
    }

    #[cfg(unix)]
    fn create_basic_harness_fixture(
        prefix: &str,
        kujo_output_lines: &[&str],
        python_output_lines: Option<&[&str]>,
    ) -> (PathBuf, PathBuf, PathBuf, Option<PathBuf>) {
        let test_dir = unique_test_dir(prefix);

        let kujo_script_path = test_dir.join("stub_bench_ssg.kujo");
        fs::write(&kujo_script_path, "# stub kujo benchmark script\n")
            .expect("kujo script fixture should be written");

        let kujo_binary_path = test_dir.join("kujo_stub.sh");
        let kujo_stdout = kujo_output_lines.join("\n");
        let kujo_stub_body = format!(
            "#!/bin/sh\nif [ \"$1\" != \"run\" ]; then\n  echo \"unexpected args: $@\" >&2\n  exit 9\nfi\ncat <<'EOF'\n{}\nEOF\n",
            kujo_stdout
        );
        write_stub_executable(&kujo_binary_path, &kujo_stub_body);

        if let Some(python_lines) = python_output_lines {
            let python_script_path = test_dir.join("stub_bench_ssg.py");
            fs::write(&python_script_path, "# stub python benchmark script\n")
                .expect("python script fixture should be written");

            let python_binary_path = test_dir.join("python_stub.sh");
            let python_stdout = python_lines.join("\n");
            let python_stub_body = format!("#!/bin/sh\ncat <<'EOF'\n{}\nEOF\n", python_stdout);
            write_stub_executable(&python_binary_path, &python_stub_body);

            (kujo_binary_path, kujo_script_path, python_binary_path, Some(python_script_path))
        } else {
            (kujo_binary_path, kujo_script_path, PathBuf::new(), None)
        }
    }

    #[cfg(unix)]
    fn create_profile_toggle_harness_fixture(prefix: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let test_dir = unique_test_dir(prefix);

        let kujo_script_path = test_dir.join("stub_bench_ssg.kujo");
        fs::write(&kujo_script_path, "# stub kujo benchmark script\n")
            .expect("kujo script fixture should be written");

        let python_script_path = test_dir.join("stub_bench_ssg.py");
        fs::write(&python_script_path, "# stub python benchmark script\n")
            .expect("python script fixture should be written");

        let kujo_binary_path = test_dir.join("kujo_profile_stub.sh");
        let kujo_stub_body = "#!/bin/sh\nif [ \"$1\" != \"run\" ]; then\n  echo \"unexpected args: $@\" >&2\n  exit 9\nfi\necho \"KUJO_SSG_FILES=10000\"\necho \"KUJO_SSG_BUILD_MS=100.0\"\necho \"KUJO_SSG_FILES_PER_SEC=1000.0\"\necho \"KUJO_SSG_CHECKSUM=777\"\nif [ \"${KUJO_BENCH_SSG_PROFILE_ASYNC}\" = \"1\" ]; then\n  echo \"KUJO_SSG_READ_MS=10.5\"\n  echo \"KUJO_SSG_RENDER_WRITE_MS=40.5\"\nfi\n";
        write_stub_executable(&kujo_binary_path, kujo_stub_body);

        let python_binary_path = test_dir.join("python_profile_stub.sh");
        let python_stub_body = "#!/bin/sh\necho \"PYTHON_SSG_FILES=10000\"\necho \"PYTHON_SSG_BUILD_MS=120.0\"\necho \"PYTHON_SSG_FILES_PER_SEC=900.0\"\necho \"PYTHON_SSG_CHECKSUM=777\"\nif [ \"${KUJO_BENCH_SSG_PROFILE_ASYNC}\" = \"1\" ]; then\n  echo \"PYTHON_SSG_READ_MS=12.0\"\n  echo \"PYTHON_SSG_RENDER_WRITE_MS=52.0\"\nfi\n";
        write_stub_executable(&python_binary_path, python_stub_body);

        (kujo_binary_path, kujo_script_path, python_binary_path, python_script_path)
    }

    #[test]
    fn test_parse_metric_value_extracts_float() {
        let output = "noise\nKUJO_SSG_BUILD_MS=12.500\nother";
        let value = parse_metric_value(output, "KUJO_SSG_BUILD_MS").unwrap();
        assert!((value - 12.5).abs() < 0.0001);
    }

    #[test]
    fn test_parse_metric_usize_extracts_integer() {
        let output = "KUJO_SSG_FILES=10000";
        let value = parse_metric_usize(output, "KUJO_SSG_FILES").unwrap();
        assert_eq!(value, 10000);
    }

    #[test]
    fn test_parse_checksum_extracts_integer() {
        let output = "PYTHON_SSG_CHECKSUM=424242";
        let value = parse_checksum(output, "PYTHON_SSG_CHECKSUM").unwrap();
        assert_eq!(value, 424242);
    }

    #[test]
    fn test_parse_checksum_invalid_value_returns_error() {
        let output = "KUJO_SSG_CHECKSUM=not_a_number";
        let err = parse_checksum(output, "KUJO_SSG_CHECKSUM").unwrap_err();
        assert!(err.contains("invalid integer value"));
    }

    #[test]
    fn test_parse_metric_value_missing_key_returns_error() {
        let output = "KUJO_SSG_BUILD_MS=1.0";
        let err = parse_metric_value(output, "PYTHON_SSG_BUILD_MS").unwrap_err();
        assert!(err.contains("Metric 'PYTHON_SSG_BUILD_MS' not found"));
    }

    #[test]
    fn test_parse_metric_usize_invalid_value_returns_error() {
        let output = "KUJO_SSG_FILES=notanumber";
        let err = parse_metric_usize(output, "KUJO_SSG_FILES").unwrap_err();
        assert!(err.contains("invalid integer value"));
    }

    #[test]
    fn test_parse_metric_value_optional_present() {
        let output = "KUJO_SSG_READ_MS=14.25";
        let value = parse_metric_value_optional(output, "KUJO_SSG_READ_MS").unwrap();
        assert_eq!(value, Some(14.25));
    }

    #[test]
    fn test_parse_metric_value_optional_absent_returns_none() {
        let output = "KUJO_SSG_BUILD_MS=120.0";
        let value = parse_metric_value_optional(output, "KUJO_SSG_RENDER_WRITE_MS").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_parse_metric_value_optional_invalid_returns_error() {
        let output = "KUJO_SSG_RENDER_WRITE_MS=fast";
        let err = parse_metric_value_optional(output, "KUJO_SSG_RENDER_WRITE_MS").unwrap_err();
        assert!(err.contains("invalid numeric value"));
    }

    #[test]
    fn test_resolve_tmp_dir_override_none_returns_none() {
        let resolved = resolve_tmp_dir_override(None).unwrap();
        assert!(resolved.is_none());
    }

    #[test]
    fn test_resolve_tmp_dir_override_some_returns_string_path() {
        let resolved = resolve_tmp_dir_override(Some(Path::new("tmp/custom_root"))).unwrap();
        assert_eq!(resolved, Some("tmp/custom_root".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_tmp_dir_override_rejects_non_utf8_path() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let invalid_path = PathBuf::from(OsString::from_vec(vec![0x66, 0x6f, 0x80, 0x6f]));
        let err = resolve_tmp_dir_override(Some(invalid_path.as_path())).unwrap_err();
        assert!(err.contains("must be valid UTF-8"));
    }

    #[test]
    fn test_stage_profile_total_profiled_ms() {
        let profile = SsgStageProfile { read_ms: 25.0, render_write_ms: 75.0 };
        assert_eq!(profile.total_profiled_ms(), 100.0);
    }

    #[test]
    fn test_stage_profile_bottleneck_read() {
        let profile = SsgStageProfile { read_ms: 80.0, render_write_ms: 20.0 };
        let bottleneck = profile.bottleneck_stage().unwrap();
        assert_eq!(bottleneck.0, "read");
        assert!((bottleneck.1 - 80.0).abs() < 0.0001);
        assert!((bottleneck.2 - 80.0).abs() < 0.0001);
    }

    #[test]
    fn test_stage_profile_bottleneck_render_write() {
        let profile = SsgStageProfile { read_ms: 30.0, render_write_ms: 70.0 };
        let bottleneck = profile.bottleneck_stage().unwrap();
        assert_eq!(bottleneck.0, "render/write");
        assert!((bottleneck.1 - 70.0).abs() < 0.0001);
        assert!((bottleneck.2 - 70.0).abs() < 0.0001);
    }

    #[test]
    fn test_stage_profile_bottleneck_zero_total_returns_none() {
        let profile = SsgStageProfile { read_ms: 0.0, render_write_ms: 0.0 };
        assert!(profile.bottleneck_stage().is_none());
    }

    #[test]
    fn test_speedup_calculation() {
        let result = SsgBenchmarkResult {
            files: 10000,
            kujo_build_ms: 1000.0,
            kujo_files_per_sec: 10000.0,
            kujo_checksum: 100,
            kujo_stage_profile: None,
            python_build_ms: Some(2500.0),
            python_files_per_sec: Some(4000.0),
            python_stage_profile: None,
        };

        assert!((result.kujo_vs_python_speedup().unwrap() - 2.5).abs() < 0.0001);
    }

    #[test]
    fn test_speedup_calculation_without_python_result() {
        let result = SsgBenchmarkResult {
            files: 10000,
            kujo_build_ms: 1000.0,
            kujo_files_per_sec: 10000.0,
            kujo_checksum: 100,
            kujo_stage_profile: None,
            python_build_ms: None,
            python_files_per_sec: None,
            python_stage_profile: None,
        };

        assert!(result.kujo_vs_python_speedup().is_none());
    }

    #[test]
    fn test_speedup_calculation_handles_zero_kujo_duration() {
        let result = SsgBenchmarkResult {
            files: 10000,
            kujo_build_ms: 0.0,
            kujo_files_per_sec: 0.0,
            kujo_checksum: 100,
            kujo_stage_profile: None,
            python_build_ms: Some(1000.0),
            python_files_per_sec: Some(10000.0),
            python_stage_profile: None,
        };

        assert_eq!(result.kujo_vs_python_speedup().unwrap(), 0.0);
    }

    #[test]
    fn test_run_statistics_from_samples_even_count() {
        let stats = SsgRunStatistics::from_samples(&[10.0, 20.0, 30.0, 40.0]).unwrap();
        assert_eq!(stats.runs, 4);
        assert!((stats.mean - 25.0).abs() < 0.0001);
        assert!((stats.median - 25.0).abs() < 0.0001);
        assert!((stats.p90 - 37.0).abs() < 0.0001);
        assert!((stats.p95 - 38.5).abs() < 0.0001);
        assert!((stats.min - 10.0).abs() < 0.0001);
        assert!((stats.max - 40.0).abs() < 0.0001);
    }

    #[test]
    fn test_run_statistics_from_samples_odd_count() {
        let stats = SsgRunStatistics::from_samples(&[8.0, 2.0, 5.0]).unwrap();
        assert_eq!(stats.runs, 3);
        assert!((stats.median - 5.0).abs() < 0.0001);
        assert!((stats.p90 - 7.4).abs() < 0.0001);
        assert!((stats.p95 - 7.7).abs() < 0.0001);
    }

    #[test]
    fn test_run_statistics_percentiles_single_sample_equal_sample() {
        let stats = SsgRunStatistics::from_samples(&[123.0]).unwrap();
        assert_eq!(stats.runs, 1);
        assert!((stats.p90 - 123.0).abs() < 0.0001);
        assert!((stats.p95 - 123.0).abs() < 0.0001);
    }

    #[test]
    fn test_run_statistics_percentiles_are_monotonic_with_distribution_bounds() {
        let stats = SsgRunStatistics::from_samples(&[5.0, 5.0, 5.0, 10.0, 30.0, 100.0]).unwrap();

        assert!(stats.min <= stats.median);
        assert!(stats.median <= stats.p90);
        assert!(stats.p90 <= stats.p95);
        assert!(stats.p95 <= stats.max);
    }

    #[test]
    fn test_run_statistics_empty_samples_returns_none() {
        assert!(SsgRunStatistics::from_samples(&[]).is_none());
    }

    #[test]
    fn test_run_statistics_coefficient_of_variation_percent() {
        let stats = SsgRunStatistics::from_samples(&[100.0, 110.0, 90.0]).unwrap();
        let cv = stats.coefficient_of_variation_percent().unwrap();
        assert!((cv - 8.1649).abs() < 0.001);
    }

    #[test]
    fn test_run_statistics_coefficient_of_variation_zero_mean_returns_none() {
        let stats = SsgRunStatistics::from_samples(&[0.0, 0.0, 0.0]).unwrap();
        assert!(stats.coefficient_of_variation_percent().is_none());
    }

    #[test]
    fn test_run_statistics_mean_median_drift_percent() {
        let stats = SsgRunStatistics::from_samples(&[100.0, 100.0, 200.0]).unwrap();
        let drift = stats.mean_median_drift_percent().unwrap();
        assert!((drift - 33.3333).abs() < 0.01);
    }

    #[test]
    fn test_run_statistics_mean_median_drift_percent_zero_median_returns_none() {
        let stats = SsgRunStatistics::from_samples(&[0.0, 0.0, 0.0]).unwrap();
        assert!(stats.mean_median_drift_percent().is_none());
    }

    #[test]
    fn test_run_statistics_range_spread_percent() {
        let stats = SsgRunStatistics::from_samples(&[80.0, 100.0, 140.0]).unwrap();
        let (lower, upper) = stats.range_spread_percent().unwrap();
        assert!((lower - 20.0).abs() < 0.0001);
        assert!((upper - 40.0).abs() < 0.0001);
    }

    #[test]
    fn test_run_statistics_range_spread_percent_zero_median_returns_none() {
        let stats = SsgRunStatistics::from_samples(&[0.0, 0.0, 0.0]).unwrap();
        assert!(stats.range_spread_percent().is_none());
    }

    #[test]
    fn test_run_statistics_high_variability_requires_three_runs() {
        let stats = SsgRunStatistics {
            runs: 2,
            mean: 10.0,
            median: 10.0,
            p90: 10.0,
            p95: 10.0,
            min: 5.0,
            max: 15.0,
            stddev: 5.0,
        };

        assert!(!stats.is_high_variability(SSG_VARIABILITY_WARNING_THRESHOLD_PERCENT));
    }

    #[test]
    fn test_run_statistics_high_mean_median_drift_requires_three_runs() {
        let stats = SsgRunStatistics {
            runs: 2,
            mean: 120.0,
            median: 100.0,
            p90: 130.0,
            p95: 135.0,
            min: 100.0,
            max: 140.0,
            stddev: 20.0,
        };

        assert!(!stats.is_high_mean_median_drift(SSG_MEAN_MEDIAN_DRIFT_WARNING_THRESHOLD_PERCENT));
    }

    #[test]
    fn test_collect_ssg_variability_warnings_flags_high_variability_metrics() {
        let summary = SsgBenchmarkAggregateResult {
            files: 100,
            kujo_checksum: 42,
            kujo_build_ms: SsgRunStatistics {
                runs: 3,
                mean: 100.0,
                median: 100.0,
                p90: 140.0,
                p95: 145.0,
                min: 50.0,
                max: 150.0,
                stddev: 30.0,
            },
            kujo_files_per_sec: SsgRunStatistics {
                runs: 3,
                mean: 1000.0,
                median: 1000.0,
                p90: 1001.0,
                p95: 1001.5,
                min: 998.0,
                max: 1002.0,
                stddev: 1.0,
            },
            kujo_stage_profile: Some(SsgStageProfileStatistics {
                read_ms: SsgRunStatistics {
                    runs: 3,
                    mean: 25.0,
                    median: 25.0,
                    p90: 37.0,
                    p95: 38.5,
                    min: 10.0,
                    max: 40.0,
                    stddev: 9.0,
                },
                render_write_ms: SsgRunStatistics {
                    runs: 3,
                    mean: 75.0,
                    median: 75.0,
                    p90: 75.8,
                    p95: 75.9,
                    min: 74.0,
                    max: 76.0,
                    stddev: 0.5,
                },
            }),
            python_build_ms: None,
            python_files_per_sec: None,
            python_stage_profile: None,
            kujo_vs_python_speedup: None,
        };

        let warnings = collect_ssg_variability_warnings_with_threshold(
            &summary,
            SSG_VARIABILITY_WARNING_THRESHOLD_PERCENT,
        );
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|warning| warning.contains("Kujo build time")));
        assert!(warnings.iter().any(|warning| warning.contains("Kujo read stage")));
    }

    #[test]
    fn test_collect_ssg_variability_warnings_skips_low_variability_metrics() {
        let stable_stats = SsgRunStatistics {
            runs: 4,
            mean: 120.0,
            median: 120.0,
            p90: 121.5,
            p95: 121.75,
            min: 118.0,
            max: 122.0,
            stddev: 1.0,
        };

        let summary = SsgBenchmarkAggregateResult {
            files: 100,
            kujo_checksum: 42,
            kujo_build_ms: stable_stats.clone(),
            kujo_files_per_sec: stable_stats.clone(),
            kujo_stage_profile: None,
            python_build_ms: Some(stable_stats.clone()),
            python_files_per_sec: Some(stable_stats.clone()),
            python_stage_profile: None,
            kujo_vs_python_speedup: Some(stable_stats),
        };

        let warnings = collect_ssg_variability_warnings_with_threshold(
            &summary,
            SSG_VARIABILITY_WARNING_THRESHOLD_PERCENT,
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_collect_ssg_variability_warnings_custom_threshold_can_suppress_warning() {
        let summary = SsgBenchmarkAggregateResult {
            files: 100,
            kujo_checksum: 42,
            kujo_build_ms: SsgRunStatistics {
                runs: 4,
                mean: 100.0,
                median: 100.0,
                p90: 140.0,
                p95: 145.0,
                min: 50.0,
                max: 150.0,
                stddev: 30.0,
            },
            kujo_files_per_sec: SsgRunStatistics {
                runs: 4,
                mean: 1000.0,
                median: 1000.0,
                p90: 1000.7,
                p95: 1000.85,
                min: 999.0,
                max: 1001.0,
                stddev: 0.6,
            },
            kujo_stage_profile: None,
            python_build_ms: None,
            python_files_per_sec: None,
            python_stage_profile: None,
            kujo_vs_python_speedup: None,
        };

        let default_warnings = collect_ssg_variability_warnings_with_threshold(
            &summary,
            SSG_VARIABILITY_WARNING_THRESHOLD_PERCENT,
        );
        assert!(!default_warnings.is_empty());

        let suppressed_warnings = collect_ssg_variability_warnings_with_threshold(&summary, 35.0);
        assert!(suppressed_warnings.is_empty());
    }

    #[test]
    fn test_format_ssg_measurement_warning_header_includes_threshold_values() {
        let header = format_ssg_measurement_warning_header(SsgWarningThresholds {
            variability_percent: 6.5,
            trend_percent: 12.0,
            mean_median_drift_percent: 9.25,
            range_spread_percent: 42.0,
        });

        assert!(header.contains("CV >= 6.50%"));
        assert!(header.contains("mean/median drift >= 9.25%"));
        assert!(header.contains("range spread >= 42.00%"));
    }

    #[test]
    fn test_format_ssg_trend_warning_header_includes_threshold_value() {
        let header = format_ssg_trend_warning_header(SsgWarningThresholds {
            variability_percent: 6.5,
            trend_percent: 12.0,
            mean_median_drift_percent: 9.25,
            range_spread_percent: 42.0,
        });

        assert!(header.contains("drift >= 12.00%"));
    }

    #[test]
    fn test_collect_ssg_warning_operator_hints_contains_all_override_flags() {
        let hints = collect_ssg_warning_operator_hints(SsgWarningThresholds {
            variability_percent: 6.5,
            trend_percent: 12.0,
            mean_median_drift_percent: 9.25,
            range_spread_percent: 42.0,
        });

        assert_eq!(hints.len(), 5);
        assert!(hints.iter().any(|hint| hint.contains("--variability-warning-threshold")));
        assert!(hints.iter().any(|hint| hint.contains("--trend-warning-threshold")));
        assert!(hints.iter().any(|hint| hint.contains("--mean-median-drift-warning-threshold")));
        assert!(hints.iter().any(|hint| hint.contains("--range-spread-warning-threshold")));
        assert!(hints.iter().any(|hint| hint.contains(
            "Current thresholds: CV 6.50%, trend 12.00%, mean/median 9.25%, range spread 42.00%"
        )));
    }

    #[test]
    fn test_evaluate_ssg_throughput_gate_passes_when_median_within_threshold() {
        let gate = evaluate_ssg_throughput_gate(9_500.0, 10_000.0).unwrap();

        assert!(gate.passed);
        assert_eq!(gate.threshold_ms, 10_000.0);
        assert_eq!(gate.observed_median_ms, 9_500.0);
        assert!((gate.margin_ms - 500.0).abs() < 0.0001);
        assert!((gate.margin_percent - 5.0).abs() < 0.0001);
    }

    #[test]
    fn test_evaluate_ssg_throughput_gate_fails_when_median_exceeds_threshold() {
        let gate = evaluate_ssg_throughput_gate(10_250.0, 10_000.0).unwrap();

        assert!(!gate.passed);
        assert!((gate.margin_ms + 250.0).abs() < 0.0001);
        assert!((gate.margin_percent + 2.5).abs() < 0.0001);
    }

    #[test]
    fn test_evaluate_ssg_throughput_gate_rejects_invalid_threshold() {
        let err = evaluate_ssg_throughput_gate(8_000.0, 0.0).unwrap_err();
        assert!(err.contains("threshold must be finite and > 0.0 ms"));
    }

    #[test]
    fn test_evaluate_ssg_throughput_gate_rejects_invalid_observed_median() {
        let err = evaluate_ssg_throughput_gate(-1.0, 10_000.0).unwrap_err();
        assert!(err.contains("observed median must be finite and >= 0.0 ms"));
    }

    #[test]
    fn test_format_ssg_throughput_gate_summary_reports_status_and_margin() {
        let passing = SsgThroughputGateStatus {
            threshold_ms: 10_000.0,
            observed_median_ms: 9_250.0,
            margin_ms: 750.0,
            margin_percent: 7.5,
            passed: true,
        };
        let failing = SsgThroughputGateStatus {
            threshold_ms: 10_000.0,
            observed_median_ms: 10_250.0,
            margin_ms: -250.0,
            margin_percent: -2.5,
            passed: false,
        };

        let pass_summary = format_ssg_throughput_gate_summary(passing);
        let fail_summary = format_ssg_throughput_gate_summary(failing);

        assert!(pass_summary.contains("Throughput gate [PASS]"));
        assert!(pass_summary.contains("<= target 10000.000 ms"));
        assert!(pass_summary.contains("margin +750.000 ms, +7.50%"));

        assert!(fail_summary.contains("Throughput gate [FAIL]"));
        assert!(fail_summary.contains("> target 10000.000 ms"));
        assert!(fail_summary.contains("margin -250.000 ms, -2.50%"));
    }

    #[test]
    fn test_collect_ssg_mean_median_drift_warnings_flags_high_drift_metrics() {
        let summary = SsgBenchmarkAggregateResult {
            files: 100,
            kujo_checksum: 42,
            kujo_build_ms: SsgRunStatistics {
                runs: 4,
                mean: 130.0,
                median: 100.0,
                p90: 200.0,
                p95: 215.0,
                min: 90.0,
                max: 230.0,
                stddev: 55.0,
            },
            kujo_files_per_sec: SsgRunStatistics {
                runs: 4,
                mean: 1000.0,
                median: 998.0,
                p90: 1002.5,
                p95: 1003.25,
                min: 996.0,
                max: 1004.0,
                stddev: 3.0,
            },
            kujo_stage_profile: None,
            python_build_ms: None,
            python_files_per_sec: None,
            python_stage_profile: None,
            kujo_vs_python_speedup: None,
        };

        let warnings = collect_ssg_mean_median_drift_warnings_with_threshold(
            &summary,
            SSG_MEAN_MEDIAN_DRIFT_WARNING_THRESHOLD_PERCENT,
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings.iter().any(|warning| warning.contains("Kujo build time")));
        assert!(warnings.iter().all(|warning| warning.contains("mean/median drift is high")));
    }

    #[test]
    fn test_collect_ssg_mean_median_drift_warnings_skips_stable_metrics() {
        let stable_stats = SsgRunStatistics {
            runs: 5,
            mean: 120.5,
            median: 120.0,
            p90: 122.0,
            p95: 122.5,
            min: 118.0,
            max: 123.0,
            stddev: 1.5,
        };

        let summary = SsgBenchmarkAggregateResult {
            files: 100,
            kujo_checksum: 42,
            kujo_build_ms: stable_stats.clone(),
            kujo_files_per_sec: stable_stats.clone(),
            kujo_stage_profile: Some(SsgStageProfileStatistics {
                read_ms: stable_stats.clone(),
                render_write_ms: stable_stats.clone(),
            }),
            python_build_ms: Some(stable_stats.clone()),
            python_files_per_sec: Some(stable_stats.clone()),
            python_stage_profile: None,
            kujo_vs_python_speedup: Some(stable_stats),
        };

        let warnings = collect_ssg_mean_median_drift_warnings_with_threshold(
            &summary,
            SSG_MEAN_MEDIAN_DRIFT_WARNING_THRESHOLD_PERCENT,
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_collect_ssg_mean_median_drift_warnings_custom_threshold_can_trigger_warning() {
        let summary = SsgBenchmarkAggregateResult {
            files: 100,
            kujo_checksum: 42,
            kujo_build_ms: SsgRunStatistics {
                runs: 4,
                mean: 106.0,
                median: 100.0,
                p90: 114.0,
                p95: 117.0,
                min: 95.0,
                max: 120.0,
                stddev: 8.0,
            },
            kujo_files_per_sec: SsgRunStatistics {
                runs: 4,
                mean: 1000.0,
                median: 1000.0,
                p90: 1008.0,
                p95: 1009.0,
                min: 990.0,
                max: 1010.0,
                stddev: 5.0,
            },
            kujo_stage_profile: None,
            python_build_ms: None,
            python_files_per_sec: None,
            python_stage_profile: None,
            kujo_vs_python_speedup: None,
        };

        let default_warnings = collect_ssg_mean_median_drift_warnings_with_threshold(
            &summary,
            SSG_MEAN_MEDIAN_DRIFT_WARNING_THRESHOLD_PERCENT,
        );
        assert!(default_warnings.is_empty());

        let lowered_threshold_warnings =
            collect_ssg_mean_median_drift_warnings_with_threshold(&summary, 5.0);
        assert_eq!(lowered_threshold_warnings.len(), 1);
        assert!(lowered_threshold_warnings
            .iter()
            .any(|warning| warning.contains("Kujo build time")));
    }

    #[test]
    fn test_collect_ssg_range_spread_warnings_flags_high_spread_metrics() {
        let summary = SsgBenchmarkAggregateResult {
            files: 100,
            kujo_checksum: 42,
            kujo_build_ms: SsgRunStatistics {
                runs: 4,
                mean: 160.0,
                median: 100.0,
                p90: 220.0,
                p95: 250.0,
                min: 60.0,
                max: 300.0,
                stddev: 90.0,
            },
            kujo_files_per_sec: SsgRunStatistics {
                runs: 4,
                mean: 1000.0,
                median: 1000.0,
                p90: 1002.0,
                p95: 1003.0,
                min: 998.0,
                max: 1004.0,
                stddev: 2.0,
            },
            kujo_stage_profile: None,
            python_build_ms: None,
            python_files_per_sec: None,
            python_stage_profile: None,
            kujo_vs_python_speedup: None,
        };

        let warnings = collect_ssg_range_spread_warnings_with_threshold(
            &summary,
            SSG_RANGE_SPREAD_WARNING_THRESHOLD_PERCENT,
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings.iter().any(|warning| warning.contains("Kujo build time")));
        assert!(warnings.iter().all(|warning| warning.contains("range spread is high")));
    }

    #[test]
    fn test_collect_ssg_range_spread_warnings_respects_threshold_override() {
        let summary = SsgBenchmarkAggregateResult {
            files: 100,
            kujo_checksum: 42,
            kujo_build_ms: SsgRunStatistics {
                runs: 4,
                mean: 125.0,
                median: 100.0,
                p90: 140.0,
                p95: 145.0,
                min: 80.0,
                max: 130.0,
                stddev: 20.0,
            },
            kujo_files_per_sec: SsgRunStatistics {
                runs: 4,
                mean: 1000.0,
                median: 1000.0,
                p90: 1001.0,
                p95: 1001.5,
                min: 999.0,
                max: 1002.0,
                stddev: 1.0,
            },
            kujo_stage_profile: None,
            python_build_ms: None,
            python_files_per_sec: None,
            python_stage_profile: None,
            kujo_vs_python_speedup: None,
        };

        let default_warnings = collect_ssg_range_spread_warnings_with_threshold(
            &summary,
            SSG_RANGE_SPREAD_WARNING_THRESHOLD_PERCENT,
        );
        assert!(default_warnings.is_empty());

        let lowered_threshold_warnings =
            collect_ssg_range_spread_warnings_with_threshold(&summary, 25.0);
        assert_eq!(lowered_threshold_warnings.len(), 1);
        assert!(lowered_threshold_warnings
            .iter()
            .any(|warning| warning.contains("Kujo build time")));
    }

    #[test]
    fn test_analyze_ssg_benchmark_trends_returns_none_for_single_run() {
        let runs = vec![SsgBenchmarkResult {
            files: 1,
            kujo_build_ms: 10.0,
            kujo_files_per_sec: 100.0,
            kujo_checksum: 9,
            kujo_stage_profile: None,
            python_build_ms: None,
            python_files_per_sec: None,
            python_stage_profile: None,
        }];

        let trends = analyze_ssg_benchmark_trends(&runs).unwrap();
        assert!(trends.is_none());
    }

    #[test]
    fn test_analyze_ssg_benchmark_trends_without_python() {
        let runs = vec![
            SsgBenchmarkResult {
                files: 2,
                kujo_build_ms: 12.0,
                kujo_files_per_sec: 200.0,
                kujo_checksum: 77,
                kujo_stage_profile: None,
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 2,
                kujo_build_ms: 9.0,
                kujo_files_per_sec: 250.0,
                kujo_checksum: 77,
                kujo_stage_profile: None,
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
        ];

        let trends = analyze_ssg_benchmark_trends(&runs).unwrap().unwrap();
        assert_eq!(trends.measured_runs, 2);
        assert!((trends.kujo_build_ms.first - 12.0).abs() < 0.0001);
        assert!((trends.kujo_build_ms.last - 9.0).abs() < 0.0001);
        assert!((trends.kujo_build_ms.absolute_delta + 3.0).abs() < 0.0001);
        assert!(matches!(trends.kujo_build_ms.percent_delta, Some(p) if (p + 25.0).abs() < 0.0001));

        assert!((trends.kujo_files_per_sec.absolute_delta - 50.0).abs() < 0.0001);
        assert!(trends.python_build_ms.is_none());
        assert!(trends.python_files_per_sec.is_none());
        assert!(trends.kujo_vs_python_speedup.is_none());
    }

    #[test]
    fn test_analyze_ssg_benchmark_trends_with_python_and_speedup() {
        let runs = vec![
            SsgBenchmarkResult {
                files: 2,
                kujo_build_ms: 10.0,
                kujo_files_per_sec: 220.0,
                kujo_checksum: 7,
                kujo_stage_profile: None,
                python_build_ms: Some(20.0),
                python_files_per_sec: Some(110.0),
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 2,
                kujo_build_ms: 8.0,
                kujo_files_per_sec: 275.0,
                kujo_checksum: 7,
                kujo_stage_profile: None,
                python_build_ms: Some(24.0),
                python_files_per_sec: Some(91.0),
                python_stage_profile: None,
            },
        ];

        let trends = analyze_ssg_benchmark_trends(&runs).unwrap().unwrap();
        assert!(trends.python_build_ms.is_some());
        assert!(trends.python_files_per_sec.is_some());
        assert!(trends.kujo_vs_python_speedup.is_some());

        let speedup = trends.kujo_vs_python_speedup.unwrap();
        assert!((speedup.first - 2.0).abs() < 0.0001);
        assert!((speedup.last - 3.0).abs() < 0.0001);
        assert!((speedup.absolute_delta - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_analyze_ssg_benchmark_trends_rejects_inconsistent_python_presence() {
        let runs = vec![
            SsgBenchmarkResult {
                files: 1,
                kujo_build_ms: 10.0,
                kujo_files_per_sec: 100.0,
                kujo_checksum: 1,
                kujo_stage_profile: None,
                python_build_ms: Some(20.0),
                python_files_per_sec: Some(50.0),
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 1,
                kujo_build_ms: 9.0,
                kujo_files_per_sec: 111.0,
                kujo_checksum: 1,
                kujo_stage_profile: None,
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
        ];

        let err = analyze_ssg_benchmark_trends(&runs).unwrap_err();
        assert!(err.contains("inconsistent Python comparison presence"));
    }

    #[test]
    fn test_analyze_ssg_benchmark_trends_handles_zero_first_value_percent_delta() {
        let runs = vec![
            SsgBenchmarkResult {
                files: 1,
                kujo_build_ms: 0.0,
                kujo_files_per_sec: 0.0,
                kujo_checksum: 2,
                kujo_stage_profile: None,
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 1,
                kujo_build_ms: 5.0,
                kujo_files_per_sec: 50.0,
                kujo_checksum: 2,
                kujo_stage_profile: None,
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
        ];

        let trends = analyze_ssg_benchmark_trends(&runs).unwrap().unwrap();
        assert!(trends.kujo_build_ms.percent_delta.is_none());
        assert!(trends.kujo_files_per_sec.percent_delta.is_none());
    }

    #[test]
    fn test_collect_ssg_trend_warnings_flags_large_percent_deltas() {
        let trends = SsgBenchmarkTrendReport {
            measured_runs: 4,
            kujo_build_ms: SsgTrendMetric {
                first: 100.0,
                last: 130.0,
                absolute_delta: 30.0,
                percent_delta: Some(30.0),
            },
            kujo_files_per_sec: SsgTrendMetric {
                first: 500.0,
                last: 430.0,
                absolute_delta: -70.0,
                percent_delta: Some(-14.0),
            },
            python_build_ms: Some(SsgTrendMetric {
                first: 180.0,
                last: 171.0,
                absolute_delta: -9.0,
                percent_delta: Some(-5.0),
            }),
            python_files_per_sec: None,
            kujo_vs_python_speedup: Some(SsgTrendMetric {
                first: 1.8,
                last: 2.3,
                absolute_delta: 0.5,
                percent_delta: Some(27.777777777),
            }),
        };

        let warnings =
            collect_ssg_trend_warnings_with_threshold(&trends, SSG_TREND_WARNING_THRESHOLD_PERCENT);
        assert_eq!(warnings.len(), 3);
        assert!(warnings.iter().any(|warning| warning.contains("Kujo build time")));
        assert!(warnings.iter().any(|warning| warning.contains("Kujo throughput")));
        assert!(warnings.iter().any(|warning| warning.contains("Kujo vs Python speedup")));
        assert!(!warnings.iter().any(|warning| warning.contains("Python build time")));
    }

    #[test]
    fn test_collect_ssg_trend_warnings_requires_three_measured_runs() {
        let trends = SsgBenchmarkTrendReport {
            measured_runs: 2,
            kujo_build_ms: SsgTrendMetric {
                first: 100.0,
                last: 150.0,
                absolute_delta: 50.0,
                percent_delta: Some(50.0),
            },
            kujo_files_per_sec: SsgTrendMetric {
                first: 500.0,
                last: 250.0,
                absolute_delta: -250.0,
                percent_delta: Some(-50.0),
            },
            python_build_ms: None,
            python_files_per_sec: None,
            kujo_vs_python_speedup: None,
        };

        let warnings =
            collect_ssg_trend_warnings_with_threshold(&trends, SSG_TREND_WARNING_THRESHOLD_PERCENT);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_collect_ssg_trend_warnings_skips_small_or_percentless_deltas() {
        let trends = SsgBenchmarkTrendReport {
            measured_runs: 5,
            kujo_build_ms: SsgTrendMetric {
                first: 100.0,
                last: 107.0,
                absolute_delta: 7.0,
                percent_delta: Some(7.0),
            },
            kujo_files_per_sec: SsgTrendMetric {
                first: 0.0,
                last: 25.0,
                absolute_delta: 25.0,
                percent_delta: None,
            },
            python_build_ms: None,
            python_files_per_sec: None,
            kujo_vs_python_speedup: None,
        };

        let warnings =
            collect_ssg_trend_warnings_with_threshold(&trends, SSG_TREND_WARNING_THRESHOLD_PERCENT);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_collect_ssg_trend_warnings_custom_threshold_can_trigger_warning() {
        let trends = SsgBenchmarkTrendReport {
            measured_runs: 5,
            kujo_build_ms: SsgTrendMetric {
                first: 100.0,
                last: 107.0,
                absolute_delta: 7.0,
                percent_delta: Some(7.0),
            },
            kujo_files_per_sec: SsgTrendMetric {
                first: 500.0,
                last: 530.0,
                absolute_delta: 30.0,
                percent_delta: Some(6.0),
            },
            python_build_ms: None,
            python_files_per_sec: None,
            kujo_vs_python_speedup: None,
        };

        let default_warnings =
            collect_ssg_trend_warnings_with_threshold(&trends, SSG_TREND_WARNING_THRESHOLD_PERCENT);
        assert!(default_warnings.is_empty());

        let lowered_threshold_warnings = collect_ssg_trend_warnings_with_threshold(&trends, 5.0);
        assert_eq!(lowered_threshold_warnings.len(), 2);
        assert!(lowered_threshold_warnings
            .iter()
            .any(|warning| warning.contains("Kujo build time")));
        assert!(lowered_threshold_warnings
            .iter()
            .any(|warning| warning.contains("Kujo throughput")));
    }

    #[test]
    fn test_aggregate_ssg_results_without_python() {
        let runs = vec![
            SsgBenchmarkResult {
                files: 100,
                kujo_build_ms: 10.0,
                kujo_files_per_sec: 10.0,
                kujo_checksum: 42,
                kujo_stage_profile: Some(SsgStageProfile { read_ms: 4.0, render_write_ms: 6.0 }),
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 100,
                kujo_build_ms: 20.0,
                kujo_files_per_sec: 5.0,
                kujo_checksum: 42,
                kujo_stage_profile: Some(SsgStageProfile { read_ms: 7.0, render_write_ms: 13.0 }),
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 100,
                kujo_build_ms: 30.0,
                kujo_files_per_sec: 3.333333333,
                kujo_checksum: 42,
                kujo_stage_profile: Some(SsgStageProfile { read_ms: 10.0, render_write_ms: 20.0 }),
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
        ];

        let summary = aggregate_ssg_results(&runs).unwrap();
        assert_eq!(summary.files, 100);
        assert_eq!(summary.kujo_checksum, 42);
        assert!((summary.kujo_build_ms.median - 20.0).abs() < 0.0001);
        assert!((summary.kujo_build_ms.p90 - 28.0).abs() < 0.0001);
        assert!((summary.kujo_build_ms.p95 - 29.0).abs() < 0.0001);
        assert!((summary.kujo_files_per_sec.median - 5.0).abs() < 0.0001);
        assert!((summary.kujo_files_per_sec.p90 - 9.0).abs() < 0.0001);
        assert!((summary.kujo_files_per_sec.p95 - 9.5).abs() < 0.0001);
        assert!(summary.python_build_ms.is_none());
        assert!(summary.python_files_per_sec.is_none());
        assert!(summary.kujo_vs_python_speedup.is_none());
        assert!(summary.kujo_stage_profile.is_some());
    }

    #[test]
    fn test_aggregate_ssg_results_with_python() {
        let runs = vec![
            SsgBenchmarkResult {
                files: 2,
                kujo_build_ms: 10.0,
                kujo_files_per_sec: 200.0,
                kujo_checksum: 7,
                kujo_stage_profile: None,
                python_build_ms: Some(20.0),
                python_files_per_sec: Some(100.0),
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 2,
                kujo_build_ms: 20.0,
                kujo_files_per_sec: 100.0,
                kujo_checksum: 7,
                kujo_stage_profile: None,
                python_build_ms: Some(40.0),
                python_files_per_sec: Some(50.0),
                python_stage_profile: None,
            },
        ];

        let summary = aggregate_ssg_results(&runs).unwrap();
        assert!(summary.python_build_ms.is_some());
        assert!(summary.python_files_per_sec.is_some());
        assert!((summary.kujo_build_ms.p90 - 19.0).abs() < 0.0001);
        assert!((summary.kujo_build_ms.p95 - 19.5).abs() < 0.0001);

        let speedup = summary.kujo_vs_python_speedup.unwrap();
        assert!((speedup.median - 2.0).abs() < 0.0001);
        assert!((speedup.p90 - 2.0).abs() < 0.0001);
        assert!((speedup.p95 - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_aggregate_ssg_results_empty_fails() {
        let err = aggregate_ssg_results(&[]).unwrap_err();
        assert!(err.contains("no runs provided"));
    }

    #[test]
    fn test_aggregate_ssg_results_inconsistent_files_fails() {
        let runs = vec![
            SsgBenchmarkResult {
                files: 10,
                kujo_build_ms: 10.0,
                kujo_files_per_sec: 10.0,
                kujo_checksum: 1,
                kujo_stage_profile: None,
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 11,
                kujo_build_ms: 12.0,
                kujo_files_per_sec: 9.0,
                kujo_checksum: 1,
                kujo_stage_profile: None,
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
        ];

        let err = aggregate_ssg_results(&runs).unwrap_err();
        assert!(err.contains("file count mismatch"));
    }

    #[test]
    fn test_aggregate_ssg_results_inconsistent_python_presence_fails() {
        let runs = vec![
            SsgBenchmarkResult {
                files: 10,
                kujo_build_ms: 10.0,
                kujo_files_per_sec: 10.0,
                kujo_checksum: 1,
                kujo_stage_profile: None,
                python_build_ms: Some(15.0),
                python_files_per_sec: Some(8.0),
                python_stage_profile: None,
            },
            SsgBenchmarkResult {
                files: 10,
                kujo_build_ms: 12.0,
                kujo_files_per_sec: 9.0,
                kujo_checksum: 1,
                kujo_stage_profile: None,
                python_build_ms: None,
                python_files_per_sec: None,
                python_stage_profile: None,
            },
        ];

        let err = aggregate_ssg_results(&runs).unwrap_err();
        assert!(err.contains("inconsistent Python comparison presence"));
    }

    #[test]
    fn test_run_ssg_benchmark_series_rejects_zero_measured_runs() {
        let err = run_ssg_benchmark_series(
            Path::new("/bin/echo"),
            Path::new("benchmarks/cross-language/bench_ssg.kujo"),
            None,
            None,
            None,
            0,
            0,
            false,
        )
        .unwrap_err();

        assert!(err.contains("SSG benchmark runs must be >= 1"));
    }

    #[cfg(unix)]
    fn create_counting_harness_fixture(prefix: &str) -> (PathBuf, PathBuf, PathBuf) {
        let test_dir = unique_test_dir(prefix);

        let kujo_script_path = test_dir.join("stub_bench_ssg.kujo");
        fs::write(&kujo_script_path, "# stub kujo benchmark script\n")
            .expect("kujo script fixture should be written");

        let counter_path = test_dir.join("run_counter.txt");
        let counter_path_str = counter_path.to_str().expect("counter path should be valid utf-8");

        let kujo_binary_path = test_dir.join("kujo_counter_stub.sh");
        let kujo_stub_body = format!(
            "#!/bin/sh\nif [ \"$1\" != \"run\" ]; then\n  echo \"unexpected args: $@\" >&2\n  exit 9\nfi\ncount=0\nif [ -f \"{counter}\" ]; then\n  count=$(cat \"{counter}\")\nfi\ncount=$((count + 1))\nprintf \"%s\" \"$count\" > \"{counter}\"\nprintf \"\\n\"\necho \"KUJO_SSG_FILES=1\"\necho \"KUJO_SSG_BUILD_MS=$count\"\necho \"KUJO_SSG_FILES_PER_SEC=1000.0\"\necho \"KUJO_SSG_CHECKSUM=777\"\n",
            counter = counter_path_str
        );
        write_stub_executable(&kujo_binary_path, &kujo_stub_body);

        (kujo_binary_path, kujo_script_path, counter_path)
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_series_warmups_are_excluded_from_measured_results() {
        let (kujo_binary, kujo_script, counter_path) =
            create_counting_harness_fixture("series_warmup_exclusion");

        let results = run_ssg_benchmark_series(
            kujo_binary.as_path(),
            kujo_script.as_path(),
            None,
            None,
            None,
            2,
            3,
            false,
        )
        .unwrap();

        assert_eq!(results.len(), 3);
        assert!((results[0].kujo_build_ms - 3.0).abs() < 0.0001);
        assert!((results[1].kujo_build_ms - 4.0).abs() < 0.0001);
        assert!((results[2].kujo_build_ms - 5.0).abs() < 0.0001);

        let counter_contents =
            fs::read_to_string(counter_path.as_path()).expect("counter file should be readable");
        assert_eq!(counter_contents.trim(), "5");
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_series_reports_warmup_failures() {
        let kujo_lines =
            ["KUJO_SSG_FILES=10000", "KUJO_SSG_BUILD_MS=100.0", "KUJO_SSG_CHECKSUM=777"];
        let (kujo_binary, kujo_script, _, _) =
            create_basic_harness_fixture("series_warmup_failure", &kujo_lines, None);

        let err = run_ssg_benchmark_series(
            kujo_binary.as_path(),
            kujo_script.as_path(),
            None,
            None,
            None,
            1,
            2,
            false,
        )
        .unwrap_err();

        assert!(err.contains("Warmup run 1/1 failed"));
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_series_reports_measured_failures() {
        let test_dir = unique_test_dir("series_measured_failure");
        let kujo_script_path = test_dir.join("stub_bench_ssg.kujo");
        fs::write(&kujo_script_path, "# stub kujo benchmark script\n")
            .expect("kujo script fixture should be written");

        let counter_path = test_dir.join("run_counter.txt");
        let counter_path_str = counter_path.to_str().expect("counter path should be valid utf-8");
        let kujo_binary_path = test_dir.join("kujo_counter_fail_stub.sh");
        let kujo_stub_body = format!(
            "#!/bin/sh\nif [ \"$1\" != \"run\" ]; then\n  echo \"unexpected args: $@\" >&2\n  exit 9\nfi\ncount=0\nif [ -f \"{counter}\" ]; then\n  count=$(cat \"{counter}\")\nfi\ncount=$((count + 1))\nprintf \"%s\" \"$count\" > \"{counter}\"\nprintf \"\\n\"\nif [ \"$count\" -eq 2 ]; then\n  echo \"KUJO_SSG_FILES=10000\"\n  echo \"KUJO_SSG_BUILD_MS=100.0\"\n  echo \"KUJO_SSG_CHECKSUM=777\"\n  exit 0\nfi\necho \"KUJO_SSG_FILES=10000\"\necho \"KUJO_SSG_BUILD_MS=100.0\"\necho \"KUJO_SSG_FILES_PER_SEC=1000.0\"\necho \"KUJO_SSG_CHECKSUM=777\"\n",
            counter = counter_path_str
        );
        write_stub_executable(&kujo_binary_path, &kujo_stub_body);

        let err = run_ssg_benchmark_series(
            kujo_binary_path.as_path(),
            kujo_script_path.as_path(),
            None,
            None,
            None,
            1,
            2,
            false,
        )
        .unwrap_err();

        assert!(err.contains("Measured run 1/2 failed"));
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_fails_when_required_kujo_metric_missing() {
        let kujo_lines =
            ["KUJO_SSG_FILES=10000", "KUJO_SSG_BUILD_MS=100.0", "KUJO_SSG_CHECKSUM=777"];
        let (kujo_binary, kujo_script, _, _) =
            create_basic_harness_fixture("missing_kujo_metric", &kujo_lines, None);

        let err = run_ssg_benchmark(
            kujo_binary.as_path(),
            kujo_script.as_path(),
            None,
            None,
            None,
            false,
        )
        .unwrap_err();

        assert!(
            err.contains("Metric 'KUJO_SSG_FILES_PER_SEC' not found in output"),
            "unexpected benchmark error: {err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_profile_toggle_controls_stage_profile_capture() {
        let (kujo_binary, kujo_script, python_binary, python_script) =
            create_profile_toggle_harness_fixture("profile_toggle_capture");

        let baseline = run_ssg_benchmark(
            kujo_binary.as_path(),
            kujo_script.as_path(),
            Some(python_binary.to_str().expect("python stub path should be utf8")),
            Some(python_script.as_path()),
            None,
            false,
        )
        .expect("baseline benchmark should succeed");

        assert!(baseline.kujo_stage_profile.is_none());
        assert!(baseline.python_stage_profile.is_none());

        let profiled = run_ssg_benchmark(
            kujo_binary.as_path(),
            kujo_script.as_path(),
            Some(python_binary.to_str().expect("python stub path should be utf8")),
            Some(python_script.as_path()),
            None,
            true,
        )
        .expect("profiled benchmark should succeed");

        let kujo_profile = profiled.kujo_stage_profile.expect("kujo profile should be present");
        assert!((kujo_profile.read_ms - 10.5).abs() < 0.0001);
        assert!((kujo_profile.render_write_ms - 40.5).abs() < 0.0001);

        let python_profile =
            profiled.python_stage_profile.expect("python profile should be present");
        assert!((python_profile.read_ms - 12.0).abs() < 0.0001);
        assert!((python_profile.render_write_ms - 52.0).abs() < 0.0001);
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_fails_when_required_python_metric_missing() {
        let kujo_lines = [
            "KUJO_SSG_FILES=10000",
            "KUJO_SSG_BUILD_MS=100.0",
            "KUJO_SSG_FILES_PER_SEC=1000.0",
            "KUJO_SSG_CHECKSUM=777",
        ];
        let python_lines =
            ["PYTHON_SSG_FILES=10000", "PYTHON_SSG_BUILD_MS=120.0", "PYTHON_SSG_CHECKSUM=777"];
        let (kujo_binary, kujo_script, python_binary, python_script_opt) =
            create_basic_harness_fixture("missing_python_metric", &kujo_lines, Some(&python_lines));
        let python_script = python_script_opt.expect("python script fixture should exist");

        let err = run_ssg_benchmark(
            kujo_binary.as_path(),
            kujo_script.as_path(),
            Some(python_binary.to_str().expect("python stub path should be utf8")),
            Some(python_script.as_path()),
            None,
            false,
        )
        .unwrap_err();

        assert!(err.contains("Metric 'PYTHON_SSG_FILES_PER_SEC' not found in output"));
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_fails_on_python_checksum_mismatch() {
        let kujo_lines = [
            "KUJO_SSG_FILES=10000",
            "KUJO_SSG_BUILD_MS=100.0",
            "KUJO_SSG_FILES_PER_SEC=1000.0",
            "KUJO_SSG_CHECKSUM=777",
        ];
        let python_lines = [
            "PYTHON_SSG_FILES=10000",
            "PYTHON_SSG_BUILD_MS=120.0",
            "PYTHON_SSG_FILES_PER_SEC=900.0",
            "PYTHON_SSG_CHECKSUM=778",
        ];
        let (kujo_binary, kujo_script, python_binary, python_script_opt) =
            create_basic_harness_fixture("checksum_mismatch", &kujo_lines, Some(&python_lines));
        let python_script = python_script_opt.expect("python script fixture should exist");

        let err = run_ssg_benchmark(
            kujo_binary.as_path(),
            kujo_script.as_path(),
            Some(python_binary.to_str().expect("python stub path should be utf8")),
            Some(python_script.as_path()),
            None,
            false,
        )
        .unwrap_err();

        assert!(err.contains("Checksum mismatch"));
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_rejects_missing_kujo_script_preflight() {
        let missing_kujo_script =
            PathBuf::from("tmp/bench_ssg_missing_kujo_script_does_not_exist.kujo");
        let err = run_ssg_benchmark(
            Path::new("/bin/echo"),
            missing_kujo_script.as_path(),
            None,
            None,
            None,
            false,
        )
        .unwrap_err();

        assert!(err.contains("Kujo SSG benchmark script not found"));
    }

    #[cfg(unix)]
    #[test]
    fn test_run_ssg_benchmark_rejects_missing_python_script_preflight() {
        let mut fixture_root = std::env::current_dir().expect("current_dir should resolve");
        fixture_root.push("tmp");
        fixture_root.push("bench_ssg_missing_python_script_preflight");
        fs::create_dir_all(&fixture_root).expect("fixture root should be created");

        let kujo_script = fixture_root.join("existing_kujo_bench.kujo");
        fs::write(&kujo_script, "# stub\n").expect("kujo script fixture should be written");

        let missing_python_script = fixture_root.join("missing_python_bench.py");

        let err = run_ssg_benchmark(
            Path::new("/bin/echo"),
            kujo_script.as_path(),
            Some("python3"),
            Some(missing_python_script.as_path()),
            None,
            false,
        )
        .unwrap_err();

        assert!(err.contains("Python SSG benchmark script not found"));
    }
}
