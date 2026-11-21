// File: src/workflow_pack/process_runner.rs
//
// Safe process execution abstraction for workflow commands.
// Supports command+args execution, output capture, timeouts, and truncation.
#![allow(dead_code)]

use crate::interpreter::native_functions::system::run_workflow_process;
use std::process::{Command, Stdio};
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

/// Maximum structured workflow report size. Doctor profiles may legitimately
/// contain hundreds of bounded checks, so their JSON contract needs a larger
/// ceiling than diagnostic command previews.
pub const MAX_STRUCTURED_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_STRUCTURED_OUTPUT_LINES: usize = 10_000;

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
    run_command_with_limits(
        program,
        args,
        timeout,
        MAX_CAPTURED_OUTPUT_BYTES,
        MAX_CAPTURED_OUTPUT_LINES,
    )
}

/// Run a command with explicit output ceilings.
pub fn run_command_with_limits(
    program: &str,
    args: &[&str],
    timeout: Option<Duration>,
    max_output_bytes: usize,
    max_output_lines: usize,
) -> Result<ProcessResult, String> {
    if max_output_bytes == 0 || max_output_lines == 0 {
        return Err("Process output limits must be greater than zero".to_string());
    }
    let start = std::time::Instant::now();

    let mut cmd = Command::new(program);
    cmd.args(args);
    if timeout.is_none() {
        // Preserve the EOF behavior of the previous Command::output path.
        cmd.stdin(Stdio::null());
    }

    let command_line = format!("{} {}", program, args.join(" "));
    // Retain enough lookahead to decode a UTF-8 character at the byte boundary.
    let output =
        run_workflow_process(cmd, timeout, max_output_bytes.saturating_add(3), &command_line)?;
    if let Some(timeout) = timeout.filter(|_| output.timed_out) {
        return Err(format!("Command '{}' timed out after {:?}", command_line, timeout));
    }
    if output.cancelled {
        return Err(format!("Command '{}' was cancelled", command_line));
    }

    let (stdout, stdout_truncated) = truncate_output(
        &String::from_utf8_lossy(&output.stdout),
        max_output_bytes,
        max_output_lines,
    );
    let (stderr, stderr_truncated) = truncate_output(
        &String::from_utf8_lossy(&output.stderr),
        max_output_bytes,
        max_output_lines,
    );
    Ok(ProcessResult {
        command_line,
        success: output.success,
        exit_code: output.exit_code,
        stdout,
        stderr,
        stdout_truncated: stdout_truncated || output.stdout_truncated,
        stderr_truncated: stderr_truncated || output.stderr_truncated,
        duration: start.elapsed(),
    })
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

/// Apply both output limits to the retained UTF-8 preview.
fn truncate_output(output: &str, max_bytes: usize, max_lines: usize) -> (String, bool) {
    const MARKER: &str = "\n... (output truncated)";
    let mut boundary = output.len().min(max_bytes);
    while !output.is_char_boundary(boundary) {
        boundary -= 1;
    }
    let preview = &output[..boundary];

    if preview.lines().nth(max_lines).is_some() {
        let retained = preview.lines().take(max_lines).collect::<Vec<_>>().join("\n");
        return (retained + MARKER, true);
    }
    if boundary < output.len() {
        return (preview.to_string() + MARKER, true);
    }
    (preview.to_string(), false)
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
        let (truncated, was_truncated) =
            truncate_output(&long, MAX_CAPTURED_OUTPUT_BYTES, MAX_CAPTURED_OUTPUT_LINES);
        assert!(was_truncated);
        assert!(truncated.contains("(output truncated)"));
    }

    #[test]
    fn truncate_output_preserves_short_output() {
        let short = "hello world";
        let (truncated, was_truncated) =
            truncate_output(short, MAX_CAPTURED_OUTPUT_BYTES, MAX_CAPTURED_OUTPUT_LINES);
        assert!(!was_truncated);
        assert_eq!(truncated, short);
    }

    #[test]
    fn truncate_output_honors_both_limits_and_keeps_valid_utf8() {
        let (preview, truncated) = truncate_output("aé\nb\nc", 2, 1);
        assert!(truncated);
        assert_eq!(preview, "a\n... (output truncated)");
        let (preview, truncated) = truncate_output("a\r\nb\r\nc", 100, 2);
        assert!(truncated);
        assert_eq!(preview, "a\nb\n... (output truncated)");
    }

    #[test]
    #[ignore = "invoked as a child by the bounded capture test"]
    fn noisy_child() {
        use std::io::Write;
        let block = [b'x'; 8192];
        for _ in 0..64 {
            std::io::stdout().write_all(&block).unwrap();
            std::io::stderr().write_all(&block).unwrap();
        }
    }

    #[test]
    fn drains_both_pipes_while_retaining_only_bounded_previews() {
        let program = std::env::current_exe().unwrap();
        let (_, module) = module_path!().split_once("::").unwrap();
        let child = format!("{module}::noisy_child");
        for timeout in [Some(Duration::from_secs(5)), None] {
            let result = run_command(
                program.to_str().unwrap(),
                &["--ignored", "--exact", "--nocapture", &child],
                timeout,
            )
            .expect("child should finish after writing to both pipes");
            assert!(result.success);
            for (preview, truncated) in
                [(result.stdout, result.stdout_truncated), (result.stderr, result.stderr_truncated)]
            {
                assert!(truncated);
                assert!(
                    preview.len() <= MAX_CAPTURED_OUTPUT_BYTES + "\n... (output truncated)".len()
                );
                assert!(preview.ends_with("(output truncated)"));
            }
        }
    }

    #[test]
    fn structured_output_limits_preserve_large_json_sized_payloads() {
        let payload = "x".repeat(8192);
        let result = run_command_with_limits(
            "printf",
            &["%s", &payload],
            Some(Duration::from_secs(5)),
            MAX_STRUCTURED_OUTPUT_BYTES,
            MAX_STRUCTURED_OUTPUT_LINES,
        )
        .expect("structured output command should run");
        assert!(!result.stdout_truncated);
        assert_eq!(result.stdout, payload);
    }
}
