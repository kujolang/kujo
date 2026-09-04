// File: src/interpreter/native_functions/network.rs
//
// Network-related native functions (TCP, UDP sockets)

use super::file_stream::write_file_range;
use crate::interpreter::{DictMap, Interpreter, Value};
use crate::network_policy;
use std::io::{Read, Write};
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard};

fn network_string_value(value: impl Into<String>) -> Value {
    Value::Str(Arc::new(value.into()))
}

fn network_insert(map: &mut DictMap, key: &str, value: Value) {
    map.insert(Arc::<str>::from(key), value);
}

fn timeout_aware_error_message(operation: &str, error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => format!(
            "{} timed out after {}ms read/{}ms write timeout policy: {}",
            operation,
            network_policy::DEFAULT_NETWORK_READ_TIMEOUT_MS,
            network_policy::DEFAULT_NETWORK_WRITE_TIMEOUT_MS,
            error
        ),
        _ => format!("{}: {}", operation, error),
    }
}

fn ip_scope(address: IpAddr) -> (&'static str, bool) {
    match address {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            if ip.is_unspecified() {
                return ("unspecified", false);
            }
            if ip.is_loopback() {
                return ("loopback", false);
            }
            if ip.is_private() {
                return ("private", false);
            }
            if ip.is_link_local() {
                return ("link_local", false);
            }
            if ip.is_multicast() {
                return ("multicast", false);
            }
            if ip.is_broadcast() {
                return ("broadcast", false);
            }
            if ip.is_documentation() {
                return ("documentation", false);
            }
            if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                return ("shared", false);
            }
            if octets[0] == 198 && (18..=19).contains(&octets[1]) {
                return ("benchmark", false);
            }
            if octets[0] == 0
                || octets[0] >= 240
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
            {
                return ("reserved", false);
            }
            ("global", true)
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            if ip.is_unspecified() {
                return ("unspecified", false);
            }
            if ip.is_loopback() {
                return ("loopback", false);
            }
            if ip.is_multicast() {
                return ("multicast", false);
            }
            if ip.is_unique_local() {
                return ("private", false);
            }
            if ip.is_unicast_link_local() {
                return ("link_local", false);
            }
            if segments[0] == 0x2001 && segments[1] == 0x0db8 {
                return ("documentation", false);
            }
            if segments[..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
                return ("ipv4_mapped", false);
            }
            if (segments[0] & 0xe000) != 0x2000 {
                return ("reserved", false);
            }
            ("global", true)
        }
    }
}

fn ip_cidr_contains(address: IpAddr, cidr: &str) -> Result<bool, String> {
    let (network_text, prefix_text) = cidr.trim().split_once('/').ok_or_else(|| {
        "ip_cidr_contains requires CIDR notation with an explicit prefix".to_string()
    })?;
    let network = network_text
        .parse::<IpAddr>()
        .map_err(|_| "ip_cidr_contains requires a valid CIDR network address".to_string())?;
    let prefix = prefix_text
        .parse::<u32>()
        .map_err(|_| "ip_cidr_contains requires a numeric CIDR prefix".to_string())?;
    match (address, network) {
        (IpAddr::V4(address), IpAddr::V4(network)) if prefix <= 32 => {
            let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
            Ok((u32::from(address) & mask) == (u32::from(network) & mask))
        }
        (IpAddr::V6(address), IpAddr::V6(network)) if prefix <= 128 => {
            let mask = if prefix == 0 { 0 } else { u128::MAX << (128 - prefix) };
            Ok((u128::from(address) & mask) == (u128::from(network) & mask))
        }
        (IpAddr::V4(_), IpAddr::V6(_)) | (IpAddr::V6(_), IpAddr::V4(_)) => Ok(false),
        _ => Err("ip_cidr_contains prefix exceeds the address-family width".to_string()),
    }
}

fn lock_or_network_error<'a, T>(
    mutex: &'a Mutex<T>,
    context: &str,
) -> Result<MutexGuard<'a, T>, Value> {
    mutex.lock().map_err(|_| Value::ErrorObject {
        message: format!("{}: shared network lock poisoned", context),
        stack: Vec::new(),
        line: None,
        cause: None,
    })
}

pub fn handle(_interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "ip_cidr_contains" => {
            if arg_values.len() != 2 {
                Value::Error(
                    "ip_cidr_contains requires string address and CIDR arguments".to_string(),
                )
            } else if let (Some(Value::Str(address)), Some(Value::Str(cidr))) =
                (arg_values.first(), arg_values.get(1))
            {
                match address.parse::<IpAddr>() {
                    Ok(address) => match ip_cidr_contains(address, cidr) {
                        Ok(contains) => Value::Bool(contains),
                        Err(message) => Value::Error(message),
                    },
                    Err(_) => Value::Error(
                        "ip_cidr_contains requires a valid IP literal address".to_string(),
                    ),
                }
            } else {
                Value::Error(
                    "ip_cidr_contains requires string address and CIDR arguments".to_string(),
                )
            }
        }
        "ip_classify" => {
            if arg_values.len() != 1 {
                Value::Error("ip_classify requires a string IP literal argument".to_string())
            } else if let Some(Value::Str(input)) = arg_values.first() {
                match input.parse::<IpAddr>() {
                    Ok(address) => {
                        let (scope, publicly_routable) = ip_scope(address);
                        let expanded_address = match address {
                            IpAddr::V4(value) => value.to_string(),
                            IpAddr::V6(value) => value
                                .segments()
                                .iter()
                                .map(|segment| format!("{segment:04x}"))
                                .collect::<Vec<_>>()
                                .join(":"),
                        };
                        let mut result = DictMap::default();
                        network_insert(
                            &mut result,
                            "schema_version",
                            network_string_value("kujo.net.ip-classification.v1"),
                        );
                        network_insert(
                            &mut result,
                            "address",
                            network_string_value(address.to_string()),
                        );
                        network_insert(
                            &mut result,
                            "expanded_address",
                            network_string_value(expanded_address),
                        );
                        network_insert(
                            &mut result,
                            "family",
                            network_string_value(if address.is_ipv4() { "ipv4" } else { "ipv6" }),
                        );
                        network_insert(&mut result, "scope", network_string_value(scope));
                        network_insert(
                            &mut result,
                            "publicly_routable",
                            Value::Bool(publicly_routable),
                        );
                        Value::dict(result)
                    }
                    Err(_) => Value::Error("ip_classify requires a valid IP literal".to_string()),
                }
            } else {
                Value::Error("ip_classify requires a string IP literal argument".to_string())
            }
        }
        "tcp_bind_probe" => {
            if arg_values.len() != 1 {
                Value::Error("tcp_bind_probe requires a string IP literal argument".to_string())
            } else if let Some(Value::Str(input)) = arg_values.first() {
                match input.parse::<IpAddr>() {
                    Ok(address) => {
                        let (scope, publicly_routable) = ip_scope(address);
                        let probe = std::net::TcpListener::bind((address, 0));
                        let bindable = probe.is_ok();
                        drop(probe);
                        let mut result = DictMap::default();
                        network_insert(
                            &mut result,
                            "schema_version",
                            network_string_value("kujo.net.tcp-bind-probe.v1"),
                        );
                        network_insert(
                            &mut result,
                            "address",
                            network_string_value(address.to_string()),
                        );
                        network_insert(&mut result, "scope", network_string_value(scope));
                        network_insert(
                            &mut result,
                            "publicly_routable",
                            Value::Bool(publicly_routable),
                        );
                        network_insert(&mut result, "bindable", Value::Bool(bindable));
                        network_insert(&mut result, "ok", Value::Bool(bindable));
                        network_insert(
                            &mut result,
                            "code",
                            network_string_value(if bindable {
                                "TCP_BIND_PROBE_AVAILABLE"
                            } else {
                                "TCP_BIND_PROBE_UNAVAILABLE"
                            }),
                        );
                        Value::dict(result)
                    }
                    Err(_) => {
                        let mut result = DictMap::default();
                        network_insert(
                            &mut result,
                            "schema_version",
                            network_string_value("kujo.net.tcp-bind-probe.v1"),
                        );
                        network_insert(
                            &mut result,
                            "address",
                            network_string_value(input.as_ref()),
                        );
                        network_insert(&mut result, "bindable", Value::Bool(false));
                        network_insert(&mut result, "ok", Value::Bool(false));
                        network_insert(
                            &mut result,
                            "code",
                            network_string_value("TCP_BIND_PROBE_ADDRESS_INVALID"),
                        );
                        Value::dict(result)
                    }
                }
            } else {
                Value::Error("tcp_bind_probe requires a string IP literal argument".to_string())
            }
        }
        "tcp_listen" => {
            if arg_values.len() != 2 {
                Value::Error("tcp_listen requires (string_host, int_port) arguments".to_string())
            } else {
                match (arg_values.first(), arg_values.get(1)) {
                    (Some(Value::Str(host)), Some(Value::Int(port))) => {
                        let address = format!("{}:{}", host.as_ref(), port);
                        match std::net::TcpListener::bind(&address) {
                            Ok(listener) => {
                                let _ = listener.set_nonblocking(false);
                                Value::TcpListener {
                                    listener: Arc::new(Mutex::new(listener)),
                                    addr: address,
                                }
                            }
                            Err(error) => Value::ErrorObject {
                                message: format!(
                                    "Failed to bind TCP listener on '{}': {}",
                                    address, error
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "tcp_listen requires (string_host, int_port) arguments".to_string(),
                    ),
                }
            }
        }

        "tcp_accept" => {
            if arg_values.len() != 1 {
                Value::Error("tcp_accept requires a TcpListener argument".to_string())
            } else {
                if let Some(Value::TcpListener { listener, .. }) = arg_values.first() {
                    let listener_guard = match lock_or_network_error(listener, "tcp_accept") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    match listener_guard.accept() {
                        Ok((stream, peer_address)) => {
                            match network_policy::apply_tcp_stream_timeouts(&stream, "tcp_accept") {
                                Ok(()) => Value::TcpStream {
                                    stream: Arc::new(Mutex::new(Some(stream))),
                                    peer_addr: peer_address.to_string(),
                                },
                                Err(error) => Value::ErrorObject {
                                    message: error,
                                    stack: Vec::new(),
                                    line: None,
                                    cause: None,
                                },
                            }
                        }
                        Err(error) => Value::ErrorObject {
                            message: timeout_aware_error_message(
                                "Failed to accept connection",
                                &error,
                            ),
                            stack: Vec::new(),
                            line: None,
                            cause: None,
                        },
                    }
                } else {
                    Value::Error("tcp_accept requires a TcpListener argument".to_string())
                }
            }
        }

        "tcp_connect" => {
            if arg_values.len() != 2 {
                Value::Error("tcp_connect requires (string_host, int_port) arguments".to_string())
            } else {
                match (arg_values.first(), arg_values.get(1)) {
                    (Some(Value::Str(host)), Some(Value::Int(port))) => {
                        if let Err(error) = network_policy::enforce_host_port_destination_policy(
                            host.as_ref(),
                            *port,
                            "tcp_connect",
                        ) {
                            return Some(Value::ErrorObject {
                                message: error,
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            });
                        }
                        let address = format!("{}:{}", host.as_ref(), port);
                        match network_policy::connect_tcp_stream(&address, "tcp_connect") {
                            Ok(stream) => Value::TcpStream {
                                stream: Arc::new(Mutex::new(Some(stream))),
                                peer_addr: address,
                            },
                            Err(error) => Value::ErrorObject {
                                message: error,
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "tcp_connect requires (string_host, int_port) arguments".to_string(),
                    ),
                }
            }
        }

        "tcp_connect_bound" => {
            if arg_values.len() != 3 {
                Value::Error(
                    "tcp_connect_bound requires (string_host, int_port, string_source_ip) arguments"
                        .to_string(),
                )
            } else {
                match (arg_values.first(), arg_values.get(1), arg_values.get(2)) {
                    (
                        Some(Value::Str(host)),
                        Some(Value::Int(port)),
                        Some(Value::Str(source_ip)),
                    ) => {
                        if let Err(error) = network_policy::enforce_host_port_destination_policy(
                            host.as_ref(),
                            *port,
                            "tcp_connect_bound",
                        ) {
                            return Some(Value::ErrorObject {
                                message: error,
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            });
                        }
                        if !(1..=65535).contains(port) {
                            return Some(Value::Error(
                                "tcp_connect_bound destination port must be 1-65535".to_string(),
                            ));
                        }
                        match network_policy::connect_tcp_stream_bound(
                            host.as_ref(),
                            *port as u16,
                            source_ip.as_ref(),
                            "tcp_connect_bound",
                        ) {
                            Ok(stream) => {
                                let peer_addr = stream
                                    .peer_addr()
                                    .map(|address| address.to_string())
                                    .unwrap_or_else(|_| format!("{}:{}", host, port));
                                Value::TcpStream {
                                    stream: Arc::new(Mutex::new(Some(stream))),
                                    peer_addr,
                                }
                            }
                            Err(error) => Value::ErrorObject {
                                message: error,
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "tcp_connect_bound requires (string_host, int_port, string_source_ip) arguments"
                            .to_string(),
                    ),
                }
            }
        }

        "tcp_send" => {
            if arg_values.len() != 2 {
                Value::Error(
                    "tcp_send requires (TcpStream, string_or_bytes_data) arguments".to_string(),
                )
            } else {
                match (arg_values.first(), arg_values.get(1)) {
                    (Some(Value::TcpStream { stream, .. }), Some(Value::Str(data))) => {
                        let mut stream_guard = match lock_or_network_error(stream, "tcp_send") {
                            Ok(guard) => guard,
                            Err(error) => return Some(error),
                        };
                        let Some(stream) = stream_guard.as_mut() else {
                            return Some(Value::Error(
                                "tcp_send: TCP stream is closed or upgraded".to_string(),
                            ));
                        };
                        match stream.write_all(data.as_ref().as_bytes()) {
                            Ok(_) => match stream.flush() {
                                Ok(_) => Value::Int(data.len() as i64),
                                Err(error) => Value::ErrorObject {
                                    message: timeout_aware_error_message(
                                        "Failed to flush TCP stream",
                                        &error,
                                    ),
                                    stack: Vec::new(),
                                    line: None,
                                    cause: None,
                                },
                            },
                            Err(error) => Value::ErrorObject {
                                message: timeout_aware_error_message(
                                    "Failed to send data over TCP",
                                    &error,
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    (Some(Value::TcpStream { stream, .. }), Some(Value::Bytes(data))) => {
                        let mut stream_guard = match lock_or_network_error(stream, "tcp_send") {
                            Ok(guard) => guard,
                            Err(error) => return Some(error),
                        };
                        let Some(stream) = stream_guard.as_mut() else {
                            return Some(Value::Error(
                                "tcp_send: TCP stream is closed or upgraded".to_string(),
                            ));
                        };
                        match stream.write_all(data) {
                            Ok(_) => match stream.flush() {
                                Ok(_) => Value::Int(data.len() as i64),
                                Err(error) => Value::ErrorObject {
                                    message: timeout_aware_error_message(
                                        "Failed to flush TCP stream",
                                        &error,
                                    ),
                                    stack: Vec::new(),
                                    line: None,
                                    cause: None,
                                },
                            },
                            Err(error) => Value::ErrorObject {
                                message: timeout_aware_error_message(
                                    "Failed to send data over TCP",
                                    &error,
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "tcp_send requires (TcpStream, string_or_bytes_data) arguments".to_string(),
                    ),
                }
            }
        }

        "tcp_send_file_range" => {
            if arg_values.len() != 4 {
                Value::Error(
                    "tcp_send_file_range requires (TcpStream, string_path, int_offset, int_count) arguments".to_string(),
                )
            } else {
                match (arg_values.first(), arg_values.get(1), arg_values.get(2), arg_values.get(3)) {
                    (
                        Some(Value::TcpStream { stream, .. }),
                        Some(Value::Str(path)),
                        Some(Value::Int(offset)),
                        Some(Value::Int(count)),
                    ) => {
                        let mut stream_guard = match lock_or_network_error(stream, "tcp_send_file_range") {
                            Ok(guard) => guard,
                            Err(error) => return Some(error),
                        };
                        let Some(stream) = stream_guard.as_mut() else {
                            return Some(Value::Error(
                                "tcp_send_file_range: TCP stream is closed or upgraded".to_string(),
                            ));
                        };
                        match write_file_range(stream, path, *offset, *count, "tcp_send_file_range") {
                            Ok(sent) => Value::Int(sent),
                            Err(message) => Value::ErrorObject {
                                message,
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "tcp_send_file_range requires (TcpStream, string_path, int_offset, int_count) arguments".to_string(),
                    ),
                }
            }
        }

        "tcp_receive" => {
            if arg_values.len() != 2 {
                Value::Error("tcp_receive requires (TcpStream, int_size) arguments".to_string())
            } else {
                match (arg_values.first(), arg_values.get(1)) {
                    (Some(Value::TcpStream { stream, .. }), Some(Value::Int(size))) => {
                        let size = match network_policy::validate_receive_size(*size, "tcp_receive")
                        {
                            Ok(size) => size,
                            Err(error) => return Some(Value::Error(error)),
                        };

                        let mut stream_guard = match lock_or_network_error(stream, "tcp_receive") {
                            Ok(guard) => guard,
                            Err(error) => return Some(error),
                        };
                        let mut buffer = vec![0u8; size];
                        let Some(stream) = stream_guard.as_mut() else {
                            return Some(Value::Error(
                                "tcp_receive: TCP stream is closed or upgraded".to_string(),
                            ));
                        };
                        match stream.read(&mut buffer) {
                            Ok(read_size) => {
                                buffer.truncate(read_size);
                                match String::from_utf8(buffer.clone()) {
                                    Ok(text) => Value::Str(Arc::new(text)),
                                    Err(_) => Value::Bytes(buffer),
                                }
                            }
                            Err(error) => Value::ErrorObject {
                                message: timeout_aware_error_message(
                                    "Failed to receive data from TCP",
                                    &error,
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "tcp_receive requires (TcpStream, int_size) arguments".to_string(),
                    ),
                }
            }
        }

        "tcp_close" => {
            if arg_values.len() != 1 {
                Value::Error("tcp_close requires a TcpStream or TcpListener argument".to_string())
            } else {
                match arg_values.first() {
                    Some(Value::TcpStream { stream, .. }) => {
                        let mut stream_guard = match lock_or_network_error(stream, "tcp_close") {
                            Ok(guard) => guard,
                            Err(error) => return Some(error),
                        };
                        if let Some(stream) = stream_guard.take() {
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                        }
                        Value::Bool(true)
                    }
                    Some(Value::TcpListener { .. }) => Value::Bool(true),
                    _ => Value::Error(
                        "tcp_close requires a TcpStream or TcpListener argument".to_string(),
                    ),
                }
            }
        }

        "tcp_set_nonblocking" => {
            if arg_values.len() != 2 {
                Value::Error(
                    "tcp_set_nonblocking requires (TcpStream/TcpListener, bool) arguments"
                        .to_string(),
                )
            } else {
                match (arg_values.first(), arg_values.get(1)) {
                    (Some(Value::TcpStream { stream, .. }), Some(Value::Bool(nonblocking))) => {
                        let stream_guard =
                            match lock_or_network_error(stream, "tcp_set_nonblocking") {
                                Ok(guard) => guard,
                                Err(error) => return Some(error),
                            };
                        let Some(stream) = stream_guard.as_ref() else {
                            return Some(Value::Error(
                                "tcp_set_nonblocking: TCP stream is closed or upgraded".to_string(),
                            ));
                        };
                        match stream.set_nonblocking(*nonblocking) {
                            Ok(_) => Value::Bool(true),
                            Err(error) => Value::ErrorObject {
                                message: format!(
                                    "Failed to set TCP stream non-blocking mode: {}",
                                    error
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    (Some(Value::TcpListener { listener, .. }), Some(Value::Bool(nonblocking))) => {
                        let listener_guard =
                            match lock_or_network_error(listener, "tcp_set_nonblocking") {
                                Ok(guard) => guard,
                                Err(error) => return Some(error),
                            };
                        match listener_guard.set_nonblocking(*nonblocking) {
                            Ok(_) => Value::Bool(true),
                            Err(error) => Value::ErrorObject {
                                message: format!(
                                    "Failed to set TCP listener non-blocking mode: {}",
                                    error
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "tcp_set_nonblocking requires (TcpStream/TcpListener, bool) arguments"
                            .to_string(),
                    ),
                }
            }
        }

        "tcp_info" => {
            if arg_values.len() != 1 {
                Value::Error("tcp_info requires a TcpStream argument".to_string())
            } else if let Some(Value::TcpStream { stream, peer_addr: _ }) = arg_values.first() {
                let stream_guard = match lock_or_network_error(stream, "tcp_info") {
                    Ok(guard) => guard,
                    Err(error) => return Some(error),
                };
                let Some(stream) = stream_guard.as_ref() else {
                    return Some(Value::Error(
                        "tcp_info: TCP stream is closed or upgraded".to_string(),
                    ));
                };
                let peer_socket_address = match stream.peer_addr() {
                    Ok(address) => address,
                    Err(error) => {
                        return Some(Value::Error(format!(
                            "tcp_info failed to read peer address: {}",
                            error
                        )))
                    }
                };
                let local_socket_address = match stream.local_addr() {
                    Ok(address) => address,
                    Err(error) => {
                        return Some(Value::Error(format!(
                            "tcp_info failed to read local address: {}",
                            error
                        )))
                    }
                };
                let read_timeout_ms = match stream.read_timeout() {
                    Ok(Some(timeout)) => timeout.as_millis().min(i64::MAX as u128) as i64,
                    Ok(None) => 0,
                    Err(error) => {
                        return Some(Value::Error(format!(
                            "tcp_info failed to read receive timeout: {}",
                            error
                        )))
                    }
                };
                let write_timeout_ms = match stream.write_timeout() {
                    Ok(Some(timeout)) => timeout.as_millis().min(i64::MAX as u128) as i64,
                    Ok(None) => 0,
                    Err(error) => {
                        return Some(Value::Error(format!(
                            "tcp_info failed to read send timeout: {}",
                            error
                        )))
                    }
                };
                let mut info = DictMap::default();
                network_insert(&mut info, "schema", network_string_value("kujo.tcp.info.v1"));
                network_insert(
                    &mut info,
                    "peer_address",
                    network_string_value(peer_socket_address.to_string()),
                );
                network_insert(
                    &mut info,
                    "peer_ip",
                    network_string_value(peer_socket_address.ip().to_string()),
                );
                network_insert(
                    &mut info,
                    "peer_port",
                    Value::Int(peer_socket_address.port() as i64),
                );
                network_insert(
                    &mut info,
                    "local_address",
                    network_string_value(local_socket_address.to_string()),
                );
                network_insert(
                    &mut info,
                    "local_ip",
                    network_string_value(local_socket_address.ip().to_string()),
                );
                network_insert(
                    &mut info,
                    "local_port",
                    Value::Int(local_socket_address.port() as i64),
                );
                network_insert(&mut info, "read_timeout_ms", Value::Int(read_timeout_ms));
                network_insert(&mut info, "write_timeout_ms", Value::Int(write_timeout_ms));
                Value::Dict(Arc::new(info))
            } else {
                Value::Error("tcp_info requires a TcpStream argument".to_string())
            }
        }

        "tcp_set_timeouts" => {
            if arg_values.len() != 3 {
                Value::Error(
                    "tcp_set_timeouts requires (TcpStream, int_read_ms, int_write_ms) arguments"
                        .to_string(),
                )
            } else {
                match (arg_values.first(), arg_values.get(1), arg_values.get(2)) {
                    (
                        Some(Value::TcpStream { stream, .. }),
                        Some(Value::Int(read_ms)),
                        Some(Value::Int(write_ms)),
                    ) => {
                        const MAX_TIMEOUT_MS: i64 = 600_000;
                        if *read_ms < 1
                            || *read_ms > MAX_TIMEOUT_MS
                            || *write_ms < 1
                            || *write_ms > MAX_TIMEOUT_MS
                        {
                            Value::Error(
                                "tcp_set_timeouts requires timeout values between 1 and 600000 milliseconds"
                                    .to_string(),
                            )
                        } else {
                            let stream_guard =
                                match lock_or_network_error(stream, "tcp_set_timeouts") {
                                    Ok(guard) => guard,
                                    Err(error) => return Some(error),
                                };
                            let Some(stream) = stream_guard.as_ref() else {
                                return Some(Value::Error(
                                    "tcp_set_timeouts: TCP stream is closed or upgraded".to_string(),
                                ));
                            };
                            let read_timeout =
                                std::time::Duration::from_millis(*read_ms as u64);
                            let write_timeout =
                                std::time::Duration::from_millis(*write_ms as u64);
                            if let Err(error) = stream.set_read_timeout(Some(read_timeout)) {
                                Value::Error(format!(
                                    "tcp_set_timeouts failed to set receive timeout: {}",
                                    error
                                ))
                            } else if let Err(error) =
                                stream.set_write_timeout(Some(write_timeout))
                            {
                                Value::Error(format!(
                                    "tcp_set_timeouts failed to set send timeout: {}",
                                    error
                                ))
                            } else {
                                Value::Bool(true)
                            }
                        }
                    }
                    _ => Value::Error(
                        "tcp_set_timeouts requires (TcpStream, int_read_ms, int_write_ms) arguments"
                            .to_string(),
                    ),
                }
            }
        }

        "udp_bind" => {
            if arg_values.len() != 2 {
                Value::Error("udp_bind requires (string_host, int_port) arguments".to_string())
            } else {
                match (arg_values.first(), arg_values.get(1)) {
                    (Some(Value::Str(host)), Some(Value::Int(port))) => {
                        let address = format!("{}:{}", host.as_ref(), port);
                        match std::net::UdpSocket::bind(&address) {
                            Ok(socket) => {
                                match network_policy::apply_udp_socket_timeouts(&socket, "udp_bind")
                                {
                                    Ok(()) => Value::UdpSocket {
                                        socket: Arc::new(Mutex::new(socket)),
                                        addr: address,
                                    },
                                    Err(error) => Value::ErrorObject {
                                        message: error,
                                        stack: Vec::new(),
                                        line: None,
                                        cause: None,
                                    },
                                }
                            }
                            Err(error) => Value::ErrorObject {
                                message: format!(
                                    "Failed to bind UDP socket on '{}': {}",
                                    address, error
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "udp_bind requires (string_host, int_port) arguments".to_string(),
                    ),
                }
            }
        }

        "udp_send_to" => {
            if arg_values.len() != 4 {
                return Some(Value::Error(
                    "udp_send_to requires (UdpSocket, string_or_bytes_data, string_host, int_port) arguments"
                        .to_string(),
                ));
            }

            match (
                arg_values.first(),
                arg_values.get(1),
                arg_values.get(2),
                arg_values.get(3),
            ) {
                (
                    Some(Value::UdpSocket { socket, .. }),
                    Some(Value::Str(data)),
                    Some(Value::Str(host)),
                    Some(Value::Int(port)),
                ) => {
                    if let Err(error) = network_policy::enforce_host_port_destination_policy(
                        host.as_ref(),
                        *port,
                        "udp_send_to",
                    ) {
                        return Some(Value::ErrorObject {
                            message: error,
                            stack: Vec::new(),
                            line: None,
                            cause: None,
                        });
                    }
                    let address = format!("{}:{}", host.as_ref(), port);
                    let socket_guard = match lock_or_network_error(socket, "udp_send") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    match socket_guard.send_to(data.as_ref().as_bytes(), &address) {
                        Ok(sent_size) => Value::Int(sent_size as i64),
                        Err(error) => Value::ErrorObject {
                            message: timeout_aware_error_message(
                                &format!("Failed to send UDP datagram to '{}'", address),
                                &error,
                            ),
                            stack: Vec::new(),
                            line: None,
                            cause: None,
                        },
                    }
                }
                (
                    Some(Value::UdpSocket { socket, .. }),
                    Some(Value::Bytes(data)),
                    Some(Value::Str(host)),
                    Some(Value::Int(port)),
                ) => {
                    if let Err(error) = network_policy::enforce_host_port_destination_policy(
                        host.as_ref(),
                        *port,
                        "udp_send_to",
                    ) {
                        return Some(Value::ErrorObject {
                            message: error,
                            stack: Vec::new(),
                            line: None,
                            cause: None,
                        });
                    }
                    let address = format!("{}:{}", host.as_ref(), port);
                    let socket_guard = match lock_or_network_error(socket, "udp_receive") {
                        Ok(guard) => guard,
                        Err(error) => return Some(error),
                    };
                    match socket_guard.send_to(data, &address) {
                        Ok(sent_size) => Value::Int(sent_size as i64),
                        Err(error) => Value::ErrorObject {
                            message: timeout_aware_error_message(
                                &format!("Failed to send UDP datagram to '{}'", address),
                                &error,
                            ),
                            stack: Vec::new(),
                            line: None,
                            cause: None,
                        },
                    }
                }
                _ => Value::Error(
                    "udp_send_to requires (UdpSocket, string_or_bytes_data, string_host, int_port) arguments"
                        .to_string(),
                ),
            }
        }

        "udp_receive_from" => {
            if arg_values.len() != 2 {
                Value::Error(
                    "udp_receive_from requires (UdpSocket, int_size) arguments".to_string(),
                )
            } else {
                match (arg_values.first(), arg_values.get(1)) {
                    (Some(Value::UdpSocket { socket, .. }), Some(Value::Int(size))) => {
                        let size = match network_policy::validate_receive_size(
                            *size,
                            "udp_receive_from",
                        ) {
                            Ok(size) => size,
                            Err(error) => return Some(Value::Error(error)),
                        };

                        let socket_guard =
                            match lock_or_network_error(socket, "udp_set_nonblocking") {
                                Ok(guard) => guard,
                                Err(error) => return Some(error),
                            };
                        let mut buffer = vec![0u8; size];
                        match socket_guard.recv_from(&mut buffer) {
                            Ok((read_size, source_address)) => {
                                buffer.truncate(read_size);
                                let data_value = match String::from_utf8(buffer.clone()) {
                                    Ok(text) => Value::Str(Arc::new(text)),
                                    Err(_) => Value::Bytes(buffer),
                                };

                                let mut result = DictMap::default();
                                result.insert(Arc::<str>::from("data"), data_value);
                                result.insert(
                                    Arc::<str>::from("from"),
                                    Value::Str(Arc::new(source_address.to_string())),
                                );
                                result
                                    .insert(Arc::<str>::from("size"), Value::Int(read_size as i64));
                                Value::Dict(Arc::new(result))
                            }
                            Err(error) => Value::ErrorObject {
                                message: timeout_aware_error_message(
                                    "Failed to receive UDP datagram",
                                    &error,
                                ),
                                stack: Vec::new(),
                                line: None,
                                cause: None,
                            },
                        }
                    }
                    _ => Value::Error(
                        "udp_receive_from requires (UdpSocket, int_size) arguments".to_string(),
                    ),
                }
            }
        }

        "udp_close" => {
            if arg_values.len() != 1 {
                Value::Error("udp_close requires a UdpSocket argument".to_string())
            } else {
                if let Some(Value::UdpSocket { .. }) = arg_values.first() {
                    Value::Bool(true)
                } else {
                    Value::Error("udp_close requires a UdpSocket argument".to_string())
                }
            }
        }

        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod cidr_tests {
    use super::*;

    #[test]
    fn cidr_membership_is_family_aware_and_prefix_bounded() {
        assert!(ip_cidr_contains("192.0.2.4".parse().unwrap(), "192.0.2.0/24").unwrap());
        assert!(!ip_cidr_contains("192.0.3.4".parse().unwrap(), "192.0.2.0/24").unwrap());
        assert!(ip_cidr_contains("2001:db8::1".parse().unwrap(), "2001:db8::/32").unwrap());
        assert!(!ip_cidr_contains("192.0.2.1".parse().unwrap(), "2001:db8::/32").unwrap());
        assert!(ip_cidr_contains("192.0.2.1".parse().unwrap(), "192.0.2.1/33").is_err());
    }
}

#[cfg(test)]
mod ip_classification_tests {
    use super::*;

    #[test]
    fn fail_closed_ip_scope_classification() {
        assert_eq!(ip_scope("8.8.8.8".parse().unwrap()), ("global", true));
        assert_eq!(ip_scope("127.0.0.1".parse().unwrap()), ("loopback", false));
        assert_eq!(ip_scope("10.0.0.1".parse().unwrap()), ("private", false));
        assert_eq!(ip_scope("192.0.2.1".parse().unwrap()), ("documentation", false));
        assert_eq!(ip_scope("100.64.0.1".parse().unwrap()), ("shared", false));
        assert_eq!(ip_scope("2001:4860:4860::8888".parse().unwrap()), ("global", true));
        assert_eq!(ip_scope("2001:db8::1".parse().unwrap()), ("documentation", false));
        assert_eq!(ip_scope("::ffff:8.8.8.8".parse().unwrap()), ("ipv4_mapped", false));
    }
}
