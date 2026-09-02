// File: src/interpreter/native_functions/filesystem.rs
//
// Filesystem operation native functions

use crate::interpreter::{AsyncRuntime, Interpreter, Value};
#[cfg(feature = "runtime-archive")]
use crate::path_security;
use crate::runtime_limits;
use crate::{builtins, interpreter::DictMap};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
use std::fs;
#[cfg(feature = "runtime-archive")]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(feature = "runtime-archive")]
use std::path::PathBuf;
use std::path::{Component, Path};
use std::process::Command;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
#[cfg(feature = "runtime-archive")]
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

const ZIP_UNIX_FILE_TYPE_MASK: u32 = 0o170000;
const ZIP_UNIX_SYMLINK_FILE_TYPE: u32 = 0o120000;
const MAX_FILE_READ_BYTES: u64 = runtime_limits::MAX_FILE_IO_BYTES as u64;
const MAX_FILE_WRITE_BYTES: usize = runtime_limits::MAX_FILE_IO_BYTES;

fn beneath_error(code: &str, detail: impl std::fmt::Display) -> String {
    format!("read_file_beneath[{}]: {}", code, detail)
}

fn validate_beneath_relative_path(relative_path: &Path) -> Result<Vec<&std::ffi::OsStr>, String> {
    if relative_path.as_os_str().is_empty() || relative_path.is_absolute() {
        return Err(beneath_error(
            "invalid_relative_path",
            "relative_path must be a non-empty relative path",
        ));
    }

    let relative_text = relative_path.to_string_lossy();
    #[cfg(windows)]
    let lexical_parts = relative_text.split(['/', '\\']);
    #[cfg(not(windows))]
    let lexical_parts = relative_text.split('/');
    if lexical_parts.clone().any(|part| part.is_empty() || part == "." || part == "..") {
        return Err(beneath_error(
            "invalid_relative_path",
            "relative_path must contain only normal non-empty components",
        ));
    }

    let mut components = Vec::new();
    for component in relative_path.components() {
        match component {
            Component::Normal(value) if !value.is_empty() => components.push(value),
            _ => {
                return Err(beneath_error(
                    "invalid_relative_path",
                    "relative_path must contain only normal components (no prefix, root, '.', or '..')",
                ));
            }
        }
    }
    if components.is_empty() {
        return Err(beneath_error("invalid_relative_path", "relative_path must name a file"));
    }
    Ok(components)
}

fn read_file_beneath_bytes(
    root: &str,
    relative_path: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    if max_bytes == 0 || max_bytes > MAX_FILE_WRITE_BYTES {
        return Err(beneath_error(
            "invalid_max_bytes",
            format!("max_bytes must be between 1 and {}", MAX_FILE_WRITE_BYTES),
        ));
    }

    let relative_path = Path::new(relative_path);
    let components = validate_beneath_relative_path(relative_path)?;
    let mut directory = Dir::open_ambient_dir(root, ambient_authority()).map_err(|error| {
        beneath_error("root_open_failed", format!("cannot open trusted root '{}': {}", root, error))
    })?;

    for component in &components[..components.len() - 1] {
        directory =
            cap_fs_ext::DirExt::open_dir_nofollow(&directory, component).map_err(|error| {
                beneath_error(
                    "component_rejected",
                    format!(
                        "cannot traverse component '{}': {}",
                        component.to_string_lossy(),
                        error
                    ),
                )
            })?;
    }

    let final_component = components[components.len() - 1];
    let mut options = CapOpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true);
    let mut file = directory.open_with(final_component, &options).map_err(|error| {
        beneath_error(
            "target_open_failed",
            format!("cannot open target '{}': {}", relative_path.display(), error),
        )
    })?;
    let metadata = file.metadata().map_err(|error| {
        beneath_error(
            "target_metadata_failed",
            format!("cannot inspect target '{}': {}", relative_path.display(), error),
        )
    })?;
    if !metadata.is_file() {
        return Err(beneath_error(
            "target_not_regular_file",
            format!("target '{}' is not a regular file", relative_path.display()),
        ));
    }
    if metadata.len() > max_bytes as u64 {
        return Err(beneath_error(
            "size_limit_exceeded",
            format!(
                "target '{}' exceeds maximum read size ({} bytes > {} bytes)",
                relative_path.display(),
                metadata.len(),
                max_bytes
            ),
        ));
    }

    let mut bytes = Vec::with_capacity((metadata.len() as usize).min(max_bytes));
    Read::by_ref(&mut file).take(max_bytes as u64 + 1).read_to_end(&mut bytes).map_err(
        |error| {
            beneath_error(
                "read_failed",
                format!("cannot read target '{}': {}", relative_path.display(), error),
            )
        },
    )?;
    if bytes.len() > max_bytes {
        return Err(beneath_error(
            "size_limit_exceeded",
            format!("target '{}' grew beyond maximum read size", relative_path.display()),
        ));
    }
    Ok(bytes)
}

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

const MAX_JSONL_LINE_BYTES: usize = 1_048_576;
const MAX_JSONL_QUERY_ROWS: usize = 100_000;

fn option_value<'a>(options: &'a DictMap, name: &str) -> Option<&'a Value> {
    options.get(name)
}

fn option_string(options: &DictMap, name: &str, fallback: &str) -> Result<String, String> {
    match option_value(options, name) {
        None => Ok(fallback.to_string()),
        Some(Value::Str(value)) => Ok(value.to_string()),
        Some(_) => Err(format!("jsonl_query option '{}' must be a string", name)),
    }
}

fn option_usize(options: &DictMap, name: &str, fallback: usize) -> Result<usize, String> {
    match option_value(options, name) {
        None => Ok(fallback),
        Some(Value::Int(value)) if *value > 0 => usize::try_from(*value)
            .map_err(|_| format!("jsonl_query option '{}' is too large", name)),
        Some(_) => Err(format!("jsonl_query option '{}' must be a positive integer", name)),
    }
}

fn dotted_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(value);
    }
    let mut current = value;
    for component in path.split('.') {
        match current {
            Value::Dict(record) => current = record.get(component)?,
            _ => return None,
        }
    }
    Some(current)
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (builtins::to_json(left), builtins::to_json(right)) {
        (Ok(left_json), Ok(right_json)) => left_json == right_json,
        _ => false,
    }
}

fn read_jsonl_values(
    path: &str,
    mut visit: impl FnMut(Value) -> Result<bool, String>,
) -> Result<(), String> {
    let file = fs::File::open(path)
        .map_err(|error| format!("Cannot read JSONL file '{}': {}", path, error))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut line_number = 0usize;
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Cannot read JSONL file '{}': {}", path, error))?;
        if bytes == 0 {
            break;
        }
        line_number += 1;
        if bytes > MAX_JSONL_LINE_BYTES {
            return Err(format!(
                "JSONL line {} exceeds {} bytes",
                line_number, MAX_JSONL_LINE_BYTES
            ));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value = builtins::parse_json(trimmed)
            .map_err(|error| format!("JSONL line {} is invalid: {}", line_number, error))?;
        if !visit(value)? {
            break;
        }
    }
    Ok(())
}

fn jsonl_query(path: &str, options: &DictMap) -> Result<Value, String> {
    let filter_field = option_string(options, "filter_field", "")?;
    let filter_value = option_value(options, "filter_equals");
    let max_rows = option_usize(options, "max_rows", 1_000)?;
    if max_rows > MAX_JSONL_QUERY_ROWS {
        return Err(format!("jsonl_query max_rows cannot exceed {}", MAX_JSONL_QUERY_ROWS));
    }
    let join_path = option_string(options, "join_path", "")?;
    let left_field = option_string(options, "left_field", "")?;
    let right_field = option_string(options, "right_field", "")?;
    let join_parts_present =
        [!join_path.is_empty(), !left_field.is_empty(), !right_field.is_empty()];
    if join_parts_present.iter().any(|present| *present)
        && !join_parts_present.iter().all(|present| *present)
    {
        return Err(
            "jsonl_query joins require join_path, left_field, and right_field together".to_string()
        );
    }

    let mut output = Vec::new();
    read_jsonl_values(path, |left| {
        if !filter_field.is_empty()
            && !filter_value.is_some_and(|wanted| {
                dotted_value(&left, &filter_field)
                    .is_some_and(|actual| values_equal(actual, wanted))
            })
        {
            return Ok(true);
        }
        if join_path.is_empty() {
            output.push(left);
            return Ok(output.len() < max_rows);
        }
        let Some(left_key) = dotted_value(&left, &left_field).cloned() else {
            return Ok(true);
        };
        read_jsonl_values(&join_path, |right| {
            if dotted_value(&right, &right_field)
                .is_some_and(|right_key| values_equal(&left_key, right_key))
            {
                let mut joined = DictMap::default();
                joined.insert(Arc::from("left"), left.clone());
                joined.insert(Arc::from("right"), right);
                output.push(Value::Dict(Arc::new(joined)));
            }
            Ok(output.len() < max_rows)
        })?;
        Ok(output.len() < max_rows)
    })?;
    Ok(Value::Array(Arc::new(output)))
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

fn write_file_atomically(path: &str, payload: &[u8], overwrite: bool) -> Result<(), String> {
    let target = Path::new(path);
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Cannot write file '{}': path must name a file", path))?;
    let temporary_path = parent.join(format!(".{}.kujo-atomic-{}.tmp", file_name, Uuid::new_v4()));

    let result = (|| {
        let mut temporary_file =
            OpenOptions::new().write(true).create_new(true).open(&temporary_path).map_err(
                |error| {
                    format!(
                        "Cannot write file '{}': unable to create temporary file '{}': {}",
                        path,
                        temporary_path.display(),
                        error
                    )
                },
            )?;
        temporary_file.write_all(payload).map_err(|error| {
            format!("Cannot write file '{}': failed writing temporary file: {}", path, error)
        })?;
        temporary_file.flush().map_err(|error| {
            format!("Cannot write file '{}': failed flushing temporary file: {}", path, error)
        })?;
        temporary_file.sync_all().map_err(|error| {
            format!("Cannot write file '{}': failed syncing temporary file: {}", path, error)
        })?;
        drop(temporary_file);

        if overwrite {
            fs::rename(&temporary_path, target)
                .map_err(|error| format!("Cannot atomically replace file '{}': {}", path, error))?;
        } else {
            // Linking the completed temporary inode into place makes the
            // no-overwrite case atomic and fails if the destination appears
            // between validation and finalization.
            fs::hard_link(&temporary_path, target).map_err(|error| {
                format!(
                    "Cannot atomically create file '{}': {} (pass overwrite=true to replace it)",
                    path, error
                )
            })?;
            fs::remove_file(&temporary_path).map_err(|error| {
                format!("Cannot clean up temporary file for '{}': {}", path, error)
            })?;
        }

        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
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

        "write_file_atomic" => {
            let overwrite = match parse_overwrite_flag("write_file_atomic", arg_values) {
                Ok(flag) => flag,
                Err(error) => return Some(error),
            };
            let (Some(Value::Str(path)), Some(payload)) = (arg_values.first(), arg_values.get(1))
            else {
                return Some(Value::Error(
                    "write_file_atomic requires path (string) and content/bytes arguments"
                        .to_string(),
                ));
            };
            let bytes = match payload {
                Value::Str(content) => content.as_bytes().to_vec(),
                Value::Bytes(bytes) => bytes.clone(),
                _ => {
                    return Some(Value::Error(
                        "write_file_atomic requires path (string) and content/bytes arguments"
                            .to_string(),
                    ));
                }
            };
            if let Err(error) = validate_write_size_limit(path.as_ref(), bytes.len()) {
                return Some(Value::Error(error));
            }
            if !overwrite && Path::new(path.as_ref()).exists() {
                return Some(Value::Error(format!(
                    "Cannot write file '{}': file already exists (pass overwrite=true to replace it)",
                    path.as_ref()
                )));
            }

            match write_file_atomically(path.as_ref(), &bytes, overwrite) {
                Ok(()) => Value::Bool(true),
                Err(error) => Value::Error(error),
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

        "read_file_beneath" | "read_binary_file_beneath" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(format!(
                    "{} requires root, relative_path, and max_bytes arguments",
                    name
                )));
            }
            let (
                Some(Value::Str(root)),
                Some(Value::Str(relative_path)),
                Some(Value::Int(max_bytes)),
            ) = (arg_values.first(), arg_values.get(1), arg_values.get(2))
            else {
                return Some(Value::Error(format!(
                    "{} requires root (string), relative_path (string), and max_bytes (int) arguments",
                    name
                )));
            };
            let max_bytes = match usize::try_from(*max_bytes) {
                Ok(value) => value,
                Err(_) => {
                    return Some(Value::Error(beneath_error(
                        "invalid_max_bytes",
                        format!("max_bytes must be between 1 and {}", MAX_FILE_WRITE_BYTES),
                    )));
                }
            };
            match read_file_beneath_bytes(root.as_ref(), relative_path.as_ref(), max_bytes) {
                Ok(bytes) if name == "read_binary_file_beneath" => Value::Bytes(bytes),
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(content) => Value::Str(Arc::new(content)),
                    Err(_) => Value::Error(beneath_error(
                        "invalid_utf8",
                        format!("target '{}' is not valid UTF-8", relative_path),
                    )),
                },
                Err(error) => Value::Error(error),
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

        "jsonl_query" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "jsonl_query requires (path, options) arguments".to_string(),
                ));
            }
            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(path)), Some(Value::Dict(options))) => {
                    match jsonl_query(path.as_ref(), options.as_ref()) {
                        Ok(value) => value,
                        Err(error) => Value::Error(error),
                    }
                }
                _ => Value::Error(
                    "jsonl_query requires a string path and options dictionary".to_string(),
                ),
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

#[cfg(test)]
mod beneath_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    fn fixture_root(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("kujo_beneath_{}_{}", label, Uuid::new_v4()));
        fs::create_dir_all(&path).expect("fixture root");
        path
    }

    #[test]
    fn read_file_beneath_reads_text_and_binary_with_exact_bounds() {
        let root = fixture_root("valid");
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/note.txt"), b"hello").unwrap();
        assert_eq!(
            read_file_beneath_bytes(root.to_str().unwrap(), "nested/note.txt", 5).unwrap(),
            b"hello"
        );
        let error =
            read_file_beneath_bytes(root.to_str().unwrap(), "nested/note.txt", 4).unwrap_err();
        assert!(error.contains("[size_limit_exceeded]"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn read_file_beneath_rejects_lexical_escape_and_non_files() {
        let root = fixture_root("lexical");
        fs::create_dir(root.join("nested")).unwrap();
        for path in ["", ".", "..", "../outside", "nested/../outside"] {
            let error = read_file_beneath_bytes(root.to_str().unwrap(), path, 16).unwrap_err();
            assert!(error.contains("[invalid_relative_path]"), "{path}: {error}");
        }
        let absolute = root.join("nested");
        let error = read_file_beneath_bytes(root.to_str().unwrap(), absolute.to_str().unwrap(), 16)
            .unwrap_err();
        assert!(error.contains("[invalid_relative_path]"));
        let error = read_file_beneath_bytes(root.to_str().unwrap(), "nested", 16).unwrap_err();
        assert!(error.contains("[target_not_regular_file]"));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn read_file_beneath_rejects_final_and_intermediate_symlinks() {
        use std::os::unix::fs::symlink;

        let root = fixture_root("symlink");
        let outside = fixture_root("outside");
        fs::create_dir(root.join("safe")).unwrap();
        fs::write(root.join("safe/note.txt"), b"safe").unwrap();
        fs::write(outside.join("secret.txt"), b"outside").unwrap();
        symlink(outside.join("secret.txt"), root.join("final-link")).unwrap();
        symlink(&outside, root.join("dir-link")).unwrap();

        let final_error =
            read_file_beneath_bytes(root.to_str().unwrap(), "final-link", 64).unwrap_err();
        assert!(final_error.contains("[target_open_failed]"));
        let intermediate_error =
            read_file_beneath_bytes(root.to_str().unwrap(), "dir-link/secret.txt", 64).unwrap_err();
        assert!(intermediate_error.contains("[component_rejected]"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(unix)]
    #[test]
    fn read_file_beneath_rejects_fifo_without_blocking() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let root = fixture_root("fifo");
        let fifo = root.join("pipe");
        let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
        let error = read_file_beneath_bytes(root.to_str().unwrap(), "pipe", 64).unwrap_err();
        assert!(error.contains("[target_not_regular_file]"), "{error}");
        let _ = fs::remove_file(fifo);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn read_file_beneath_race_never_reads_outside_root() {
        use std::os::unix::fs::symlink;

        let root = fixture_root("race");
        let outside = fixture_root("race_outside");
        let live = root.join("live");
        fs::create_dir(&live).unwrap();
        fs::write(live.join("note.txt"), b"safe").unwrap();
        fs::write(outside.join("note.txt"), b"outside").unwrap();
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let stop_attacker = stop.clone();
        let root_attacker = root.clone();
        let outside_attacker = outside.clone();
        let attacker = thread::spawn(move || {
            while !stop_attacker.load(Ordering::Relaxed) {
                if fs::rename(root_attacker.join("live"), root_attacker.join("parked")).is_ok() {
                    let _ = symlink(&outside_attacker, root_attacker.join("live"));
                    let _ = fs::remove_file(root_attacker.join("live"));
                    let _ = fs::rename(root_attacker.join("parked"), root_attacker.join("live"));
                }
            }
        });
        for _ in 0..2_000 {
            if let Ok(bytes) = read_file_beneath_bytes(root.to_str().unwrap(), "live/note.txt", 64)
            {
                assert_eq!(bytes, b"safe", "capability traversal escaped trusted root");
            }
        }
        stop.store(true, Ordering::Relaxed);
        attacker.join().unwrap();
        let _ = fs::remove_file(&live);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(windows)]
    #[test]
    fn read_file_beneath_rejects_windows_directory_reparse_points() {
        let root = fixture_root("windows_reparse");
        let outside = fixture_root("windows_reparse_outside");
        fs::write(outside.join("secret.txt"), b"outside").unwrap();
        let link = root.join("dir-link");
        let command = format!("mklink /J \"{}\" \"{}\"", link.display(), outside.display());
        let output = std::process::Command::new("cmd")
            .args(["/C", command.as_str()])
            .output()
            .expect("cmd should create a junction fixture");
        assert!(
            output.status.success(),
            "junction fixture failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let error =
            read_file_beneath_bytes(root.to_str().unwrap(), "dir-link/secret.txt", 64).unwrap_err();
        assert!(error.contains("[component_rejected]"), "{error}");
        let _ = fs::remove_dir(&link);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }
}
