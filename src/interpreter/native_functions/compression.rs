#[cfg(feature = "runtime-archive")]
use super::super::DictMap;
use super::super::Value;
#[cfg(feature = "runtime-archive")]
use std::sync::Arc;

#[cfg(feature = "runtime-archive")]
const MAX_DECOMPRESSED_BYTES: usize = 64 * 1024 * 1024;

#[cfg(feature = "runtime-archive")]
fn compression_string(value: impl Into<String>) -> Value {
    Value::Str(Arc::new(value.into()))
}

#[cfg(feature = "runtime-archive")]
fn validated_input<'a>(name: &str, arg_values: &'a [Value]) -> Result<(&'a [u8], usize), Value> {
    if arg_values.len() != 2 {
        return Err(Value::Error(format!(
            "{} requires (bytes_data, int_max_output_bytes) arguments",
            name
        )));
    }
    let bytes = match &arg_values[0] {
        Value::Bytes(bytes) => bytes.as_slice(),
        _ => {
            return Err(Value::Error(format!(
                "{} requires (bytes_data, int_max_output_bytes) arguments",
                name
            )))
        }
    };
    let maximum = match arg_values[1] {
        Value::Int(value) if value > 0 && value as usize <= MAX_DECOMPRESSED_BYTES => {
            value as usize
        }
        _ => {
            return Err(Value::Error(format!(
                "{} max_output_bytes must be between 1 and {}",
                name, MAX_DECOMPRESSED_BYTES
            )))
        }
    };
    Ok((bytes, maximum))
}

#[cfg(feature = "runtime-archive")]
fn read_bounded(reader: impl std::io::Read, maximum: usize, name: &str) -> Result<Vec<u8>, Value> {
    use std::io::Read;
    let mut output = Vec::with_capacity(maximum.min(64 * 1024));
    reader
        .take((maximum as u64).saturating_add(1))
        .read_to_end(&mut output)
        .map_err(|error| Value::Error(format!("{} failed: {}", name, error)))?;
    if output.len() > maximum {
        return Err(Value::Error(format!("{} output exceeds max_output_bytes {}", name, maximum)));
    }
    Ok(output)
}

#[cfg(feature = "runtime-archive")]
fn gzip_decompress(arg_values: &[Value]) -> Value {
    use flate2::read::MultiGzDecoder;
    let (bytes, maximum) = match validated_input("gzip_decompress", arg_values) {
        Ok(values) => values,
        Err(error) => return error,
    };
    if bytes.len() < 2 || bytes[..2] != [0x1f, 0x8b] {
        return Value::Error("gzip_decompress requires a valid gzip stream".to_string());
    }
    match read_bounded(MultiGzDecoder::new(bytes), maximum, "gzip_decompress") {
        Ok(output) => Value::Bytes(output),
        Err(error) => error,
    }
}

#[cfg(feature = "runtime-archive")]
fn gzip_compress(arg_values: &[Value]) -> Value {
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;
    if arg_values.len() != 1 {
        return Value::Error("gzip_compress requires one bytes argument".to_string());
    }
    let bytes = match &arg_values[0] {
        Value::Bytes(bytes) if bytes.len() <= MAX_DECOMPRESSED_BYTES => bytes,
        Value::Bytes(_) => {
            return Value::Error(format!(
                "gzip_compress input exceeds maximum {}",
                MAX_DECOMPRESSED_BYTES
            ))
        }
        _ => return Value::Error("gzip_compress requires one bytes argument".to_string()),
    };
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    if let Err(error) = encoder.write_all(bytes) {
        return Value::Error(format!("gzip_compress failed: {}", error));
    }
    match encoder.finish() {
        Ok(output) => Value::Bytes(output),
        Err(error) => Value::Error(format!("gzip_compress failed: {}", error)),
    }
}

#[cfg(feature = "runtime-archive")]
fn zip_single_file_read(arg_values: &[Value]) -> Value {
    use std::io::Cursor;
    use zip::ZipArchive;

    let (bytes, maximum) = match validated_input("zip_single_file_read", arg_values) {
        Ok(values) => values,
        Err(error) => return error,
    };
    let mut archive = match ZipArchive::new(Cursor::new(bytes)) {
        Ok(archive) => archive,
        Err(error) => {
            return Value::Error(format!("zip_single_file_read invalid archive: {}", error))
        }
    };
    if archive.len() != 1 {
        return Value::Error(
            "zip_single_file_read requires exactly one regular file entry".to_string(),
        );
    }
    let mut entry = match archive.by_index(0) {
        Ok(entry) => entry,
        Err(error) => return Value::Error(format!("zip_single_file_read failed: {}", error)),
    };
    let name = entry.name().to_string();
    if entry.is_dir()
        || entry.unix_mode().map(|mode| (mode & 0o170000) == 0o120000).unwrap_or(false)
        || crate::path_security::sanitize_relative_path(&name, "zip entry").is_err()
    {
        return Value::Error(
            "zip_single_file_read requires one safe regular file entry".to_string(),
        );
    }
    if entry.size() > maximum as u64 {
        return Value::Error(format!(
            "zip_single_file_read output exceeds max_output_bytes {}",
            maximum
        ));
    }
    let compressed_bytes = entry.compressed_size();
    let output = match read_bounded(&mut entry, maximum, "zip_single_file_read") {
        Ok(output) => output,
        Err(error) => return error,
    };
    let mut result = DictMap::default();
    result.insert("schema_version".into(), compression_string("kujo.compression.zip-single.v1"));
    result.insert("name".into(), compression_string(name));
    result.insert("compressed_bytes".into(), Value::Int(compressed_bytes as i64));
    result.insert("uncompressed_bytes".into(), Value::Int(output.len() as i64));
    result.insert("bytes".into(), Value::Bytes(output));
    Value::dict(result)
}

pub fn handle(name: &str, _arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        #[cfg(feature = "runtime-archive")]
        "gzip_compress" => gzip_compress(_arg_values),
        #[cfg(feature = "runtime-archive")]
        "gzip_decompress" => gzip_decompress(_arg_values),
        #[cfg(feature = "runtime-archive")]
        "zip_single_file_read" => zip_single_file_read(_arg_values),
        #[cfg(not(feature = "runtime-archive"))]
        "gzip_compress" | "gzip_decompress" | "zip_single_file_read" => Value::Error(
            "Compression native APIs are disabled in this build (enable the 'runtime-archive' feature)"
                .to_string(),
        ),
        _ => return None,
    };
    Some(result)
}

#[cfg(all(test, feature = "runtime-archive"))]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    fn gzip_bytes(input: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn gzip_is_bounded_and_binary_safe() {
        let compressed = gzip_bytes(b"kujo email");
        let result = gzip_decompress(&[Value::Bytes(compressed.clone()), Value::Int(64)]);
        assert!(matches!(result, Value::Bytes(bytes) if bytes == b"kujo email"));
        let bounded = gzip_decompress(&[Value::Bytes(compressed), Value::Int(4)]);
        assert!(
            matches!(bounded, Value::Error(message) if message.contains("exceeds max_output_bytes"))
        );
    }

    #[test]
    fn gzip_compress_round_trips_binary_bytes() {
        let original = Value::Bytes(vec![0, 1, 0x80, 0xff]);
        let compressed = gzip_compress(std::slice::from_ref(&original));
        let Value::Bytes(compressed) = compressed else { panic!("expected compressed bytes") };
        let decompressed = gzip_decompress(&[Value::Bytes(compressed), Value::Int(64)]);
        assert!(matches!(decompressed, Value::Bytes(bytes) if bytes == vec![0, 1, 0x80, 0xff]));
    }

    #[test]
    fn zip_requires_one_safe_regular_file() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        writer.start_file("report.xml", SimpleFileOptions::default()).unwrap();
        writer.write_all(b"<feedback/>").unwrap();
        let archive = writer.finish().unwrap().into_inner();
        let result = zip_single_file_read(&[Value::Bytes(archive), Value::Int(64)]);
        let Value::Dict(result) = result else { panic!("expected dictionary") };
        assert!(
            matches!(result.get("name"), Some(Value::Str(name)) if name.as_ref() == "report.xml")
        );
        assert!(
            matches!(result.get("bytes"), Some(Value::Bytes(bytes)) if bytes == b"<feedback/>")
        );
    }

    #[test]
    fn zip_rejects_multiple_and_traversing_entries() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        writer.start_file("one.xml", SimpleFileOptions::default()).unwrap();
        writer.write_all(b"one").unwrap();
        writer.start_file("two.xml", SimpleFileOptions::default()).unwrap();
        writer.write_all(b"two").unwrap();
        let multiple = writer.finish().unwrap().into_inner();
        let result = zip_single_file_read(&[Value::Bytes(multiple), Value::Int(64)]);
        assert!(matches!(result, Value::Error(message) if message.contains("exactly one")));

        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        writer.start_file("../report.xml", SimpleFileOptions::default()).unwrap();
        writer.write_all(b"unsafe").unwrap();
        let traversal = writer.finish().unwrap().into_inner();
        let result = zip_single_file_read(&[Value::Bytes(traversal), Value::Int(64)]);
        assert!(matches!(result, Value::Error(message) if message.contains("safe regular file")));
    }
}
