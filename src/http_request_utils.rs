use std::collections::HashMap;
use std::io::{ErrorKind, Read};
use std::time::Duration;

pub const DEFAULT_ROUTED_HTTP_READ_TIMEOUT_MS: u64 = 10_000;
pub const MIN_ROUTED_HTTP_READ_TIMEOUT_MS: u64 = 100;
pub const MAX_ROUTED_HTTP_READ_TIMEOUT_MS: u64 = 300_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpRequestBodyError {
    TooLarge,
    TimedOut,
    ReadFailed,
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
pub fn read_bounded_http_request_body(
    reader: &mut dyn Read,
) -> Result<String, HttpRequestBodyError> {
    let maximum = crate::runtime_limits::MAX_NETWORK_BODY_BYTES;
    let mut limited = reader.take((maximum + 1) as u64);
    let mut buffer = Vec::with_capacity(maximum.min(64 * 1024));
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
    Ok(String::from_utf8_lossy(&buffer).to_string())
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
        read_bounded_http_request_body, routed_http_read_timeout_from, split_http_path_and_query,
        HttpRequestBodyError, DEFAULT_ROUTED_HTTP_READ_TIMEOUT_MS, MAX_ROUTED_HTTP_READ_TIMEOUT_MS,
        MIN_ROUTED_HTTP_READ_TIMEOUT_MS,
    };
    use std::collections::HashMap;

    #[test]
    fn bounded_request_body_accepts_limit_and_rejects_overflow() {
        let maximum = crate::runtime_limits::MAX_NETWORK_BODY_BYTES;
        let mut exact = std::io::Cursor::new(vec![b'a'; maximum]);
        assert_eq!(read_bounded_http_request_body(&mut exact).unwrap().len(), maximum);

        let mut oversized = std::io::Cursor::new(vec![b'b'; maximum + 1]);
        assert_eq!(
            read_bounded_http_request_body(&mut oversized),
            Err(HttpRequestBodyError::TooLarge)
        );
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
