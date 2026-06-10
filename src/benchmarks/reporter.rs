// Results formatting and reporting

use crate::benchmarks::{BenchmarkResult, ExecutionMode, Statistics};
use colored::*;
use std::time::Duration;

pub struct Reporter;

impl Reporter {
    const WIDTH: usize = 80;

    fn render_header_text(title: &str) -> String {
        format!(
            "{}\n{:^width$}\n{}\n\n",
            "=".repeat(Self::WIDTH),
            title,
            "=".repeat(Self::WIDTH),
            width = Self::WIDTH
        )
    }

    fn render_separator_text() -> String {
        "-".repeat(Self::WIDTH)
    }

    pub fn print_header(title: &str) {
        println!("{}", "=".repeat(Self::WIDTH).bright_blue());
        println!("{:^width$}", title.bright_white().bold(), width = Self::WIDTH);
        println!("{}", "=".repeat(Self::WIDTH).bright_blue());
        println!();
    }

    pub fn print_benchmark_result(result: &BenchmarkResult) {
        if !result.success {
            println!(
                "{} {}",
                "✗".red().bold(),
                format!(
                    "{} ({}) - {}",
                    result.name,
                    result.mode.name(),
                    result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                )
                .red()
            );
            return;
        }

        if let Some(stats) = Statistics::from_samples(&result.samples) {
            println!(
                "{} {} ({})",
                "✓".green().bold(),
                result.name.bright_white(),
                result.mode.name().cyan()
            );
            println!("  Mean:   {}", Statistics::format_duration(stats.mean).yellow());
            println!("  Median: {}", Statistics::format_duration(stats.median).yellow());
            println!("  Min:    {}", Statistics::format_duration(stats.min).green());
            println!("  Max:    {}", Statistics::format_duration(stats.max).red());
            println!("  StdDev: {}", Statistics::format_duration(stats.stddev).blue());
            println!("  Samples: {}", stats.samples);
        }
    }

    pub fn render_comparison_table_text(results: &[(String, Vec<BenchmarkResult>)]) -> String {
        let mut out = String::new();
        out.push('\n');
        out.push_str(&Self::render_header_text("Performance Comparison"));

        out.push_str(&format!(
            "{:<30} {:>15} {:>15} {:>15} {:>15}\n",
            "Benchmark", "Interpreter", "VM", "JIT", "Speedup"
        ));
        out.push_str(&Self::render_separator_text());
        out.push('\n');

        for (name, bench_results) in results {
            let mut interp_time = None;
            let mut vm_time = None;
            let mut jit_time = None;

            for result in bench_results {
                if result.success {
                    let mean = result.mean();
                    match result.mode {
                        ExecutionMode::Interpreter => interp_time = mean,
                        ExecutionMode::VM => vm_time = mean,
                        ExecutionMode::JIT => jit_time = mean,
                    }
                }
            }

            let interp_str =
                interp_time.map(Statistics::format_duration).unwrap_or_else(|| "N/A".into());
            let vm_str = vm_time.map(Statistics::format_duration).unwrap_or_else(|| "N/A".into());
            let jit_str = jit_time.map(Statistics::format_duration).unwrap_or_else(|| "N/A".into());
            let speedup_str = if let (Some(interp), Some(jit)) = (interp_time, jit_time) {
                format!("{:.2}x", interp.as_nanos() as f64 / jit.as_nanos() as f64)
            } else {
                "N/A".to_string()
            };

            out.push_str(&format!(
                "{:<30} {:>15} {:>15} {:>15} {:>15}\n",
                name, interp_str, vm_str, jit_str, speedup_str
            ));
        }

        out.push('\n');
        out
    }

    pub fn print_comparison_table(results: &[(String, Vec<BenchmarkResult>)]) {
        print!("{}", Self::render_comparison_table_text(results));
    }

    pub fn render_summary_text(results: &[(String, Vec<BenchmarkResult>)]) -> Option<String> {
        let mut total_interp = Duration::ZERO;
        let mut total_vm = Duration::ZERO;
        let mut total_jit = Duration::ZERO;
        let mut count = 0;

        for (_name, bench_results) in results {
            for result in bench_results {
                if result.success {
                    if let Some(mean) = result.mean() {
                        match result.mode {
                            ExecutionMode::Interpreter => total_interp += mean,
                            ExecutionMode::VM => total_vm += mean,
                            ExecutionMode::JIT => total_jit += mean,
                        }
                    }
                }
            }
            count += 1;
        }

        if count == 0 {
            return None;
        }

        let mut out = String::new();
        out.push('\n');
        out.push_str(&Self::render_header_text("Summary"));
        out.push_str(&format!("Total Benchmarks: {}\n", count));
        out.push_str(&format!(
            "Total Time (Interpreter): {}\n",
            Statistics::format_duration(total_interp)
        ));
        out.push_str(&format!("Total Time (VM): {}\n", Statistics::format_duration(total_vm)));
        out.push_str(&format!("Total Time (JIT): {}\n", Statistics::format_duration(total_jit)));

        if total_interp.as_nanos() > 0 {
            let vm_speedup = total_interp.as_nanos() as f64 / total_vm.as_nanos() as f64;
            let jit_speedup = total_interp.as_nanos() as f64 / total_jit.as_nanos() as f64;
            out.push('\n');
            out.push_str(&format!("Average VM Speedup: {:.2}x\n", vm_speedup));
            out.push_str(&format!("Average JIT Speedup: {:.2}x\n", jit_speedup));
        }

        Some(out)
    }

    pub fn print_summary(results: &[(String, Vec<BenchmarkResult>)]) {
        if let Some(summary) = Self::render_summary_text(results) {
            print!("{}", summary);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_with_mean(name: &str, mode: ExecutionMode, millis: u64) -> BenchmarkResult {
        let mut result = BenchmarkResult::new(name.to_string(), mode);
        result.add_sample(Duration::from_millis(millis));
        result
    }

    #[test]
    fn render_comparison_table_text_is_deterministic() {
        let rows = vec![(
            "json-parse".to_string(),
            vec![
                result_with_mean("json-parse", ExecutionMode::Interpreter, 10),
                result_with_mean("json-parse", ExecutionMode::VM, 5),
                result_with_mean("json-parse", ExecutionMode::JIT, 2),
            ],
        )];

        let output = Reporter::render_comparison_table_text(&rows);
        assert!(output.contains("Performance Comparison"));
        assert!(output.contains("json-parse"));
        assert!(output.contains("10.00 ms"));
        assert!(output.contains("5.00 ms"));
        assert!(output.contains("2.00 ms"));
        assert!(output.contains("5.00x"));
    }

    #[test]
    fn render_summary_text_includes_totals_and_speedups() {
        let rows = vec![(
            "json-parse".to_string(),
            vec![
                result_with_mean("json-parse", ExecutionMode::Interpreter, 10),
                result_with_mean("json-parse", ExecutionMode::VM, 5),
                result_with_mean("json-parse", ExecutionMode::JIT, 2),
            ],
        )];

        let output = Reporter::render_summary_text(&rows).expect("expected summary output");
        assert!(output.contains("Summary"));
        assert!(output.contains("Total Benchmarks: 1"));
        assert!(output.contains("Total Time (Interpreter): 10.00 ms"));
        assert!(output.contains("Total Time (VM): 5.00 ms"));
        assert!(output.contains("Total Time (JIT): 2.00 ms"));
        assert!(output.contains("Average VM Speedup: 2.00x"));
        assert!(output.contains("Average JIT Speedup: 5.00x"));
    }
}
