// File: src/interpreter/native_functions/filesystem.rs
//
// Filesystem operation native functions

use crate::interpreter::{AsyncRuntime, Interpreter, Value};
#[cfg(feature = "runtime-archive")]
use crate::path_security;
use crate::runtime_limits;
use crate::{builtins, interpreter::DictMap};
use std::fs;
#[cfg(feature = "runtime-archive")]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
#[cfg(feature = "runtime-archive")]
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
#[cfg(feature = "runtime-archive")]
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

const ZIP_UNIX_FILE_TYPE_MASK: u32 = 0o170000;
const ZIP_UNIX_SYMLINK_FILE_TYPE: u32 = 0o120000;
const MAX_FILE_READ_BYTES: u64 = runtime_limits::MAX_FILE_IO_BYTES as u64;
const MAX_FILE_WRITE_BYTES: usize = runtime_limits::MAX_FILE_IO_BYTES;

#[derive(Clone, Copy)]
struct ZipExtractionLimits {
    max_entries: usize,
    max_total_uncompressed_bytes: u64,
    max_single_entry_uncompressed_bytes: u64,
}

impl ZipExtractionLimits {
    const DEFAULT: Self = Self {
        max_entries: 1024,
        max_total_uncompressed_bytes: 64 * 1024 * 1024,
        max_single_entry_uncompressed_bytes: 16 * 1024 * 1024,
    };
}

fn validate_read_size_limit(path: &str) -> Result<(), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Cannot read file '{}': {}", path, error))?;
    let file_size = metadata.len();
    if file_size > MAX_FILE_READ_BYTES {
        return Err(format!(
            "Cannot read file '{}': exceeds maximum read size ({} bytes > {} bytes)",
            path, file_size, MAX_FILE_READ_BYTES
        ));
    }

    Ok(())
}

fn validate_write_size_limit(path: &str, payload_size: usize) -> Result<(), String> {
    if payload_size > MAX_FILE_WRITE_BYTES {
        return Err(format!(
            "Cannot write file '{}': payload exceeds maximum write size ({} bytes > {} bytes)",
            path, payload_size, MAX_FILE_WRITE_BYTES
        ));
    }

    Ok(())
}

fn validate_append_size_limit(path: &str, payload_size: usize) -> Result<(), String> {
    if payload_size > MAX_FILE_WRITE_BYTES {
        return Err(format!(
            "Cannot append to file '{}': payload exceeds maximum write size ({} bytes > {} bytes)",
            path, payload_size, MAX_FILE_WRITE_BYTES
        ));
    }

    Ok(())
}

fn parse_overwrite_flag(function_name: &str, arg_values: &[Value]) -> Result<bool, Value> {
    match arg_values.len() {
        2 => Ok(false),
        3 => match arg_values.get(2) {
            Some(Value::Bool(flag)) => Ok(*flag),
            _ => Err(Value::Error(format!(
                "{} optional overwrite flag must be a bool",
                function_name
            ))),
        },
        _ => Err(Value::Error(format!(
            "{} requires 2 or 3 arguments: path, content/bytes, [overwrite]",
            function_name
        ))),
    }
}

fn enforce_overwrite_policy(path: &str, overwrite: bool) -> Result<(), String> {
    if !overwrite && Path::new(path).exists() {
        return Err(format!(
            "Cannot write file '{}': file already exists (pass overwrite=true to replace it)",
            path
        ));
    }

    Ok(())
}

#[cfg(feature = "runtime-archive")]
fn zip_add_dir_recursive(
    zip_writer: &mut ZipWriter<File>,
    directory_path: &Path,
    zip_prefix: &str,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory_path)
        .map_err(|error| format!("Failed to read directory: {}", error))?;

    for entry in entries {
        let entry = entry.map_err(|error| format!("Failed to read entry: {}", error))?;
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let zip_path = if zip_prefix.is_empty() {
            file_name.to_string()
        } else {
            format!("{}/{}", zip_prefix, file_name)
        };

        if entry_path.is_dir() {
            let options = SimpleFileOptions::default();
            zip_writer
                .add_directory(&zip_path, options)
                .map_err(|error| format!("Failed to add directory '{}': {}", zip_path, error))?;
            zip_add_dir_recursive(zip_writer, &entry_path, &zip_path)?;
        } else {
            let file_contents = std::fs::read(&entry_path)
                .map_err(|error| format!("Failed to read '{}': {}", entry_path.display(), error))?;
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip_writer
                .start_file(&zip_path, options)
                .map_err(|error| format!("Failed to start file '{}': {}", zip_path, error))?;
            zip_writer
                .write_all(&file_contents)
                .map_err(|error| format!("Failed to write file '{}': {}", zip_path, error))?;
        }
    }

    Ok(())
}

#[cfg(feature = "runtime-archive")]
fn sanitize_archive_entry_path(raw_name: &str) -> Result<PathBuf, String> {
    path_security::sanitize_relative_path(raw_name, "archive entry")
}

#[cfg(feature = "runtime-archive")]
fn archive_entry_is_symlink<R: std::io::Read>(entry: &zip::read::ZipFile<'_, R>) -> bool {
    entry
        .unix_mode()
        .map(|mode| (mode & ZIP_UNIX_FILE_TYPE_MASK) == ZIP_UNIX_SYMLINK_FILE_TYPE)
        .unwrap_or(false)
}

#[cfg(feature = "runtime-archive")]
fn resolve_extraction_output_path(
    output_root: &Path,
    relative_entry_path: &Path,
    entry_name: &str,
) -> Result<PathBuf, String> {
    path_security::join_within_root(output_root, relative_entry_path, "archive entry").map_err(
        |_| {
            format!(
                "Unsafe archive entry '{}': extraction path escapes output directory",
                entry_name
            )
        },
    )
}

#[cfg(feature = "runtime-archive")]
fn ensure_canonical_path_within_root(
    path: &Path,
    canonical_output_root: &Path,
    entry_name: &str,
) -> Result<(), String> {
    match path_security::ensure_canonical_path_within_root(
        path,
        canonical_output_root,
        "extraction path",
    ) {
        Ok(()) => Ok(()),
        Err(error) => {
            if error.starts_with("Failed to resolve") {
                return Err(format!(
                    "Failed to resolve extraction path for '{}': {}",
                    entry_name, error
                ));
            }

            Err(format!(
                "Unsafe archive entry '{}': extraction path escapes output directory",
                entry_name
            ))
        }
    }
}

#[cfg(feature = "runtime-archive")]
fn reject_symlink_target_path(path: &Path, entry_name: &str) -> Result<(), String> {
    path_security::reject_symlink_target_path(path, "archive entry target path").map_err(|_| {
        format!(
            "Unsafe archive entry '{}': symbolic link target path '{}' is not allowed",
            entry_name,
            path.display()
        )
    })
}

#[cfg(feature = "runtime-archive")]
fn extract_zip_archive_with_limits(
    archive: &mut ZipArchive<File>,
    output_root: &Path,
    limits: ZipExtractionLimits,
) -> Result<Vec<Value>, String> {
    if archive.len() > limits.max_entries {
        return Err(format!(
            "Archive contains {} entries which exceeds maximum entry count ({})",
            archive.len(),
            limits.max_entries
        ));
    }

    std::fs::create_dir_all(output_root).map_err(|error| {
        format!("Failed to create output directory '{}': {}", output_root.display(), error)
    })?;

    let canonical_output_root = path_security::canonicalize_root(output_root, "output directory")?;

    let mut extracted_files = Vec::new();
    let mut total_uncompressed_bytes = 0_u64;

    for entry_index in 0..archive.len() {
        let mut archive_file = archive
            .by_index(entry_index)
            .map_err(|error| format!("Failed to read zip entry {}: {}", entry_index, error))?;
        let entry_name = archive_file.name().to_string();

        if archive_entry_is_symlink(&archive_file) {
            return Err(format!(
                "Unsafe archive entry '{}': symbolic links are not allowed",
                entry_name
            ));
        }

        let relative_entry_path = sanitize_archive_entry_path(&entry_name)?;
        let entry_size = archive_file.size();

        if entry_size > limits.max_single_entry_uncompressed_bytes {
            return Err(format!(
                "Archive entry '{}' exceeds maximum per-entry size ({} bytes > {} bytes)",
                entry_name, entry_size, limits.max_single_entry_uncompressed_bytes
            ));
        }

        total_uncompressed_bytes =
            total_uncompressed_bytes.checked_add(entry_size).ok_or_else(|| {
                format!("Archive extraction size overflow while processing entry '{}'", entry_name)
            })?;

        if total_uncompressed_bytes > limits.max_total_uncompressed_bytes {
            return Err(format!(
                "Archive extraction exceeds maximum total extraction size ({} bytes > {} bytes)",
                total_uncompressed_bytes, limits.max_total_uncompressed_bytes
            ));
        }

        let output_path =
            resolve_extraction_output_path(output_root, &relative_entry_path, &entry_name)?;

        if archive_file.is_dir() {
            reject_symlink_target_path(&output_path, &entry_name)?;
            std::fs::create_dir_all(&output_path).map_err(|error| {
                format!("Failed to create directory '{}': {}", output_path.display(), error)
            })?;
            ensure_canonical_path_within_root(&output_path, &canonical_output_root, &entry_name)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Failed to create parent directory for '{}': {}",
                    output_path.display(),
                    error
                )
            })?;
            ensure_canonical_path_within_root(parent, &canonical_output_root, &entry_name)?;
        }

        reject_symlink_target_path(&output_path, &entry_name)?;

        let mut output_file = File::create(&output_path).map_err(|error| {
            format!("Failed to create output file '{}': {}", output_path.display(), error)
        })?;

        std::io::copy(&mut archive_file, &mut output_file)
            .map_err(|error| format!("Failed to extract file '{}': {}", entry_name, error))?;

        ensure_canonical_path_within_root(&output_path, &canonical_output_root, &entry_name)?;
        extracted_files.push(Value::Str(Arc::new(output_path.to_string_lossy().to_string())));
    }

    Ok(extracted_files)
}

pub fn handle(_interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        // Async file operations - return Promises for true concurrency
        "read_file_async" => {
            if let Some(Value::Str(path)) = arg_values.first() {
                let path_clone = path.clone();

                // Create oneshot channel for result
                let (tx, rx) = tokio::sync::oneshot::channel();

                // Spawn async task to read file
                AsyncRuntime::spawn_task(async move {
                    if let Err(error) = validate_read_size_limit(path_clone.as_ref()) {
                        let _ = tx.send(Err(error));
                        return Value::Null;
                    }

                    match tokio::fs::read_to_string(path_clone.as_ref()).await {
                        Ok(content) => {
                            let _ = tx.send(Ok(Value::Str(Arc::new(content))));
                        }
                        Err(e) => {
                            let path_str = path_clone.as_ref().clone();
                            let _ = tx.send(Err(format!("Cannot read file '{}': {}", path_str, e)));
                        }
                    }
                    Value::Null
                });

                Value::Promise {
                    receiver: Arc::new(Mutex::new(rx)),
                    is_polled: Arc::new(Mutex::new(false)),
                    cached_result: Arc::new(Mutex::new(None)),
                    task_handle: None,
                }
            } else {
                Value::Error("read_file requires a string path argument".to_string())
            }
        }

        "write_file_async" => {
            let overwrite = match parse_overwrite_flag("write_file", arg_values) {
                Ok(flag) => flag,
                Err(error) => return Some(error),
            };
            if let (Some(Value::Str(path)), Some(Value::Str(content))) =
                (arg_values.first(), arg_values.get(1))
            {
                let path_clone = path.clone();
                let content_clone = content.clone();
                let payload_size = content_clone.as_ref().as_bytes().len();

                // Create oneshot channel for result
                let (tx, rx) = tokio::sync::oneshot::channel();

                // Spawn async task to write file
                AsyncRuntime::spawn_task(async move {
                    if let Err(error) = validate_write_size_limit(path_clone.as_ref(), payload_size)
                    {
                        let _ = tx.send(Err(error));
                        return Value::Null;
                    }

                    if let Err(error) = enforce_overwrite_policy(path_clone.as_ref(), overwrite) {
                        let _ = tx.send(Err(error));
                        return Value::Null;
                    }

                    match tokio::fs::write(path_clone.as_ref(), content_clone.as_ref()).await {
                        Ok(_) => {
                            let _ = tx.send(Ok(Value::Bool(true)));
                        }
                        Err(e) => {
                            let path_str = path_clone.as_ref().clone();
                            let _ =
                                tx.send(Err(format!("Cannot write file '{}': {}", path_str, e)));
                        }
                    }
                    Value::Null
                });

                Value::Promise {
                    receiver: Arc::new(Mutex::new(rx)),
                    is_polled: Arc::new(Mutex::new(false)),
                    cached_result: Arc::new(Mutex::new(None)),
                    task_handle: None,
                }
            } else {
                Value::Error("write_file requires string arguments".to_string())
            }
        }

        "list_dir_async" => {
            if let Some(Value::Str(path)) = arg_values.first() {
                let path_clone = path.clone();

                // Create oneshot channel for result
                let (tx, rx) = tokio::sync::oneshot::channel();

                // Spawn async task to list directory
                AsyncRuntime::spawn_task(async move {
                    match tokio::fs::read_dir(path_clone.as_ref()).await {
                        Ok(mut entries) => {
                            let mut files = Vec::new();
                            while let Ok(Some(entry)) = entries.next_entry().await {
                                if let Some(name) = entry.file_name().to_str() {
                                    files.push(Value::Str(Arc::new(name.to_string())));
                                }
                            }
                            let _ = tx.send(Ok(Value::Array(Arc::new(files))));
                        }
                        Err(e) => {
                            let path_str = path_clone.as_ref().clone();
                            let _ = tx
                                .send(Err(format!("Cannot list directory '{}': {}", path_str, e)));
                        }
                    }
                    Value::Null
                });

                Value::Promise {
                    receiver: Arc::new(Mutex::new(rx)),
                    is_polled: Arc::new(Mutex::new(false)),
                    cached_result: Arc::new(Mutex::new(None)),
                    task_handle: None,
                }
            } else {
                Value::Error("list_dir requires a string path argument".to_string())
            }
        }

        "write_file" => {
            let overwrite = match parse_overwrite_flag("write_file", arg_values) {
                Ok(flag) => flag,
                Err(error) => return Some(error),
            };
            if let (Some(Value::Str(path)), Some(Value::Str(content))) =
                (arg_values.first(), arg_values.get(1))
            {
                if let Err(error) = validate_write_size_limit(path.as_ref(), content.len()) {
                    return Some(Value::Error(error));
                }

                if let Err(error) = enforce_overwrite_policy(path.as_ref(), overwrite) {
                    return Some(Value::Error(error));
                }

                match std::fs::write(path.as_ref(), content.as_ref()) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => Value::Error(format!("Cannot write file '{}': {}", path.as_ref(), e)),
                }
            } else {
                Value::Error("write_file requires string arguments".to_string())
            }
        }

        // Synchronous fallback versions for compatibility
        "read_file_sync" | "read_file" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "read_file_sync requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                if let Err(error) = validate_read_size_limit(path.as_ref()) {
                    return Some(Value::Error(error));
                }

                match std::fs::read_to_string(path.as_ref()) {
                    Ok(content) => Value::Str(Arc::new(content)),
                    Err(e) => Value::Error(format!("Cannot read file '{}': {}", path.as_ref(), e)),
                }
            } else {
                Value::Error("read_file_sync requires a string path argument".to_string())
            }
        }

        "read_file_lossy" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "read_file_lossy requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                if let Err(error) = validate_read_size_limit(path.as_ref()) {
                    return Some(Value::Error(error));
                }

                match std::fs::read(path.as_ref()) {
                    Ok(bytes) => Value::Str(Arc::new(String::from_utf8_lossy(&bytes).into_owned())),
                    Err(e) => Value::Error(format!("Cannot read file '{}': {}", path.as_ref(), e)),
                }
            } else {
                Value::Error("read_file_lossy requires a string path argument".to_string())
            }
        }

        "write_file_sync" => {
            let overwrite = match parse_overwrite_flag("write_file_sync", arg_values) {
                Ok(flag) => flag,
                Err(error) => return Some(error),
            };
            if let (Some(Value::Str(path)), Some(Value::Str(content))) =
                (arg_values.first(), arg_values.get(1))
            {
                if let Err(error) = validate_write_size_limit(path.as_ref(), content.len()) {
                    return Some(Value::Error(error));
                }

                if let Err(error) = enforce_overwrite_policy(path.as_ref(), overwrite) {
                    return Some(Value::Error(error));
                }

                match std::fs::write(path.as_ref(), content.as_ref()) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => Value::Error(format!("Cannot write file '{}': {}", path.as_ref(), e)),
                }
            } else {
                Value::Error("write_file_sync requires string arguments".to_string())
            }
        }

        "list_dir_sync" => {
            if let Some(Value::Str(path)) = arg_values.first() {
                match std::fs::read_dir(path.as_ref()) {
                    Ok(entries) => {
                        let mut files = Vec::new();
                        for entry in entries.flatten() {
                            if let Some(name) = entry.file_name().to_str() {
                                files.push(Value::Str(Arc::new(name.to_string())));
                            }
                        }
                        Value::Array(Arc::new(files))
                    }
                    Err(e) => {
                        Value::Error(format!("Cannot list directory '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("list_dir_sync requires a string path argument".to_string())
            }
        }

        "read_binary_file" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "read_binary_file requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                if let Err(error) = validate_read_size_limit(path.as_ref()) {
                    return Some(Value::Error(error));
                }

                match std::fs::read(path.as_ref()) {
                    Ok(bytes) => Value::Bytes(bytes),
                    Err(e) => {
                        Value::Error(format!("Cannot read binary file '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("read_binary_file requires a string path argument".to_string())
            }
        }

        "write_binary_file" => {
            let overwrite = match parse_overwrite_flag("write_binary_file", arg_values) {
                Ok(flag) => flag,
                Err(error) => return Some(error),
            };
            if let (Some(Value::Str(path)), Some(Value::Bytes(bytes))) =
                (arg_values.first(), arg_values.get(1))
            {
                if let Err(error) = validate_write_size_limit(path.as_ref(), bytes.len()) {
                    return Some(Value::Error(error));
                }

                if let Err(error) = enforce_overwrite_policy(path.as_ref(), overwrite) {
                    return Some(Value::Error(error));
                }

                match std::fs::write(path.as_ref(), bytes) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => {
                        Value::Error(format!("Cannot write binary file '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error(
                    "write_binary_file requires path (string) and bytes arguments".to_string(),
                )
            }
        }

        #[cfg(feature = "runtime-image")]
        "load_image" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "load_image requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match image::open(path.as_ref()) {
                    Ok(image_data) => {
                        let format = Path::new(path.as_ref())
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .unwrap_or("unknown")
                            .to_lowercase();

                        Value::Image { data: Arc::new(Mutex::new(image_data)), format }
                    }
                    Err(error) => {
                        Value::Error(format!("Cannot load image '{}': {}", path.as_ref(), error))
                    }
                }
            } else {
                Value::Error("load_image requires a string path argument".to_string())
            }
        }

        #[cfg(not(feature = "runtime-image"))]
        "load_image" => Value::Error(
            "Image native APIs are disabled in this build (enable the 'runtime-image' feature)"
                .to_string(),
        ),

        "gif_to_webp" => {
            if arg_values.len() < 2 || arg_values.len() > 5 {
                return Some(Value::Error("gif_to_webp requires 2 to 5 arguments: input_path, output_path, [quality(0-100)], [method(0-6)], [lossless(bool)]".to_string()));
            }

            let input_path = match arg_values.first() {
                Some(Value::Str(path)) => path.as_ref().clone(),
                _ => {
                    return Some(Value::Error(
                        "gif_to_webp requires a string input_path argument".to_string(),
                    ));
                }
            };

            let output_path = match arg_values.get(1) {
                Some(Value::Str(path)) => path.as_ref().clone(),
                _ => {
                    return Some(Value::Error(
                        "gif_to_webp requires a string output_path argument".to_string(),
                    ));
                }
            };

            let quality = match arg_values.get(2) {
                Some(Value::Int(n)) => *n,
                Some(Value::Float(n)) => *n as i64,
                Some(_) => {
                    return Some(Value::Error(
                        "gif_to_webp quality must be numeric (0-100)".to_string(),
                    ));
                }
                None => 85,
            };

            let method = match arg_values.get(3) {
                Some(Value::Int(n)) => *n,
                Some(Value::Float(n)) => *n as i64,
                Some(_) => {
                    return Some(Value::Error(
                        "gif_to_webp method must be numeric (0-6)".to_string(),
                    ));
                }
                None => 4,
            };

            let lossless = match arg_values.get(4) {
                Some(Value::Bool(flag)) => *flag,
                Some(_) => {
                    return Some(Value::Error(
                        "gif_to_webp lossless flag must be bool".to_string(),
                    ));
                }
                None => false,
            };

            if quality < 0 || quality > 100 {
                return Some(Value::Error(
                    "gif_to_webp quality must be in range 0-100".to_string(),
                ));
            }

            if method < 0 || method > 6 {
                return Some(Value::Error("gif_to_webp method must be in range 0-6".to_string()));
            }

            if !Path::new(&input_path).exists() {
                return Some(Value::Error(format!(
                    "gif_to_webp input file does not exist: {}",
                    input_path
                )));
            }

            let mut command = Command::new("gif2webp");
            command
                .arg(&input_path)
                .arg("-o")
                .arg(&output_path)
                .arg("-q")
                .arg(quality.to_string())
                .arg("-m")
                .arg(method.to_string())
                .arg("-mt");

            if lossless {
                command.arg("-lossless");
            } else {
                command.arg("-lossy");
            }

            match command.output() {
                Ok(output) => {
                    if output.status.success() {
                        Value::Bool(true)
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        if stderr.is_empty() {
                            Value::Error("gif_to_webp failed with unknown error".to_string())
                        } else {
                            Value::Error(format!("gif_to_webp failed: {}", stderr))
                        }
                    }
                }
                Err(error) => {
                    if std::io::ErrorKind::NotFound == error.kind() {
                        Value::Error("gif_to_webp requires the 'gif2webp' CLI tool to be installed and available in PATH".to_string())
                    } else {
                        Value::Error(format!("gif_to_webp command failed: {}", error))
                    }
                }
            }
        }

        #[cfg(feature = "runtime-archive")]
        "zip_create" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "zip_create requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match File::create(path.as_ref()) {
                    Ok(file) => {
                        let writer = ZipWriter::new(file);
                        Value::ZipArchive {
                            writer: Arc::new(Mutex::new(Some(writer))),
                            path: path.as_ref().clone(),
                        }
                    }
                    Err(error) => Value::ErrorObject {
                        message: format!(
                            "Failed to create zip file '{}': {}",
                            path.as_ref(),
                            error
                        ),
                        stack: Vec::new(),
                        line: None,
                        cause: None,
                    },
                }
            } else {
                Value::Error("zip_create requires a string path argument".to_string())
            }
        }

        #[cfg(feature = "runtime-archive")]
        "zip_add_file" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "zip_add_file requires (ZipArchive, string_path) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::ZipArchive { writer, .. }), Some(Value::Str(source_path))) => {
                    let mut writer_guard = writer.lock().unwrap();
                    if let Some(zip_writer) = writer_guard.as_mut() {
                        match std::fs::read(source_path.as_ref()) {
                            Ok(file_contents) => {
                                let file_name = Path::new(source_path.as_ref())
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or(source_path.as_ref());
                                let options = SimpleFileOptions::default()
                                    .compression_method(zip::CompressionMethod::Deflated);

                                match zip_writer.start_file(file_name, options) {
                                    Ok(_) => match zip_writer.write_all(&file_contents) {
                                        Ok(_) => Value::Bool(true),
                                        Err(error) => Value::ErrorObject {
                                            message: format!(
                                                "Failed to write file to zip: {}",
                                                error
                                            ),
                                            stack: Vec::new(),
                                            line: None,
                                            cause: None,
                                        },
                                    },
                                    Err(error) => Value::ErrorObject {
                                        message: format!(
                                            "Failed to start zip entry '{}': {}",
                                            file_name, error
                                        ),
                                        stack: Vec::new(),
                                        line: None,
                                        cause: None,
                                    },
                                }
                            }
                            Err(error) => Value::ErrorObject {
                                message: format!(
                                    "Failed to read source file '{}': {}",
                                    source_path.as_ref(),
                                    error
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    } else {
                        Value::Error("Zip archive has been closed".to_string())
                    }
                }
                _ => Value::Error(
                    "zip_add_file requires (ZipArchive, string_path) arguments".to_string(),
                ),
            }
        }

        #[cfg(feature = "runtime-archive")]
        "zip_add_dir" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "zip_add_dir requires (ZipArchive, string_path) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::ZipArchive { writer, .. }), Some(Value::Str(directory_path))) => {
                    let mut writer_guard = writer.lock().unwrap();
                    if let Some(zip_writer) = writer_guard.as_mut() {
                        let directory_path = Path::new(directory_path.as_ref());
                        match zip_add_dir_recursive(zip_writer, directory_path, "") {
                            Ok(_) => Value::Bool(true),
                            Err(message) => Value::ErrorObject {
                                message,
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    } else {
                        Value::Error("Zip archive has been closed".to_string())
                    }
                }
                _ => Value::Error(
                    "zip_add_dir requires (ZipArchive, string_path) arguments".to_string(),
                ),
            }
        }

        #[cfg(feature = "runtime-archive")]
        "zip_close" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("zip_close requires a ZipArchive argument".to_string()));
            }

            if let Some(Value::ZipArchive { writer, .. }) = arg_values.first() {
                let mut writer_guard = writer.lock().unwrap();
                if let Some(zip_writer) = writer_guard.take() {
                    match zip_writer.finish() {
                        Ok(_) => Value::Bool(true),
                        Err(error) => Value::ErrorObject {
                            message: format!("Failed to finalize zip archive: {}", error),
                            stack: Vec::new(),
                            line: None,
                            cause: None,
                        },
                    }
                } else {
                    Value::Error("Zip archive has already been closed".to_string())
                }
            } else {
                Value::Error("zip_close requires a ZipArchive argument".to_string())
            }
        }

        #[cfg(feature = "runtime-archive")]
        "unzip" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "unzip requires (string_zip_path, string_output_dir) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(zip_path)), Some(Value::Str(output_dir))) => {
                    match File::open(zip_path.as_ref()) {
                        Ok(file) => match ZipArchive::new(file) {
                            Ok(mut archive) => match extract_zip_archive_with_limits(
                                &mut archive,
                                Path::new(output_dir.as_ref()),
                                ZipExtractionLimits::DEFAULT,
                            ) {
                                Ok(extracted_files) => Value::Array(Arc::new(extracted_files)),
                                Err(message) => Value::ErrorObject {
                                    message,
                                    stack: Vec::new(),
                                    line: None,
                                    cause: None,
                                },
                            },
                            Err(error) => Value::ErrorObject {
                                message: format!(
                                    "Failed to open zip archive '{}': {}",
                                    zip_path.as_ref(),
                                    error
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        },
                        Err(error) => Value::ErrorObject {
                            message: format!(
                                "Failed to open file '{}': {}",
                                zip_path.as_ref(),
                                error
                            ),
                            stack: Vec::new(),
                            line: None,
                            cause: None,
                        },
                    }
                }
                _ => Value::Error(
                    "unzip requires (string_zip_path, string_output_dir) arguments".to_string(),
                ),
            }
        }

        #[cfg(not(feature = "runtime-archive"))]
        "zip_create" | "zip_add_file" | "zip_add_dir" | "zip_close" | "unzip" => Value::Error(
            "Archive native APIs are disabled in this build (enable the 'runtime-archive' feature)"
                .to_string(),
        ),

        "append_file" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "append_file requires two arguments: path and content".to_string(),
                ));
            }
            if let (Some(Value::Str(path)), Some(Value::Str(content))) =
                (arg_values.first(), arg_values.get(1))
            {
                if let Err(error) = validate_append_size_limit(path.as_ref(), content.len()) {
                    return Some(Value::Error(error));
                }

                match OpenOptions::new().create(true).append(true).open(path.as_ref()) {
                    Ok(mut file) => match file.write_all(content.as_ref().as_bytes()) {
                        Ok(_) => Value::Bool(true),
                        Err(e) => Value::Error(format!(
                            "Cannot append to file '{}': {}",
                            path.as_ref(),
                            e
                        )),
                    },
                    Err(e) => Value::Error(format!("Cannot open file '{}': {}", path.as_ref(), e)),
                }
            } else {
                Value::Error("append_file requires string arguments".to_string())
            }
        }

        "file_exists" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "file_exists requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                if Path::new(path.as_ref()).exists() {
                    Value::Bool(true)
                } else {
                    Value::Bool(false)
                }
            } else {
                Value::Error("file_exists requires a string path argument".to_string())
            }
        }

        "read_lines" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "read_lines requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                if let Err(error) = validate_read_size_limit(path.as_ref()) {
                    return Some(Value::Error(error));
                }

                match std::fs::read_to_string(path.as_ref()) {
                    Ok(content) => {
                        let lines: Vec<Value> = content
                            .lines()
                            .map(|line| Value::Str(Arc::new(line.to_string())))
                            .collect();
                        Value::Array(Arc::new(lines))
                    }
                    Err(e) => Value::Error(format!("Cannot read file '{}': {}", path.as_ref(), e)),
                }
            } else {
                Value::Error("read_lines requires a string path argument".to_string())
            }
        }

        "list_dir" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("list_dir requires a string path argument".to_string()));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match std::fs::read_dir(path.as_ref()) {
                    Ok(entries) => {
                        let mut files = Vec::new();
                        for entry in entries.flatten() {
                            if let Some(name) = entry.file_name().to_str() {
                                files.push(Value::Str(Arc::new(name.to_string())));
                            }
                        }
                        Value::Array(Arc::new(files))
                    }
                    Err(e) => {
                        Value::Error(format!("Cannot list directory '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("list_dir requires a string path argument".to_string())
            }
        }

        "create_dir" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "create_dir requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match std::fs::create_dir_all(path.as_ref()) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => {
                        Value::Error(format!("Cannot create directory '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("create_dir requires a string path argument".to_string())
            }
        }

        "file_size" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("file_size requires a string path argument".to_string()));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match std::fs::metadata(path.as_ref()) {
                    Ok(metadata) => Value::Int(metadata.len() as i64),
                    Err(e) => {
                        Value::Error(format!("Cannot get file size for '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("file_size requires a string path argument".to_string())
            }
        }

        "delete_file" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "delete_file requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                if Path::new(path.as_ref()).is_dir() {
                    return Some(Value::Error(format!(
                        "Cannot delete file '{}': path is a directory",
                        path.as_ref()
                    )));
                }

                match std::fs::remove_file(path.as_ref()) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => {
                        Value::Error(format!("Cannot delete file '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("delete_file requires a string path argument".to_string())
            }
        }

        "rename_file" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "rename_file requires two arguments: old_path and new_path".to_string(),
                ));
            }
            if let (Some(Value::Str(old_path)), Some(Value::Str(new_path))) =
                (arg_values.first(), arg_values.get(1))
            {
                match std::fs::rename(old_path.as_ref(), new_path.as_ref()) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => Value::Error(format!(
                        "Cannot rename file '{}' to '{}': {}",
                        old_path.as_ref(),
                        new_path.as_ref(),
                        e
                    )),
                }
            } else {
                Value::Error("rename_file requires string arguments".to_string())
            }
        }

        "copy_file" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "copy_file requires two arguments: source and dest".to_string(),
                ));
            }
            if let (Some(Value::Str(source)), Some(Value::Str(dest))) =
                (arg_values.first(), arg_values.get(1))
            {
                match std::fs::copy(source.as_ref(), dest.as_ref()) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => Value::Error(format!(
                        "Cannot copy file '{}' to '{}': {}",
                        source.as_ref(),
                        dest.as_ref(),
                        e
                    )),
                }
            } else {
                Value::Error("copy_file requires string arguments".to_string())
            }
        }

        // OS module functions
        "os_getcwd" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "os_getcwd() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            match std::env::current_dir() {
                Ok(path) => Value::Str(Arc::new(path.to_string_lossy().to_string())),
                Err(e) => Value::Error(format!("Cannot get current directory: {}", e)),
            }
        }

        "os_chdir" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "os_chdir() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match std::env::set_current_dir(path.as_ref()) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => Value::Error(format!(
                        "Cannot change directory to '{}': {}",
                        path.as_ref(),
                        e
                    )),
                }
            } else {
                Value::Error("os_chdir requires a string argument (path)".to_string())
            }
        }

        "os_rmdir" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "os_rmdir() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match std::fs::remove_dir(path.as_ref()) {
                    Ok(_) => Value::Bool(true),
                    Err(e) => {
                        Value::Error(format!("Cannot remove directory '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("os_rmdir requires a string argument (path)".to_string())
            }
        }

        "os_environ" => {
            if !arg_values.is_empty() {
                return Some(Value::Error(format!(
                    "os_environ() expects 0 arguments, got {}",
                    arg_values.len()
                )));
            }

            let mut dict = DictMap::default();
            for (key, value) in std::env::vars() {
                dict.insert(Arc::<str>::from(key), Value::Str(Arc::new(value)));
            }
            Value::Dict(Arc::new(dict))
        }

        // Path operation functions
        "join_path" | "path_join" => {
            if arg_values.is_empty() {
                Value::Error(format!("{} requires at least one string argument", name))
            } else {
                let mut parts: Vec<String> = Vec::with_capacity(arg_values.len());
                for (index, value) in arg_values.iter().enumerate() {
                    match value {
                        Value::Str(s) => parts.push(s.as_ref().clone()),
                        _ => {
                            return Some(Value::Error(format!(
                                "{} argument {} must be a string",
                                name,
                                index + 1
                            )));
                        }
                    }
                }

                Value::Str(Arc::new(builtins::join_path(&parts)))
            }
        }

        "dirname" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "dirname() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                Value::Str(Arc::new(builtins::dirname(path.as_ref())))
            } else {
                Value::Error("dirname requires a string argument (path)".to_string())
            }
        }

        "basename" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "basename() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                Value::Str(Arc::new(builtins::basename(path.as_ref())))
            } else {
                Value::Error("basename requires a string argument (path)".to_string())
            }
        }

        "path_exists" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "path_exists() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                Value::Bool(builtins::path_exists(path.as_ref()))
            } else {
                Value::Error("path_exists requires a string argument (path)".to_string())
            }
        }

        "path_absolute" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "path_absolute() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match std::fs::canonicalize(Path::new(path.as_ref())) {
                    Ok(abs_path) => Value::Str(Arc::new(abs_path.to_string_lossy().to_string())),
                    Err(e) => Value::Error(format!(
                        "Cannot get absolute path for '{}': {}",
                        path.as_ref(),
                        e
                    )),
                }
            } else {
                Value::Error("path_absolute requires a string argument (path)".to_string())
            }
        }

        "path_is_dir" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "path_is_dir() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                Value::Bool(Path::new(path.as_ref()).is_dir())
            } else {
                Value::Error("path_is_dir requires a string argument (path)".to_string())
            }
        }

        "path_is_file" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "path_is_file() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                Value::Bool(Path::new(path.as_ref()).is_file())
            } else {
                Value::Error("path_is_file requires a string argument (path)".to_string())
            }
        }

        "path_is_symlink" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "path_is_symlink() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match fs::symlink_metadata(path.as_ref()) {
                    Ok(metadata) => Value::Bool(metadata.file_type().is_symlink()),
                    Err(e) => Value::Error(format!(
                        "Cannot inspect symlink status for '{}': {}",
                        path.as_ref(),
                        e
                    )),
                }
            } else {
                Value::Error("path_is_symlink requires a string argument (path)".to_string())
            }
        }

        "path_extension" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(format!(
                    "path_extension() expects 1 argument (path), got {}",
                    arg_values.len()
                )));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match Path::new(path.as_ref()).extension() {
                    Some(ext) => Value::Str(Arc::new(ext.to_string_lossy().to_string())),
                    None => Value::Str(Arc::new(String::new())),
                }
            } else {
                Value::Error("path_extension requires a string argument (path)".to_string())
            }
        }

        _ => return None,
    };

    Some(result)
}
