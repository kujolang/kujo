// File: src/interpreter/native_functions/system.rs
//
// System-related native functions (env vars, time, etc.)

use crate::builtins;
use crate::interpreter::{DictMap, Value};
use crate::runtime_limits;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::sync::{Arc, Once};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_PROCESS_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_PROCESS_MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_PROCESS_MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
const PROCESS_POLL_INTERVAL_MS: u64 = 10;

#[derive(Clone, Debug)]
struct ProcessExecOptions {
    timeout_ms: u64,
    max_output_bytes: usize,
    inherit_env: bool,
    env_allow: Option<Vec<String>>,
    env_deny: Vec<String>,
    env: HashMap<String, String>,
    stream_channel: Option<SyncSender<Value>>,
    stream_stdout_path: Option<String>,
    stream_stderr_path: Option<String>,
    redact_values: Vec<String>,
    cancel_file: Option<String>,
}

impl Default for ProcessExecOptions {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_PROCESS_TIMEOUT_MS,
            max_output_bytes: DEFAULT_PROCESS_MAX_OUTPUT_BYTES,
            inherit_env: true,
            env_allow: None,
            env_deny: Vec::new(),
            env: HashMap::new(),
            stream_channel: None,
            stream_stdout_path: None,
            stream_stderr_path: None,
            redact_values: Vec::new(),
            cancel_file: None,
        }
    }
}

#[derive(Clone, Debug)]
struct ProcessExecutionResult {
    exitcode: i64,
    success: bool,
    timed_out: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
    cancelled: bool,
}

static PROCESS_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);
static PROCESS_SIGNAL_HANDLER: Once = Once::new();

#[cfg(unix)]
extern "C" fn process_signal_handler(_signal: libc::c_int) {
    PROCESS_CANCEL_REQUESTED.store(true, Ordering::SeqCst);
}

fn install_process_signal_handler() {
    PROCESS_SIGNAL_HANDLER.call_once(|| {
        #[cfg(unix)]
        unsafe {
            libc::signal(libc::SIGINT, process_signal_handler as *const () as libc::sighandler_t);
            libc::signal(libc::SIGTERM, process_signal_handler as *const () as libc::sighandler_t);
        }
    });
}

fn error_object(message: impl Into<String>) -> Value {
    Value::ErrorObject { message: message.into(), stack: Vec::new(), line: None, cause: None }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::Str(_) => "string",
        Value::Bool(_) => "bool",
        Value::Array(_) => "array",
        Value::Dict(_) | Value::FixedDict { .. } => "dict",
        Value::Null => "null",
        _ => "value",
    }
}

fn dict_entries(value: &Value) -> Option<Vec<(String, Value)>> {
    match value {
        Value::Dict(map) => {
            Some(map.iter().map(|(key, value)| (key.as_ref().to_string(), value.clone())).collect())
        }
        Value::FixedDict { keys, values } => Some(
            keys.iter()
                .zip(values.iter())
                .map(|(key, value)| (key.as_ref().to_string(), value.clone()))
                .collect(),
        ),
        _ => None,
    }
}

fn parse_string_list(value: &Value, field_name: &str) -> Result<Vec<String>, Value> {
    let Value::Array(items) = value else {
        return Err(Value::Error(format!(
            "{} must be an array of strings, got {}",
            field_name,
            value_type_name(value)
        )));
    };

    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let Value::Str(text) = item else {
            return Err(Value::Error(format!("{} must be an array of strings", field_name)));
        };
        parsed.push(text.as_ref().to_string());
    }

    Ok(parsed)
}

fn parse_string_dict(value: &Value, field_name: &str) -> Result<HashMap<String, String>, Value> {
    let Some(entries) = dict_entries(value) else {
        return Err(Value::Error(format!(
            "{} must be a dict of string keys and values",
            field_name
        )));
    };

    let mut parsed = HashMap::new();
    for (key, value) in entries {
        match value {
            Value::Str(text) => {
                parsed.insert(key, text.as_ref().to_string());
            }
            _ => {
                return Err(Value::Error(format!(
                    "{} must contain only string values",
                    field_name
                )));
            }
        }
    }

    Ok(parsed)
}

fn parse_stream_channel(value: &Value) -> Result<SyncSender<Value>, Value> {
    let Value::Channel(channel) = value else {
        return Err(Value::Error("stream_channel must be a channel".to_string()));
    };
    channel.lock().map(|guard| guard.0.clone()).map_err(|_| {
        Value::Error("stream_channel is unavailable because its channel is poisoned".to_string())
    })
}

fn parse_positive_u64(value: &Value, field_name: &str) -> Result<u64, Value> {
    match value {
        Value::Int(number) if *number > 0 => Ok(*number as u64),
        Value::Float(number) if number.is_finite() && *number > 0.0 => Ok(*number as u64),
        _ => Err(Value::Error(format!("{} must be a positive number", field_name))),
    }
}

fn validate_shell_command_text(command_text: &str, surface: &str) -> Result<(), Value> {
    if command_text.trim().is_empty() {
        return Err(Value::Error(format!(
            "{} command must not be empty; use spawn_process([...]) for structured argv execution",
            surface
        )));
    }

    if command_text.contains('\0') {
        return Err(Value::Error(format!("{} command contains NUL byte", surface)));
    }

    if command_text.contains('\n') || command_text.contains('\r') {
        return Err(Value::Error(format!(
            "{} command contains newline; use spawn_process([...]) for structured argv execution",
            surface
        )));
    }

    Ok(())
}

fn parse_process_options(options: Option<&Value>) -> Result<ProcessExecOptions, Value> {
    let Some(options_value) = options else {
        return Ok(ProcessExecOptions::default());
    };

    let Some(entries) = dict_entries(options_value) else {
        return Err(Value::Error("process options must be provided as a dict".to_string()));
    };

    let mut options = ProcessExecOptions::default();

    for (key, value) in entries {
        match key.as_str() {
            "timeout_ms" => {
                options.timeout_ms = parse_positive_u64(&value, "timeout_ms")?;
            }
            "max_output_bytes" => {
                let parsed = parse_positive_u64(&value, "max_output_bytes")? as usize;
                if parsed > MAX_PROCESS_MAX_OUTPUT_BYTES {
                    return Err(Value::Error(format!(
                        "max_output_bytes exceeds the maximum supported size ({})",
                        MAX_PROCESS_MAX_OUTPUT_BYTES
                    )));
                }
                options.max_output_bytes = parsed;
            }
            "inherit_env" => match value {
                Value::Bool(flag) => options.inherit_env = flag,
                _ => {
                    return Err(Value::Error("inherit_env must be a boolean".to_string()));
                }
            },
            "env_allow" => {
                options.env_allow = Some(parse_string_list(&value, "env_allow")?);
            }
            "env_deny" => {
                options.env_deny = parse_string_list(&value, "env_deny")?;
            }
            "env" => {
                options.env = parse_string_dict(&value, "env")?;
            }
            "stream_channel" => {
                options.stream_channel = Some(parse_stream_channel(&value)?);
            }
            "stream_stdout_path" | "stream_stderr_path" | "cancel_file" => {
                let Value::Str(path) = value else {
                    return Err(Value::Error(format!("{} must be a string", key)));
                };
                if path.is_empty() {
                    return Err(Value::Error(format!("{} must not be empty", key)));
                }
                match key.as_str() {
                    "stream_stdout_path" => {
                        options.stream_stdout_path = Some(path.as_ref().clone())
                    }
                    "stream_stderr_path" => {
                        options.stream_stderr_path = Some(path.as_ref().clone())
                    }
                    "cancel_file" => options.cancel_file = Some(path.as_ref().clone()),
                    _ => unreachable!(),
                }
            }
            "redact_values" => {
                options.redact_values = parse_string_list(&value, "redact_values")?
                    .into_iter()
                    .filter(|value| !value.is_empty())
                    .collect();
            }
            _ => {
                return Err(Value::Error(format!(
                    "unsupported process option '{}'; supported options are timeout_ms, max_output_bytes, inherit_env, env_allow, env, stream_channel, stream_stdout_path, stream_stderr_path, redact_values, cancel_file",
                    key
                )));
            }
        }
    }

    Ok(options)
}

fn apply_env_policy(command: &mut Command, options: &ProcessExecOptions) {
    match &options.env_allow {
        Some(allow_list) => {
            command.env_clear();
            for key in allow_list {
                if let Some(value) = std::env::var_os(key) {
                    command.env(key, value);
                }
            }
        }
        None if !options.inherit_env => {
            command.env_clear();
        }
        None => {}
    }

    for key in &options.env_deny {
        command.env_remove(key);
    }

    for (key, value) in &options.env {
        command.env(key, value);
    }
}

struct StreamingRedactor {
    secrets: Vec<Vec<u8>>,
    pending: Vec<u8>,
    hold_bytes: usize,
}

impl StreamingRedactor {
    fn new(values: &[String]) -> Self {
        let mut secrets: Vec<Vec<u8>> = values
            .iter()
            .filter(|value| !value.is_empty())
            .map(|value| value.as_bytes().to_vec())
            .collect();
        secrets.sort_by_key(|value| std::cmp::Reverse(value.len()));
        // Retain enough context on both sides of a chunk boundary to avoid
        // flushing the prefix of a secret before the remaining suffix arrives.
        let hold_bytes = secrets.first().map_or(0, |value| value.len().saturating_mul(2));
        Self { secrets, pending: Vec::new(), hold_bytes }
    }

    fn redact(&self, bytes: &[u8]) -> Vec<u8> {
        self.secrets.iter().fold(bytes.to_vec(), |output, secret| {
            let mut redacted = Vec::with_capacity(output.len());
            let mut cursor = 0;
            while cursor < output.len() {
                if output[cursor..].starts_with(secret) {
                    redacted.extend_from_slice(b"[REDACTED]");
                    cursor += secret.len();
                } else {
                    redacted.push(output[cursor]);
                    cursor += 1;
                }
            }
            redacted
        })
    }

    fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        if self.secrets.is_empty() {
            return bytes.to_vec();
        }
        self.pending.extend_from_slice(bytes);
        if self.pending.len() <= self.hold_bytes {
            return Vec::new();
        }
        let split_at = self.pending.len() - self.hold_bytes;
        let prefix = self.pending[..split_at].to_vec();
        self.pending = self.pending[split_at..].to_vec();
        self.redact(&prefix)
    }

    fn finish(&mut self) -> Vec<u8> {
        if self.pending.is_empty() {
            return Vec::new();
        }
        let pending = std::mem::take(&mut self.pending);
        self.redact(&pending)
    }
}

struct StreamSink {
    sender: Option<SyncSender<Value>>,
    file: Option<File>,
    stream_name: &'static str,
    max_output_bytes: usize,
    emitted_bytes: usize,
    sequence: u64,
    truncated: bool,
    cancel_flag: Arc<AtomicBool>,
}

impl StreamSink {
    fn new(
        sender: Option<SyncSender<Value>>,
        file_path: Option<String>,
        stream_name: &'static str,
        max_output_bytes: usize,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<Option<Self>, String> {
        if sender.is_none() && file_path.is_none() {
            return Ok(None);
        }
        let file = file_path
            .map(|path| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|error| error.to_string())
            })
            .transpose()?;
        Ok(Some(Self {
            sender,
            file,
            stream_name,
            max_output_bytes,
            emitted_bytes: 0,
            sequence: 0,
            truncated: false,
            cancel_flag,
        }))
    }

    fn send_event(&mut self, event: Value) {
        let Some(sender) = self.sender.take() else {
            return;
        };
        let mut event = event;
        loop {
            match sender.try_send(event) {
                Ok(()) => {
                    self.sender = Some(sender);
                    return;
                }
                Err(TrySendError::Disconnected(_)) => return,
                Err(TrySendError::Full(next)) => {
                    if self.cancel_flag.load(Ordering::SeqCst) {
                        return;
                    }
                    event = next;
                    thread::sleep(Duration::from_millis(PROCESS_POLL_INTERVAL_MS));
                }
            }
        }
    }

    fn emit(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.is_empty() {
            return Ok(());
        }
        if self.emitted_bytes >= self.max_output_bytes {
            self.truncated = true;
            return Ok(());
        }
        let remaining = self.max_output_bytes - self.emitted_bytes;
        let length = remaining.min(bytes.len());
        if length < bytes.len() {
            self.truncated = true;
        }
        let payload = &bytes[..length];
        self.emitted_bytes += length;
        if let Some(file) = &mut self.file {
            file.write_all(payload).map_err(|error| error.to_string())?;
            file.flush().map_err(|error| error.to_string())?;
        }
        if self.sender.is_some() {
            let mut fields = HashMap::new();
            fields.insert("stream".to_string(), Value::Str(Arc::new(self.stream_name.to_string())));
            fields.insert(
                "data".to_string(),
                Value::Str(Arc::new(String::from_utf8_lossy(payload).to_string())),
            );
            fields.insert("sequence".to_string(), Value::Int(self.sequence as i64));
            fields.insert("eof".to_string(), Value::Bool(false));
            self.sequence += 1;
            self.send_event(Value::Struct { name: "ProcessStreamEvent".to_string(), fields });
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        if let Some(file) = &mut self.file {
            file.flush().map_err(|error| error.to_string())?;
        }
        if self.sender.is_some() {
            let mut fields = HashMap::new();
            fields.insert("stream".to_string(), Value::Str(Arc::new(self.stream_name.to_string())));
            fields.insert("data".to_string(), Value::Str(Arc::new(String::new())));
            fields.insert("sequence".to_string(), Value::Int(self.sequence as i64));
            fields.insert("eof".to_string(), Value::Bool(true));
            self.send_event(Value::Struct { name: "ProcessStreamEvent".to_string(), fields });
        }
        Ok(())
    }
}

fn collect_stream_with_limit<R: Read>(
    mut reader: R,
    max_output_bytes: usize,
    stream_name: &'static str,
    stream_channel: Option<SyncSender<Value>>,
    stream_file_path: Option<String>,
    redact_values: Vec<String>,
    cancel_flag: Arc<AtomicBool>,
) -> Result<(Vec<u8>, bool), String> {
    let mut collected = Vec::new();
    let mut truncated = false;
    let mut redactor = StreamingRedactor::new(&redact_values);
    let mut sink = StreamSink::new(
        stream_channel,
        stream_file_path,
        stream_name,
        max_output_bytes,
        cancel_flag,
    )?;
    let mut chunk = [0_u8; 8192];

    loop {
        let read_count = reader.read(&mut chunk).map_err(|error| error.to_string())?;
        if 0 == read_count {
            break;
        }
        let output = redactor.push(&chunk[..read_count]);
        if !output.is_empty() {
            if let Some(sink) = &mut sink {
                sink.emit(&output)?;
            }
            if collected.len() < max_output_bytes {
                let remaining = max_output_bytes - collected.len();
                let to_copy = remaining.min(output.len());
                collected.extend_from_slice(&output[..to_copy]);
                if to_copy < output.len() {
                    truncated = true;
                }
            } else {
                truncated = true;
            }
        }
    }

    let output = redactor.finish();
    if !output.is_empty() {
        if let Some(sink) = &mut sink {
            sink.emit(&output)?;
        }
        if collected.len() < max_output_bytes {
            let remaining = max_output_bytes - collected.len();
            let to_copy = remaining.min(output.len());
            collected.extend_from_slice(&output[..to_copy]);
            if to_copy < output.len() {
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }
    if let Some(sink) = &mut sink {
        sink.finish()?;
        truncated |= sink.truncated;
    }

    Ok((collected, truncated))
}

fn render_command_for_error(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    }
}

#[cfg(unix)]
fn start_process_group(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn start_process_group(_command: &mut Command) {}

fn terminate_process_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let process_group = -(child.id() as libc::pid_t);
        if unsafe { libc::kill(process_group, libc::SIGKILL) } == 0 {
            return;
        }
    }
    let _ = child.kill();
}

fn run_command_with_options(
    mut command: Command,
    options: &ProcessExecOptions,
    stdin_input: Option<Vec<u8>>,
    command_label: &str,
) -> Result<ProcessExecutionResult, Value> {
    install_process_signal_handler();
    apply_env_policy(&mut command, options);
    start_process_group(&mut command);

    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    if stdin_input.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return Err(error_object(format!(
                "Failed to spawn process '{}': {}",
                command_label, error
            )));
        }
    };

    if let Some(input) = stdin_input {
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(error) = stdin.write_all(&input) {
                return Err(error_object(format!(
                    "Failed to write stdin for process '{}': {}",
                    command_label, error
                )));
            }
        }
    }

    let Some(stdout_reader) = child.stdout.take() else {
        return Err(error_object(format!(
            "Failed to capture stdout for process '{}'",
            command_label
        )));
    };
    let Some(stderr_reader) = child.stderr.take() else {
        return Err(error_object(format!(
            "Failed to capture stderr for process '{}'",
            command_label
        )));
    };

    let max_output_bytes = options.max_output_bytes;
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let stdout_handle = {
        let cancel_flag = cancel_flag.clone();
        let stream_channel = options.stream_channel.clone();
        let stream_file_path = options.stream_stdout_path.clone();
        let redact_values = options.redact_values.clone();
        thread::spawn(move || {
            collect_stream_with_limit(
                stdout_reader,
                max_output_bytes,
                "stdout",
                stream_channel,
                stream_file_path,
                redact_values,
                cancel_flag,
            )
        })
    };
    let stderr_handle = {
        let cancel_flag = cancel_flag.clone();
        let stream_channel = options.stream_channel.clone();
        let stream_file_path = options.stream_stderr_path.clone();
        let redact_values = options.redact_values.clone();
        thread::spawn(move || {
            collect_stream_with_limit(
                stderr_reader,
                max_output_bytes,
                "stderr",
                stream_channel,
                stream_file_path,
                redact_values,
                cancel_flag,
            )
        })
    };

    let timeout = Duration::from_millis(options.timeout_ms);
    let start = Instant::now();
    let mut timed_out = false;
    let mut cancelled = false;

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                let cancel_file_requested = options
                    .cancel_file
                    .as_ref()
                    .is_some_and(|path| std::path::Path::new(path).exists());
                if PROCESS_CANCEL_REQUESTED.load(Ordering::SeqCst) || cancel_file_requested {
                    cancelled = true;
                    cancel_flag.store(true, Ordering::SeqCst);
                    terminate_process_tree(&mut child);
                    match child.wait() {
                        Ok(status) => break status,
                        Err(error) => {
                            return Err(error_object(format!(
                                "Failed to terminate cancelled process '{}': {}",
                                command_label, error
                            )));
                        }
                    }
                }
                if start.elapsed() >= timeout {
                    timed_out = true;
                    cancel_flag.store(true, Ordering::SeqCst);
                    terminate_process_tree(&mut child);
                    match child.wait() {
                        Ok(status) => break status,
                        Err(error) => {
                            return Err(error_object(format!(
                                "Failed to terminate timed-out process '{}': {}",
                                command_label, error
                            )));
                        }
                    }
                }

                thread::sleep(Duration::from_millis(PROCESS_POLL_INTERVAL_MS));
            }
            Err(error) => {
                return Err(error_object(format!(
                    "Failed while waiting for process '{}': {}",
                    command_label, error
                )));
            }
        }
    };

    let (stdout, stdout_truncated) = match stdout_handle.join() {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            return Err(error_object(format!(
                "Failed to read stdout for process '{}': {}",
                command_label, error
            )));
        }
        Err(_) => {
            return Err(error_object(format!(
                "Failed to join stdout reader for process '{}'",
                command_label
            )));
        }
    };

    let (stderr, stderr_truncated) = match stderr_handle.join() {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            return Err(error_object(format!(
                "Failed to read stderr for process '{}': {}",
                command_label, error
            )));
        }
        Err(_) => {
            return Err(error_object(format!(
                "Failed to join stderr reader for process '{}'",
                command_label
            )));
        }
    };

    Ok(ProcessExecutionResult {
        exitcode: status.code().unwrap_or(-1) as i64,
        success: status.success() && !timed_out,
        timed_out,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        cancelled,
    })
}

fn process_result_to_value(result: ProcessExecutionResult) -> Value {
    let mut fields = HashMap::new();
    fields.insert("exitcode".to_string(), Value::Int(result.exitcode));
    fields.insert(
        "stdout".to_string(),
        Value::Str(Arc::new(String::from_utf8_lossy(&result.stdout).to_string())),
    );
    fields.insert(
        "stderr".to_string(),
        Value::Str(Arc::new(String::from_utf8_lossy(&result.stderr).to_string())),
    );
    fields.insert("success".to_string(), Value::Bool(result.success));
    fields.insert("timed_out".to_string(), Value::Bool(result.timed_out));
    fields.insert("cancelled".to_string(), Value::Bool(result.cancelled));
    fields.insert("stdout_truncated".to_string(), Value::Bool(result.stdout_truncated));
    fields.insert("stderr_truncated".to_string(), Value::Bool(result.stderr_truncated));

    Value::Struct { name: "ProcessResult".to_string(), fields }
}

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        // Random functions
        "random" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "random() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Float(builtins::random())
        }

        "random_int" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "random_int() expects 2 arguments (min, max), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(min_val), Some(max_val)) = (arg_values.first(), arg_values.get(1)) {
                let min = match min_val {
                    Value::Int(n) => *n as f64,
                    Value::Float(n) => *n,
                    _ => {
                        return Some(Value::Error(
                            "random_int requires number arguments".to_string(),
                        ))
                    }
                };
                let max = match max_val {
                    Value::Int(n) => *n as f64,
                    Value::Float(n) => *n,
                    _ => {
                        return Some(Value::Error(
                            "random_int requires number arguments".to_string(),
                        ))
                    }
                };
                if !min.is_finite() || !max.is_finite() {
                    return Some(Value::Error(
                        "random_int requires finite number arguments".to_string(),
                    ));
                }
                if min > max {
                    return Some(Value::Error(
                        "random_int min must be less than or equal to max".to_string(),
                    ));
                }
                Value::Int(builtins::random_int(min, max) as i64)
            } else {
                Value::Error("random_int requires two number arguments: min and max".to_string())
            }
        }

        "random_choice" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "random_choice() expects 1 argument (array), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Array(arr)) = arg_values.first() {
                builtins::random_choice(arr)
            } else {
                Value::Error("random_choice requires an array argument".to_string())
            }
        }

        "uuid_v4" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "uuid_v4() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Str(Arc::new(builtins::uuid_v4()))
        }

        "random_id" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "random_id() expects 1 argument (length), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Int(length)) = arg_values.first() {
                if *length < 0 {
                    Value::Error("random_id length must be >= 0".to_string())
                } else if *length as usize > runtime_limits::MAX_GENERATED_STRING_CHARS {
                    Value::Error(format!(
                        "random_id length exceeds maximum generated string length {}",
                        runtime_limits::MAX_GENERATED_STRING_CHARS
                    ))
                } else {
                    Value::Str(Arc::new(builtins::random_id(*length as usize)))
                }
            } else {
                Value::Error("random_id requires an integer length argument".to_string())
            }
        }

        // Random seed control (for deterministic testing)
        "set_random_seed" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "set_random_seed() expects 1 argument (seed), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Int(seed)) = arg_values.first() {
                builtins::set_random_seed(*seed as u64);
                Value::Null
            } else if let Some(Value::Float(seed)) = arg_values.first() {
                builtins::set_random_seed(*seed as u64);
                Value::Null
            } else {
                Value::Error("set_random_seed requires a number argument".to_string())
            }
        }

        "clear_random_seed" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "clear_random_seed() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            builtins::clear_random_seed();
            Value::Null
        }

        // Date/Time functions
        "now" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "now() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Float(builtins::now())
        }

        "now_utc" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "now_utc() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Str(Arc::new(builtins::now_utc()))
        }

        "now_unix" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "now_unix() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Int(builtins::now_unix())
        }

        "current_timestamp" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "current_timestamp() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Int(builtins::current_timestamp())
        }

        "performance_now" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "performance_now() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Float(builtins::performance_now())
        }

        "time_us" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "time_us() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Float(builtins::time_us())
        }

        "time_ns" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "time_ns() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            Value::Float(builtins::time_ns())
        }

        "format_duration" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "format_duration() expects 1 argument (milliseconds), got {}",
                    arg_values.len()
                )));
            }

            if let Some(ms_val) = arg_values.first() {
                let ms = match ms_val {
                    Value::Int(n) => *n as f64,
                    Value::Float(n) => *n,
                    _ => {
                        return Some(Value::Error(
                            "format_duration requires a number argument".to_string(),
                        ))
                    }
                };
                Value::Str(Arc::new(builtins::format_duration(ms)))
            } else {
                Value::Error(
                    "format_duration requires a number argument (milliseconds)".to_string(),
                )
            }
        }

        "elapsed" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "elapsed() expects 2 arguments (start, end), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(start_val), Some(end_val)) = (arg_values.first(), arg_values.get(1)) {
                let start = match start_val {
                    Value::Int(n) => *n as f64,
                    Value::Float(n) => *n,
                    _ => {
                        return Some(Value::Error("elapsed requires number arguments".to_string()))
                    }
                };
                let end = match end_val {
                    Value::Int(n) => *n as f64,
                    Value::Float(n) => *n,
                    _ => {
                        return Some(Value::Error("elapsed requires number arguments".to_string()))
                    }
                };
                Value::Float(builtins::elapsed(start, end))
            } else {
                Value::Error("elapsed requires two number arguments: start and end".to_string())
            }
        }

        "format_date" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "format_date() expects 2 arguments (timestamp, format), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(ts_val), Some(Value::Str(format))) =
                (arg_values.first(), arg_values.get(1))
            {
                let timestamp = match ts_val {
                    Value::Int(n) => *n as f64,
                    Value::Float(n) => *n,
                    _ => {
                        return Some(Value::Error(
                            "format_date requires a number timestamp".to_string(),
                        ))
                    }
                };
                Value::Str(Arc::new(builtins::format_date(timestamp, format.as_ref())))
            } else {
                Value::Error(
                    "format_date requires timestamp (number) and format (string)".to_string(),
                )
            }
        }

        "parse_date" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "parse_date() expects 2 arguments (date, format), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Str(date_str)), Some(Value::Str(format))) =
                (arg_values.first(), arg_values.get(1))
            {
                match builtins::parse_date(date_str, format) {
                    Ok(timestamp) => Value::Float(timestamp),
                    Err(message) => {
                        Value::ErrorObject { message, stack: Vec::new(), line: None, cause: None }
                    }
                }
            } else {
                Value::Error("parse_date requires date string and format string".to_string())
            }
        }

        "kv_set" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "kv_set() expects 2 arguments (key, value), got {}",
                    arg_values.len()
                )));
            }

            if let (Some(Value::Str(key)), Some(Value::Str(value))) =
                (arg_values.first(), arg_values.get(1))
            {
                match builtins::kv_set(key.as_ref(), value.as_ref()) {
                    Ok(saved) => Value::Bool(saved),
                    Err(message) => Value::Error(message),
                }
            } else {
                Value::Error("kv_set requires key and value string arguments".to_string())
            }
        }

        "kv_get" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "kv_get() expects 1 argument (key), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(key)) = arg_values.first() {
                match builtins::kv_get(key.as_ref()) {
                    Ok(Some(value)) => Value::Str(Arc::new(value)),
                    Ok(None) => Value::Null,
                    Err(message) => Value::Error(message),
                }
            } else {
                Value::Error("kv_get requires a key string argument".to_string())
            }
        }

        // System operation functions
        "env" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "env() expects 1 argument (variable name), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(var_name)) = arg_values.first() {
                Value::Str(Arc::new(builtins::get_env(var_name.as_ref())))
            } else {
                Value::Error("env requires a string argument (variable name)".to_string())
            }
        }

        "env_or" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "env_or() expects 2 arguments (variable name, default value), got {}",
                    arg_values.len()
                )));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(var_name)), Some(Value::Str(default_value))) => Value::Str(
                    Arc::new(builtins::env_or(var_name.as_ref(), default_value.as_ref())),
                ),
                _ => Value::Error(
                    "env_or requires two string arguments (variable name, default value)"
                        .to_string(),
                ),
            }
        }

        "env_int" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "env_int() expects 1 argument (variable name), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(var_name)) = arg_values.first() {
                match builtins::env_int(var_name.as_ref()) {
                    Ok(value) => Value::Int(value),
                    Err(message) => {
                        Value::ErrorObject { message, stack: Vec::new(), line: None, cause: None }
                    }
                }
            } else {
                Value::Error("env_int requires a string argument (variable name)".to_string())
            }
        }

        "env_float" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "env_float() expects 1 argument (variable name), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(var_name)) = arg_values.first() {
                match builtins::env_float(var_name.as_ref()) {
                    Ok(value) => Value::Float(value),
                    Err(message) => {
                        Value::ErrorObject { message, stack: Vec::new(), line: None, cause: None }
                    }
                }
            } else {
                Value::Error("env_float requires a string argument (variable name)".to_string())
            }
        }

        "env_bool" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "env_bool() expects 1 argument (variable name), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(var_name)) = arg_values.first() {
                match builtins::env_bool(var_name.as_ref()) {
                    Ok(value) => Value::Bool(value),
                    Err(message) => {
                        Value::ErrorObject { message, stack: Vec::new(), line: None, cause: None }
                    }
                }
            } else {
                Value::Error("env_bool requires a string argument (variable name)".to_string())
            }
        }

        "env_required" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "env_required() expects 1 argument (variable name), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(var_name)) = arg_values.first() {
                match builtins::env_required(var_name.as_ref()) {
                    Ok(value) => Value::Str(Arc::new(value)),
                    Err(message) => {
                        Value::ErrorObject { message, stack: Vec::new(), line: None, cause: None }
                    }
                }
            } else {
                Value::Error("env_required requires a string argument (variable name)".to_string())
            }
        }

        "env_set" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(format!(
                    "env_set() expects 2 arguments (variable name, value), got {}",
                    arg_values.len()
                )));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(var_name)), Some(Value::Str(value))) => {
                    builtins::env_set(var_name.as_ref(), value.as_ref());
                    Value::Null
                }
                _ => Value::Error(
                    "env_set requires two string arguments (variable name, value)".to_string(),
                ),
            }
        }

        "env_list" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "env_list() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            let env_vars = builtins::env_list();
            let mut dict = DictMap::default();
            for (key, value) in env_vars {
                dict.insert(Arc::<str>::from(key), Value::Str(Arc::new(value)));
            }
            Value::Dict(Arc::new(dict))
        }

        "args" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "args() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            let args = builtins::get_args();
            let values: Vec<Value> =
                args.into_iter().map(|value| Value::Str(Arc::new(value))).collect();
            Value::Array(Arc::new(values))
        }

        "arg_parser" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "arg_parser() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            let mut fields: HashMap<String, Value> = HashMap::new();
            fields.insert("_args".to_string(), Value::Array(Arc::new(Vec::new())));
            fields.insert("_app_name".to_string(), Value::Str(Arc::new(String::new())));
            fields.insert("_description".to_string(), Value::Str(Arc::new(String::new())));
            Value::Struct { name: "ArgParser".to_string(), fields }
        }

        "input" => {
            if arg_values.len() > 1 {
                return Some(Value::Error("input() expects 0-1 arguments".to_string()));
            }

            let prompt = match arg_values.first() {
                Some(Value::Str(value)) => value.to_string(),
                Some(_) => {
                    return Some(Value::Error(
                        "input() requires a string prompt when provided".to_string(),
                    ))
                }
                None => String::new(),
            };

            print!("{}", prompt);
            let _ = std::io::stdout().flush();

            let mut input = String::new();
            match std::io::stdin().read_line(&mut input) {
                Ok(_) => Value::Str(Arc::new(input.trim_end().to_string())),
                Err(_) => Value::Str(Arc::new(String::new())),
            }
        }

        "exit" => {
            if arg_values.len() > 1 {
                return Some(Value::Error("exit() expects 0-1 arguments".to_string()));
            }

            let exit_code = match arg_values.first() {
                Some(Value::Int(code)) => *code as i32,
                Some(Value::Float(code)) => *code as i32,
                Some(_) => {
                    return Some(Value::Error(
                        "exit() requires a numeric exit code when provided".to_string(),
                    ))
                }
                None => 0,
            };

            std::process::exit(exit_code);
        }

        "sleep" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("sleep() expects 1 argument".to_string()));
            }

            let millis = match arg_values.first() {
                Some(Value::Int(value)) => *value as f64,
                Some(Value::Float(value)) => *value,
                _ => return Some(Value::Error("sleep() requires a number argument".to_string())),
            };

            if millis < 0.0 {
                return Some(Value::Error(
                    "sleep() requires non-negative milliseconds".to_string(),
                ));
            }

            builtins::sleep_ms(millis);
            Value::Int(0)
        }

        "execute" => {
            if arg_values.is_empty() || arg_values.len() > 2 {
                return Some(Value::Error(
                    "execute() expects 1-2 arguments (command, [options])".to_string(),
                ));
            }

            let Some(Value::Str(command)) = arg_values.first() else {
                return Some(Value::Error("execute() requires a string command".to_string()));
            };

            let options = match parse_process_options(arg_values.get(1)) {
                Ok(options) => options,
                Err(error) => return Some(error),
            };

            let command_text = command.as_ref().to_string();
            if let Err(error) = validate_shell_command_text(&command_text, "execute()") {
                return Some(error);
            }
            let process = if cfg!(target_os = "windows") {
                let mut cmd = Command::new("cmd");
                cmd.args(["/C", command_text.as_str()]);
                cmd
            } else {
                let mut cmd = Command::new("sh");
                cmd.arg("-c").arg(command_text.as_str());
                cmd
            };

            let result =
                match run_command_with_options(process, &options, None, command_text.as_str()) {
                    Ok(result) => result,
                    Err(error) => return Some(error),
                };

            if result.timed_out {
                return Some(error_object(format!(
                    "execute() timed out after {}ms",
                    options.timeout_ms
                )));
            }

            if result.stdout_truncated || result.stderr_truncated {
                return Some(error_object(format!(
                    "execute() output exceeded max_output_bytes ({})",
                    options.max_output_bytes
                )));
            }

            if !result.success {
                return Some(error_object(format!(
                    "execute() command failed with exit code {}: {}",
                    result.exitcode,
                    String::from_utf8_lossy(&result.stderr)
                )));
            }

            Value::Str(Arc::new(String::from_utf8_lossy(&result.stdout).to_string()))
        }

        "execute_status" => {
            if arg_values.is_empty() || arg_values.len() > 2 {
                return Some(Value::Error(
                    "execute_status() expects 1-2 arguments (command, [options])".to_string(),
                ));
            }

            let Some(Value::Str(command)) = arg_values.first() else {
                return Some(Value::Error(
                    "execute_status() requires a string command".to_string(),
                ));
            };

            let options = match parse_process_options(arg_values.get(1)) {
                Ok(options) => options,
                Err(error) => return Some(error),
            };

            let command_text = command.as_ref().to_string();
            if let Err(error) = validate_shell_command_text(&command_text, "execute_status()") {
                return Some(error);
            }
            let process = if cfg!(target_os = "windows") {
                let mut cmd = Command::new("cmd");
                cmd.args(["/C", command_text.as_str()]);
                cmd
            } else {
                let mut cmd = Command::new("sh");
                cmd.arg("-c").arg(command_text.as_str());
                cmd
            };

            match run_command_with_options(process, &options, None, command_text.as_str()) {
                Ok(result) => process_result_to_value(result),
                Err(error) => error,
            }
        }

        "spawn_process" => {
            if arg_values.is_empty() || arg_values.len() > 2 {
                return Some(Value::Error(
                    "spawn_process requires an array of command arguments and optional options"
                        .to_string(),
                ));
            }

            let options = match parse_process_options(arg_values.get(1)) {
                Ok(options) => options,
                Err(error) => return Some(error),
            };

            if let Some(Value::Array(args)) = arg_values.first() {
                if args.is_empty() {
                    return Some(Value::Error(
                        "spawn_process requires a non-empty array of command arguments".to_string(),
                    ));
                }

                let mut cmd_parts: Vec<String> = Vec::new();
                for arg in args.iter() {
                    if let Value::Str(s) = arg {
                        cmd_parts.push(s.as_ref().clone());
                    } else {
                        return Some(Value::Error(
                            "spawn_process requires an array of strings".to_string(),
                        ));
                    }
                }

                let program = &cmd_parts[0];
                let args_slice = &cmd_parts[1..];
                let mut command = Command::new(program);
                command.args(args_slice);
                let label = render_command_for_error(program, args_slice);

                match run_command_with_options(command, &options, None, label.as_str()) {
                    Ok(result) => process_result_to_value(result),
                    Err(error) => error,
                }
            } else {
                Value::Error("spawn_process requires an array of command arguments".to_string())
            }
        }

        "pipe_commands" => {
            if arg_values.is_empty() || arg_values.len() > 2 {
                return Some(Value::Error(
                    "pipe_commands requires an array of command arrays and optional options"
                        .to_string(),
                ));
            }

            let options = match parse_process_options(arg_values.get(1)) {
                Ok(options) => options,
                Err(error) => return Some(error),
            };

            if let Some(Value::Array(commands)) = arg_values.first() {
                if commands.is_empty() {
                    return Some(Value::Error(
                        "pipe_commands requires a non-empty array of commands".to_string(),
                    ));
                }

                let mut parsed_commands: Vec<Vec<String>> = Vec::new();
                for cmd in commands.iter() {
                    if let Value::Array(args) = cmd {
                        let mut cmd_parts: Vec<String> = Vec::new();
                        for arg in args.iter() {
                            if let Value::Str(s) = arg {
                                cmd_parts.push(s.as_ref().clone());
                            } else {
                                return Some(Value::Error(
                                    "Each command must be an array of strings".to_string(),
                                ));
                            }
                        }

                        if cmd_parts.is_empty() {
                            return Some(Value::Error(
                                "Each command array must not be empty".to_string(),
                            ));
                        }

                        parsed_commands.push(cmd_parts);
                    } else {
                        return Some(Value::Error(
                            "pipe_commands requires an array of command arrays".to_string(),
                        ));
                    }
                }

                let mut previous_output: Option<Vec<u8>> = None;

                for cmd_parts in parsed_commands.iter() {
                    let program = &cmd_parts[0];
                    let args = &cmd_parts[1..];

                    let mut command = Command::new(program);
                    command.args(args);
                    let label = render_command_for_error(program, args);

                    let run_result = match run_command_with_options(
                        command,
                        &options,
                        previous_output.take(),
                        label.as_str(),
                    ) {
                        Ok(result) => result,
                        Err(error) => return Some(error),
                    };

                    if run_result.timed_out {
                        return Some(error_object(format!(
                            "Command '{}' timed out after {}ms",
                            cmd_parts.join(" "),
                            options.timeout_ms
                        )));
                    }

                    if run_result.stdout_truncated || run_result.stderr_truncated {
                        return Some(error_object(format!(
                            "Command '{}' output exceeded max_output_bytes ({})",
                            cmd_parts.join(" "),
                            options.max_output_bytes
                        )));
                    }

                    if !run_result.success {
                        return Some(error_object(format!(
                            "Command '{}' failed with exit code {}: {}",
                            cmd_parts.join(" "),
                            run_result.exitcode,
                            String::from_utf8_lossy(&run_result.stderr)
                        )));
                    }

                    previous_output = Some(run_result.stdout);
                }

                if let Some(output) = previous_output {
                    match String::from_utf8(output) {
                        Ok(result) => Value::Str(Arc::new(result)),
                        Err(e) => Value::ErrorObject {
                            message: format!("Failed to decode output as UTF-8: {}", e),
                            stack: Vec::new(),
                            line: None,
                            cause: None,
                        },
                    }
                } else {
                    Value::Str(Arc::new(String::new()))
                }
            } else {
                Value::Error("pipe_commands requires an array of command arrays".to_string())
            }
        }

        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::{handle, StreamingRedactor};
    use crate::interpreter::{DictMap, Value};
    use crate::runtime_limits;
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::{Duration, Instant};

    fn string_value(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    #[test]
    fn test_env_set_and_env_round_trip() {
        let key = "KUJO_NATIVE_SYSTEM_ENV_TEST";
        let value = "hardening_round_trip";

        let set_result = handle("env_set", &[string_value(key), string_value(value)]).unwrap();
        assert!(matches!(set_result, Value::Null));

        let get_result = handle("env", &[string_value(key)]).unwrap();
        match get_result {
            Value::Str(actual) => assert_eq!(actual.as_ref(), value),
            other => panic!("Expected Value::Str from env(), got {:?}", other),
        }
    }

    #[test]
    fn test_env_int_missing_variable_returns_error_object() {
        let missing_key = "KUJO_NATIVE_SYSTEM_MISSING_INT_TEST";
        std::env::remove_var(missing_key);

        let result = handle("env_int", &[string_value(missing_key)]).unwrap();
        match result {
            Value::ErrorObject { message, .. } => {
                assert!(message.contains("not found"));
            }
            other => panic!("Expected Value::ErrorObject from env_int(), got {:?}", other),
        }
    }

    #[test]
    fn test_env_bool_invalid_value_returns_error_object() {
        let key = "KUJO_NATIVE_SYSTEM_ENV_BOOL_INVALID_TEST";
        std::env::set_var(key, "definitely-not-bool");

        let result = handle("env_bool", &[string_value(key)]).unwrap();
        match result {
            Value::ErrorObject { message, .. } => {
                assert!(message.contains("not a valid bool"));
            }
            other => panic!("Expected Value::ErrorObject from env_bool(), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_date_invalid_input_returns_error_object() {
        let result =
            handle("parse_date", &[string_value("2026/05/16"), string_value("YYYY-MM-DD")])
                .unwrap();
        match result {
            Value::ErrorObject { message, .. } => {
                assert!(message.contains("parse_date failed"));
            }
            other => panic!("Expected Value::ErrorObject from parse_date(), got {:?}", other),
        }
    }

    #[test]
    fn test_args_returns_array() {
        let result = handle("args", &[]).unwrap();
        assert!(matches!(result, Value::Array(_)));
    }

    #[test]
    fn test_arg_parser_returns_struct_shape() {
        let result = handle("arg_parser", &[]).unwrap();

        match result {
            Value::Struct { name, fields } => {
                assert_eq!(name, "ArgParser");
                assert!(fields.contains_key("_args"));
                assert!(fields.contains_key("_app_name"));
                assert!(fields.contains_key("_description"));
            }
            other => panic!("Expected Value::Struct from arg_parser(), got {:?}", other),
        }
    }

    #[test]
    fn test_uuid_random_id_and_time_helpers() {
        let uuid = handle("uuid_v4", &[]).unwrap();
        assert!(
            matches!(uuid, Value::Str(value) if value.len() == 36 && value.chars().nth(14) == Some('4'))
        );

        let random_id = handle("random_id", &[Value::Int(12)]).unwrap();
        assert!(matches!(random_id, Value::Str(value) if value.len() == 12));

        let now_utc = handle("now_utc", &[]).unwrap();
        assert!(
            matches!(now_utc, Value::Str(value) if value.ends_with('Z') && value.contains('T'))
        );

        let now_unix = handle("now_unix", &[]).unwrap();
        assert!(matches!(now_unix, Value::Int(value) if value > 0));
    }

    #[test]
    fn test_kv_set_and_get_round_trip() {
        let kv_path = std::env::temp_dir().join(format!(
            "kujo_native_kv_store_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after UNIX_EPOCH")
                .as_nanos()
        ));

        std::env::set_var("KUJO_KV_PATH", kv_path.to_string_lossy().to_string());

        let set_result = handle("kv_set", &[string_value("device:android"), string_value("ok")])
            .expect("kv_set should return a value");
        assert!(matches!(set_result, Value::Bool(true)));

        let get_result = handle("kv_get", &[string_value("device:android")])
            .expect("kv_get should return a value");
        assert!(matches!(get_result, Value::Str(value) if value.as_ref() == "ok"));

        let missing_result =
            handle("kv_get", &[string_value("device:missing")]).expect("kv_get should return");
        assert!(matches!(missing_result, Value::Null));

        std::env::remove_var("KUJO_KV_PATH");
        let _ = std::fs::remove_file(kv_path);
    }

    #[test]
    fn test_random_time_env_and_args_api_strict_arity_rejects_extra_arguments() {
        let random_extra = handle("random", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            random_extra,
            Value::Error(message) if message.contains("random() expects 0 arguments")
        ));

        let random_int_extra =
            handle("random_int", &[Value::Int(1), Value::Int(2), Value::Int(3)]).unwrap();
        assert!(matches!(
            random_int_extra,
            Value::Error(message) if message.contains("random_int() expects 2 arguments")
        ));

        let uuid_extra = handle("uuid_v4", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            uuid_extra,
            Value::Error(message) if message.contains("uuid_v4() expects 0 arguments")
        ));

        let random_id_extra = handle("random_id", &[Value::Int(8), Value::Int(1)]).unwrap();
        assert!(matches!(
            random_id_extra,
            Value::Error(message) if message.contains("random_id() expects 1 argument")
        ));

        let now_utc_extra = handle("now_utc", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            now_utc_extra,
            Value::Error(message) if message.contains("now_utc() expects 0 arguments")
        ));

        let now_unix_extra = handle("now_unix", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            now_unix_extra,
            Value::Error(message) if message.contains("now_unix() expects 0 arguments")
        ));

        let kv_set_extra =
            handle("kv_set", &[string_value("k"), string_value("v"), Value::Int(1)]).unwrap();
        assert!(matches!(
            kv_set_extra,
            Value::Error(message) if message.contains("kv_set() expects 2 arguments")
        ));

        let kv_get_extra = handle("kv_get", &[string_value("k"), Value::Int(1)]).unwrap();
        assert!(matches!(
            kv_get_extra,
            Value::Error(message) if message.contains("kv_get() expects 1 argument")
        ));

        let format_duration_extra =
            handle("format_duration", &[Value::Int(10), Value::Int(1)]).unwrap();
        assert!(matches!(
            format_duration_extra,
            Value::Error(message) if message.contains("format_duration() expects 1 argument")
        ));

        let parse_date_extra = handle(
            "parse_date",
            &[string_value("1970-01-01"), string_value("YYYY-MM-DD"), Value::Int(1)],
        )
        .unwrap();
        assert!(matches!(
            parse_date_extra,
            Value::Error(message) if message.contains("parse_date() expects 2 arguments")
        ));

        let env_extra = handle("env", &[string_value("PATH"), Value::Int(1)]).unwrap();
        assert!(matches!(
            env_extra,
            Value::Error(message) if message.contains("env() expects 1 argument")
        ));

        let env_set_extra =
            handle("env_set", &[string_value("A"), string_value("B"), Value::Int(1)]).unwrap();
        assert!(matches!(
            env_set_extra,
            Value::Error(message) if message.contains("env_set() expects 2 arguments")
        ));

        let env_list_extra = handle("env_list", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            env_list_extra,
            Value::Error(message) if message.contains("env_list() expects 0 arguments")
        ));

        let args_extra = handle("args", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            args_extra,
            Value::Error(message) if message.contains("args() expects 0 arguments")
        ));

        let arg_parser_extra = handle("arg_parser", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            arg_parser_extra,
            Value::Error(message) if message.contains("arg_parser() expects 0 arguments")
        ));
    }

    #[test]
    fn test_random_helpers_reject_invalid_bounds_without_panicking() {
        let reversed = handle("random_int", &[Value::Int(10), Value::Int(1)]).unwrap();
        assert!(
            matches!(reversed, Value::Error(message) if message.contains("min must be less than or equal to max"))
        );

        let non_finite = handle("random_int", &[Value::Float(f64::NAN), Value::Int(1)]).unwrap();
        assert!(matches!(non_finite, Value::Error(message) if message.contains("finite number")));

        let too_large = handle(
            "random_id",
            &[Value::Int(runtime_limits::MAX_GENERATED_STRING_CHARS as i64 + 1)],
        )
        .unwrap();
        assert!(
            matches!(too_large, Value::Error(message) if message.contains("random_id length exceeds maximum generated string length"))
        );
    }

    #[test]
    fn test_spawn_process_rejects_invalid_argument_shape() {
        let non_array_result = handle("spawn_process", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            non_array_result,
            Value::Error(message) if message == "spawn_process requires an array of command arguments"
        ));

        let empty_args_result = handle("spawn_process", &[Value::Array(Arc::new(vec![]))]).unwrap();
        assert!(matches!(
            empty_args_result,
            Value::Error(message) if message == "spawn_process requires a non-empty array of command arguments"
        ));

        let invalid_elem_result =
            handle("spawn_process", &[Value::Array(Arc::new(vec![Value::Int(1)]))]).unwrap();
        assert!(matches!(
            invalid_elem_result,
            Value::Error(message) if message == "spawn_process requires an array of strings"
        ));
    }

    #[test]
    fn test_spawn_process_returns_process_result_struct() {
        let exe_path = std::env::current_exe().expect("current exe path should be available");
        let args = vec![
            Value::Str(Arc::new(exe_path.to_string_lossy().to_string())),
            Value::Str(Arc::new("--help".to_string())),
        ];

        let result = handle("spawn_process", &[Value::Array(Arc::new(args))]).unwrap();
        match result {
            Value::Struct { name, fields } => {
                assert_eq!(name, "ProcessResult");
                assert!(matches!(fields.get("exitcode"), Some(Value::Int(_))));
                assert!(matches!(fields.get("stdout"), Some(Value::Str(_))));
                assert!(matches!(fields.get("stderr"), Some(Value::Str(_))));
                assert!(matches!(fields.get("success"), Some(Value::Bool(true))));
                assert!(matches!(fields.get("timed_out"), Some(Value::Bool(false))));
                assert!(matches!(fields.get("cancelled"), Some(Value::Bool(false))));
                assert!(matches!(fields.get("stdout_truncated"), Some(Value::Bool(false))));
                assert!(matches!(fields.get("stderr_truncated"), Some(Value::Bool(false))));
            }
            other => panic!("Expected ProcessResult struct from spawn_process, got {:?}", other),
        }
    }

    #[test]
    fn test_streaming_redactor_preserves_utf8_and_redacts_chunk_boundary() {
        let mut redactor = StreamingRedactor::new(&["秘密".to_string()]);
        let mut output = redactor.push("prefix-秘".as_bytes());
        output.extend(redactor.push("密-suffix-😀".as_bytes()));
        output.extend(redactor.finish());
        assert_eq!(String::from_utf8(output).unwrap(), "prefix-[REDACTED]-suffix-😀");
    }

    #[cfg(unix)]
    #[test]
    fn test_spawn_process_streams_redacted_events_with_backpressure() {
        let (sender, receiver) = mpsc::sync_channel(2);
        let channel = Value::Channel(Arc::new(Mutex::new((sender, receiver))));
        let mut options = DictMap::default();
        options.insert(Arc::from("stream_channel"), channel.clone());
        options.insert(
            Arc::from("redact_values"),
            Value::Array(Arc::new(vec![string_value("secret")])),
        );
        let args = vec![
            string_value("sh"),
            string_value("-c"),
            string_value("printf 'token-secret\\n'; printf 'err-secret\\n' >&2"),
        ];
        let worker = std::thread::spawn(move || {
            handle("spawn_process", &[Value::Array(Arc::new(args)), Value::Dict(Arc::new(options))])
                .expect("spawn_process should return a result")
        });

        let receiver_channel = match &channel {
            Value::Channel(channel) => channel.clone(),
            _ => unreachable!(),
        };
        let mut events = Vec::new();
        let mut eof_streams = 0;
        let deadline = Instant::now() + Duration::from_secs(2);
        while eof_streams < 2 {
            let event = loop {
                let received = receiver_channel.lock().unwrap().1.try_recv();
                match received {
                    Ok(event) => break event,
                    Err(mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(mpsc::TryRecvError::Empty) => panic!("stream event should arrive"),
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("stream channel should remain connected")
                    }
                }
            };
            if matches!(
                &event,
                Value::Struct { fields, .. }
                    if matches!(fields.get("eof"), Some(Value::Bool(true)))
            ) {
                eof_streams += 1;
            }
            events.push(event);
        }
        let result = worker.join().expect("process worker should join");
        if let Value::Struct { fields, .. } = result {
            assert!(matches!(fields.get("success"), Some(Value::Bool(true))));
            assert!(matches!(
                fields.get("stdout"),
                Some(Value::Str(value)) if value.as_ref() == "token-[REDACTED]\n"
            ));
            assert!(matches!(
                fields.get("stderr"),
                Some(Value::Str(value)) if value.as_ref() == "err-[REDACTED]\n"
            ));
        } else {
            panic!("expected ProcessResult");
        }
        assert!(events.iter().any(|event| match event {
            Value::Struct { fields, .. } => fields.get("data").is_some_and(|data| {
                matches!(data, Value::Str(value) if value.contains("[REDACTED]"))
            }),
            _ => false,
        }));
        assert!(events.iter().any(|event| matches!(
            event,
            Value::Struct { fields, .. }
                if matches!(fields.get("eof"), Some(Value::Bool(true)))
        )));
    }

    #[cfg(unix)]
    #[test]
    fn test_spawn_process_cancel_file_terminates_child_and_reports_cancellation() {
        let cancel_path =
            std::env::temp_dir().join(format!("kujo-process-cancel-{}", std::process::id()));
        let _ = std::fs::remove_file(&cancel_path);
        let mut options = DictMap::default();
        options
            .insert(Arc::from("cancel_file"), string_value(cancel_path.to_string_lossy().as_ref()));
        let args = vec![string_value("sh"), string_value("-c"), string_value("sleep 5")];
        let started = std::time::Instant::now();
        let worker = std::thread::spawn(move || {
            handle("spawn_process", &[Value::Array(Arc::new(args)), Value::Dict(Arc::new(options))])
                .expect("spawn_process should return a result")
        });
        std::thread::sleep(Duration::from_millis(50));
        std::fs::write(&cancel_path, "cancel").expect("cancel marker should be writable");
        let result = worker.join().expect("cancelled process worker should join");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "descendant process kept cancellation pipes open"
        );
        let _ = std::fs::remove_file(cancel_path);
        match result {
            Value::Struct { fields, .. } => {
                assert!(matches!(fields.get("cancelled"), Some(Value::Bool(true))));
                assert!(matches!(fields.get("timed_out"), Some(Value::Bool(false))));
                assert!(matches!(fields.get("success"), Some(Value::Bool(false))));
            }
            other => panic!("expected ProcessResult, got {:?}", other),
        }
    }

    #[test]
    fn test_execute_status_returns_process_result_struct() {
        let result = handle("execute_status", &[string_value("echo kujo")]).unwrap();
        match result {
            Value::Struct { name, fields } => {
                assert_eq!(name, "ProcessResult");
                assert!(matches!(fields.get("exitcode"), Some(Value::Int(_))));
                assert!(matches!(fields.get("stdout"), Some(Value::Str(_))));
                assert!(matches!(fields.get("stderr"), Some(Value::Str(_))));
                assert!(matches!(fields.get("success"), Some(Value::Bool(_))));
                assert!(matches!(fields.get("timed_out"), Some(Value::Bool(_))));
                assert!(matches!(fields.get("cancelled"), Some(Value::Bool(false))));
                assert!(matches!(fields.get("stdout_truncated"), Some(Value::Bool(_))));
                assert!(matches!(fields.get("stderr_truncated"), Some(Value::Bool(_))));
            }
            other => panic!("Expected ProcessResult struct from execute_status, got {:?}", other),
        }
    }

    #[test]
    fn test_pipe_commands_rejects_invalid_argument_shape() {
        let non_array_result = handle("pipe_commands", &[Value::Int(1)]).unwrap();
        assert!(matches!(
            non_array_result,
            Value::Error(message) if message == "pipe_commands requires an array of command arrays"
        ));

        let empty_commands_result =
            handle("pipe_commands", &[Value::Array(Arc::new(vec![]))]).unwrap();
        assert!(matches!(
            empty_commands_result,
            Value::Error(message) if message == "pipe_commands requires a non-empty array of commands"
        ));

        let invalid_command_shape = handle(
            "pipe_commands",
            &[Value::Array(Arc::new(vec![Value::Str(Arc::new("echo".to_string()))]))],
        )
        .unwrap();
        assert!(matches!(
            invalid_command_shape,
            Value::Error(message) if message == "pipe_commands requires an array of command arrays"
        ));

        let invalid_command_arg = handle(
            "pipe_commands",
            &[Value::Array(Arc::new(vec![Value::Array(Arc::new(vec![Value::Int(1)]))]))],
        )
        .unwrap();
        assert!(matches!(
            invalid_command_arg,
            Value::Error(message) if message == "Each command must be an array of strings"
        ));
    }

    #[test]
    fn test_pipe_commands_returns_string_output_for_single_command_pipeline() {
        let exe_path = std::env::current_exe().expect("current exe path should be available");
        let command = Value::Array(Arc::new(vec![
            Value::Str(Arc::new(exe_path.to_string_lossy().to_string())),
            Value::Str(Arc::new("--help".to_string())),
        ]));

        let result = handle("pipe_commands", &[Value::Array(Arc::new(vec![command]))]).unwrap();
        assert!(matches!(result, Value::Str(_)));
    }

    #[test]
    fn test_process_api_strict_arity_rejects_extra_arguments() {
        let spawn_extra = handle(
            "spawn_process",
            &[
                Value::Array(Arc::new(vec![Value::Str(Arc::new("echo".to_string()))])),
                Value::Dict(Arc::new(DictMap::default())),
                Value::Int(1),
            ],
        )
        .unwrap();
        assert!(matches!(
            spawn_extra,
            Value::Error(message)
                if message
                    == "spawn_process requires an array of command arguments and optional options"
        ));

        let pipe_extra = handle(
            "pipe_commands",
            &[
                Value::Array(Arc::new(vec![Value::Array(Arc::new(vec![Value::Str(Arc::new(
                    "echo".to_string(),
                ))]))])),
                Value::Dict(Arc::new(DictMap::default())),
                Value::Int(1),
            ],
        )
        .unwrap();
        assert!(matches!(
            pipe_extra,
            Value::Error(message)
                if message
                    == "pipe_commands requires an array of command arrays and optional options"
        ));

        let execute_status_extra =
            handle("execute_status", &[string_value("echo ok"), Value::Int(1), Value::Int(2)])
                .unwrap();
        assert!(matches!(
            execute_status_extra,
            Value::Error(message)
                if message == "execute_status() expects 1-2 arguments (command, [options])"
        ));
    }
}
