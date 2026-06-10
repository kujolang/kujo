use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ProcessPoolComparison {
    pub kujo_parallel_map_ms: f64,
    pub python_process_pool_ms: f64,
    pub python_serial_ms: Option<f64>,
}

impl ProcessPoolComparison {
    pub fn kujo_vs_process_pool_speedup(&self) -> f64 {
        if self.kujo_parallel_map_ms <= 0.0 {
            return 0.0;
        }

        self.python_process_pool_ms / self.kujo_parallel_map_ms
    }

    pub fn process_pool_vs_serial_speedup(&self) -> Option<f64> {
        self.python_serial_ms.map(|serial_ms| {
            if self.python_process_pool_ms <= 0.0 {
                0.0
            } else {
                serial_ms / self.python_process_pool_ms
            }
        })
    }
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

fn run_and_capture(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run '{}': {}", program, e))?;

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

pub fn run_process_pool_comparison(
    kujo_binary: &Path,
    kujo_script: &Path,
    python_binary: &str,
    python_script: &Path,
) -> Result<ProcessPoolComparison, String> {
    let kujo_binary_str = kujo_binary
        .to_str()
        .ok_or_else(|| format!("Invalid Kujo binary path: {}", kujo_binary.display()))?;
    let kujo_script_str = kujo_script
        .to_str()
        .ok_or_else(|| format!("Invalid Kujo script path: {}", kujo_script.display()))?;
    let python_script_str = python_script
        .to_str()
        .ok_or_else(|| format!("Invalid Python script path: {}", python_script.display()))?;

    let kujo_output = run_and_capture(kujo_binary_str, &["run", kujo_script_str])?;
    let python_output = run_and_capture(python_binary, &[python_script_str])?;

    let kujo_parallel_map_ms = parse_metric_value(&kujo_output, "KUJO_PARALLEL_MAP_MS")?;
    let python_process_pool_ms = parse_metric_value(&python_output, "PYTHON_PROCESS_POOL_MS")?;
    let python_serial_ms = parse_metric_value(&python_output, "PYTHON_SERIAL_MS").ok();

    let kujo_checksum = parse_checksum(&kujo_output, "KUJO_PARALLEL_MAP_CHECKSUM")?;
    let python_checksum = parse_checksum(&python_output, "PYTHON_PROCESS_POOL_CHECKSUM")?;

    if kujo_checksum != python_checksum {
        return Err(format!(
            "Checksum mismatch: Kujo={} Python={} (workloads/results are not equivalent)",
            kujo_checksum, python_checksum
        ));
    }

    Ok(ProcessPoolComparison { kujo_parallel_map_ms, python_process_pool_ms, python_serial_ms })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metric_value_extracts_float() {
        let output = "noise\nKUJO_PARALLEL_MAP_MS=12.50\nother";
        let value = parse_metric_value(output, "KUJO_PARALLEL_MAP_MS").unwrap();
        assert!((value - 12.50).abs() < 0.0001);
    }

    #[test]
    fn test_parse_metric_value_trims_spaces() {
        let output = "PYTHON_PROCESS_POOL_MS =  42.25  ";
        let value = parse_metric_value(output, "PYTHON_PROCESS_POOL_MS").unwrap();
        assert!((value - 42.25).abs() < 0.0001);
    }

    #[test]
    fn test_parse_metric_value_missing_key_returns_error() {
        let output = "KUJO_PARALLEL_MAP_MS=9.8";
        let err = parse_metric_value(output, "PYTHON_PROCESS_POOL_MS").unwrap_err();
        assert!(err.contains("Metric 'PYTHON_PROCESS_POOL_MS' not found"));
    }

    #[test]
    fn test_parse_checksum_extracts_integer() {
        let output = "KUJO_PARALLEL_MAP_CHECKSUM=123456";
        let checksum = parse_checksum(output, "KUJO_PARALLEL_MAP_CHECKSUM").unwrap();
        assert_eq!(checksum, 123456);
    }

    #[test]
    fn test_parse_checksum_invalid_value_returns_error() {
        let output = "PYTHON_PROCESS_POOL_CHECKSUM=notanumber";
        let err = parse_checksum(output, "PYTHON_PROCESS_POOL_CHECKSUM").unwrap_err();
        assert!(err.contains("invalid integer value"));
    }

    #[test]
    fn test_speedup_calculations() {
        let comparison = ProcessPoolComparison {
            kujo_parallel_map_ms: 20.0,
            python_process_pool_ms: 40.0,
            python_serial_ms: Some(80.0),
        };

        assert!((comparison.kujo_vs_process_pool_speedup() - 2.0).abs() < 0.0001);
        assert!((comparison.process_pool_vs_serial_speedup().unwrap() - 2.0).abs() < 0.0001);
    }
}
