// Capability-gated TLS sockets and STARTTLS-style TCP stream upgrades.

use super::file_stream::write_file_range;
use crate::interpreter::{DictMap, Interpreter, Value};
use crate::network_policy;
use openssl::hash::MessageDigest;
use openssl::ssl::{
    HandshakeError, SslAcceptor, SslConnector, SslFiletype, SslMethod, SslStream, SslVersion,
};
use openssl::x509::X509;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Debug)]
struct ClientOptions {
    min_version: SslVersion,
    ca_pem: Option<String>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self { min_version: SslVersion::TLS1_2, ca_pem: None }
    }
}

fn error(message: impl Into<String>) -> Value {
    Value::ErrorObject { message: message.into(), stack: Vec::new(), line: None, cause: None }
}

fn string_value(value: impl Into<String>) -> Value {
    Value::Str(Arc::new(value.into()))
}

fn insert(map: &mut DictMap, key: &'static str, value: Value) {
    map.insert(Arc::from(key), value);
}

fn lock_or_tls_error<'a, T>(
    mutex: &'a Mutex<T>,
    context: &str,
) -> Result<MutexGuard<'a, T>, Value> {
    mutex.lock().map_err(|_| error(format!("{}: shared TLS lock poisoned", context)))
}

fn parse_min_version(value: Option<&Value>) -> Result<SslVersion, String> {
    match value {
        None => Ok(SslVersion::TLS1_2),
        Some(Value::Str(version)) if version.as_ref() == "1.2" => Ok(SslVersion::TLS1_2),
        Some(Value::Str(version)) if version.as_ref() == "1.3" => Ok(SslVersion::TLS1_3),
        Some(Value::Str(_)) => {
            Err("TLS min_version must be '1.2' or '1.3'; legacy TLS is not supported".to_string())
        }
        Some(_) => Err("TLS min_version must be a string".to_string()),
    }
}

fn dict_like_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Dict(options) => options.get(key),
        Value::FixedDict { keys, values } => keys
            .iter()
            .position(|candidate| candidate.as_ref() == key)
            .and_then(|index| values.get(index)),
        _ => None,
    }
}

fn dict_like_keys(value: &Value) -> Option<Vec<&str>> {
    match value {
        Value::Dict(options) => Some(options.keys().map(|key| key.as_ref()).collect()),
        Value::FixedDict { keys, .. } => Some(keys.iter().map(|key| key.as_ref()).collect()),
        _ => None,
    }
}

fn parse_client_options(value: Option<&Value>) -> Result<ClientOptions, String> {
    let Some(value) = value else {
        return Ok(ClientOptions::default());
    };
    let Some(keys) = dict_like_keys(value) else {
        return Err("TLS client options must be a dictionary".to_string());
    };
    let allowed = ["min_version", "ca_pem"];
    if let Some(unknown) = keys.into_iter().find(|key| !allowed.contains(key)) {
        return Err(format!("unknown TLS client option '{}'", unknown));
    }
    let ca_pem = match dict_like_get(value, "ca_pem") {
        None => None,
        Some(Value::Str(value)) if value.len() <= 1024 * 1024 => Some(value.as_ref().clone()),
        Some(Value::Str(_)) => return Err("TLS ca_pem exceeds the 1 MiB limit".to_string()),
        Some(_) => return Err("TLS ca_pem must be a string".to_string()),
    };
    Ok(ClientOptions {
        min_version: parse_min_version(dict_like_get(value, "min_version"))?,
        ca_pem,
    })
}

fn parse_server_min_version(value: Option<&Value>) -> Result<SslVersion, String> {
    let Some(value) = value else {
        return Ok(SslVersion::TLS1_2);
    };
    let Some(keys) = dict_like_keys(value) else {
        return Err("TLS acceptor options must be a dictionary".to_string());
    };
    if let Some(unknown) = keys.into_iter().find(|key| *key != "min_version") {
        return Err(format!("unknown TLS acceptor option '{}'", unknown));
    }
    parse_min_version(dict_like_get(value, "min_version"))
}

fn version_name(version: SslVersion) -> &'static str {
    if version == SslVersion::TLS1_3 {
        "TLSv1.3"
    } else {
        "TLSv1.2"
    }
}

fn certificate_sha256(certificate: &X509) -> Result<String, String> {
    certificate
        .digest(MessageDigest::sha256())
        .map(|digest| digest.iter().map(|byte| format!("{:02x}", byte)).collect())
        .map_err(|_| "TLS certificate fingerprint calculation failed".to_string())
}

fn build_connector(options: &ClientOptions) -> Result<SslConnector, String> {
    let mut builder = SslConnector::builder(SslMethod::tls_client())
        .map_err(|_| "TLS client context initialization failed".to_string())?;
    builder
        .set_min_proto_version(Some(options.min_version))
        .map_err(|_| "TLS minimum protocol configuration failed".to_string())?;
    if let Some(ca_pem) = &options.ca_pem {
        let certificates = X509::stack_from_pem(ca_pem.as_bytes())
            .map_err(|_| "TLS ca_pem does not contain valid PEM certificates".to_string())?;
        if certificates.is_empty() {
            return Err("TLS ca_pem does not contain a certificate".to_string());
        }
        for certificate in certificates {
            builder
                .cert_store_mut()
                .add_cert(certificate)
                .map_err(|_| "TLS CA certificate installation failed".to_string())?;
        }
    }
    Ok(builder.build())
}

#[cfg(unix)]
fn validate_private_key_permissions(path: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(path)
        .map_err(|_| "TLS private-key metadata could not be read".to_string())?;
    let mode = metadata.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(format!(
            "TLS private key permissions must deny group/other access; observed mode {:03o}",
            mode
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_private_key_permissions(path: &str) -> Result<(), String> {
    std::fs::metadata(path)
        .map(|_| ())
        .map_err(|_| "TLS private-key metadata could not be read".to_string())
}

fn build_acceptor(
    certificate_chain_path: &str,
    private_key_path: &str,
    min_version: SslVersion,
) -> Result<(SslAcceptor, String), String> {
    validate_private_key_permissions(private_key_path)?;
    let certificate_bytes = std::fs::read(certificate_chain_path)
        .map_err(|_| "TLS certificate chain could not be read".to_string())?;
    if certificate_bytes.len() > 4 * 1024 * 1024 {
        return Err("TLS certificate chain exceeds the 4 MiB limit".to_string());
    }
    let certificates = X509::stack_from_pem(&certificate_bytes)
        .map_err(|_| "TLS certificate chain is not valid PEM".to_string())?;
    let certificate =
        certificates.first().ok_or_else(|| "TLS certificate chain is empty".to_string())?;
    let fingerprint = certificate_sha256(certificate)?;

    let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server())
        .map_err(|_| "TLS server context initialization failed".to_string())?;
    builder
        .set_min_proto_version(Some(min_version))
        .map_err(|_| "TLS minimum protocol configuration failed".to_string())?;
    builder
        .set_certificate_chain_file(certificate_chain_path)
        .map_err(|_| "TLS certificate chain installation failed".to_string())?;
    builder
        .set_private_key_file(private_key_path, SslFiletype::PEM)
        .map_err(|_| "TLS private key installation failed".to_string())?;
    builder
        .check_private_key()
        .map_err(|_| "TLS private key does not match the certificate".to_string())?;
    Ok((builder.build(), fingerprint))
}

fn handshake_error(role: &str, err: HandshakeError<TcpStream>) -> Value {
    let category = match err {
        HandshakeError::SetupFailure(_) => "SETUP_FAILED",
        HandshakeError::Failure(_) => "HANDSHAKE_FAILED",
        HandshakeError::WouldBlock(_) => "HANDSHAKE_TIMED_OUT",
    };
    error(format!("TLS {} {}", role, category))
}

fn tls_value(
    stream: SslStream<TcpStream>,
    peer_addr: String,
    server_name: String,
    peer_verified: bool,
) -> Value {
    let protocol = stream.ssl().version_str().to_string();
    let cipher = stream
        .ssl()
        .current_cipher()
        .map(|cipher| cipher.name().to_string())
        .unwrap_or_else(|| "UNKNOWN".to_string());
    let peer_certificate_sha256 = stream
        .ssl()
        .peer_certificate()
        .and_then(|certificate| certificate_sha256(&certificate).ok());
    Value::TlsStream {
        stream: Arc::new(Mutex::new(Some(stream))),
        peer_addr,
        server_name,
        protocol,
        cipher,
        peer_certificate_sha256,
        peer_verified,
    }
}

fn take_tcp_stream(value: &Value, context: &str) -> Result<(TcpStream, String), Value> {
    let Value::TcpStream { stream, peer_addr } = value else {
        return Err(Value::Error(format!("{} requires a TcpStream", context)));
    };
    let mut guard = lock_or_tls_error(stream, context)?;
    let stream = guard.take().ok_or_else(|| {
        Value::Error(format!("{}: TCP stream is closed or already upgraded", context))
    })?;
    Ok((stream, peer_addr.clone()))
}

fn tls_connect(arg_values: &[Value]) -> Value {
    if !(2..=3).contains(&arg_values.len()) {
        return Value::Error(
            "tls_connect requires (string_host, int_port, optional_options) arguments".to_string(),
        );
    }
    let (Some(Value::Str(host)), Some(Value::Int(port))) = (arg_values.first(), arg_values.get(1))
    else {
        return Value::Error(
            "tls_connect requires (string_host, int_port, optional_options) arguments".to_string(),
        );
    };
    if let Err(message) =
        network_policy::enforce_host_port_destination_policy(host.as_ref(), *port, "tls_connect")
    {
        return error(message);
    }
    let options = match parse_client_options(arg_values.get(2)) {
        Ok(options) => options,
        Err(message) => return Value::Error(message),
    };
    let connector = match build_connector(&options) {
        Ok(connector) => connector,
        Err(message) => return error(message),
    };
    let address = format!("{}:{}", host, port);
    let stream = match network_policy::connect_tcp_stream(&address, "tls_connect") {
        Ok(stream) => stream,
        Err(message) => return error(message),
    };
    match connector.connect(host.as_ref(), stream) {
        Ok(stream) => tls_value(stream, address, host.as_ref().clone(), true),
        Err(err) => handshake_error("client", err),
    }
}

fn tls_upgrade_client(arg_values: &[Value]) -> Value {
    if !(2..=3).contains(&arg_values.len()) {
        return Value::Error(
            "tls_upgrade_client requires (TcpStream, string_server_name, optional_options) arguments"
                .to_string(),
        );
    }
    let Some(Value::Str(server_name)) = arg_values.get(1) else {
        return Value::Error("tls_upgrade_client requires a string server_name".to_string());
    };
    let options = match parse_client_options(arg_values.get(2)) {
        Ok(options) => options,
        Err(message) => return Value::Error(message),
    };
    let connector = match build_connector(&options) {
        Ok(connector) => connector,
        Err(message) => return error(message),
    };
    let (stream, peer_addr) = match take_tcp_stream(&arg_values[0], "tls_upgrade_client") {
        Ok(stream) => stream,
        Err(err) => return err,
    };
    match connector.connect(server_name.as_ref(), stream) {
        Ok(stream) => tls_value(stream, peer_addr, server_name.as_ref().clone(), true),
        Err(err) => handshake_error("client", err),
    }
}

fn tls_acceptor(arg_values: &[Value]) -> Value {
    if !(2..=3).contains(&arg_values.len()) {
        return Value::Error(
            "tls_acceptor requires (string_certificate_chain_path, string_private_key_path, optional_options) arguments"
                .to_string(),
        );
    }
    let (Some(Value::Str(certificate_path)), Some(Value::Str(private_key_path))) =
        (arg_values.first(), arg_values.get(1))
    else {
        return Value::Error(
            "tls_acceptor requires string certificate-chain and private-key paths".to_string(),
        );
    };
    let min_version = match parse_server_min_version(arg_values.get(2)) {
        Ok(version) => version,
        Err(message) => return Value::Error(message),
    };
    match build_acceptor(certificate_path, private_key_path, min_version) {
        Ok((acceptor, certificate_sha256)) => Value::TlsAcceptor {
            acceptor: Arc::new(acceptor),
            certificate_sha256,
            min_protocol: version_name(min_version).to_string(),
        },
        Err(message) => error(message),
    }
}

fn tls_upgrade_server(arg_values: &[Value]) -> Value {
    if arg_values.len() != 2 {
        return Value::Error(
            "tls_upgrade_server requires (TcpStream, TlsAcceptor) arguments".to_string(),
        );
    }
    let Some(Value::TlsAcceptor { acceptor, .. }) = arg_values.get(1) else {
        return Value::Error(
            "tls_upgrade_server requires a TlsAcceptor as its second argument".to_string(),
        );
    };
    let (stream, peer_addr) = match take_tcp_stream(&arg_values[0], "tls_upgrade_server") {
        Ok(stream) => stream,
        Err(err) => return err,
    };
    match acceptor.accept(stream) {
        Ok(stream) => tls_value(stream, peer_addr, String::new(), false),
        Err(err) => handshake_error("server", err),
    }
}

fn tls_send(arg_values: &[Value]) -> Value {
    if arg_values.len() != 2 {
        return Value::Error(
            "tls_send requires (TlsStream, string_or_bytes_data) arguments".to_string(),
        );
    }
    let Some(Value::TlsStream { stream, .. }) = arg_values.first() else {
        return Value::Error("tls_send requires a TlsStream".to_string());
    };
    let data: &[u8] = match arg_values.get(1) {
        Some(Value::Str(value)) => value.as_bytes(),
        Some(Value::Bytes(value)) => value,
        _ => return Value::Error("tls_send requires string or bytes data".to_string()),
    };
    let mut guard = match lock_or_tls_error(stream, "tls_send") {
        Ok(guard) => guard,
        Err(err) => return err,
    };
    let Some(stream) = guard.as_mut() else {
        return Value::Error("tls_send: TLS stream is closed".to_string());
    };
    if stream.write_all(data).is_err() || stream.flush().is_err() {
        return error("TLS write failed");
    }
    Value::Int(data.len() as i64)
}

fn tls_send_file_range(arg_values: &[Value]) -> Value {
    let (
        Some(Value::TlsStream { stream, .. }),
        Some(Value::Str(path)),
        Some(Value::Int(offset)),
        Some(Value::Int(count)),
    ) = (arg_values.first(), arg_values.get(1), arg_values.get(2), arg_values.get(3))
    else {
        return Value::Error(
            "tls_send_file_range requires (TlsStream, string_path, int_offset, int_count) arguments".to_string(),
        );
    };
    if arg_values.len() != 4 {
        return Value::Error(
            "tls_send_file_range requires (TlsStream, string_path, int_offset, int_count) arguments".to_string(),
        );
    }
    let mut guard = match lock_or_tls_error(stream, "tls_send_file_range") {
        Ok(guard) => guard,
        Err(err) => return err,
    };
    let Some(stream) = guard.as_mut() else {
        return Value::Error("tls_send_file_range: TLS stream is closed".to_string());
    };
    match write_file_range(stream, path, *offset, *count, "tls_send_file_range") {
        Ok(sent) => Value::Int(sent),
        Err(message) => error(message),
    }
}

fn tls_receive(arg_values: &[Value]) -> Value {
    if arg_values.len() != 2 {
        return Value::Error("tls_receive requires (TlsStream, int_size) arguments".to_string());
    }
    let (Some(Value::TlsStream { stream, .. }), Some(Value::Int(size))) =
        (arg_values.first(), arg_values.get(1))
    else {
        return Value::Error("tls_receive requires (TlsStream, int_size) arguments".to_string());
    };
    let size = match network_policy::validate_receive_size(*size, "tls_receive") {
        Ok(size) => size,
        Err(message) => return Value::Error(message),
    };
    let mut guard = match lock_or_tls_error(stream, "tls_receive") {
        Ok(guard) => guard,
        Err(err) => return err,
    };
    let Some(stream) = guard.as_mut() else {
        return Value::Error("tls_receive: TLS stream is closed".to_string());
    };
    let mut buffer = vec![0_u8; size];
    match stream.read(&mut buffer) {
        Ok(read) => {
            buffer.truncate(read);
            match String::from_utf8(buffer.clone()) {
                Ok(text) => string_value(text),
                Err(_) => Value::Bytes(buffer),
            }
        }
        Err(_) => error("TLS read failed or timed out"),
    }
}

fn tls_close(arg_values: &[Value]) -> Value {
    if arg_values.len() != 1 {
        return Value::Error("tls_close requires a TlsStream".to_string());
    }
    let Some(Value::TlsStream { stream, .. }) = arg_values.first() else {
        return Value::Error("tls_close requires a TlsStream".to_string());
    };
    let mut guard = match lock_or_tls_error(stream, "tls_close") {
        Ok(guard) => guard,
        Err(err) => return err,
    };
    if let Some(mut stream) = guard.take() {
        let _ = stream.shutdown();
        let _ = stream.get_ref().shutdown(Shutdown::Both);
    }
    Value::Bool(true)
}

fn tls_info(arg_values: &[Value]) -> Value {
    if arg_values.len() != 1 {
        return Value::Error("tls_info requires a TlsStream or TlsAcceptor".to_string());
    }
    let mut map = DictMap::default();
    insert(&mut map, "schema_version", string_value("kujo.tls.info.v1"));
    match arg_values.first() {
        Some(Value::TlsStream {
            peer_addr,
            server_name,
            protocol,
            cipher,
            peer_certificate_sha256,
            peer_verified,
            ..
        }) => {
            insert(&mut map, "kind", string_value("stream"));
            insert(&mut map, "peer_address", string_value(peer_addr.clone()));
            insert(&mut map, "server_name", string_value(server_name.clone()));
            insert(&mut map, "protocol", string_value(protocol.clone()));
            insert(&mut map, "cipher", string_value(cipher.clone()));
            insert(&mut map, "peer_verified", Value::Bool(*peer_verified));
            insert(
                &mut map,
                "peer_certificate_sha256",
                peer_certificate_sha256
                    .as_ref()
                    .map(|value| string_value(value.clone()))
                    .unwrap_or(Value::Null),
            );
        }
        Some(Value::TlsAcceptor { certificate_sha256, min_protocol, .. }) => {
            insert(&mut map, "kind", string_value("acceptor"));
            insert(&mut map, "certificate_sha256", string_value(certificate_sha256.clone()));
            insert(&mut map, "min_protocol", string_value(min_protocol.clone()));
        }
        _ => return Value::Error("tls_info requires a TlsStream or TlsAcceptor".to_string()),
    }
    Value::dict(map)
}

pub fn handle(_interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value> {
    Some(match name {
        "tls_connect" => tls_connect(arg_values),
        "tls_upgrade_client" => tls_upgrade_client(arg_values),
        "tls_acceptor" => tls_acceptor(arg_values),
        "tls_upgrade_server" => tls_upgrade_server(arg_values),
        "tls_send" => tls_send(arg_values),
        "tls_send_file_range" => tls_send_file_range(arg_values),
        "tls_receive" => tls_receive(arg_values),
        "tls_close" => tls_close(arg_values),
        "tls_info" => tls_info(arg_values),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::asn1::Asn1Time;
    use openssl::bn::BigNum;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectAlternativeName};
    use openssl::x509::{X509Builder, X509NameBuilder};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEST_ID: AtomicUsize = AtomicUsize::new(1);

    fn local_test_certificate() -> (Vec<u8>, Vec<u8>) {
        let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "localhost").unwrap();
        let name = name.build();
        let mut builder = X509Builder::new().unwrap();
        builder.set_version(2).unwrap();
        let serial = BigNum::from_u32(1).unwrap().to_asn1_integer().unwrap();
        builder.set_serial_number(&serial).unwrap();
        builder.set_subject_name(&name).unwrap();
        builder.set_issuer_name(&name).unwrap();
        builder.set_pubkey(&key).unwrap();
        builder.set_not_before(&Asn1Time::days_from_now(0).unwrap()).unwrap();
        builder.set_not_after(&Asn1Time::days_from_now(1).unwrap()).unwrap();
        builder.append_extension(BasicConstraints::new().critical().ca().build().unwrap()).unwrap();
        builder
            .append_extension(
                KeyUsage::new()
                    .critical()
                    .digital_signature()
                    .key_encipherment()
                    .key_cert_sign()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let context = builder.x509v3_context(None, None);
        builder
            .append_extension(
                SubjectAlternativeName::new()
                    .dns("localhost")
                    .ip("127.0.0.1")
                    .build(&context)
                    .unwrap(),
            )
            .unwrap();
        builder.sign(&key, MessageDigest::sha256()).unwrap();
        (builder.build().to_pem().unwrap(), key.private_key_to_pem_pkcs8().unwrap())
    }

    #[test]
    fn rejects_legacy_tls_and_unknown_options() {
        assert!(parse_min_version(Some(&string_value("1.0"))).is_err());
        let mut options = DictMap::default();
        insert(&mut options, "verify_hostname", Value::Bool(false));
        assert!(parse_client_options(Some(&Value::dict(options))).is_err());
    }

    #[test]
    fn accepts_bounded_public_ca_pem_input_shape() {
        let mut options = DictMap::default();
        insert(&mut options, "min_version", string_value("1.3"));
        insert(&mut options, "ca_pem", string_value("not-yet-parsed"));
        let parsed = parse_client_options(Some(&Value::dict(options))).unwrap();
        assert_eq!(parsed.min_version, SslVersion::TLS1_3);
        assert_eq!(parsed.ca_pem.as_deref(), Some("not-yet-parsed"));
    }

    #[test]
    fn accepts_vm_fixed_dictionary_tls_options() {
        let server_options = Value::FixedDict {
            keys: Arc::new(vec![Arc::from("min_version")]),
            values: vec![string_value("1.3")],
        };
        assert_eq!(parse_server_min_version(Some(&server_options)).unwrap(), SslVersion::TLS1_3);

        let client_options = Value::FixedDict {
            keys: Arc::new(vec![Arc::from("min_version"), Arc::from("ca_pem")]),
            values: vec![string_value("1.2"), string_value("fixture-ca")],
        };
        let parsed = parse_client_options(Some(&client_options)).unwrap();
        assert_eq!(parsed.min_version, SslVersion::TLS1_2);
        assert_eq!(parsed.ca_pem.as_deref(), Some("fixture-ca"));
    }

    #[test]
    fn client_server_upgrade_round_trip_and_consume_plain_stream() {
        let test_id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let test_dir = std::env::temp_dir().join(format!(
            "kujo-tls-native-test-{}-{}",
            std::process::id(),
            test_id
        ));
        std::fs::create_dir(&test_dir).unwrap();
        let certificate_path = test_dir.join("certificate.pem");
        let private_key_path = test_dir.join("private-key.pem");
        let (certificate_pem, private_key_pem) = local_test_certificate();
        std::fs::write(&certificate_path, &certificate_pem).unwrap();
        std::fs::write(&private_key_path, private_key_pem).unwrap();
        let payload_path = test_dir.join("payload.bin");
        std::fs::write(&payload_path, b"prefix-ping-suffix").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&private_key_path, std::fs::Permissions::from_mode(0o600))
                .unwrap();
        }

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server_certificate_path = certificate_path.to_string_lossy().into_owned();
        let server_private_key_path = private_key_path.to_string_lossy().into_owned();
        let server = std::thread::spawn(move || {
            let (stream, peer) = listener.accept().unwrap();
            network_policy::apply_tcp_stream_timeouts(&stream, "tls test server").unwrap();
            let plain = Value::TcpStream {
                stream: Arc::new(Mutex::new(Some(stream))),
                peer_addr: peer.to_string(),
            };
            let mut interpreter = Interpreter::new();
            let acceptor = handle(
                &mut interpreter,
                "tls_acceptor",
                &[string_value(server_certificate_path), string_value(server_private_key_path)],
            )
            .unwrap();
            assert!(matches!(acceptor, Value::TlsAcceptor { .. }), "{acceptor:?}");
            let tls =
                handle(&mut interpreter, "tls_upgrade_server", &[plain.clone(), acceptor]).unwrap();
            assert!(matches!(tls, Value::TlsStream { .. }), "{tls:?}");
            let received =
                handle(&mut interpreter, "tls_receive", &[tls.clone(), Value::Int(64)]).unwrap();
            assert!(matches!(received, Value::Str(value) if value.as_ref() == "ping"));
            let sent =
                handle(&mut interpreter, "tls_send", &[tls.clone(), string_value("pong")]).unwrap();
            assert!(matches!(sent, Value::Int(4)));
            handle(&mut interpreter, "tls_close", &[tls]).unwrap();
        });

        let stream = TcpStream::connect(address).unwrap();
        network_policy::apply_tcp_stream_timeouts(&stream, "tls test client").unwrap();
        let plain = Value::TcpStream {
            stream: Arc::new(Mutex::new(Some(stream))),
            peer_addr: address.to_string(),
        };
        let mut options = DictMap::default();
        insert(&mut options, "ca_pem", string_value(String::from_utf8(certificate_pem).unwrap()));
        let mut interpreter = Interpreter::new();
        let tls = handle(
            &mut interpreter,
            "tls_upgrade_client",
            &[plain.clone(), string_value("localhost"), Value::dict(options)],
        )
        .unwrap();
        assert!(matches!(tls, Value::TlsStream { peer_verified: true, .. }), "{tls:?}");
        let plain_reuse = crate::interpreter::native_functions::network::handle(
            &mut interpreter,
            "tcp_send",
            &[plain, string_value("unsafe")],
        )
        .unwrap();
        assert!(
            matches!(plain_reuse, Value::Error(message) if message.contains("closed or upgraded"))
        );
        let sent = handle(
            &mut interpreter,
            "tls_send_file_range",
            &[
                tls.clone(),
                string_value(payload_path.to_string_lossy().into_owned()),
                Value::Int(7),
                Value::Int(4),
            ],
        )
        .unwrap();
        assert!(matches!(sent, Value::Int(4)));
        let received =
            handle(&mut interpreter, "tls_receive", &[tls.clone(), Value::Int(64)]).unwrap();
        assert!(matches!(received, Value::Str(value) if value.as_ref() == "pong"));
        let info = tls_info(&[tls.clone()]);
        let Value::Dict(info) = info else { panic!("expected tls info dictionary") };
        assert!(matches!(info.get("peer_verified"), Some(Value::Bool(true))));
        assert!(matches!(
            info.get("peer_certificate_sha256"),
            Some(Value::Str(value)) if value.len() == 64
        ));
        handle(&mut interpreter, "tls_close", &[tls]).unwrap();
        server.join().unwrap();
        std::fs::remove_dir_all(&test_dir).unwrap();
    }
}
