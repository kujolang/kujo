// File: src/interpreter/native_functions/io.rs
//
// I/O-related native functions (print, input, etc.)

#[cfg(unix)]
use crate::interpreter::value::PrivateSpoolState;
use crate::interpreter::{DictMap, Interpreter, Value};
use base64::Engine;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

const MAX_PRIVATE_SPOOL_BYTES: u64 = 1_073_741_824;
const PRIVATE_SPOOL_FILE_RANGE_MAX_BYTES: u64 = 64 * 1024 * 1024;
const PRIVATE_SPOOL_FILE_RANGE_CHUNK_BYTES: usize = 64 * 1024;
const PRIVATE_SPOOL_FILE_RANGE_MAX_PATH_BYTES: usize = 4096;

fn private_spool_content_bytes(content: &Value) -> Option<Vec<u8>> {
    match content {
        Value::Str(value) => Some(value.as_bytes().to_vec()),
        Value::Bytes(value) => Some(value.clone()),
        _ => None,
    }
}

fn private_spool_digest_hex(hasher: Sha256) -> String {
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn private_spool_parent(destination: &std::path::Path) -> &std::path::Path {
    match destination.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => std::path::Path::new("."),
    }
}

#[cfg(unix)]
fn private_spool_append(
    state: &mut PrivateSpoolState,
    bytes: &[u8],
    operation: &str,
) -> Result<(), String> {
    let next_size = state
        .bytes_written
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| format!("{operation} size overflow"))?;
    if next_size > state.max_bytes {
        return Err(format!(
            "{operation} exceeds maximum bytes ({} > {})",
            next_size, state.max_bytes
        ));
    }
    let file = state.file.as_mut().ok_or_else(|| format!("{operation} spool is closed"))?;
    file.write_all(bytes).map_err(|error| format!("{operation} write failed: {error}"))?;
    state.hasher.update(bytes);
    state.bytes_written = next_size;
    Ok(())
}

#[cfg(unix)]
fn private_spool_open_regular_range(
    path: &str,
    offset: u64,
    count: u64,
    operation: &str,
) -> Result<File, String> {
    if path.is_empty() || path.len() > PRIVATE_SPOOL_FILE_RANGE_MAX_PATH_BYTES {
        return Err(format!("{operation} path must be 1-4096 bytes"));
    }
    if count > PRIVATE_SPOOL_FILE_RANGE_MAX_BYTES {
        return Err(format!("{operation} count exceeds the 64 MiB limit"));
    }
    let end =
        offset.checked_add(count).ok_or_else(|| format!("{operation} file range overflows"))?;
    let path = std::path::Path::new(path);
    let link_metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("{operation} cannot inspect file: {error}"))?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        return Err(format!("{operation} requires a non-symlink regular file"));
    }
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(libc::O_NOFOLLOW);
    let mut file = options
        .open(path)
        .map_err(|error| format!("{operation} cannot open file safely: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("{operation} cannot inspect opened file: {error}"))?;
    if !metadata.is_file() || end > metadata.len() {
        return Err(format!("{operation} range exceeds the regular file"));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| format!("{operation} cannot seek file: {error}"))?;
    Ok(file)
}

#[cfg(unix)]
fn private_spool_write_file_range(
    state: &mut PrivateSpoolState,
    path: &str,
    offset: u64,
    count: u64,
    mode: &str,
) -> Result<u64, String> {
    const OPERATION: &str = "io_private_spool_write_file_range";
    if !matches!(mode, "raw" | "crlf" | "base64-crlf" | "smtp-dot-stuff-crlf") {
        return Err(format!("{OPERATION} mode is unsupported"));
    }
    let mut file = private_spool_open_regular_range(path, offset, count, OPERATION)?;
    let starting_size = state.bytes_written;
    let mut buffer = vec![0_u8; PRIVATE_SPOOL_FILE_RANGE_CHUNK_BYTES];
    let mut remaining = count;

    if mode == "raw" {
        while remaining > 0 {
            let requested = remaining.min(buffer.len() as u64) as usize;
            file.read_exact(&mut buffer[..requested])
                .map_err(|error| format!("{OPERATION} file read failed: {error}"))?;
            private_spool_append(state, &buffer[..requested], OPERATION)?;
            remaining -= requested as u64;
        }
    } else if mode == "base64-crlf" {
        let mut carry = Vec::with_capacity(57 + PRIVATE_SPOOL_FILE_RANGE_CHUNK_BYTES);
        while remaining > 0 {
            let requested = remaining.min(buffer.len() as u64) as usize;
            file.read_exact(&mut buffer[..requested])
                .map_err(|error| format!("{OPERATION} file read failed: {error}"))?;
            carry.extend_from_slice(&buffer[..requested]);
            let complete = (carry.len() / 57) * 57;
            let mut output = Vec::with_capacity((complete / 57) * 78);
            for chunk in carry[..complete].chunks_exact(57) {
                output.extend_from_slice(
                    base64::engine::general_purpose::STANDARD.encode(chunk).as_bytes(),
                );
                output.extend_from_slice(b"\r\n");
            }
            if !output.is_empty() {
                private_spool_append(state, &output, OPERATION)?;
            }
            carry.drain(..complete);
            remaining -= requested as u64;
        }
        if !carry.is_empty() {
            let mut output = base64::engine::general_purpose::STANDARD.encode(&carry).into_bytes();
            output.extend_from_slice(b"\r\n");
            private_spool_append(state, &output, OPERATION)?;
        }
    } else {
        let dot_stuff = mode == "smtp-dot-stuff-crlf";
        let mut pending_cr = false;
        let mut line_start = true;
        while remaining > 0 {
            let requested = remaining.min(buffer.len() as u64) as usize;
            file.read_exact(&mut buffer[..requested])
                .map_err(|error| format!("{OPERATION} file read failed: {error}"))?;
            let mut output = Vec::with_capacity(requested.saturating_mul(2).saturating_add(2));
            for byte in &buffer[..requested] {
                if pending_cr {
                    output.extend_from_slice(b"\r\n");
                    line_start = true;
                    pending_cr = false;
                    if *byte == b'\n' {
                        continue;
                    }
                }
                if *byte == b'\r' {
                    pending_cr = true;
                } else if *byte == b'\n' {
                    output.extend_from_slice(b"\r\n");
                    line_start = true;
                } else {
                    if dot_stuff && line_start && *byte == b'.' {
                        output.push(b'.');
                    }
                    output.push(*byte);
                    line_start = false;
                }
            }
            if !output.is_empty() {
                private_spool_append(state, &output, OPERATION)?;
            }
            remaining -= requested as u64;
        }
        if pending_cr {
            private_spool_append(state, b"\r\n", OPERATION)?;
            line_start = true;
        }
        if dot_stuff && !line_start {
            private_spool_append(state, b"\r\n", OPERATION)?;
        }
    }
    Ok(state.bytes_written - starting_size)
}

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

fn parse_non_negative_u64(value: &Value, error_message: &str) -> Result<u64, Value> {
    match value {
        Value::Int(number) if *number >= 0 => Ok(*number as u64),
        Value::Float(number) if *number >= 0.0 => Ok(*number as u64),
        _ => Err(Value::Error(error_message.to_string())),
    }
}

fn parse_non_negative_usize(value: &Value, error_message: &str) -> Result<usize, Value> {
    parse_non_negative_u64(value, error_message).map(|number| number as usize)
}

/// Handle I/O-related function calls
/// Returns Some(value) if the function was handled, None if not recognized
pub fn handle(interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "print" => {
            let output_parts: Vec<String> =
                arg_values.iter().map(Interpreter::stringify_value).collect();
            interp.write_output(&output_parts.join(" "));
            Value::Null
        }

        "eprint" => {
            let output_parts: Vec<String> =
                arg_values.iter().map(Interpreter::stringify_value).collect();
            eprintln!("{}", output_parts.join(" "));
            Value::Null
        }

        "io_read_bytes" => {
            if 2 != arg_values.len() {
                Value::Error("io_read_bytes requires two arguments: path and count".to_string())
            } else if let (Some(Value::Str(path)), Some(count_value)) =
                (arg_values.first(), arg_values.get(1))
            {
                match parse_non_negative_usize(
                    count_value,
                    "io_read_bytes count must be non-negative",
                ) {
                    Ok(count) => match File::open(path.as_ref()) {
                        Ok(mut file) => {
                            let mut buffer = vec![0u8; count];
                            match file.read(&mut buffer) {
                                Ok(bytes_read) => {
                                    buffer.truncate(bytes_read);
                                    Value::Bytes(buffer)
                                }
                                Err(error) => Value::Error(format!(
                                    "Cannot read {} bytes from '{}': {}",
                                    count,
                                    path.as_ref(),
                                    error
                                )),
                            }
                        }
                        Err(error) => {
                            Value::Error(format!("Cannot open file '{}': {}", path.as_ref(), error))
                        }
                    },
                    Err(error) => error,
                }
            } else {
                Value::Error(
                    "io_read_bytes requires path (string) and count (int) arguments".to_string(),
                )
            }
        }

        "io_write_bytes" => {
            if 2 != arg_values.len() {
                Value::Error("io_write_bytes requires two arguments: path and bytes".to_string())
            } else if let (Some(Value::Str(path)), Some(Value::Bytes(bytes))) =
                (arg_values.first(), arg_values.get(1))
            {
                match fs::write(path.as_ref(), bytes) {
                    Ok(_) => Value::Bool(true),
                    Err(error) => Value::Error(format!(
                        "Cannot write bytes to file '{}': {}",
                        path.as_ref(),
                        error
                    )),
                }
            } else {
                Value::Error(
                    "io_write_bytes requires path (string) and bytes arguments".to_string(),
                )
            }
        }

        "io_append_bytes" => {
            if 2 != arg_values.len() {
                Value::Error("io_append_bytes requires two arguments: path and bytes".to_string())
            } else if let (Some(Value::Str(path)), Some(Value::Bytes(bytes))) =
                (arg_values.first(), arg_values.get(1))
            {
                match OpenOptions::new().create(true).append(true).open(path.as_ref()) {
                    Ok(mut file) => match file.write_all(bytes) {
                        Ok(_) => Value::Bool(true),
                        Err(error) => Value::Error(format!(
                            "Cannot append bytes to file '{}': {}",
                            path.as_ref(),
                            error
                        )),
                    },
                    Err(error) => {
                        Value::Error(format!("Cannot open file '{}': {}", path.as_ref(), error))
                    }
                }
            } else {
                Value::Error(
                    "io_append_bytes requires path (string) and bytes arguments".to_string(),
                )
            }
        }

        "io_read_at" => {
            if 3 != arg_values.len() {
                Value::Error(
                    "io_read_at requires three arguments: path, offset, and count".to_string(),
                )
            } else if let (Some(Value::Str(path)), Some(offset_value), Some(count_value)) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                let offset = match parse_non_negative_u64(
                    offset_value,
                    "io_read_at offset must be non-negative",
                ) {
                    Ok(value) => value,
                    Err(error) => return Some(error),
                };
                let count = match parse_non_negative_usize(
                    count_value,
                    "io_read_at count must be non-negative",
                ) {
                    Ok(value) => value,
                    Err(error) => return Some(error),
                };

                match File::open(path.as_ref()) {
                    Ok(mut file) => {
                        if let Err(error) = file.seek(SeekFrom::Start(offset)) {
                            return Some(Value::Error(format!(
                                "Cannot seek to offset {} in '{}': {}",
                                offset,
                                path.as_ref(),
                                error
                            )));
                        }

                        let mut buffer = vec![0u8; count];
                        match file.read(&mut buffer) {
                            Ok(bytes_read) => {
                                buffer.truncate(bytes_read);
                                Value::Bytes(buffer)
                            }
                            Err(error) => Value::Error(format!(
                                "Cannot read {} bytes at offset {} from '{}': {}",
                                count,
                                offset,
                                path.as_ref(),
                                error
                            )),
                        }
                    }
                    Err(error) => {
                        Value::Error(format!("Cannot open file '{}': {}", path.as_ref(), error))
                    }
                }
            } else {
                Value::Error(
                    "io_read_at requires path (string), offset (int), and count (int) arguments"
                        .to_string(),
                )
            }
        }

        "io_write_at" => {
            if 3 != arg_values.len() {
                Value::Error(
                    "io_write_at requires three arguments: path, bytes, and offset".to_string(),
                )
            } else if let (Some(Value::Str(path)), Some(Value::Bytes(bytes)), Some(offset_value)) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                let offset = match parse_non_negative_u64(
                    offset_value,
                    "io_write_at offset must be non-negative",
                ) {
                    Ok(value) => value,
                    Err(error) => return Some(error),
                };

                match OpenOptions::new().write(true).open(path.as_ref()) {
                    Ok(mut file) => {
                        if let Err(error) = file.seek(SeekFrom::Start(offset)) {
                            return Some(Value::Error(format!(
                                "Cannot seek to offset {} in '{}': {}",
                                offset,
                                path.as_ref(),
                                error
                            )));
                        }

                        match file.write_all(bytes) {
                            Ok(_) => Value::Bool(true),
                            Err(error) => Value::Error(format!(
                                "Cannot write bytes at offset {} to '{}': {}",
                                offset,
                                path.as_ref(),
                                error
                            )),
                        }
                    }
                    Err(error) => {
                        Value::Error(format!("Cannot open file '{}': {}", path.as_ref(), error))
                    }
                }
            } else {
                Value::Error(
                    "io_write_at requires path (string), bytes, and offset (int) arguments"
                        .to_string(),
                )
            }
        }

        "io_seek_read" => {
            if 2 != arg_values.len() {
                Value::Error("io_seek_read requires two arguments: path and offset".to_string())
            } else if let (Some(Value::Str(path)), Some(offset_value)) =
                (arg_values.first(), arg_values.get(1))
            {
                let offset = match parse_non_negative_u64(
                    offset_value,
                    "io_seek_read offset must be non-negative",
                ) {
                    Ok(value) => value,
                    Err(error) => return Some(error),
                };

                match File::open(path.as_ref()) {
                    Ok(mut file) => {
                        if let Err(error) = file.seek(SeekFrom::Start(offset)) {
                            return Some(Value::Error(format!(
                                "Cannot seek to offset {} in '{}': {}",
                                offset,
                                path.as_ref(),
                                error
                            )));
                        }

                        let mut buffer = Vec::new();
                        match file.read_to_end(&mut buffer) {
                            Ok(_) => Value::Bytes(buffer),
                            Err(error) => Value::Error(format!(
                                "Cannot read from offset {} in '{}': {}",
                                offset,
                                path.as_ref(),
                                error
                            )),
                        }
                    }
                    Err(error) => {
                        Value::Error(format!("Cannot open file '{}': {}", path.as_ref(), error))
                    }
                }
            } else {
                Value::Error(
                    "io_seek_read requires path (string) and offset (int) arguments".to_string(),
                )
            }
        }

        "io_file_metadata" => {
            if 1 != arg_values.len() {
                Value::Error("io_file_metadata requires a string path argument".to_string())
            } else if let Some(Value::Str(path)) = arg_values.first() {
                match fs::metadata(path.as_ref()) {
                    Ok(metadata) => {
                        let mut map = DictMap::default();

                        map.insert("size".into(), Value::Int(metadata.len() as i64));
                        map.insert("is_file".into(), Value::Bool(metadata.is_file()));
                        map.insert("is_dir".into(), Value::Bool(metadata.is_dir()));
                        map.insert(
                            "readonly".into(),
                            Value::Bool(metadata.permissions().readonly()),
                        );

                        #[cfg(unix)]
                        {
                            map.insert(
                                "device_id".into(),
                                Value::Str(Arc::new(metadata.dev().to_string())),
                            );
                            map.insert(
                                "mode".into(),
                                Value::Int((metadata.permissions().mode() & 0o7777) as i64),
                            );
                        }

                        if let Ok(modified) = metadata.modified() {
                            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                                map.insert(
                                    "modified".into(),
                                    Value::Int(duration.as_secs() as i64),
                                );
                            }
                        }

                        if let Ok(created) = metadata.created() {
                            if let Ok(duration) = created.duration_since(UNIX_EPOCH) {
                                map.insert("created".into(), Value::Int(duration.as_secs() as i64));
                            }
                        }

                        if let Ok(accessed) = metadata.accessed() {
                            if let Ok(duration) = accessed.duration_since(UNIX_EPOCH) {
                                map.insert(
                                    "accessed".into(),
                                    Value::Int(duration.as_secs() as i64),
                                );
                            }
                        }

                        Value::Dict(Arc::new(map))
                    }
                    Err(error) => Value::Error(format!(
                        "Cannot get metadata for '{}': {}",
                        path.as_ref(),
                        error
                    )),
                }
            } else {
                Value::Error("io_file_metadata requires a string path argument".to_string())
            }
        }

        "io_set_permissions" => {
            if 2 != arg_values.len() {
                Value::Error(
                    "io_set_permissions requires two arguments: path and POSIX mode".to_string(),
                )
            } else if let (Some(Value::Str(path)), Some(Value::Int(mode))) =
                (arg_values.first(), arg_values.get(1))
            {
                if *mode < 0 || *mode > 0o777 {
                    Value::Error(
                        "io_set_permissions mode must be between 0 and 511 (0o777)".to_string(),
                    )
                } else {
                    #[cfg(unix)]
                    {
                        match OpenOptions::new().read(true).open(path.as_ref()) {
                            Ok(file) => {
                                let requested = *mode as u32;
                                match file.set_permissions(fs::Permissions::from_mode(requested)) {
                                    Ok(_) => match file.metadata() {
                                        Ok(metadata) => {
                                            let actual = metadata.permissions().mode() & 0o777;
                                            let mut map = DictMap::default();
                                            map.insert(
                                                "requested_mode".into(),
                                                Value::Int(requested as i64),
                                            );
                                            map.insert(
                                                "actual_mode".into(),
                                                Value::Int(actual as i64),
                                            );
                                            map.insert(
                                                "verified".into(),
                                                Value::Bool(actual == requested),
                                            );
                                            Value::Dict(Arc::new(map))
                                        }
                                        Err(error) => Value::Error(format!(
                                            "Cannot verify permissions for '{}': {}",
                                            path.as_ref(),
                                            error
                                        )),
                                    },
                                    Err(error) => Value::Error(format!(
                                        "Cannot set permissions for '{}': {}",
                                        path.as_ref(),
                                        error
                                    )),
                                }
                            }
                            Err(error) => Value::Error(format!(
                                "Cannot open file '{}' for permission update: {}",
                                path.as_ref(),
                                error
                            )),
                        }
                    }

                    #[cfg(not(unix))]
                    {
                        Value::Error(
                            "io_set_permissions is unavailable on this platform; use a managed key provider"
                                .to_string(),
                        )
                    }
                }
            } else {
                Value::Error(
                    "io_set_permissions requires path (string) and mode (int) arguments"
                        .to_string(),
                )
            }
        }

        "io_write_private_file" => {
            if 3 != arg_values.len() {
                Value::Error(
                    "io_write_private_file requires path, content, and POSIX mode".to_string(),
                )
            } else if let (Some(Value::Str(path)), Some(content), Some(Value::Int(mode))) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                let bytes = match content {
                    Value::Str(value) => Some(value.as_bytes().to_vec()),
                    Value::Bytes(value) => Some(value.clone()),
                    _ => None,
                };
                if bytes.is_none() {
                    Value::Error(
                        "io_write_private_file content must be a string or bytes".to_string(),
                    )
                } else if *mode < 0 || *mode > 0o700 {
                    Value::Error(
                        "io_write_private_file mode must be between 0 and 448 (0o700)".to_string(),
                    )
                } else {
                    #[cfg(unix)]
                    {
                        let destination = std::path::Path::new(path.as_ref());
                        if destination.exists() {
                            Value::Error(format!(
                                "Refusing to overwrite private file '{}'",
                                destination.display()
                            ))
                        } else {
                            let parent =
                                destination.parent().unwrap_or_else(|| std::path::Path::new("."));
                            let temp =
                                parent.join(format!(".kujo-private-{}.tmp", uuid::Uuid::new_v4()));
                            let requested = *mode as u32;
                            let operation = (|| -> Result<Value, String> {
                                let mut file = OpenOptions::new()
                                    .write(true)
                                    .create_new(true)
                                    .mode(requested)
                                    .open(&temp)
                                    .map_err(|error| error.to_string())?;
                                file.set_permissions(fs::Permissions::from_mode(requested))
                                    .map_err(|error| error.to_string())?;
                                let actual = file
                                    .metadata()
                                    .map_err(|error| error.to_string())?
                                    .permissions()
                                    .mode()
                                    & 0o777;
                                if actual != requested {
                                    return Err(format!(
                                        "private file mode verification failed: requested {:o}, actual {:o}",
                                        requested, actual
                                    ));
                                }
                                file.write_all(&bytes.unwrap())
                                    .map_err(|error| error.to_string())?;
                                file.sync_all().map_err(|error| error.to_string())?;
                                drop(file);
                                fs::rename(&temp, destination)
                                    .map_err(|error| error.to_string())?;
                                let mut receipt = DictMap::default();
                                receipt.insert("mode".into(), Value::Int(actual as i64));
                                receipt.insert("verified".into(), Value::Bool(true));
                                receipt.insert(
                                    "path".into(),
                                    Value::Str(Arc::new(destination.to_string_lossy().to_string())),
                                );
                                Ok(Value::Dict(Arc::new(receipt)))
                            })();
                            if operation.is_err() {
                                let _ = fs::remove_file(&temp);
                            }
                            match operation {
                                Ok(value) => value,
                                Err(error) => Value::Error(format!(
                                    "Cannot atomically write private file '{}': {}",
                                    destination.display(),
                                    error
                                )),
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        Value::Error(
                            "io_write_private_file is unavailable on this platform; use a managed key provider"
                                .to_string(),
                        )
                    }
                }
            } else {
                Value::Error(
                    "io_write_private_file requires path (string), content (string or bytes), and mode (int)"
                        .to_string(),
                )
            }
        }

        "io_private_spool_open" => {
            if 3 != arg_values.len() {
                Value::Error(
                    "io_private_spool_open requires path, maximum bytes, and POSIX mode"
                        .to_string(),
                )
            } else if let (
                Some(Value::Str(path)),
                Some(Value::Int(max_bytes)),
                Some(Value::Int(mode)),
            ) = (arg_values.first(), arg_values.get(1), arg_values.get(2))
            {
                if *max_bytes <= 0 || *max_bytes as u64 > MAX_PRIVATE_SPOOL_BYTES {
                    Value::Error(format!(
                        "io_private_spool_open maximum bytes must be between 1 and {}",
                        MAX_PRIVATE_SPOOL_BYTES
                    ))
                } else if *mode < 0 || *mode > 0o700 {
                    Value::Error(
                        "io_private_spool_open mode must be between 0 and 448 (0o700)".to_string(),
                    )
                } else {
                    #[cfg(unix)]
                    {
                        let destination = std::path::Path::new(path.as_ref());
                        if destination.exists() {
                            Value::Error(format!(
                                "Refusing to overwrite private spool destination '{}'",
                                destination.display()
                            ))
                        } else {
                            let parent = private_spool_parent(destination);
                            let parent_metadata = match fs::metadata(parent) {
                                Ok(metadata) => metadata,
                                Err(error) => {
                                    return Some(Value::Error(format!(
                                        "Cannot inspect private spool directory '{}': {}",
                                        parent.display(),
                                        error
                                    )));
                                }
                            };
                            if !parent_metadata.is_dir()
                                || parent_metadata.permissions().mode() & 0o022 != 0
                            {
                                return Some(Value::Error(format!(
                                    "Refusing unsafe private spool directory '{}' (must be a directory without group/other write permission)",
                                    parent.display()
                                )));
                            }
                            let temp = parent
                                .join(format!(".kujo-private-spool-{}.tmp", uuid::Uuid::new_v4()));
                            let requested = *mode as u32;
                            match OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .mode(requested)
                                .open(&temp)
                            {
                                Ok(file) => {
                                    if let Err(error) =
                                        file.set_permissions(fs::Permissions::from_mode(requested))
                                    {
                                        let _ = fs::remove_file(&temp);
                                        Value::Error(format!(
                                            "Cannot set private spool permissions for '{}': {}",
                                            destination.display(),
                                            error
                                        ))
                                    } else {
                                        match file.metadata() {
                                            Ok(metadata)
                                                if metadata.permissions().mode() & 0o777
                                                    == requested =>
                                            {
                                                Value::PrivateSpool {
                                                    spool: Arc::new(std::sync::Mutex::new(Some(
                                                        PrivateSpoolState {
                                                            file: Some(file),
                                                            destination: destination.to_path_buf(),
                                                            temp,
                                                            mode: requested,
                                                            max_bytes: *max_bytes as u64,
                                                            bytes_written: 0,
                                                            hasher: Sha256::new(),
                                                        },
                                                    ))),
                                                    destination: destination
                                                        .to_string_lossy()
                                                        .to_string(),
                                                }
                                            }
                                            Ok(metadata) => {
                                                let actual = metadata.permissions().mode() & 0o777;
                                                drop(file);
                                                let _ = fs::remove_file(&temp);
                                                Value::Error(format!(
                                                    "private spool mode verification failed: requested {:o}, actual {:o}",
                                                    requested, actual
                                                ))
                                            }
                                            Err(error) => {
                                                drop(file);
                                                let _ = fs::remove_file(&temp);
                                                Value::Error(format!(
                                                    "Cannot verify private spool '{}': {}",
                                                    destination.display(),
                                                    error
                                                ))
                                            }
                                        }
                                    }
                                }
                                Err(error) => Value::Error(format!(
                                    "Cannot create private spool '{}': {}",
                                    destination.display(),
                                    error
                                )),
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        Value::Error(
                            "io_private_spool_open is unavailable on this platform; use a managed private storage provider"
                                .to_string(),
                        )
                    }
                }
            } else {
                Value::Error(
                    "io_private_spool_open requires path (string), maximum bytes (int), and mode (int)"
                        .to_string(),
                )
            }
        }

        "io_private_spool_write" => {
            if 2 != arg_values.len() {
                Value::Error("io_private_spool_write requires spool and content".to_string())
            } else if let Some(Value::PrivateSpool { spool, .. }) = arg_values.first() {
                match private_spool_content_bytes(&arg_values[1]) {
                    None => Value::Error(
                        "io_private_spool_write content must be a string or bytes".to_string(),
                    ),
                    Some(bytes) => {
                        let mut guard = match spool.lock() {
                            Ok(value) => value,
                            Err(_) => {
                                return Some(Value::Error(
                                    "io_private_spool_write spool lock is poisoned".to_string(),
                                ));
                            }
                        };
                        let Some(state) = guard.as_mut() else {
                            return Some(Value::Error(
                                "io_private_spool_write spool is closed".to_string(),
                            ));
                        };
                        let chunk_len = bytes.len() as u64;
                        let Some(next_size) = state.bytes_written.checked_add(chunk_len) else {
                            return Some(Value::Error(
                                "io_private_spool_write size overflow".to_string(),
                            ));
                        };
                        if next_size > state.max_bytes {
                            Value::Error(format!(
                                "io_private_spool_write exceeds maximum bytes ({} > {})",
                                next_size, state.max_bytes
                            ))
                        } else {
                            let Some(file) = state.file.as_mut() else {
                                return Some(Value::Error(
                                    "io_private_spool_write spool is closed".to_string(),
                                ));
                            };
                            match file.write_all(&bytes) {
                                Ok(()) => {
                                    state.hasher.update(&bytes);
                                    state.bytes_written = next_size;
                                    let mut receipt = DictMap::default();
                                    receipt.insert(
                                        "bytes_written".into(),
                                        Value::Int(next_size as i64),
                                    );
                                    receipt.insert(
                                        "remaining_bytes".into(),
                                        Value::Int((state.max_bytes - next_size) as i64),
                                    );
                                    Value::Dict(Arc::new(receipt))
                                }
                                Err(error) => Value::Error(format!(
                                    "Cannot write private spool '{}': {}",
                                    state.destination.display(),
                                    error
                                )),
                            }
                        }
                    }
                }
            } else {
                Value::Error(
                    "io_private_spool_write requires a private spool handle and string or bytes"
                        .to_string(),
                )
            }
        }

        #[cfg(unix)]
        "io_private_spool_write_file_range" => {
            if 5 != arg_values.len() {
                Value::Error(
                    "io_private_spool_write_file_range requires spool, path, offset, count, and mode"
                        .to_string(),
                )
            } else if let (
                Some(Value::PrivateSpool { spool, .. }),
                Some(Value::Str(path)),
                Some(Value::Int(offset)),
                Some(Value::Int(count)),
                Some(Value::Str(mode)),
            ) = (
                arg_values.first(),
                arg_values.get(1),
                arg_values.get(2),
                arg_values.get(3),
                arg_values.get(4),
            ) {
                if *offset < 0 || *count < 0 {
                    Value::Error(
                        "io_private_spool_write_file_range offset and count must be non-negative"
                            .to_string(),
                    )
                } else {
                    let mut guard = match spool.lock() {
                        Ok(value) => value,
                        Err(_) => {
                            return Some(Value::Error(
                                "io_private_spool_write_file_range spool lock is poisoned"
                                    .to_string(),
                            ));
                        }
                    };
                    let Some(state) = guard.as_mut() else {
                        return Some(Value::Error(
                            "io_private_spool_write_file_range spool is closed".to_string(),
                        ));
                    };
                    match private_spool_write_file_range(
                        state,
                        path.as_ref(),
                        *offset as u64,
                        *count as u64,
                        mode.as_ref(),
                    ) {
                        Ok(output_bytes) => {
                            let mut receipt = DictMap::default();
                            receipt.insert("input_bytes".into(), Value::Int(*count));
                            receipt.insert(
                                "output_bytes".into(),
                                Value::Int(output_bytes as i64),
                            );
                            receipt.insert(
                                "bytes_written".into(),
                                Value::Int(state.bytes_written as i64),
                            );
                            receipt.insert(
                                "remaining_bytes".into(),
                                Value::Int((state.max_bytes - state.bytes_written) as i64),
                            );
                            receipt.insert("mode".into(), Value::Str(mode.clone()));
                            Value::Dict(Arc::new(receipt))
                        }
                        Err(error) => Value::Error(error),
                    }
                }
            } else {
                Value::Error(
                    "io_private_spool_write_file_range requires (PrivateSpool, string, int, int, string) arguments"
                        .to_string(),
                )
            }
        }
        #[cfg(not(unix))]
        "io_private_spool_write_file_range" => Value::Error(
            "io_private_spool_write_file_range is unavailable on this platform; use a managed private storage provider"
                .to_string(),
        ),

        #[cfg(unix)]
        "io_private_spool_finish" => {
            if 1 != arg_values.len() {
                Value::Error("io_private_spool_finish requires one spool argument".to_string())
            } else if let Some(Value::PrivateSpool { spool, .. }) = arg_values.first() {
                let mut guard = match spool.lock() {
                    Ok(value) => value,
                    Err(_) => {
                        return Some(Value::Error(
                            "io_private_spool_finish spool lock is poisoned".to_string(),
                        ));
                    }
                };
                let Some(mut state) = guard.take() else {
                    return Some(Value::Error(
                        "io_private_spool_finish spool is closed".to_string(),
                    ));
                };
                let operation = (|| -> Result<Value, String> {
                    let Some(file) = state.file.take() else {
                        return Err("private spool file is closed".to_string());
                    };
                    file.sync_all().map_err(|error| error.to_string())?;
                    let metadata = file.metadata().map_err(|error| error.to_string())?;
                    let actual_mode = metadata.permissions().mode() & 0o777;
                    if actual_mode != state.mode {
                        return Err(format!(
                            "private spool mode verification failed: requested {:o}, actual {:o}",
                            state.mode, actual_mode
                        ));
                    }
                    if metadata.len() != state.bytes_written {
                        return Err("private spool size changed outside its handle".to_string());
                    }
                    let path_metadata =
                        fs::symlink_metadata(&state.temp).map_err(|error| error.to_string())?;
                    if !path_metadata.file_type().is_file()
                        || path_metadata.dev() != metadata.dev()
                        || path_metadata.ino() != metadata.ino()
                        || path_metadata.nlink() != 1
                    {
                        return Err(
                            "private spool temporary path no longer identifies the retained file"
                                .to_string(),
                        );
                    }
                    drop(file);
                    // Open the directory before publication so every operation that can
                    // unambiguously fail still does so while the destination is absent.
                    // Once hard_link succeeds, returning an error would leave callers
                    // unable to tell whether the destination was published. Post-publish
                    // cleanup and durability are therefore explicit receipt facts.
                    let parent = private_spool_parent(&state.destination);
                    let parent_file = File::open(parent).map_err(|error| error.to_string())?;
                    fs::hard_link(&state.temp, &state.destination)
                        .map_err(|error| error.to_string())?;
                    let temporary_removed = fs::remove_file(&state.temp).is_ok();
                    let directory_synced = parent_file.sync_all().is_ok();
                    let verified = temporary_removed && directory_synced;
                    let mut receipt = DictMap::default();
                    receipt.insert("mode".into(), Value::Int(actual_mode as i64));
                    receipt.insert("published".into(), Value::Bool(true));
                    receipt.insert("temporary_removed".into(), Value::Bool(temporary_removed));
                    receipt.insert("directory_synced".into(), Value::Bool(directory_synced));
                    receipt.insert("verified".into(), Value::Bool(verified));
                    receipt.insert(
                        "path".into(),
                        Value::Str(Arc::new(state.destination.to_string_lossy().to_string())),
                    );
                    receipt.insert("bytes_written".into(), Value::Int(state.bytes_written as i64));
                    receipt.insert(
                        "sha256".into(),
                        Value::Str(Arc::new(private_spool_digest_hex(state.hasher.clone()))),
                    );
                    Ok(Value::Dict(Arc::new(receipt)))
                })();
                match operation {
                    Ok(value) => value,
                    Err(error) => Value::Error(format!(
                        "Cannot publish private spool '{}': {}",
                        state.destination.display(),
                        error
                    )),
                }
            } else {
                Value::Error("io_private_spool_finish requires a private spool handle".to_string())
            }
        }
        #[cfg(not(unix))]
        "io_private_spool_finish" => Value::Error(
            "io_private_spool_finish is unavailable on this platform; use a managed private storage provider"
                .to_string(),
        ),

        "io_private_spool_abort" => {
            if 1 != arg_values.len() {
                Value::Error("io_private_spool_abort requires one spool argument".to_string())
            } else if let Some(Value::PrivateSpool { spool, .. }) = arg_values.first() {
                match spool.lock() {
                    Ok(mut guard) => {
                        if guard.take().is_some() {
                            Value::Bool(true)
                        } else {
                            Value::Bool(false)
                        }
                    }
                    Err(_) => {
                        Value::Error("io_private_spool_abort spool lock is poisoned".to_string())
                    }
                }
            } else {
                Value::Error("io_private_spool_abort requires a private spool handle".to_string())
            }
        }

        "io_truncate" => {
            if 2 != arg_values.len() {
                Value::Error("io_truncate requires two arguments: path and size".to_string())
            } else if let (Some(Value::Str(path)), Some(size_value)) =
                (arg_values.first(), arg_values.get(1))
            {
                let size = match parse_non_negative_u64(
                    size_value,
                    "io_truncate size must be non-negative",
                ) {
                    Ok(value) => value,
                    Err(error) => return Some(error),
                };

                match OpenOptions::new().write(true).open(path.as_ref()) {
                    Ok(file) => match file.set_len(size) {
                        Ok(_) => Value::Bool(true),
                        Err(error) => Value::Error(format!(
                            "Cannot truncate file '{}' to {} bytes: {}",
                            path.as_ref(),
                            size,
                            error
                        )),
                    },
                    Err(error) => {
                        Value::Error(format!("Cannot open file '{}': {}", path.as_ref(), error))
                    }
                }
            } else {
                Value::Error(
                    "io_truncate requires path (string) and size (int) arguments".to_string(),
                )
            }
        }

        "io_copy_range" => {
            if 4 != arg_values.len() {
                Value::Error(
                    "io_copy_range requires four arguments: source, dest, offset, and count"
                        .to_string(),
                )
            } else if let (
                Some(Value::Str(source)),
                Some(Value::Str(dest)),
                Some(offset_value),
                Some(count_value),
            ) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2), arg_values.get(3))
            {
                let offset = match parse_non_negative_u64(
                    offset_value,
                    "io_copy_range offset must be non-negative",
                ) {
                    Ok(value) => value,
                    Err(error) => return Some(error),
                };
                let count = match parse_non_negative_usize(
                    count_value,
                    "io_copy_range count must be non-negative",
                ) {
                    Ok(value) => value,
                    Err(error) => return Some(error),
                };

                match File::open(source.as_ref()) {
                    Ok(mut source_file) => {
                        if let Err(error) = source_file.seek(SeekFrom::Start(offset)) {
                            return Some(Value::Error(format!(
                                "Cannot seek to offset {} in '{}': {}",
                                offset,
                                source.as_ref(),
                                error
                            )));
                        }

                        let mut buffer = vec![0u8; count];
                        match source_file.read(&mut buffer) {
                            Ok(bytes_read) => {
                                buffer.truncate(bytes_read);
                                match File::create(dest.as_ref()) {
                                    Ok(mut dest_file) => match dest_file.write_all(&buffer) {
                                        Ok(_) => Value::Bool(true),
                                        Err(error) => Value::Error(format!(
                                            "Cannot write to '{}': {}",
                                            dest.as_ref(),
                                            error
                                        )),
                                    },
                                    Err(error) => Value::Error(format!(
                                        "Cannot create file '{}': {}",
                                        dest.as_ref(),
                                        error
                                    )),
                                }
                            }
                            Err(error) => Value::Error(format!(
                                "Cannot read from '{}': {}",
                                source.as_ref(),
                                error
                            )),
                        }
                    }
                    Err(error) => Value::Error(format!(
                        "Cannot open source file '{}': {}",
                        source.as_ref(),
                        error
                    )),
                }
            } else {
                Value::Error(
                    "io_copy_range requires source (string), dest (string), offset (int), and count (int) arguments".to_string(),
                )
            }
        }

        _ => return None,
    };
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_test_path(file_name: &str) -> String {
        let mut path = std::env::current_dir().expect("current_dir should resolve");
        path.push("tmp");
        path.push("native_io_tests");
        std::fs::create_dir_all(&path).expect("test tmp dir should be created");
        path.push(file_name);
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_io_read_write_append_bytes_round_trip() {
        let mut interpreter = Interpreter::new();
        let path = tmp_test_path("io_round_trip.bin");

        let write_result = handle(
            &mut interpreter,
            "io_write_bytes",
            &[Value::Str(Arc::new(path.clone())), Value::Bytes(vec![1, 2, 3])],
        )
        .unwrap();
        assert!(matches!(write_result, Value::Bool(true)));

        let append_result = handle(
            &mut interpreter,
            "io_append_bytes",
            &[Value::Str(Arc::new(path.clone())), Value::Bytes(vec![4, 5])],
        )
        .unwrap();
        assert!(matches!(append_result, Value::Bool(true)));

        let read_result = handle(
            &mut interpreter,
            "io_read_bytes",
            &[Value::Str(Arc::new(path.clone())), Value::Int(10)],
        )
        .unwrap();
        assert!(matches!(read_result, Value::Bytes(bytes) if bytes == vec![1, 2, 3, 4, 5]));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_io_eprint_returns_null() {
        let mut interpreter = Interpreter::new();
        let result =
            handle(&mut interpreter, "eprint", &[Value::Str(Arc::new("err".to_string()))]).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[cfg(unix)]
    #[test]
    fn test_io_metadata_and_permissions_are_device_bound_and_verified() {
        let mut interpreter = Interpreter::new();
        let path = tmp_test_path("io_permissions.pem");
        std::fs::write(&path, b"private fixture").expect("permission fixture should be written");

        let updated = handle(
            &mut interpreter,
            "io_set_permissions",
            &[Value::Str(Arc::new(path.clone())), Value::Int(0o600)],
        )
        .expect("permission function should be handled");
        assert!(matches!(updated, Value::Dict(ref fields)
            if matches!(fields.get("requested_mode"), Some(Value::Int(0o600)))
                && matches!(fields.get("actual_mode"), Some(Value::Int(0o600)))
                && matches!(fields.get("verified"), Some(Value::Bool(true)))));

        let metadata =
            handle(&mut interpreter, "io_file_metadata", &[Value::Str(Arc::new(path.clone()))])
                .expect("metadata function should be handled");
        assert!(matches!(metadata, Value::Dict(ref fields)
            if matches!(fields.get("mode"), Some(Value::Int(0o600)))
                && matches!(fields.get("device_id"), Some(Value::Str(value)) if !value.is_empty())));

        let rejected = handle(
            &mut interpreter,
            "io_set_permissions",
            &[Value::Str(Arc::new(path.clone())), Value::Int(0o1777)],
        )
        .expect("permission function should be handled");
        assert!(matches!(rejected, Value::Error(message) if message.contains("between 0 and 511")));

        let _ = std::fs::remove_file(path);
    }

    #[cfg(unix)]
    #[test]
    fn test_io_write_private_file_is_atomic_restrictive_and_no_overwrite() {
        let path = tmp_test_path("io_private_atomic.pem");
        let _ = fs::remove_file(&path);
        let written = handle(
            &mut Interpreter::new(),
            "io_write_private_file",
            &[
                Value::Str(Arc::new(path.clone())),
                Value::Str(Arc::new("private-material".to_string())),
                Value::Int(0o600),
            ],
        )
        .expect("private write should be handled");
        assert!(
            matches!(written, Value::Dict(ref fields) if matches!(fields.get("verified"), Some(Value::Bool(true))))
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "private-material");
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        let overwrite = handle(
            &mut Interpreter::new(),
            "io_write_private_file",
            &[
                Value::Str(Arc::new(path.clone())),
                Value::Str(Arc::new("replacement".to_string())),
                Value::Int(0o600),
            ],
        )
        .unwrap();
        assert!(
            matches!(overwrite, Value::Error(message) if message.contains("Refusing to overwrite"))
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "private-material");
        fs::remove_file(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_io_private_spool_is_bounded_private_atomic_and_single_use() {
        let path = tmp_test_path("io_private_spool.eml");
        let _ = fs::remove_file(&path);
        let spool = handle(
            &mut Interpreter::new(),
            "io_private_spool_open",
            &[Value::Str(Arc::new(path.clone())), Value::Int(5), Value::Int(0o600)],
        )
        .expect("spool open should be handled");
        assert!(matches!(spool, Value::PrivateSpool { .. }));
        assert!(!std::path::Path::new(&path).exists());

        let first = handle(
            &mut Interpreter::new(),
            "io_private_spool_write",
            &[spool.clone(), Value::Str(Arc::new("abc".to_string()))],
        )
        .expect("spool write should be handled");
        assert!(matches!(first, Value::Dict(ref fields)
            if matches!(fields.get("bytes_written"), Some(Value::Int(3)))
                && matches!(fields.get("remaining_bytes"), Some(Value::Int(2)))));

        let second = handle(
            &mut Interpreter::new(),
            "io_private_spool_write",
            &[spool.clone(), Value::Bytes(vec![0, 255])],
        )
        .expect("binary spool write should be handled");
        assert!(matches!(second, Value::Dict(ref fields)
            if matches!(fields.get("bytes_written"), Some(Value::Int(5)))));

        let overflow = handle(
            &mut Interpreter::new(),
            "io_private_spool_write",
            &[spool.clone(), Value::Bytes(vec![1])],
        )
        .expect("overflow should be handled");
        assert!(
            matches!(overflow, Value::Error(message) if message.contains("exceeds maximum bytes"))
        );

        let finished = handle(&mut Interpreter::new(), "io_private_spool_finish", &[spool.clone()])
            .expect("spool finish should be handled");
        assert!(matches!(finished, Value::Dict(ref fields)
            if matches!(fields.get("verified"), Some(Value::Bool(true)))
                && matches!(fields.get("published"), Some(Value::Bool(true)))
                && matches!(fields.get("temporary_removed"), Some(Value::Bool(true)))
                && matches!(fields.get("directory_synced"), Some(Value::Bool(true)))
                && matches!(fields.get("mode"), Some(Value::Int(0o600)))
                && matches!(fields.get("bytes_written"), Some(Value::Int(5)))
                && matches!(fields.get("sha256"), Some(Value::Str(value)) if value.len() == 64)));
        assert_eq!(fs::read(&path).unwrap(), vec![b'a', b'b', b'c', 0, 255]);
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);

        let closed = handle(&mut Interpreter::new(), "io_private_spool_finish", &[spool])
            .expect("second finish should be handled");
        assert!(matches!(closed, Value::Error(message) if message.contains("spool is closed")));
        fs::remove_file(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_io_private_spool_write_file_range_streams_all_modes_and_denies_symlinks() {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let source = directory.path().join("source.bin");
        fs::write(&source, b".first\nsecond\rthird\r\n.fourth").unwrap();

        let write_mode = |mode: &str, destination_name: &str| -> Vec<u8> {
            let destination = directory.path().join(destination_name);
            let spool = handle(
                &mut Interpreter::new(),
                "io_private_spool_open",
                &[
                    Value::Str(Arc::new(destination.to_string_lossy().to_string())),
                    Value::Int(1024),
                    Value::Int(0o600),
                ],
            )
            .unwrap();
            let progress = handle(
                &mut Interpreter::new(),
                "io_private_spool_write_file_range",
                &[
                    spool.clone(),
                    Value::Str(Arc::new(source.to_string_lossy().to_string())),
                    Value::Int(0),
                    Value::Int(fs::metadata(&source).unwrap().len() as i64),
                    Value::Str(Arc::new(mode.to_string())),
                ],
            )
            .unwrap();
            assert!(matches!(progress, Value::Dict(ref fields)
                if matches!(fields.get("input_bytes"), Some(Value::Int(28)))
                    && matches!(fields.get("mode"), Some(Value::Str(value)) if value.as_ref() == mode)));
            let finished =
                handle(&mut Interpreter::new(), "io_private_spool_finish", &[spool]).unwrap();
            assert!(matches!(finished, Value::Dict(ref fields)
                if matches!(fields.get("verified"), Some(Value::Bool(true)))));
            fs::read(destination).unwrap()
        };

        assert_eq!(write_mode("raw", "raw.bin"), b".first\nsecond\rthird\r\n.fourth");
        assert_eq!(write_mode("crlf", "crlf.bin"), b".first\r\nsecond\r\nthird\r\n.fourth");
        assert_eq!(
            write_mode("smtp-dot-stuff-crlf", "smtp.bin"),
            b"..first\r\nsecond\r\nthird\r\n..fourth\r\n"
        );
        assert_eq!(
            write_mode("base64-crlf", "base64.bin"),
            b"LmZpcnN0CnNlY29uZA10aGlyZA0KLmZvdXJ0aA==\r\n"
        );

        let link = directory.path().join("source-link.bin");
        std::os::unix::fs::symlink(&source, &link).unwrap();
        let denied_destination = directory.path().join("denied.bin");
        let spool = handle(
            &mut Interpreter::new(),
            "io_private_spool_open",
            &[
                Value::Str(Arc::new(denied_destination.to_string_lossy().to_string())),
                Value::Int(1024),
                Value::Int(0o600),
            ],
        )
        .unwrap();
        let denied = handle(
            &mut Interpreter::new(),
            "io_private_spool_write_file_range",
            &[
                spool.clone(),
                Value::Str(Arc::new(link.to_string_lossy().to_string())),
                Value::Int(0),
                Value::Int(28),
                Value::Str(Arc::new("raw".to_string())),
            ],
        )
        .unwrap();
        assert!(matches!(denied, Value::Error(message) if message.contains("non-symlink")));
        assert!(matches!(
            handle(&mut Interpreter::new(), "io_private_spool_abort", &[spool]).unwrap(),
            Value::Bool(true)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_io_private_spool_abort_drop_and_publish_race_fail_closed() {
        let abort_path = tmp_test_path("io_private_spool_abort.eml");
        let _ = fs::remove_file(&abort_path);
        let spool = handle(
            &mut Interpreter::new(),
            "io_private_spool_open",
            &[Value::Str(Arc::new(abort_path.clone())), Value::Int(16), Value::Int(0o600)],
        )
        .unwrap();
        let temp_path = match &spool {
            Value::PrivateSpool { spool, .. } => {
                spool.lock().unwrap().as_ref().unwrap().temp.clone()
            }
            _ => panic!("expected spool"),
        };
        assert!(temp_path.exists());
        let aborted = handle(&mut Interpreter::new(), "io_private_spool_abort", &[spool]).unwrap();
        assert!(matches!(aborted, Value::Bool(true)));
        assert!(!temp_path.exists());
        assert!(!std::path::Path::new(&abort_path).exists());

        let race_path = tmp_test_path("io_private_spool_publish_race.eml");
        let _ = fs::remove_file(&race_path);
        let race_spool = handle(
            &mut Interpreter::new(),
            "io_private_spool_open",
            &[Value::Str(Arc::new(race_path.clone())), Value::Int(16), Value::Int(0o600)],
        )
        .unwrap();
        let write = handle(
            &mut Interpreter::new(),
            "io_private_spool_write",
            &[race_spool.clone(), Value::Str(Arc::new("private".to_string()))],
        )
        .unwrap();
        assert!(matches!(write, Value::Dict(_)));
        fs::write(&race_path, b"existing").unwrap();
        let refused =
            handle(&mut Interpreter::new(), "io_private_spool_finish", &[race_spool]).unwrap();
        assert!(matches!(refused, Value::Error(message) if message.contains("Cannot publish")));
        assert_eq!(fs::read(&race_path).unwrap(), b"existing");
        fs::remove_file(race_path).unwrap();

        let unsafe_directory = tmp_test_path("unsafe_private_spool_parent");
        let _ = fs::remove_dir_all(&unsafe_directory);
        fs::create_dir_all(&unsafe_directory).unwrap();
        fs::set_permissions(&unsafe_directory, fs::Permissions::from_mode(0o777)).unwrap();
        let unsafe_path = std::path::Path::new(&unsafe_directory).join("message.eml");
        let refused = handle(
            &mut Interpreter::new(),
            "io_private_spool_open",
            &[
                Value::Str(Arc::new(unsafe_path.to_string_lossy().to_string())),
                Value::Int(16),
                Value::Int(0o600),
            ],
        )
        .unwrap();
        assert!(
            matches!(refused, Value::Error(message) if message.contains("unsafe private spool directory"))
        );
        fs::set_permissions(&unsafe_directory, fs::Permissions::from_mode(0o700)).unwrap();
        fs::remove_dir_all(unsafe_directory).unwrap();
    }

    #[test]
    fn test_io_read_at_write_at_and_seek_read() {
        let mut interpreter = Interpreter::new();
        let path = tmp_test_path("io_offset_ops.bin");
        std::fs::write(&path, vec![10u8, 20, 30, 40, 50]).expect("seed file should be written");

        let read_at = handle(
            &mut interpreter,
            "io_read_at",
            &[Value::Str(Arc::new(path.clone())), Value::Int(1), Value::Int(3)],
        )
        .unwrap();
        assert!(matches!(read_at, Value::Bytes(bytes) if bytes == vec![20, 30, 40]));

        let write_at = handle(
            &mut interpreter,
            "io_write_at",
            &[Value::Str(Arc::new(path.clone())), Value::Bytes(vec![99, 88]), Value::Int(2)],
        )
        .unwrap();
        assert!(matches!(write_at, Value::Bool(true)));

        let seek_read = handle(
            &mut interpreter,
            "io_seek_read",
            &[Value::Str(Arc::new(path.clone())), Value::Int(1)],
        )
        .unwrap();
        assert!(matches!(seek_read, Value::Bytes(bytes) if bytes == vec![20, 99, 88, 50]));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_io_file_metadata_and_truncate() {
        let mut interpreter = Interpreter::new();
        let path = tmp_test_path("io_metadata.bin");
        std::fs::write(&path, vec![1u8, 2, 3, 4, 5, 6]).expect("seed file should be written");

        let metadata_result =
            handle(&mut interpreter, "io_file_metadata", &[Value::Str(Arc::new(path.clone()))])
                .unwrap();

        match metadata_result {
            Value::Dict(map) => {
                assert!(matches!(map.get("is_file"), Some(Value::Bool(true))));
                assert!(matches!(map.get("size"), Some(Value::Int(6))));
            }
            _ => panic!("Expected metadata dictionary"),
        }

        let truncate_result = handle(
            &mut interpreter,
            "io_truncate",
            &[Value::Str(Arc::new(path.clone())), Value::Int(3)],
        )
        .unwrap();
        assert!(matches!(truncate_result, Value::Bool(true)));

        let bytes = std::fs::read(&path).expect("truncated file should be readable");
        assert_eq!(bytes, vec![1u8, 2, 3]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_io_copy_range() {
        let mut interpreter = Interpreter::new();
        let source = tmp_test_path("io_copy_source.bin");
        let dest = tmp_test_path("io_copy_dest.bin");
        std::fs::write(&source, vec![7u8, 8, 9, 10, 11]).expect("source file should be written");

        let result = handle(
            &mut interpreter,
            "io_copy_range",
            &[
                Value::Str(Arc::new(source.clone())),
                Value::Str(Arc::new(dest.clone())),
                Value::Int(1),
                Value::Int(3),
            ],
        )
        .unwrap();
        assert!(matches!(result, Value::Bool(true)));

        let copied = std::fs::read(&dest).expect("destination file should be readable");
        assert_eq!(copied, vec![8u8, 9, 10]);

        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(dest);
    }

    #[test]
    fn test_io_argument_shape_errors() {
        let mut interpreter = Interpreter::new();

        let read_error = handle(
            &mut interpreter,
            "io_read_bytes",
            &[Value::Str(Arc::new("x".to_string())), Value::Int(-1)],
        )
        .unwrap();
        assert!(
            matches!(read_error, Value::Error(message) if message.contains("count must be non-negative"))
        );

        let write_error = handle(
            &mut interpreter,
            "io_write_bytes",
            &[Value::Str(Arc::new("x".to_string())), Value::Int(1)],
        )
        .unwrap();
        assert!(
            matches!(write_error, Value::Error(message) if message.contains("requires path (string) and bytes"))
        );

        let read_at_error = handle(
            &mut interpreter,
            "io_read_at",
            &[Value::Str(Arc::new("x".to_string())), Value::Int(-1), Value::Int(1)],
        )
        .unwrap();
        assert!(
            matches!(read_at_error, Value::Error(message) if message.contains("offset must be non-negative"))
        );
    }

    #[test]
    fn test_io_strict_arity_rejects_trailing_arguments() {
        let mut interpreter = Interpreter::new();

        let strict_arity_cases: Vec<(&str, Vec<Value>, &str)> = vec![
            (
                "io_read_bytes",
                vec![
                    Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                    Value::Int(1),
                    Value::Int(99),
                ],
                "io_read_bytes requires two arguments: path and count",
            ),
            (
                "io_write_bytes",
                vec![
                    Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                    Value::Bytes(vec![]),
                    Value::Int(99),
                ],
                "io_write_bytes requires two arguments: path and bytes",
            ),
            (
                "io_append_bytes",
                vec![
                    Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                    Value::Bytes(vec![]),
                    Value::Int(99),
                ],
                "io_append_bytes requires two arguments: path and bytes",
            ),
            (
                "io_read_at",
                vec![
                    Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                    Value::Int(0),
                    Value::Int(1),
                    Value::Int(99),
                ],
                "io_read_at requires three arguments: path, offset, and count",
            ),
            (
                "io_write_at",
                vec![
                    Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                    Value::Bytes(vec![]),
                    Value::Int(0),
                    Value::Int(99),
                ],
                "io_write_at requires three arguments: path, bytes, and offset",
            ),
            (
                "io_seek_read",
                vec![
                    Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                    Value::Int(0),
                    Value::Int(99),
                ],
                "io_seek_read requires two arguments: path and offset",
            ),
            (
                "io_file_metadata",
                vec![Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())), Value::Int(99)],
                "io_file_metadata requires a string path argument",
            ),
            (
                "io_truncate",
                vec![
                    Value::Str(Arc::new("/tmp/kujo_io.bin".to_string())),
                    Value::Int(0),
                    Value::Int(99),
                ],
                "io_truncate requires two arguments: path and size",
            ),
            (
                "io_copy_range",
                vec![
                    Value::Str(Arc::new("/tmp/source.bin".to_string())),
                    Value::Str(Arc::new("/tmp/dest.bin".to_string())),
                    Value::Int(0),
                    Value::Int(1),
                    Value::Int(99),
                ],
                "io_copy_range requires four arguments: source, dest, offset, and count",
            ),
        ];

        for (name, args, expected_message) in strict_arity_cases {
            let result =
                handle(&mut interpreter, name, &args).expect("io function should be handled");
            assert!(
                matches!(result, Value::Error(ref message) if message.contains(expected_message)),
                "Expected strict-arity rejection for {} with message containing '{}', got {:?}",
                name,
                expected_message,
                result
            );
        }
    }
}
