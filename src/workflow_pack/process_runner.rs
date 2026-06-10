// File: src/workflow_pack/process_runner.rs
//
// Safe process execution abstraction for workflow commands.
// Supports command+args execution, output capture, timeouts, and truncation.
#![allow(dead_code)]

use std::process::{Command, Output};
use std::time::Duration;

/// Result of running an external process.
#[derive(Debug, Clone)]
pub struct ProcessResult {
    /// The command that was run (for display).
    pub command_line: String,
    /// Whether the process exited successfully (exit code 0).
    pub success: bool,
    /// Exit code.
    pub exit_code: Option<i32>,
    /// Captured stdout, truncated if too long.
    pub stdout: String,
    /// Captured stderr, truncated if too long.
    pub stderr: String,
    /// Whether stdout was truncated.
    pub stdout_truncated: bool,
    /// Whether stderr was truncated.
    pub stderr_truncated: bool,
    /// Wall-clock duration.
    pub duration: Duration,
}

/// Maximum captured stdout/stderr bytes before truncation.
const MAX_CAPTURED_OUTPUT_BYTES: usize = 4096;

/// Maximum captured stdout/stderr lines before truncation.
const MAX_CAPTURED_OUTPUT_LINES: usize = 50;

/// Run a command with arguments and capture output.
///
/// # Arguments
/// * `program` - The executable to run.
/// * `args` - Arguments to pass to the executable.
/// * `timeout` - Optional timeout duration.
///
/// # Returns
/// A `ProcessResult` with captured and possibly truncated output.
pub fn run_command(
    program: &str,
    args: &[&str],
    timeout: Option<Duration>,
) -> Result<ProcessResult, String> {
    let start = std::time::Instant::now();

    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Build display command line.
    let command_line = format!("{} {}", program, args.join(" "));

    let output: Output = if let Some(timeout_dur) = timeout {
        // Spawn and wait with timeout.
        let mut child =
            cmd.spawn().map_err(|e| format!("Failed to spawn '{}': {}", command_line, e))?;

        // Simple polling-based timeout approach.
        let poll_interval = Duration::from_millis(100);
        let mut elapsed = Duration::from_millis(0);

        loop {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    let output = child
                        .wait_with_output()
                        .map_err(|e| format!("Failed to wait on '{}': {}", command_line, e))?;
                    // Reconstruct output with status for consistency.
                    return build_process_result(command_line, output, start.elapsed());
                }
                Ok(None) => {
                    if elapsed >= timeout_dur {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!(
                            "Command '{}' timed out after {:?}",
                            command_line, timeout_dur
                        ));
                    }
                    std::thread::sleep(poll_interval);
                    elapsed += poll_interval;
                }
                Err(e) => {
                    return Err(format!("Failed to check status of '{}': {}", command_line, e));
                }
            }
        }
    } else {
        cmd.output().map_err(|e| format!("Failed to execute '{}': {}", command_line, e))?
    };

    build_process_result(command_line, output, start.elapsed())
}

/// Run a command and return only the first line of stdout (trimmed).
/// Useful for version checks like `node --version`.
pub fn run_command_first_line(
    program: &str,
    args: &[&str],
    timeout: Option<Duration>,
) -> Result<Option<String>, String> {
    let result = run_command(program, args, timeout)?;
    let first_line = result.stdout.lines().next().map(|s| s.trim().to_string());
    Ok(first_line)
}

/// Run a command and check if it exists (exit code 0).
pub fn command_exists(program: &str) -> bool {
    run_command(program, &["--version"], Some(Duration::from_secs(5)))
        .map(|r| r.success)
        .unwrap_or(false)
}

/// Build a ProcessResult from raw process output.
fn build_process_result(
    command_line: String,
    output: Output,
    duration: Duration,
) -> Result<ProcessResult, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let (stdout, stdout_truncated) = truncate_output(&stdout);
    let (stderr, stderr_truncated) = truncate_output(&stderr);

    Ok(ProcessResult {
        command_line,
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        duration,
    })
}

/// Truncate output that exceeds line or byte limits.
fn truncate_output(output: &str) -> (String, bool) {
    let lines: Vec<&str> = output.lines().collect();

    if lines.len() > MAX_CAPTURED_OUTPUT_LINES {
        let truncated: String =
            lines[..MAX_CAPTURED_OUTPUT_LINES].join("\n") + "\n... (output truncated)";
        return (truncated, true);
    }

    if output.len() > MAX_CAPTURED_OUTPUT_BYTES {
        let truncated: String = output.chars().take(MAX_CAPTURED_OUTPUT_BYTES).collect::<String>()
            + "\n... (output truncated)";
        return (truncated, true);
    }

    (output.to_string(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_exists_returns_true_for_known_command() {
        // `echo` should exist on all platforms.
        assert!(command_exists("echo"));
    }

    #[test]
    fn command_exists_returns_false_for_nonexistent() {
        assert!(!command_exists("__nonexistent_command_xyzzy__"));
    }

    #[test]
    fn run_command_captures_output() {
        let result = run_command("echo", &["hello", "world"], Some(Duration::from_secs(5)))
            .expect("echo should succeed");
        assert!(result.success);
        assert!(result.stdout.contains("hello world"));
    }

    #[test]
    fn run_command_first_line_works() {
        let line = run_command_first_line("echo", &["test"], Some(Duration::from_secs(5)))
            .expect("echo should succeed");
        assert_eq!(line, Some("test".to_string()));
    }

    #[test]
    fn run_command_handles_failure() {
        let result = run_command("sh", &["-c", "exit 1"], Some(Duration::from_secs(5)))
            .expect("sh should run");
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    fn truncate_output_limits_lines() {
        let long = (0..100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let (truncated, was_truncated) = truncate_output(&long);
        assert!(was_truncated);
        assert!(truncated.contains("(output truncated)"));
    }

    #[test]
    fn truncate_output_preserves_short_output() {
        let short = "hello world";
        let (truncated, was_truncated) = truncate_output(short);
        assert!(!was_truncated);
        assert_eq!(truncated, short);
    }
}
