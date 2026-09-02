use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
#[cfg(unix)]
use cap_std::fs::{MetadataExt as CapMetadataExt, OpenOptionsExt as CapOpenOptionsExt};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

pub const DEFAULT_ROUTED_HTTP_READ_TIMEOUT_MS: u64 = 10_000;
pub const MIN_ROUTED_HTTP_READ_TIMEOUT_MS: u64 = 100;
pub const MAX_ROUTED_HTTP_READ_TIMEOUT_MS: u64 = 300_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpRequestBodyError {
    TooLarge,
    TimedOut,
    ReadFailed,
    InvalidSpoolPolicy,
    SpoolFailed,
}

pub const MAX_ROUTED_HTTP_UPLOAD_BYTES: u64 = 64 * 1024 * 1024;
const ROUTED_HTTP_UPLOAD_CHUNK_BYTES: usize = 64 * 1024;
const MAX_ROUTED_HTTP_UPLOAD_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestBodySpool {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

/// Stream a routed upload into a private same-directory artifact. The caller
/// owns the returned file and must either adopt it or remove it after dispatch.
/// Partial files are removed on every error path.
pub fn spool_bounded_http_request_body(
    reader: &mut dyn Read,
    declared_length: Option<usize>,
    spool_directory: &str,
    max_body_bytes: u64,
) -> Result<HttpRequestBodySpool, HttpRequestBodyError> {
    #[cfg(not(unix))]
    {
        let _ = (reader, declared_length, spool_directory, max_body_bytes);
        return Err(HttpRequestBodyError::InvalidSpoolPolicy);
    }

    if max_body_bytes == 0
        || max_body_bytes > MAX_ROUTED_HTTP_UPLOAD_BYTES
        || spool_directory.is_empty()
        || spool_directory.len() > MAX_ROUTED_HTTP_UPLOAD_PATH_BYTES
    {
        return Err(HttpRequestBodyError::InvalidSpoolPolicy);
    }
    if declared_length.is_some_and(|length| length as u64 > max_body_bytes) {
        return Err(HttpRequestBodyError::TooLarge);
    }

    let directory = Path::new(spool_directory);
    let metadata = std::fs::symlink_metadata(directory)
        .map_err(|_| HttpRequestBodyError::InvalidSpoolPolicy)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(HttpRequestBodyError::InvalidSpoolPolicy);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o777 != 0o700 {
            return Err(HttpRequestBodyError::InvalidSpoolPolicy);
        }
    }

    let directory_handle = Dir::open_ambient_dir(directory, ambient_authority())
        .map_err(|_| HttpRequestBodyError::InvalidSpoolPolicy)?;
    #[cfg(unix)]
    {
        let handle_metadata = directory_handle
            .dir_metadata()
            .map_err(|_| HttpRequestBodyError::InvalidSpoolPolicy)?;
        if handle_metadata.dev() != std::os::unix::fs::MetadataExt::dev(&metadata)
            || handle_metadata.ino() != std::os::unix::fs::MetadataExt::ino(&metadata)
            || handle_metadata.mode() & 0o777 != 0o700
        {
            return Err(HttpRequestBodyError::InvalidSpoolPolicy);
        }
    }
    let filename = format!(".kujo-http-upload-{}.body", Uuid::new_v4());
    let path = directory.join(&filename);
    let mut options = CapOpenOptions::new();
    options.write(true).create_new(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }
    let mut file = directory_handle
        .open_with(&filename, &options)
        .map_err(|_| HttpRequestBodyError::SpoolFailed)?;
    #[cfg(unix)]
    {
        let file_metadata = file.metadata().map_err(|_| HttpRequestBodyError::SpoolFailed)?;
        let directory_metadata = directory_handle
            .dir_metadata()
            .map_err(|_| HttpRequestBodyError::InvalidSpoolPolicy)?;
        if file_metadata.mode() & 0o777 != 0o600 || file_metadata.uid() != directory_metadata.uid()
        {
            drop(file);
            let _ = directory_handle.remove_file(&filename);
            return Err(HttpRequestBodyError::InvalidSpoolPolicy);
        }
    }
    let result = (|| {
        let mut hasher = Sha256::new();
        let mut total = 0_u64;
        let mut buffer = vec![0_u8; ROUTED_HTTP_UPLOAD_CHUNK_BYTES];
        loop {
            let count = reader.read(&mut buffer).map_err(|error| {
                if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) {
                    HttpRequestBodyError::TimedOut
                } else {
                    HttpRequestBodyError::ReadFailed
                }
            })?;
            if count == 0 {
                break;
            }
            total = total.checked_add(count as u64).ok_or(HttpRequestBodyError::TooLarge)?;
            if total > max_body_bytes {
                return Err(HttpRequestBodyError::TooLarge);
            }
            file.write_all(&buffer[..count]).map_err(|_| HttpRequestBodyError::SpoolFailed)?;
            hasher.update(&buffer[..count]);
        }
        if declared_length.is_some_and(|length| total != length as u64) {
            return Err(HttpRequestBodyError::ReadFailed);
        }
        file.sync_all().map_err(|_| HttpRequestBodyError::SpoolFailed)?;
        Ok(HttpRequestBodySpool {
            path: path.clone(),
            bytes: total,
            sha256: format!("{:x}", hasher.finalize()),
        })
    })();
    drop(file);
    if result.is_err() {
        let _ = directory_handle.remove_file(&filename);
    }
    result
}

/// Remove a completed routed-upload spool after dispatch. A missing path means
/// the handler atomically adopted the artifact and is therefore successful.
pub fn cleanup_routed_http_upload(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn routed_http_read_timeout() -> Duration {
    let configured = std::env::var("KUJO_HTTP_SERVER_READ_TIMEOUT_MS").ok();
    routed_http_read_timeout_from(configured.as_deref())
}

fn routed_http_read_timeout_from(configured: Option<&str>) -> Duration {
    let milliseconds = configured
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_ROUTED_HTTP_READ_TIMEOUT_MS)
        .clamp(MIN_ROUTED_HTTP_READ_TIMEOUT_MS, MAX_ROUTED_HTTP_READ_TIMEOUT_MS);
    Duration::from_millis(milliseconds)
}

/// Read an inbound HTTP request body with the same global bound used for
/// outbound network bodies. One extra byte is read so overflow is detected
/// without buffering an unbounded request.
pub fn read_bounded_http_request_body_bytes(
    reader: &mut dyn Read,
    declared_length: Option<usize>,
) -> Result<Vec<u8>, HttpRequestBodyError> {
    let maximum = crate::runtime_limits::MAX_NETWORK_BODY_BYTES;
    if declared_length.is_some_and(|length| length > maximum) {
        return Err(HttpRequestBodyError::TooLarge);
    }
    let read_limit = declared_length.unwrap_or(maximum + 1);
    let mut limited = reader.take(read_limit as u64);
    let mut buffer = Vec::with_capacity(read_limit.min(64 * 1024));
    limited.read_to_end(&mut buffer).map_err(|error| {
        if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) {
            HttpRequestBodyError::TimedOut
        } else {
            HttpRequestBodyError::ReadFailed
        }
    })?;
    if buffer.len() > maximum {
        return Err(HttpRequestBodyError::TooLarge);
    }
    if declared_length.is_some_and(|length| buffer.len() != length) {
        return Err(HttpRequestBodyError::ReadFailed);
    }
    Ok(buffer)
}

/// Split a request URL into a path, parsed query parameters, and raw query string.
///
/// Query parsing is intentionally lexical-only:
/// - no URL decoding
/// - empty key pairs are ignored
/// - `key` without `=` maps to empty-string value
#[allow(dead_code)] // Kept for compatibility with callers that expect raw lexical query parsing.
pub fn split_http_path_and_query(url: &str) -> (String, HashMap<String, String>, String) {
    let (path, query_params, _decoded_query_params, raw_query) =
        split_http_path_and_query_with_decoded(url);
    (path, query_params, raw_query)
}

/// Split a request URL into a path, parsed query parameters, decoded query parameters,
/// and raw query string.
///
/// `query_params` preserves lexical values (no URL decoding) for backward compatibility.
/// `decoded_query_params` applies single-pass percent-decoding and `+` -> space normalization.
pub fn split_http_path_and_query_with_decoded(
    url: &str,
) -> (String, HashMap<String, String>, HashMap<String, String>, String) {
    if let Some((path, raw_query)) = url.split_once('?') {
        let query_params = parse_http_query_params(raw_query, false);
        let decoded_query_params = parse_http_query_params(raw_query, true);
        (path.to_string(), query_params, decoded_query_params, raw_query.to_string())
    } else {
        (url.to_string(), HashMap::new(), HashMap::new(), String::new())
    }
}

fn parse_http_query_params(raw_query: &str, decode_values: bool) -> HashMap<String, String> {
    let mut query_params = HashMap::new();

    for pair in raw_query.split('&') {
        if pair.is_empty() {
            continue;
        }

        let (raw_key, raw_value) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };

        if raw_key.is_empty() {
            continue;
        }

        let (key, value) = if decode_values {
            let decoded_key =
                decode_query_component(raw_key).unwrap_or_else(|| raw_key.to_string());
            let decoded_value =
                decode_query_component(raw_value).unwrap_or_else(|| raw_value.to_string());
            (decoded_key, decoded_value)
        } else {
            (raw_key.to_string(), raw_value.to_string())
        };

        query_params.insert(key, value);
    }

    query_params
}

fn decode_query_component(component: &str) -> Option<String> {
    let bytes = component.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => {
                if index + 2 >= bytes.len() {
                    return None;
                }

                let hi = decode_hex_nibble(bytes[index + 1])?;
                let lo = decode_hex_nibble(bytes[index + 2])?;
                decoded.push((hi << 4) | lo);
                index += 3;
            }
            other => {
                decoded.push(other);
                index += 1;
            }
        }
    }

    String::from_utf8(decoded).ok()
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cleanup_routed_http_upload, read_bounded_http_request_body_bytes,
        routed_http_read_timeout_from, split_http_path_and_query, spool_bounded_http_request_body,
        HttpRequestBodyError, DEFAULT_ROUTED_HTTP_READ_TIMEOUT_MS, MAX_ROUTED_HTTP_READ_TIMEOUT_MS,
        MIN_ROUTED_HTTP_READ_TIMEOUT_MS,
    };
    use std::collections::HashMap;

    #[test]
    fn bounded_request_body_accepts_limit_and_rejects_overflow() {
        let maximum = crate::runtime_limits::MAX_NETWORK_BODY_BYTES;
        let mut exact = std::io::Cursor::new(vec![b'a'; maximum]);
        assert_eq!(read_bounded_http_request_body_bytes(&mut exact, None).unwrap().len(), maximum);

        let mut oversized = std::io::Cursor::new(vec![b'b'; maximum + 1]);
        assert_eq!(
            read_bounded_http_request_body_bytes(&mut oversized, None),
            Err(HttpRequestBodyError::TooLarge)
        );
    }

    #[test]
    fn bounded_request_body_rejects_declared_overflow_before_reading() {
        let maximum = crate::runtime_limits::MAX_NETWORK_BODY_BYTES;
        let mut empty = std::io::Cursor::new(Vec::<u8>::new());
        assert_eq!(
            read_bounded_http_request_body_bytes(&mut empty, Some(maximum + 1)),
            Err(HttpRequestBodyError::TooLarge)
        );
    }

    #[test]
    fn bounded_request_body_stops_at_declared_length() {
        let mut body_with_trailing_bytes = std::io::Cursor::new(b"bodytrailing".to_vec());
        assert_eq!(
            read_bounded_http_request_body_bytes(&mut body_with_trailing_bytes, Some(4)).unwrap(),
            b"body"
        );
    }

    #[test]
    fn routed_upload_streams_beyond_buffered_limit_and_hashes_exact_bytes() {
        use sha2::{Digest, Sha256};
        let directory = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let body = vec![b'u'; crate::runtime_limits::MAX_NETWORK_BODY_BYTES + 1024];
        let mut reader = std::io::Cursor::new(body.clone());
        let result = spool_bounded_http_request_body(
            &mut reader,
            Some(body.len()),
            directory.path().to_str().unwrap(),
            body.len() as u64,
        )
        .unwrap();
        assert_eq!(result.bytes, body.len() as u64);
        assert_eq!(result.sha256, format!("{:x}", Sha256::digest(&body)));
        assert_eq!(std::fs::metadata(&result.path).unwrap().len(), body.len() as u64);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(std::fs::metadata(&result.path).unwrap().mode() & 0o777, 0o600);
        }
        std::fs::remove_file(result.path).unwrap();
    }

    #[test]
    fn routed_upload_rejects_overflow_and_unsafe_directory_without_residue() {
        let directory = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let mut reader = std::io::Cursor::new(vec![b'x'; 9]);
        assert_eq!(
            spool_bounded_http_request_body(
                &mut reader,
                None,
                directory.path().to_str().unwrap(),
                8,
            ),
            Err(HttpRequestBodyError::TooLarge)
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755))
                .unwrap();
            let mut empty = std::io::Cursor::new(Vec::<u8>::new());
            assert_eq!(
                spool_bounded_http_request_body(
                    &mut empty,
                    Some(0),
                    directory.path().to_str().unwrap(),
                    8,
                ),
                Err(HttpRequestBodyError::InvalidSpoolPolicy)
            );
        }
    }

    #[test]
    fn routed_upload_cleanup_accepts_adoption_and_rejects_non_file_parent() {
        let directory = tempfile::tempdir().unwrap();
        let adopted = directory.path().join("adopted.body");
        assert!(cleanup_routed_http_upload(&adopted).is_ok());
        assert!(cleanup_routed_http_upload(directory.path()).is_err());
    }

    #[test]
    fn routed_http_read_timeout_defaults_and_clamps() {
        assert_eq!(
            routed_http_read_timeout_from(None).as_millis(),
            u128::from(DEFAULT_ROUTED_HTTP_READ_TIMEOUT_MS)
        );
        assert_eq!(
            routed_http_read_timeout_from(Some("1")).as_millis(),
            u128::from(MIN_ROUTED_HTTP_READ_TIMEOUT_MS)
        );
        assert_eq!(
            routed_http_read_timeout_from(Some("999999")).as_millis(),
            u128::from(MAX_ROUTED_HTTP_READ_TIMEOUT_MS)
        );
        assert_eq!(routed_http_read_timeout_from(Some("250")).as_millis(), 250);
    }

    #[test]
    fn split_http_path_and_query_without_query_returns_empty_metadata() {
        let (path, query_map, raw_query) = split_http_path_and_query("/health");
        assert_eq!(path, "/health");
        assert!(query_map.is_empty());
        assert_eq!(raw_query, "");
    }

    #[test]
    fn split_http_path_and_query_parses_pairs_without_decoding() {
        let (path, query_map, raw_query) =
            split_http_path_and_query("/search?q=kujo%20lang&limit=10");
        assert_eq!(path, "/search");
        assert_eq!(raw_query, "q=kujo%20lang&limit=10");

        let mut expected = HashMap::new();
        expected.insert("q".to_string(), "kujo%20lang".to_string());
        expected.insert("limit".to_string(), "10".to_string());
        assert_eq!(query_map, expected);
    }

    #[test]
    fn split_http_path_and_query_ignores_empty_keys_and_accepts_missing_values() {
        let (_, query_map, raw_query) =
            split_http_path_and_query("/x?=skip&flag&name=kujo&&empty=");
        assert_eq!(raw_query, "=skip&flag&name=kujo&&empty=");
        assert_eq!(query_map.get("flag").map(String::as_str), Some(""));
        assert_eq!(query_map.get("name").map(String::as_str), Some("kujo"));
        assert_eq!(query_map.get("empty").map(String::as_str), Some(""));
        assert!(!query_map.contains_key(""));
    }

    #[test]
    fn split_http_path_and_query_with_decoded_parses_percent_and_plus_components() {
        let (_, raw_query_map, decoded_query_map, raw_query) =
            super::split_http_path_and_query_with_decoded(
                "/search?q=kujo%20lang&tag=enterprise+ready",
            );

        assert_eq!(raw_query, "q=kujo%20lang&tag=enterprise+ready");
        assert_eq!(raw_query_map.get("q").map(String::as_str), Some("kujo%20lang"));
        assert_eq!(decoded_query_map.get("q").map(String::as_str), Some("kujo lang"));
        assert_eq!(decoded_query_map.get("tag").map(String::as_str), Some("enterprise ready"));
    }

    #[test]
    fn split_http_path_and_query_with_decoded_falls_back_to_raw_on_invalid_encoding() {
        let (_, _raw_query_map, decoded_query_map, _raw_query) =
            super::split_http_path_and_query_with_decoded("/x?bad=%2");

        assert_eq!(decoded_query_map.get("bad").map(String::as_str), Some("%2"));
    }
}
