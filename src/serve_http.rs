use crate::path_security;
use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const MAX_REQUEST_TARGET_BYTES: usize = 4096;
const BLOCKED_PRIVATE_PATH_NAMES: [&str; 5] = [".env", ".git", ".svn", ".hg", ".ds_store"];
const BLOCKED_PRIVATE_PATH_SUFFIXES: [&str; 7] =
    [".bak", ".backup", ".tmp", ".old", ".orig", ".swp", ".swo"];

#[derive(Debug, Clone)]
pub struct ServeServerOptions {
    pub index: String,
    pub hardened: bool,
    pub cache_max_age: Option<u64>,
    pub access_log: bool,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub max_request_line_bytes: usize,
    pub max_header_bytes: usize,
    pub max_header_count: usize,
    pub max_request_body_bytes: usize,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub max_connections: usize,
}

struct PlannedResponse {
    status_code: u16,
    body: Box<dyn Read + Send>,
    headers: Vec<(String, String)>,
    content_length: Option<usize>,
}

#[derive(Debug)]
enum RequestTargetValidationError {
    BadRequest,
    Forbidden,
    UriTooLong,
}

#[derive(Debug)]
enum RequestLimitsError {
    RequestLineTooLong,
    HeadersTooLarge,
    TooManyHeaders,
    PayloadTooLarge,
}

pub fn run_static_server(
    dir: PathBuf,
    host: String,
    port: u16,
    options: ServeServerOptions,
) -> Result<(), String> {
    validate_server_options(&options)?;

    let root_dir = fs::canonicalize(&dir)
        .map_err(|err| format!("Failed to resolve serve directory '{}': {}", dir.display(), err))?;

    if !root_dir.is_dir() {
        return Err(format!("Serve target is not a directory: {}", root_dir.display()));
    }

    let tls_enabled = options.tls_cert.is_some() || options.tls_key.is_some();
    if tls_enabled && (options.tls_cert.is_none() || options.tls_key.is_none()) {
        return Err(
            "HTTPS requires both --tls-cert and --tls-key to be provided together".to_string()
        );
    }

    let bind_addr = format!("{}:{}", host, port);
    let listener = std::net::TcpListener::bind(&bind_addr)
        .map_err(|err| format!("Failed to bind {}: {}", bind_addr, err))?;

    let server = if tls_enabled {
        let certificate = fs::read(options.tls_cert.as_ref().expect("tls cert exists"))
            .map_err(|err| format!("Failed to read TLS certificate file: {}", err))?;
        let private_key = fs::read(options.tls_key.as_ref().expect("tls key exists"))
            .map_err(|err| format!("Failed to read TLS private key file: {}", err))?;

        Server::from_listener(listener, Some(tiny_http::SslConfig { certificate, private_key }))
            .map_err(|err| format!("Failed to start HTTPS server on {}: {}", bind_addr, err))?
    } else {
        Server::from_listener(listener, None)
            .map_err(|err| format!("Failed to start server on {}: {}", bind_addr, err))?
    };

    let scheme = if tls_enabled { "https" } else { "http" };
    println!("Serving {} on {}://{}", root_dir.display(), scheme, bind_addr);
    println!("Press Ctrl+C to stop");

    let root_dir = Arc::new(root_dir);
    let options = Arc::new(options);
    let active_requests = Arc::new(AtomicUsize::new(0));

    loop {
        let request = match server.recv_timeout(options.read_timeout) {
            Ok(Some(request)) => request,
            Ok(None) => continue,
            Err(error) => return Err(format!("HTTP server receive error: {}", error)),
        };

        if !try_acquire_request_slot(&active_requests, options.max_connections) {
            let response = static_text_response(
                503,
                "Service Unavailable",
                options.as_ref(),
                request.secure(),
                &[],
            );
            let response = planned_to_http_response(response);
            let _ = request.respond(response);
            continue;
        }

        let root_dir = Arc::clone(&root_dir);
        let options = Arc::clone(&options);
        let active_requests = Arc::clone(&active_requests);

        thread::spawn(move || {
            let _request_slot_guard = ActiveRequestSlotGuard { active: active_requests };
            let started = Instant::now();
            let method = request.method().clone();
            let url = request.url().to_string();
            let secure = request.secure();

            let planned = build_response(root_dir.as_ref(), options.as_ref(), &request);
            let status_code = planned.status_code;
            let response_bytes = planned.content_length.unwrap_or(0);
            let response = planned_to_http_response(planned);
            let _ = request.respond(response);

            log_access(
                options.access_log,
                &method,
                &url,
                status_code,
                response_bytes,
                started.elapsed().as_millis(),
                secure,
            );
        });
    }
}

fn validate_server_options(options: &ServeServerOptions) -> Result<(), String> {
    if 0 == options.max_request_line_bytes {
        return Err("serve max request line bytes must be greater than 0".to_string());
    }
    if 0 == options.max_header_bytes {
        return Err("serve max header bytes must be greater than 0".to_string());
    }
    if 0 == options.max_header_count {
        return Err("serve max header count must be greater than 0".to_string());
    }
    if 0 == options.max_request_body_bytes {
        return Err("serve max request body bytes must be greater than 0".to_string());
    }
    if 0 == options.max_connections {
        return Err("serve max connections must be greater than 0".to_string());
    }
    if options.read_timeout.is_zero() {
        return Err("serve read timeout must be greater than 0ms".to_string());
    }
    if options.write_timeout.is_zero() {
        return Err("serve write timeout must be greater than 0ms".to_string());
    }

    Ok(())
}

fn try_acquire_request_slot(active: &AtomicUsize, max_connections: usize) -> bool {
    loop {
        let current = active.load(Ordering::Acquire);
        if current >= max_connections {
            return false;
        }

        match active.compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return true,
            Err(_) => continue,
        }
    }
}

struct ActiveRequestSlotGuard {
    active: Arc<AtomicUsize>,
}

impl Drop for ActiveRequestSlotGuard {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

fn planned_to_http_response(plan: PlannedResponse) -> tiny_http::ResponseBox {
    let mut response = Response::new(
        StatusCode(plan.status_code),
        Vec::new(),
        plan.body,
        plan.content_length,
        None,
    )
    .with_chunked_threshold(usize::MAX);
    if let Some(content_length) = plan.content_length {
        if let Ok(content_length_header) =
            Header::from_bytes("Content-Length".as_bytes(), content_length.to_string().as_bytes())
        {
            response.add_header(content_length_header);
        }
    }

    for (name, value) in plan.headers.iter() {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response.add_header(header);
        }
    }

    response
}

fn build_response(
    root_dir: &Path,
    options: &ServeServerOptions,
    request: &Request,
) -> PlannedResponse {
    let method = request.method();
    let is_head = method == &Method::Head;

    if let Err(error) = validate_request_limits(request, options) {
        let (status_code, body) = match error {
            RequestLimitsError::RequestLineTooLong => (414, "URI Too Long"),
            RequestLimitsError::HeadersTooLarge => (413, "Payload Too Large"),
            RequestLimitsError::TooManyHeaders => (413, "Payload Too Large"),
            RequestLimitsError::PayloadTooLarge => (413, "Payload Too Large"),
        };
        return headify(
            static_text_response(status_code, body, options, request.secure(), &[]),
            is_head,
        );
    }

    if method != &Method::Get && !is_head {
        let response = if matches!(method, Method::NonStandard(_)) {
            static_text_response(501, "Not Implemented", options, request.secure(), &[])
        } else {
            static_text_response(
                405,
                "Method Not Allowed",
                options,
                request.secure(),
                &[("Allow", "GET, HEAD")],
            )
        };
        return headify(response, is_head);
    }

    let relative_path = match validate_request_target(request.url(), &options.index) {
        Ok(path) => path,
        Err(error) => {
            let (status_code, body) = match error {
                RequestTargetValidationError::BadRequest => (400, "Bad Request"),
                RequestTargetValidationError::Forbidden => (403, "Forbidden"),
                RequestTargetValidationError::UriTooLong => (414, "URI Too Long"),
            };
            return headify(
                static_text_response(status_code, body, options, request.secure(), &[]),
                is_head,
            );
        }
    };

    let mut target_path =
        match path_security::join_within_root(root_dir, &relative_path, "request path") {
            Ok(path) => path,
            Err(_) => {
                return headify(
                    static_text_response(403, "Forbidden", options, request.secure(), &[]),
                    is_head,
                );
            }
        };
    if target_path.is_dir() {
        target_path = target_path.join(&options.index);
    }

    let canonical_target = match fs::canonicalize(&target_path) {
        Ok(path) => path,
        Err(error) => {
            return headify(status_mapped_error(&error, options, request.secure()), is_head);
        }
    };

    if path_security::ensure_path_within_root(&canonical_target, root_dir, "request path").is_err()
    {
        return headify(
            static_text_response(403, "Forbidden", options, request.secure(), &[]),
            is_head,
        );
    }

    let accept_encoding = header_value(request.headers(), "Accept-Encoding");
    let selected_target = choose_precompressed_path(&canonical_target, accept_encoding.as_deref());

    let (mut file, file_len) = match open_file_nofollow(&selected_target) {
        Ok(file_info) => file_info,
        Err(error) => {
            return headify(status_mapped_error(&error, options, request.secure()), is_head);
        }
    };

    let etag = weak_etag(&selected_target, file_len);
    if let Some(if_none_match) = header_value(request.headers(), "If-None-Match") {
        if if_none_match_matches(&if_none_match, &etag) {
            let mut response = PlannedResponse {
                status_code: 304,
                body: boxed_bytes(Vec::new()),
                headers: Vec::new(),
                content_length: Some(0),
            };
            add_common_success_headers(
                &mut response.headers,
                &selected_target,
                &[],
                Some(&etag),
                options,
                request.secure(),
            );
            return headify(response, true);
        }
    }

    if let Some(range_header) = header_value(request.headers(), "Range") {
        match parse_single_range(&range_header, file_len) {
            Ok(Some((start, end))) => {
                let range_len = end - start + 1;
                if let Err(error) = file.seek(SeekFrom::Start(start as u64)) {
                    return headify(
                        status_mapped_error(&error, options, request.secure()),
                        is_head,
                    );
                }
                let ranged_reader = file.take(range_len as u64);
                let mut response = PlannedResponse {
                    status_code: 206,
                    content_length: Some(range_len),
                    body: Box::new(ranged_reader),
                    headers: Vec::new(),
                };
                add_common_success_headers(
                    &mut response.headers,
                    &selected_target,
                    &[],
                    Some(&etag),
                    options,
                    request.secure(),
                );
                add_header(
                    &mut response.headers,
                    "Content-Range",
                    &format!("bytes {}-{}/{}", start, end, file_len),
                );
                return headify(response, is_head);
            }
            Ok(None) => {}
            Err(_) => {
                let mut response = PlannedResponse {
                    status_code: 416,
                    body: boxed_bytes(b"Range Not Satisfiable".to_vec()),
                    headers: Vec::new(),
                    content_length: Some("Range Not Satisfiable".len()),
                };
                add_header(
                    &mut response.headers,
                    "Content-Range",
                    &format!("bytes */{}", file_len),
                );
                add_header(&mut response.headers, "Content-Type", "text/plain; charset=utf-8");
                add_header(&mut response.headers, "Cache-Control", "no-store");
                add_default_security_headers(&mut response.headers, options, request.secure());
                return headify(response, is_head);
            }
        }
    }

    let mut response = PlannedResponse {
        status_code: 200,
        content_length: Some(file_len),
        body: Box::new(file),
        headers: Vec::new(),
    };
    add_common_success_headers(
        &mut response.headers,
        &selected_target,
        &[],
        Some(&etag),
        options,
        request.secure(),
    );
    headify(response, is_head)
}

fn validate_request_limits(
    request: &Request,
    options: &ServeServerOptions,
) -> Result<(), RequestLimitsError> {
    let request_line =
        format!("{} {} HTTP/{}", request.method().as_str(), request.url(), request.http_version());
    if request_line.len() + 2 > options.max_request_line_bytes {
        return Err(RequestLimitsError::RequestLineTooLong);
    }

    if request.headers().len() > options.max_header_count {
        return Err(RequestLimitsError::TooManyHeaders);
    }

    let header_bytes: usize = request
        .headers()
        .iter()
        .map(|header| header.field.as_str().as_str().len() + 2 + header.value.as_str().len() + 2)
        .sum();
    if header_bytes > options.max_header_bytes {
        return Err(RequestLimitsError::HeadersTooLarge);
    }

    if request.body_length().unwrap_or(0) > options.max_request_body_bytes {
        return Err(RequestLimitsError::PayloadTooLarge);
    }

    Ok(())
}

fn add_common_success_headers(
    headers: &mut Vec<(String, String)>,
    path: &Path,
    file_bytes: &[u8],
    etag: Option<&str>,
    options: &ServeServerOptions,
    secure: bool,
) {
    let (content_type, content_encoding) = guess_response_headers(path, file_bytes);
    add_header(headers, "Content-Type", &content_type);
    if let Some(encoding) = content_encoding {
        add_header(headers, "Content-Encoding", encoding);
    }
    add_header(headers, "Accept-Ranges", "bytes");
    if let Some(etag_value) = etag {
        add_header(headers, "ETag", etag_value);
    }
    if let Some(cache_max_age) = options.cache_max_age {
        add_header(headers, "Cache-Control", &format!("public, max-age={}", cache_max_age));
    } else {
        add_header(headers, "Cache-Control", "public, max-age=0, must-revalidate");
    }
    add_default_security_headers(headers, options, secure);
}

fn add_default_security_headers(
    headers: &mut Vec<(String, String)>,
    options: &ServeServerOptions,
    secure: bool,
) {
    add_header(headers, "X-Content-Type-Options", "nosniff");
    add_header(headers, "Referrer-Policy", "no-referrer");
    if options.hardened {
        add_header(headers, "X-Frame-Options", "DENY");
        add_header(headers, "Cross-Origin-Opener-Policy", "same-origin");
        add_header(headers, "Cross-Origin-Resource-Policy", "same-origin");
        add_header(headers, "Permissions-Policy", "geolocation=(), microphone=(), camera=()");
        add_header(
            headers,
            "Content-Security-Policy",
            "default-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'",
        );
        if secure {
            add_header(headers, "Strict-Transport-Security", "max-age=31536000; includeSubDomains");
        }
    }
}

fn status_mapped_error(
    error: &std::io::Error,
    options: &ServeServerOptions,
    secure: bool,
) -> PlannedResponse {
    let status_code = match error.kind() {
        std::io::ErrorKind::NotFound => 404,
        std::io::ErrorKind::PermissionDenied => 403,
        std::io::ErrorKind::InvalidInput => 404,
        _ => 500,
    };

    if 500 == status_code {
        static_text_response(500, "Internal Server Error", options, secure, &[])
    } else if 403 == status_code {
        static_text_response(403, "Forbidden", options, secure, &[])
    } else {
        static_text_response(404, "Not Found", options, secure, &[])
    }
}

fn static_text_response(
    status_code: u16,
    body: &str,
    options: &ServeServerOptions,
    secure: bool,
    extra_headers: &[(&str, &str)],
) -> PlannedResponse {
    let mut headers = Vec::new();
    add_header(&mut headers, "Content-Type", "text/plain; charset=utf-8");
    add_header(&mut headers, "Cache-Control", "no-store");
    for (name, value) in extra_headers {
        add_header(&mut headers, name, value);
    }
    add_default_security_headers(&mut headers, options, secure);
    PlannedResponse {
        status_code,
        body: boxed_bytes(body.as_bytes().to_vec()),
        headers,
        content_length: Some(body.len()),
    }
}

fn validate_request_target(
    request_target: &str,
    index_file: &str,
) -> Result<PathBuf, RequestTargetValidationError> {
    if request_target.len() > MAX_REQUEST_TARGET_BYTES {
        return Err(RequestTargetValidationError::UriTooLong);
    }

    if request_target.contains('#') {
        return Err(RequestTargetValidationError::BadRequest);
    }

    let raw_path = request_target.split('?').next().unwrap_or("/");
    let raw_path = if raw_path.is_empty() { "/" } else { raw_path };
    let decoded_path =
        percent_decode_once(raw_path).map_err(|_| RequestTargetValidationError::BadRequest)?;

    if decoded_path.contains('\0') {
        return Err(RequestTargetValidationError::BadRequest);
    }

    let candidate_path = {
        let trimmed = decoded_path.trim_start_matches('/');
        if trimmed.is_empty() {
            index_file
        } else {
            trimmed
        }
    };

    let sanitized_path = path_security::sanitize_relative_path(candidate_path, "request path")
        .map_err(|_| RequestTargetValidationError::Forbidden)?;

    if path_has_blocked_private_components(&sanitized_path) {
        return Err(RequestTargetValidationError::Forbidden);
    }

    Ok(sanitized_path)
}

fn path_has_blocked_private_components(path: &Path) -> bool {
    let components: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str().map(|text| text.to_string()),
            _ => None,
        })
        .collect();

    if components.is_empty() {
        return false;
    }

    let last_index = components.len() - 1;
    for (index, component) in components.iter().enumerate() {
        let lower = component.to_ascii_lowercase();
        if lower.starts_with('.') || BLOCKED_PRIVATE_PATH_NAMES.contains(&lower.as_str()) {
            return true;
        }

        if index == last_index && private_leaf_name_is_blocked(&lower) {
            return true;
        }
    }

    false
}

fn private_leaf_name_is_blocked(file_name_lower: &str) -> bool {
    if file_name_lower.ends_with('~') {
        return true;
    }

    BLOCKED_PRIVATE_PATH_SUFFIXES.iter().any(|suffix| file_name_lower.ends_with(suffix))
}

fn percent_decode_once(path: &str) -> Result<String, RequestTargetValidationError> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if b'%' == bytes[index] {
            if index + 2 >= bytes.len() {
                return Err(RequestTargetValidationError::BadRequest);
            }

            let hi = decode_hex_nibble(bytes[index + 1])
                .ok_or(RequestTargetValidationError::BadRequest)?;
            let lo = decode_hex_nibble(bytes[index + 2])
                .ok_or(RequestTargetValidationError::BadRequest)?;
            decoded.push((hi << 4) | lo);
            index += 3;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(decoded).map_err(|_| RequestTargetValidationError::BadRequest)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn headify(mut response: PlannedResponse, is_head: bool) -> PlannedResponse {
    if is_head {
        if response.content_length.is_none() {
            response.content_length = Some(0);
        }
        response.body = boxed_bytes(Vec::new());
    }
    response
}

fn open_file_nofollow(path: &Path) -> std::io::Result<(fs::File, usize)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        let file = fs::OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW).open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Requested path is not a regular file",
            ));
        }

        let file_len = usize::try_from(metadata.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Requested file is too large")
        })?;
        Ok((file, file_len))
    }

    #[cfg(not(unix))]
    {
        let file = fs::File::open(path)?;
        let metadata = fs::metadata(path)?;
        if !metadata.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Requested path is not a regular file",
            ));
        }
        let file_len = usize::try_from(metadata.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Requested file is too large")
        })?;
        Ok((file, file_len))
    }
}

fn boxed_bytes(bytes: Vec<u8>) -> Box<dyn Read + Send> {
    Box::new(Cursor::new(bytes))
}

fn header_value(headers: &[Header], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.field.as_str().to_string().eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str().to_string())
}

fn add_header(headers: &mut Vec<(String, String)>, name: &str, value: &str) {
    if value.bytes().any(|byte| byte < 0x20 || byte == 0x7f) {
        return;
    }
    headers.push((name.to_string(), value.to_string()));
}

fn choose_precompressed_path(path: &Path, accept_encoding: Option<&str>) -> PathBuf {
    let extension =
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase());
    if extension.as_deref() == Some("br") || extension.as_deref() == Some("gz") {
        return path.to_path_buf();
    }

    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return path.to_path_buf();
    };
    let Some(header_value) = accept_encoding else {
        return path.to_path_buf();
    };

    if encoding_allowed(header_value, "br") {
        let br_path = path.with_file_name(format!("{}.br", file_name));
        if br_path.is_file() {
            return br_path;
        }
    }

    if encoding_allowed(header_value, "gzip") {
        let gz_path = path.with_file_name(format!("{}.gz", file_name));
        if gz_path.is_file() {
            return gz_path;
        }
    }

    path.to_path_buf()
}

fn encoding_allowed(header_value: &str, encoding: &str) -> bool {
    header_value.split(',').map(|item| item.trim().to_ascii_lowercase()).any(|item| {
        if item.starts_with('*') {
            return !item.contains("q=0");
        }
        if item.starts_with(encoding) {
            return !item.contains("q=0");
        }
        false
    })
}

fn parse_single_range(range_header: &str, total_len: usize) -> Result<Option<(usize, usize)>, ()> {
    let value = range_header.trim();
    if !value.starts_with("bytes=") {
        return Err(());
    }

    let range_value = &value[6..];
    if range_value.contains(',') {
        return Err(());
    }

    let parts: Vec<&str> = range_value.split('-').collect();
    if 2 != parts.len() || 0 == total_len {
        return Err(());
    }

    if parts[0].is_empty() {
        let suffix_len = parts[1].parse::<usize>().map_err(|_| ())?;
        if 0 == suffix_len {
            return Err(());
        }

        let start = total_len.saturating_sub(suffix_len);
        let end = total_len - 1;
        if start > end {
            return Err(());
        }
        return Ok(Some((start, end)));
    }

    let start = parts[0].parse::<usize>().map_err(|_| ())?;
    let end = if parts[1].is_empty() {
        total_len - 1
    } else {
        parts[1].parse::<usize>().map_err(|_| ())?
    };

    if start >= total_len || start > end {
        return Err(());
    }

    Ok(Some((start, end.min(total_len - 1))))
}

fn weak_etag(path: &Path, file_len: usize) -> String {
    let modified_secs = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("W/\"{:x}-{:x}\"", file_len, modified_secs)
}

fn if_none_match_matches(if_none_match: &str, etag: &str) -> bool {
    if "*" == if_none_match.trim() {
        return true;
    }
    if_none_match.split(',').map(|value| value.trim()).any(|candidate| candidate == etag)
}

fn guess_response_headers(path: &Path, file_bytes: &[u8]) -> (String, Option<&'static str>) {
    let (content_path, content_encoding) = split_content_path_and_encoding(path);
    let content_type = guess_content_type(&content_path, file_bytes);
    (content_type, content_encoding)
}

fn split_content_path_and_encoding(path: &Path) -> (PathBuf, Option<&'static str>) {
    let extension =
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase());

    match extension.as_deref() {
        Some("gz") => {
            if let Some(file_stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                let stem_path = path.with_file_name(file_stem);
                if stem_path.extension().is_some() {
                    return (stem_path, Some("gzip"));
                }
            } else {
                return (path.to_path_buf(), Some("gzip"));
            }
            (path.to_path_buf(), None)
        }
        Some("br") => {
            if let Some(file_stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                let stem_path = path.with_file_name(file_stem);
                if stem_path.extension().is_some() {
                    return (stem_path, Some("br"));
                }
            } else {
                return (path.to_path_buf(), Some("br"));
            }
            (path.to_path_buf(), None)
        }
        _ => (path.to_path_buf(), None),
    }
}

fn guess_content_type(path: &Path, file_bytes: &[u8]) -> String {
    let extension =
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase());

    if let Some(ext) = extension.as_deref() {
        if let Some(known_mime) = known_mime_for_extension(ext) {
            return known_mime.to_string();
        }
        // Unknown extensions are served as download-only bytes to avoid active-content sniffing.
        return "application/octet-stream".to_string();
    }

    let _ = file_bytes;
    // Extensionless files intentionally use octet-stream fallback.
    "application/octet-stream".to_string()
}

fn known_mime_for_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "html" | "htm" => Some("text/html; charset=utf-8"),
        "css" => Some("text/css; charset=utf-8"),
        "js" | "mjs" | "cjs" => Some("application/javascript; charset=utf-8"),
        "json" | "map" => Some("application/json; charset=utf-8"),
        "xml" => Some("application/xml; charset=utf-8"),
        "txt" | "text" | "md" | "csv" => Some("text/plain; charset=utf-8"),
        "wasm" => Some("application/wasm"),
        "pdf" => Some("application/pdf"),
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        "ico" => Some("image/x-icon"),
        "bmp" => Some("image/bmp"),
        "tif" | "tiff" => Some("image/tiff"),
        "woff" => Some("font/woff"),
        "woff2" => Some("font/woff2"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        "eot" => Some("application/vnd.ms-fontobject"),
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "mp4" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mov" => Some("video/quicktime"),
        "zip" => Some("application/zip"),
        "tar" => Some("application/x-tar"),
        "gz" | "tgz" => Some("application/gzip"),
        "7z" => Some("application/x-7z-compressed"),
        _ => None,
    }
}

fn log_access(
    enabled: bool,
    method: &Method,
    url: &str,
    status_code: u16,
    response_bytes: usize,
    elapsed_ms: u128,
    secure: bool,
) {
    if !enabled {
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    println!(
        "serve_access ts={} secure={} method={} path=\"{}\" status={} bytes={} elapsed_ms={}",
        timestamp, secure, method, url, status_code, response_bytes, elapsed_ms
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn planned_body_bytes(response: &mut PlannedResponse) -> Vec<u8> {
        let mut bytes = Vec::new();
        response.body.read_to_end(&mut bytes).expect("planned response body should be readable");
        bytes
    }

    fn test_options() -> ServeServerOptions {
        ServeServerOptions {
            index: "index.html".to_string(),
            hardened: false,
            cache_max_age: Some(300),
            access_log: false,
            tls_cert: None,
            tls_key: None,
            max_request_line_bytes: 8192,
            max_header_bytes: 16384,
            max_header_count: 100,
            max_request_body_bytes: 1048576,
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            max_connections: 128,
        }
    }

    #[test]
    fn parse_single_range_supports_standard_and_suffix_ranges() {
        assert_eq!(parse_single_range("bytes=0-9", 100), Ok(Some((0, 9))));
        assert_eq!(parse_single_range("bytes=10-", 100), Ok(Some((10, 99))));
        assert_eq!(parse_single_range("bytes=-10", 100), Ok(Some((90, 99))));
    }

    #[test]
    fn parse_single_range_rejects_invalid_ranges() {
        assert!(parse_single_range("bytes=50-10", 100).is_err());
        assert!(parse_single_range("items=0-10", 100).is_err());
        assert!(parse_single_range("bytes=100-200", 100).is_err());
    }

    #[test]
    fn if_none_match_detects_exact_match() {
        assert!(if_none_match_matches("W/\"a-b\"", "W/\"a-b\""));
        assert!(if_none_match_matches("W/\"x-y\", W/\"a-b\"", "W/\"a-b\""));
        assert!(!if_none_match_matches("W/\"x-y\"", "W/\"a-b\""));
    }

    #[test]
    fn guess_content_type_is_case_insensitive_for_known_extensions() {
        assert_eq!(guess_content_type(Path::new("INDEX.HTML"), b""), "text/html; charset=utf-8");
        assert_eq!(guess_content_type(Path::new("asset.SVG"), b""), "image/svg+xml");
        assert_eq!(guess_content_type(Path::new("font.WOFF2"), b""), "font/woff2");
    }

    #[test]
    fn guess_content_type_covers_v1_http_007_required_mappings() {
        assert_eq!(guess_content_type(Path::new("index.html"), b""), "text/html; charset=utf-8");
        assert_eq!(
            guess_content_type(Path::new("feed.xml"), b""),
            "application/xml; charset=utf-8"
        );
        assert_eq!(guess_content_type(Path::new("styles.css"), b""), "text/css; charset=utf-8");
        assert_eq!(
            guess_content_type(Path::new("app.js"), b""),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            guess_content_type(Path::new("data.json"), b""),
            "application/json; charset=utf-8"
        );
        assert_eq!(guess_content_type(Path::new("image.png"), b""), "image/png");
        assert_eq!(guess_content_type(Path::new("image.jpg"), b""), "image/jpeg");
        assert_eq!(guess_content_type(Path::new("vector.svg"), b""), "image/svg+xml");
        assert_eq!(guess_content_type(Path::new("scan.tiff"), b""), "image/tiff");
        assert_eq!(guess_content_type(Path::new("app.wasm"), b""), "application/wasm");
        assert_eq!(
            guess_content_type(Path::new("legacy.eot"), b""),
            "application/vnd.ms-fontobject"
        );
        assert_eq!(guess_content_type(Path::new("font.woff2"), b""), "font/woff2");
        assert_eq!(guess_content_type(Path::new("song.mp3"), b""), "audio/mpeg");
        assert_eq!(guess_content_type(Path::new("movie.mov"), b""), "video/quicktime");
        assert_eq!(guess_content_type(Path::new("manual.pdf"), b""), "application/pdf");
        assert_eq!(guess_content_type(Path::new("notes.txt"), b""), "text/plain; charset=utf-8");
        assert_eq!(guess_content_type(Path::new("archive.tar"), b""), "application/x-tar");
        assert_eq!(guess_content_type(Path::new("archive.7z"), b""), "application/x-7z-compressed");
        assert_eq!(guess_content_type(Path::new("archive.tgz"), b""), "application/gzip");
        assert_eq!(guess_content_type(Path::new("archive.gz"), b""), "application/gzip");
    }

    #[test]
    fn guess_content_type_uses_octet_stream_for_unknown_extension() {
        assert_eq!(
            guess_content_type(Path::new("payload.unknown"), b"not-a-known-format"),
            "application/octet-stream"
        );
    }

    #[test]
    fn guess_content_type_uses_octet_stream_for_extensionless_files() {
        assert_eq!(
            guess_content_type(Path::new("LICENSE"), b"plain text"),
            "application/octet-stream"
        );
    }

    #[test]
    fn guess_content_type_blocks_inferred_active_content_for_unknown_extension() {
        let html_bytes = b"<!DOCTYPE html><html><body>hello</body></html>";
        assert_eq!(
            guess_content_type(Path::new("payload.unknown"), html_bytes),
            "application/octet-stream"
        );
    }

    #[test]
    fn guess_content_type_uses_last_extension_for_double_extension_fallback() {
        assert_eq!(
            guess_content_type(Path::new("payload.js.unknown"), b"console.log('nope')"),
            "application/octet-stream"
        );
    }

    #[test]
    fn split_content_path_and_encoding_only_treats_multi_extension_files_as_precompressed() {
        let (content_path, encoding) = split_content_path_and_encoding(Path::new("index.html.gz"));
        assert_eq!(Path::new("index.html"), content_path.as_path());
        assert_eq!(Some("gzip"), encoding);

        let (content_path, encoding) = split_content_path_and_encoding(Path::new("archive.gz"));
        assert_eq!(Path::new("archive.gz"), content_path.as_path());
        assert_eq!(None, encoding);

        let (content_path, encoding) = split_content_path_and_encoding(Path::new("bundle.tar.br"));
        assert_eq!(Path::new("bundle.tar"), content_path.as_path());
        assert_eq!(Some("br"), encoding);
    }

    #[test]
    fn add_common_success_headers_sets_nosniff_for_file_responses() {
        let options = test_options();
        let mut headers = Vec::new();
        add_common_success_headers(
            &mut headers,
            Path::new("index.html"),
            b"<html></html>",
            Some("W/\"b-1\""),
            &options,
            false,
        );

        let nosniff = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("x-content-type-options"))
            .map(|(_, value)| value.as_str());
        assert_eq!(nosniff, Some("nosniff"));
    }

    #[test]
    fn add_common_success_headers_sets_conservative_cache_when_no_max_age() {
        let mut options = test_options();
        options.cache_max_age = None;
        let mut headers = Vec::new();
        add_common_success_headers(
            &mut headers,
            Path::new("index.html"),
            b"<html></html>",
            Some("W/\"b-1\""),
            &options,
            false,
        );

        let cache_control = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("cache-control"))
            .map(|(_, value)| value.as_str());
        assert_eq!(cache_control, Some("public, max-age=0, must-revalidate"));
    }

    #[test]
    fn static_text_response_includes_extra_headers_and_default_security_headers() {
        let mut options = test_options();
        options.cache_max_age = None;
        let mut response = static_text_response(
            405,
            "Method Not Allowed",
            &options,
            false,
            &[("Allow", "GET, HEAD")],
        );

        assert_eq!(405, response.status_code);
        assert_eq!("Method Not Allowed".as_bytes(), planned_body_bytes(&mut response).as_slice());

        let allow = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("allow"))
            .map(|(_, value)| value.as_str());
        let content_type = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.as_str());
        let cache_control = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("cache-control"))
            .map(|(_, value)| value.as_str());
        let nosniff = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("x-content-type-options"))
            .map(|(_, value)| value.as_str());
        let referrer_policy = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("referrer-policy"))
            .map(|(_, value)| value.as_str());

        assert_eq!(allow, Some("GET, HEAD"));
        assert_eq!(content_type, Some("text/plain; charset=utf-8"));
        assert_eq!(cache_control, Some("no-store"));
        assert_eq!(nosniff, Some("nosniff"));
        assert_eq!(referrer_policy, Some("no-referrer"));
    }

    #[test]
    fn status_mapped_error_uses_500_contract_for_unexpected_io_errors() {
        let mut options = test_options();
        options.cache_max_age = None;
        let error = std::io::Error::other("synthetic unexpected io error");
        let mut response = status_mapped_error(&error, &options, false);

        assert_eq!(500, response.status_code);
        assert_eq!(
            "Internal Server Error".as_bytes(),
            planned_body_bytes(&mut response).as_slice()
        );

        let content_type = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.as_str());
        let cache_control = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("cache-control"))
            .map(|(_, value)| value.as_str());
        let nosniff = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("x-content-type-options"))
            .map(|(_, value)| value.as_str());
        let referrer_policy = response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("referrer-policy"))
            .map(|(_, value)| value.as_str());

        assert_eq!(content_type, Some("text/plain; charset=utf-8"));
        assert_eq!(cache_control, Some("no-store"));
        assert_eq!(nosniff, Some("nosniff"));
        assert_eq!(referrer_policy, Some("no-referrer"));
    }

    #[test]
    fn validate_request_target_accepts_query_strings_without_affecting_path_resolution() {
        let path =
            validate_request_target("/index.html?download=%2e%2e%2fsecret.txt", "index.html")
                .expect("expected request target with query string to resolve");
        assert_eq!(PathBuf::from("index.html"), path);
    }

    #[test]
    fn validate_request_target_rejects_invalid_percent_encoding() {
        let err = validate_request_target("/bad%2", "index.html")
            .expect_err("expected invalid percent encoding to fail");
        assert!(matches!(err, RequestTargetValidationError::BadRequest));
    }

    #[test]
    fn validate_request_target_rejects_fragment_targets() {
        let err = validate_request_target("/index.html#fragment", "index.html")
            .expect_err("expected fragment in request target to fail");
        assert!(matches!(err, RequestTargetValidationError::BadRequest));
    }

    #[test]
    fn validate_request_target_rejects_decoded_null_bytes() {
        let err = validate_request_target("/%00secret.txt", "index.html")
            .expect_err("expected decoded null byte to fail");
        assert!(matches!(err, RequestTargetValidationError::BadRequest));
    }

    #[test]
    fn validate_request_target_rejects_oversized_targets() {
        let oversized_target = format!("/{}", "a".repeat(MAX_REQUEST_TARGET_BYTES + 1));
        let err = validate_request_target(&oversized_target, "index.html")
            .expect_err("expected oversized request target to fail");
        assert!(matches!(err, RequestTargetValidationError::UriTooLong));
    }

    #[test]
    fn validate_request_target_rejects_single_decoded_parent_traversal() {
        let err = validate_request_target("/%2e%2e/secret.txt", "index.html")
            .expect_err("expected decoded parent traversal to fail");
        assert!(matches!(err, RequestTargetValidationError::Forbidden));
    }

    #[test]
    fn validate_request_target_rejects_hidden_and_private_paths() {
        for path in [
            "/.env",
            "/.git/config",
            "/.DS_Store",
            "/notes.txt.bak",
            "/settings.json.swp",
            "/tmp/scratch~",
        ] {
            let err = validate_request_target(path, "index.html")
                .expect_err("expected private path request target to fail");
            assert!(matches!(err, RequestTargetValidationError::Forbidden), "path: {}", path);
        }
    }

    #[test]
    fn validate_request_target_keeps_non_private_paths_available() {
        let path = validate_request_target("/public/readme.txt", "index.html")
            .expect("expected non-private path to resolve");
        assert_eq!(PathBuf::from("public/readme.txt"), path);
    }

    #[test]
    fn validate_request_target_decodes_once_so_double_encoded_parent_is_not_treated_as_parent() {
        let path = validate_request_target("/%252e%252e/secret.txt", "index.html")
            .expect("expected double-encoded parent segment to remain a literal path segment");
        assert_eq!(PathBuf::from("%2e%2e/secret.txt"), path);
    }

    #[test]
    fn validate_server_options_rejects_zero_timeout_and_connection_limits() {
        let mut options = test_options();
        options.read_timeout = Duration::ZERO;
        let read_timeout_error = validate_server_options(&options)
            .expect_err("expected zero read timeout to be rejected");
        assert_eq!("serve read timeout must be greater than 0ms", read_timeout_error);

        options = test_options();
        options.write_timeout = Duration::ZERO;
        let write_timeout_error = validate_server_options(&options)
            .expect_err("expected zero write timeout to be rejected");
        assert_eq!("serve write timeout must be greater than 0ms", write_timeout_error);

        options = test_options();
        options.max_connections = 0;
        let max_connections_error = validate_server_options(&options)
            .expect_err("expected zero max connections to be rejected");
        assert_eq!("serve max connections must be greater than 0", max_connections_error);
    }
}
