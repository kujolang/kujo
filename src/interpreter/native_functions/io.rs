// File: src/interpreter/native_functions/io.rs
//
// I/O-related native functions (print, input, etc.)

use crate::interpreter::{DictMap, Interpreter, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

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
