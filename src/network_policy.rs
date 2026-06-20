use crate::runtime_limits;
use reqwest::blocking::{Client, Response as BlockingResponse};
use std::io::Read;
use std::net::{IpAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;

pub const DEFAULT_NETWORK_CONNECT_TIMEOUT_MS: u64 = 10_000;
pub const DEFAULT_NETWORK_READ_TIMEOUT_MS: u64 = 30_000;
pub const DEFAULT_NETWORK_WRITE_TIMEOUT_MS: u64 = 30_000;
pub const DEFAULT_HTTP_TIMEOUT_MS: u64 = 30_000;
pub const MAX_NETWORK_BODY_BYTES: usize = runtime_limits::MAX_NETWORK_BODY_BYTES;
pub const OUTBOUND_DESTINATION_POLICY_ENV: &str = "KUJO_NET_DESTINATION_POLICY";
pub const ALLOW_PRIVATE_DESTINATIONS_ENV: &str = "KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS";
const ALLOWED_HTTP_URL_SCHEMES: [&str; 2] = ["http", "https"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutboundDestinationPolicy {
    AllowAll,
    DenyPrivate,
}

fn parse_policy_mode() -> Result<OutboundDestinationPolicy, String> {
    match std::env::var(OUTBOUND_DESTINATION_POLICY_ENV) {
        Ok(raw) => {
            let normalized = raw.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "" | "allow_all" | "allow-all" | "permissive" => {
                    Ok(OutboundDestinationPolicy::AllowAll)
                }
                "deny_private" | "deny-private" | "strict" => {
                    Ok(OutboundDestinationPolicy::DenyPrivate)
                }
                _ => Err(format!(
                    "Invalid {} value '{}'; expected one of: allow_all, deny_private",
                    OUTBOUND_DESTINATION_POLICY_ENV, raw
                )),
            }
        }
        Err(_) => Ok(OutboundDestinationPolicy::AllowAll),
    }
}

fn private_destination_override_enabled() -> bool {
    match std::env::var(ALLOW_PRIVATE_DESTINATIONS_ENV) {
        Ok(raw) => matches!(raw.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => false,
    }
}

fn is_blocked_destination(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_private()
                || ipv4.is_loopback()
                || ipv4.is_link_local()
                || ipv4.is_multicast()
                || ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.is_multicast()
                || ipv6.is_unique_local()
                || ipv6.is_unicast_link_local()
        }
    }
}

fn blocked_addresses_for_host_port(host: &str, port: u16) -> Result<Vec<IpAddr>, String> {
    let addresses = (host, port).to_socket_addrs().map_err(|error| {
        format!("Failed to resolve network destination '{}:{}': {}", host, port, error)
    })?;

    let mut blocked = Vec::new();
    for socket_address in addresses {
        if is_blocked_destination(socket_address.ip()) {
            blocked.push(socket_address.ip());
        }
    }

    blocked.sort_unstable();
    blocked.dedup();
    Ok(blocked)
}

pub fn enforce_host_port_destination_policy(
    host: &str,
    port: i64,
    surface: &str,
) -> Result<(), String> {
    if !(1..=65535).contains(&port) {
        return Err(format!("{} failed: destination port must be 1-65535", surface));
    }

    let policy = parse_policy_mode()?;
    if policy == OutboundDestinationPolicy::AllowAll || private_destination_override_enabled() {
        return Ok(());
    }

    let port = port as u16;
    let blocked = blocked_addresses_for_host_port(host, port)?;
    if blocked.is_empty() {
        return Ok(());
    }

    let blocked_list = blocked.iter().map(IpAddr::to_string).collect::<Vec<_>>().join(", ");

    Err(format!(
        "{} blocked by outbound destination policy '{}' (resolved to blocked addresses: {}). \
Set {}=1 to allow trusted local/private destinations.",
        surface, "deny_private", blocked_list, ALLOW_PRIVATE_DESTINATIONS_ENV
    ))
}

pub fn enforce_http_url_destination_policy(url: &str, surface: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|error| format!("{} failed: invalid URL '{}': {}", surface, url, error))?;

    let scheme = parsed.scheme().to_ascii_lowercase();
    if !ALLOWED_HTTP_URL_SCHEMES.iter().any(|allowed| *allowed == scheme) {
        return Err(format!(
            "{} failed: unsupported URL scheme '{}'; expected http or https",
            surface, scheme
        ));
    }

    let host = parsed.host_str().ok_or_else(|| {
        format!(
            "{} failed: URL '{}' is missing a host for destination policy evaluation",
            surface, url
        )
    })?;
    let port = parsed.port_or_known_default().ok_or_else(|| {
        format!(
            "{} failed: URL '{}' has unsupported scheme/port for destination policy evaluation",
            surface, url
        )
    })? as i64;

    enforce_host_port_destination_policy(host, port, surface)
}

pub fn run_blocking_http_task<T, F>(surface: &str, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    match std::thread::spawn(task).join() {
        Ok(result) => result,
        Err(_) => Err(format!("{} failed: blocking HTTP worker thread panicked", surface)),
    }
}

pub fn connect_timeout() -> Duration {
    Duration::from_millis(DEFAULT_NETWORK_CONNECT_TIMEOUT_MS)
}

pub fn read_timeout() -> Duration {
    Duration::from_millis(DEFAULT_NETWORK_READ_TIMEOUT_MS)
}

pub fn write_timeout() -> Duration {
    Duration::from_millis(DEFAULT_NETWORK_WRITE_TIMEOUT_MS)
}

pub fn default_http_timeout() -> Duration {
    Duration::from_millis(DEFAULT_HTTP_TIMEOUT_MS)
}

pub fn build_http_client(timeout: Duration) -> Result<Client, String> {
    Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("Failed to create HTTP client: {}", error))
}

pub fn build_async_http_client(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("Failed to create async HTTP client: {}", error))
}

pub fn read_http_response_bytes(
    response: BlockingResponse,
    surface: &str,
) -> Result<(u16, reqwest::header::HeaderMap, Vec<u8>), String> {
    let status = response.status().as_u16();
    let headers = response.headers().clone();

    if let Some(content_length) = response.content_length() {
        if content_length > MAX_NETWORK_BODY_BYTES as u64 {
            return Err(format!(
                "{} failed: response body exceeds maximum network body size ({} bytes > {} bytes)",
                surface, content_length, MAX_NETWORK_BODY_BYTES
            ));
        }
    }

    let max_plus_one = (MAX_NETWORK_BODY_BYTES as u64).saturating_add(1);
    let mut bytes = Vec::new();
    response
        .take(max_plus_one)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{} failed while reading response body: {}", surface, error))?;

    if bytes.len() > MAX_NETWORK_BODY_BYTES {
        return Err(format!(
            "{} failed: response body exceeds maximum network body size ({} bytes > {} bytes)",
            surface,
            bytes.len(),
            MAX_NETWORK_BODY_BYTES
        ));
    }

    Ok((status, headers, bytes))
}

pub async fn read_async_http_response_text(
    mut response: reqwest::Response,
    surface: &str,
) -> Result<(u16, reqwest::header::HeaderMap, String), String> {
    let status = response.status().as_u16();
    let headers = response.headers().clone();

    if let Some(content_length) = response.content_length() {
        if content_length > MAX_NETWORK_BODY_BYTES as u64 {
            return Err(format!(
                "{} failed: response body exceeds maximum network body size ({} bytes > {} bytes)",
                surface, content_length, MAX_NETWORK_BODY_BYTES
            ));
        }
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("{} failed while reading response body: {}", surface, error))?
    {
        let next_len = bytes.len().saturating_add(chunk.len());
        if next_len > MAX_NETWORK_BODY_BYTES {
            return Err(format!(
                "{} failed: response body exceeds maximum network body size ({} bytes > {} bytes)",
                surface, next_len, MAX_NETWORK_BODY_BYTES
            ));
        }
        bytes.extend_from_slice(&chunk);
    }

    let body = String::from_utf8(bytes).map_err(|error| {
        format!("{} failed: response body is not valid UTF-8: {}", surface, error)
    })?;

    Ok((status, headers, body))
}

pub fn apply_tcp_stream_timeouts(stream: &TcpStream, surface: &str) -> Result<(), String> {
    stream
        .set_read_timeout(Some(read_timeout()))
        .map_err(|error| format!("{} failed to set read timeout: {}", surface, error))?;
    stream
        .set_write_timeout(Some(write_timeout()))
        .map_err(|error| format!("{} failed to set write timeout: {}", surface, error))?;
    Ok(())
}

pub fn apply_udp_socket_timeouts(socket: &UdpSocket, surface: &str) -> Result<(), String> {
    socket
        .set_read_timeout(Some(read_timeout()))
        .map_err(|error| format!("{} failed to set read timeout: {}", surface, error))?;
    socket
        .set_write_timeout(Some(write_timeout()))
        .map_err(|error| format!("{} failed to set write timeout: {}", surface, error))?;
    Ok(())
}

pub fn connect_tcp_stream(address: &str, surface: &str) -> Result<TcpStream, String> {
    let timeout = connect_timeout();
    let addresses = address
        .to_socket_addrs()
        .map_err(|error| format!("{} failed to resolve '{}': {}", surface, address, error))?;

    let mut last_error = None;
    for candidate in addresses {
        match TcpStream::connect_timeout(&candidate, timeout) {
            Ok(stream) => {
                apply_tcp_stream_timeouts(&stream, surface)?;
                return Ok(stream);
            }
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    match last_error {
        Some(error) => Err(format!("{} failed to connect to '{}': {}", surface, address, error)),
        None => Err(format!("{} failed: no socket addresses resolved for '{}'", surface, address)),
    }
}

pub fn validate_receive_size(size: i64, surface: &str) -> Result<usize, String> {
    if size <= 0 {
        return Err(format!("{} size must be positive", surface));
    }

    let size = size as usize;
    if size > MAX_NETWORK_BODY_BYTES {
        return Err(format!(
            "{} size exceeds maximum network body size ({} bytes > {} bytes)",
            surface, size, MAX_NETWORK_BODY_BYTES
        ));
    }

    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_policy_env<F, T>(policy: Option<&str>, allow_private: Option<&str>, run: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");
        let previous_policy = std::env::var(OUTBOUND_DESTINATION_POLICY_ENV).ok();
        let previous_allow = std::env::var(ALLOW_PRIVATE_DESTINATIONS_ENV).ok();

        match policy {
            Some(value) => std::env::set_var(OUTBOUND_DESTINATION_POLICY_ENV, value),
            None => std::env::remove_var(OUTBOUND_DESTINATION_POLICY_ENV),
        }
        match allow_private {
            Some(value) => std::env::set_var(ALLOW_PRIVATE_DESTINATIONS_ENV, value),
            None => std::env::remove_var(ALLOW_PRIVATE_DESTINATIONS_ENV),
        }

        let result = run();

        match previous_policy {
            Some(value) => std::env::set_var(OUTBOUND_DESTINATION_POLICY_ENV, value),
            None => std::env::remove_var(OUTBOUND_DESTINATION_POLICY_ENV),
        }
        match previous_allow {
            Some(value) => std::env::set_var(ALLOW_PRIVATE_DESTINATIONS_ENV, value),
            None => std::env::remove_var(ALLOW_PRIVATE_DESTINATIONS_ENV),
        }

        result
    }

    #[test]
    fn outbound_policy_default_mode_allows_loopback_destinations() {
        with_policy_env(None, None, || {
            enforce_host_port_destination_policy("127.0.0.1", 8080, "tcp_connect")
                .expect("default policy should keep backward-compatible permissive behavior");
        });
    }

    #[test]
    fn outbound_policy_deny_private_blocks_loopback_destinations() {
        with_policy_env(Some("deny_private"), None, || {
            let error = enforce_host_port_destination_policy("127.0.0.1", 8080, "tcp_connect")
                .expect_err("deny_private should reject loopback destinations");
            assert!(error.contains("blocked by outbound destination policy"));
            assert!(error.contains("127.0.0.1"));
            assert!(error.contains(ALLOW_PRIVATE_DESTINATIONS_ENV));
        });
    }

    #[test]
    fn outbound_policy_deny_private_allows_public_ip_destinations() {
        with_policy_env(Some("deny_private"), None, || {
            enforce_host_port_destination_policy("93.184.216.34", 80, "tcp_connect")
                .expect("public destinations should not be blocked");
        });
    }

    #[test]
    fn outbound_policy_override_allows_private_destinations_in_strict_mode() {
        with_policy_env(Some("deny_private"), Some("1"), || {
            enforce_host_port_destination_policy("127.0.0.1", 8080, "tcp_connect")
                .expect("override should permit trusted local destinations");
        });
    }

    #[test]
    fn outbound_policy_invalid_mode_returns_deterministic_error() {
        with_policy_env(Some("invalid_mode"), None, || {
            let error = enforce_host_port_destination_policy("93.184.216.34", 80, "tcp_connect")
                .expect_err("invalid policy mode should fail deterministically");
            assert!(error.contains("Invalid"));
            assert!(error.contains(OUTBOUND_DESTINATION_POLICY_ENV));
        });
    }

    #[test]
    fn outbound_policy_http_url_evaluation_blocks_loopback_when_strict() {
        with_policy_env(Some("deny_private"), None, || {
            let error = enforce_http_url_destination_policy("http://127.0.0.1:8080", "HTTP GET")
                .expect_err("strict policy should reject loopback URLs");
            assert!(error.contains("blocked by outbound destination policy"));
            assert!(error.contains("127.0.0.1"));
        });
    }

    #[test]
    fn outbound_policy_http_url_evaluation_rejects_unsupported_scheme() {
        with_policy_env(None, None, || {
            let error = enforce_http_url_destination_policy("ftp://127.0.0.1", "HTTP GET")
                .expect_err("unsupported URL schemes should be rejected deterministically");
            assert!(error.contains("unsupported URL scheme 'ftp'"));
            assert!(error.contains("expected http or https"));
        });
    }
}
