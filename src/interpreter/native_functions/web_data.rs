//! Generic bounded web-data mechanisms. Product policy belongs in Kujo modules.
use crate::builtins::{json_to_kujo_value, kujo_value_to_json, validate_json_nesting_depth};
use crate::interpreter::capabilities::NativeCapability;
use crate::interpreter::{Interpreter, Value};
use html5ever::tokenizer::{
    states::RawKind, BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer,
    TokenizerOpts,
};
use serde_json::{json, Value as Json};
use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const BUILTINS: &[(&str, usize)] = &[
    ("http_get_file_response", 3),
    ("xml_file_select", 2),
    ("http_destination_check", 1),
    ("process_usage", 0),
    ("rate_limit_wait", 2),
    ("regular_file_digest", 2),
    ("jsonl_wrap_array", 5),
    ("html_tokens", 2),
    ("url_normalize", 3),
    ("url_components", 1),
    ("decode_text_lossy", 3),
    ("text_file_read", 2),
    ("json_file_read", 2),
    ("json_file_write", 3),
    ("jsonl_read_chunk", 4),
    ("jsonl_write", 4),
    ("jsonl_sort", 4),
    ("create_temp_dir", 2),
    ("publish_directory_noreplace", 2),
];
const FILE_MAX: u64 = 512 * 1024 * 1024;
fn text(v: &Value) -> Result<&str, String> {
    if let Value::Str(s) = v {
        Ok(s)
    } else {
        Err("expected string".into())
    }
}
fn size(v: &Value, max: u64) -> Result<u64, String> {
    match v {
        Value::Int(n) if *n > 0 && (*n as u64) <= max => Ok(*n as u64),
        _ => Err(format!("limit must be 1..{max}")),
    }
}
fn io(e: std::io::Error) -> String {
    e.to_string()
}
fn regular(path: &Path, max: u64) -> Result<File, String> {
    let before = fs::symlink_metadata(path).map_err(io)?;
    if !before.is_file() || before.len() > max {
        return Err("expected bounded regular non-symlink file".into());
    }
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = opts.open(path).map_err(io)?;
    let after = file.metadata().map_err(io)?;
    if !after.is_file() || after.len() > max {
        return Err("expected bounded regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.ino() != after.ino() || before.dev() != after.dev() {
            return Err("file identity changed".into());
        }
    }
    Ok(file)
}
fn decoded(json: Json) -> Result<Value, String> {
    validate_json_nesting_depth(&json, 64)?;
    Ok(json_to_kujo_value(json))
}
fn parse_line(line: &str) -> Result<Json, String> {
    let v = serde_json::from_str(line).map_err(|e| e.to_string())?;
    validate_json_nesting_depth(&v, 64)?;
    Ok(v)
}
fn line_read(reader: &mut impl BufRead, max: usize) -> Result<Option<String>, String> {
    let mut bytes = Vec::new();
    let n = reader.take(max as u64 + 1).read_until(b'\n', &mut bytes).map_err(io)?;
    if n == 0 {
        return Ok(None);
    }
    if n > max {
        return Err("JSONL record exceeds byte limit".into());
    }
    String::from_utf8(bytes).map(Some).map_err(|e| e.to_string())
}
struct BoundedWriter<W> {
    inner: W,
    remaining: u64,
}
impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if b.len() as u64 > self.remaining {
            return Err(std::io::Error::other("output byte limit exceeded"));
        }
        let n = self.inner.write(b)?;
        self.remaining -= n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

struct HtmlSink {
    events: RefCell<Vec<Json>>,
    max: usize,
    failed: Cell<bool>,
}
impl TokenSink for HtmlSink {
    type Handle = ();
    fn process_token(&self, token: Token, _line: u64) -> TokenSinkResult<()> {
        let mut events = self.events.borrow_mut();
        if self.failed.get() {
            return TokenSinkResult::Continue;
        }
        let mut raw = None;
        match token {
            Token::TagToken(tag) => {
                let name = tag.name.to_string();
                let start = tag.kind == TagKind::StartTag;
                let mut attrs = serde_json::Map::new();
                for a in tag.attrs {
                    attrs.insert(a.name.local.to_string(), Json::String(a.value.to_string()));
                }
                events.push(json!({"kind":if start {"start"} else {"end"},"name":name,"attrs":attrs,"self_closing":tag.self_closing}));
                if start && !tag.self_closing && (name == "script" || name == "style") {
                    raw = Some(RawKind::Rawtext)
                }
            }
            Token::CharacterTokens(s) => {
                if let Some(last) = events.last_mut().filter(|e| e["kind"] == "text") {
                    if let Json::String(value) = &mut last["text"] {
                        value.push_str(&s);
                    }
                } else {
                    events.push(json!({"kind":"text","text":s.to_string()}));
                }
            }
            Token::NullCharacterToken => events.push(json!({"kind":"text","text":"\u{fffd}"})),
            _ => {}
        }
        if events.len() > self.max {
            self.failed.set(true);
            events.clear();
        }
        raw.map(TokenSinkResult::RawData).unwrap_or(TokenSinkResult::Continue)
    }
}
fn html(input: &str, max: usize) -> Result<Value, String> {
    if input.len() > 8 * 1024 * 1024 {
        return Err("HTML input exceeds 8 MiB".into());
    }
    let sink = HtmlSink { events: RefCell::new(Vec::new()), max, failed: Cell::new(false) };
    let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
    let queue = BufferQueue::default();
    queue.push_back(input.into());
    let _ = tokenizer.feed(&queue);
    tokenizer.end();
    if tokenizer.sink.failed.get() {
        return Err("HTML event limit exceeded".into());
    }
    decoded(Json::Array(tokenizer.sink.events.into_inner()))
}
fn normalize(input: &str, base: &str, opts: Json) -> Result<Value, String> {
    if !opts.is_object() {
        return Err("URL options must be an object".into());
    }
    if opts.get("query_mode").is_some_and(|v| !v.is_string()) {
        return Err("query_mode must be a string".into());
    }
    if opts
        .get("deny_params")
        .is_some_and(|v| !v.as_array().is_some_and(|a| a.iter().all(Json::is_string)))
    {
        return Err("deny_params must be an array of strings".into());
    }
    if input.chars().any(|c| c < ' ' || c == '\u{7f}') {
        return Ok(Value::Str(Arc::new(String::new())));
    }
    let parsed = if base.is_empty() {
        reqwest::Url::parse(input.trim())
    } else {
        reqwest::Url::parse(base).and_then(|b| b.join(input.trim()))
    };
    let mut u = match parsed {
        Ok(u) => u,
        Err(_) => return Ok(Value::Str(Arc::new(String::new()))),
    };
    if !["http", "https"].contains(&u.scheme())
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Ok(Value::Str(Arc::new(String::new())));
    }
    u.set_fragment(None);
    // Collapse literal separators before decoding: encoded separators retain
    // their resource identity, matching quote(unquote(path)) consumers.
    let mut raw_path = String::new();
    let mut slash = false;
    for character in u.path().chars() {
        if character == '/' {
            if slash {
                continue;
            }
            slash = true;
        } else {
            slash = false;
        }
        raw_path.push(character);
    }
    let decoded_path = urlencoding::decode_binary(raw_path.as_bytes());
    let path = String::from_utf8_lossy(&decoded_path);
    let mut encoded = String::new();
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/%:@!$&'()*+,;=-._~".contains(&b) {
            encoded.push(b as char)
        } else {
            encoded.push_str(&format!("%{b:02X}"))
        }
    }
    u.set_path(&encoded);
    let denied = opts.get("deny_params").and_then(Json::as_array).cloned().unwrap_or_default();
    let mut pairs: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| {
            !denied.iter().any(|v| v.as_str().is_some_and(|x| x.eq_ignore_ascii_case(k)))
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    match opts.get("query_mode").and_then(Json::as_str).unwrap_or("preserve") {
        "drop" => pairs.clear(),
        "sort" => pairs.sort(),
        "preserve" => {}
        _ => return Err("invalid URL query_mode".into()),
    }
    u.set_query(None);
    if !pairs.is_empty() {
        let query = pairs
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}={}",
                    urlencoding::encode(key).replace("%20", "+"),
                    urlencoding::encode(value).replace("%20", "+")
                )
            })
            .collect::<Vec<_>>()
            .join("&");
        u.set_query(Some(&query));
    }
    Ok(Value::Str(Arc::new(u.to_string())))
}
fn sort_key(line: &str, fields: &[String]) -> Result<Vec<String>, String> {
    let v = parse_line(line)?;
    Ok(fields
        .iter()
        .map(|f| match v.get(f) {
            Some(Json::String(s)) => s.clone(),
            Some(x) => x.to_string(),
            None => String::new(),
        })
        .collect())
}
fn merge(paths: &[PathBuf], dest: &Path, fields: &[String]) -> Result<(), String> {
    let mut readers: Vec<_> = paths
        .iter()
        .map(|p| File::open(p).map(BufReader::new))
        .collect::<Result<_, _>>()
        .map_err(io)?;
    let mut heap = BinaryHeap::new();
    for (i, r) in readers.iter_mut().enumerate() {
        if let Some(line) = line_read(r, FILE_MAX as usize)? {
            heap.push(Reverse((sort_key(&line, fields)?, line, i)));
        }
    }
    let mut out = BufWriter::new(File::create(dest).map_err(io)?);
    while let Some(Reverse((_, line, i))) = heap.pop() {
        out.write_all(line.as_bytes()).map_err(io)?;
        if let Some(line) = line_read(&mut readers[i], FILE_MAX as usize)? {
            heap.push(Reverse((sort_key(&line, fields)?, line, i)));
        }
    }
    out.flush().map_err(io)
}
fn sort_file(source: &Path, dest: &Path, fields: Vec<String>, budget: usize) -> Result<(), String> {
    let temporary = tempfile::tempdir_in(dest.parent().unwrap_or(Path::new("."))).map_err(io)?;
    let mut reader =
        BufReader::new(LimitedReader { inner: regular(source, FILE_MAX)?, remaining: FILE_MAX });
    let mut paths = Vec::new();
    let mut eof = false;
    let mut largest_record = 1;
    while !eof {
        let mut rows = Vec::new();
        let mut bytes = 0;
        while bytes < budget {
            match line_read(&mut reader, FILE_MAX as usize)? {
                Some(mut line) => {
                    if !line.ends_with('\n') {
                        line.push('\n');
                    }
                    largest_record = largest_record.max(line.len());
                    bytes += line.len();
                    rows.push((sort_key(&line, &fields)?, line));
                }
                None => {
                    eof = true;
                    break;
                }
            }
        }
        if rows.is_empty() {
            break;
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        let path = temporary.path().join(format!("chunk-{}", paths.len()));
        let mut output = BufWriter::new(File::create(&path).map_err(io)?);
        for (_, line) in rows {
            output.write_all(line.as_bytes()).map_err(io)?
        }
        output.flush().map_err(io)?;
        paths.push(path);
    }
    let fan_in = (budget / largest_record).clamp(2, 32);
    let mut generation = 0;
    while paths.len() > fan_in {
        let mut next = Vec::new();
        for (i, group) in paths.chunks(fan_in).enumerate() {
            let target = temporary.path().join(format!("merge-{generation}-{i}"));
            merge(group, &target, &fields)?;
            next.push(target);
            for p in group {
                fs::remove_file(p).map_err(io)?
            }
        }
        paths = next;
        generation += 1;
    }
    let output = temporary.path().join("sorted");
    merge(&paths, &output, &fields)?;
    // Hard-link gives atomic no-replace publication for this regular-file result.
    fs::hard_link(&output, dest).map_err(io)?;
    Ok(())
}
fn publish_dir(source: &Path, dest: &Path) -> Result<(), String> {
    if !fs::symlink_metadata(source).map_err(io)?.is_dir() {
        return Err("source must be a non-symlink directory".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let src =
            std::ffi::CString::new(source.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        let dst = std::ffi::CString::new(dest.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        let result = unsafe { libc::renamex_np(src.as_ptr(), dst.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                src.as_ptr(),
                libc::AT_FDCWD,
                dst.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        return Err("atomic directory publication unsupported".into());
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        if result != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide_path = |path: &Path| -> Result<Vec<u16>, String> {
            let mut units: Vec<u16> = path.as_os_str().encode_wide().collect();
            if units.contains(&0) {
                return Err("path contains a NUL character".into());
            }
            units.push(0);
            Ok(units)
        };
        let src = wide_path(source)?;
        let dst = wide_path(dest)?;
        // SAFETY: both owned, NUL-terminated UTF-16 arrays remain alive for
        // this call. Flags zero prohibit replacing an existing destination.
        let result = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(src.as_ptr(), dst.as_ptr(), 0)
        };
        if result == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}
fn http_file(url: String, destination: String, options: Json) -> Result<Value, String> {
    let opts = match json_to_kujo_value(options.clone()) {
        Value::Dict(d) => d,
        _ => return Err("options must be dictionary".into()),
    };
    let (timeout, deny, pin, follow) =
        super::http::file_http_policy_options(&opts, "http_get_file_response")?;
    if follow {
        return Err(
            "http_get_file_response requires redirects=none; evaluate hops explicitly".into()
        );
    }
    let maximum = options["max_bytes"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= FILE_MAX)
        .ok_or("max_bytes must be 1..536870912")?;
    let result = crate::network_policy::run_blocking_http_task("HTTP file response", move || {
        let client = crate::network_policy::build_policy_http_client(
            &url,
            timeout,
            deny,
            pin,
            false,
            "HTTP file response",
        )?;
        let mut request = client.get(&url);
        if let Some(headers) = options.get("headers") {
            for (name, value) in headers.as_object().ok_or("headers must be dictionary")? {
                request =
                    request.header(name.as_str(), value.as_str().ok_or("header must be string")?);
            }
        }
        let mut response = request.send().map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let mut headers = serde_json::Map::new();
        for name in response.headers().keys() {
            let values = response
                .headers()
                .get_all(name)
                .iter()
                .map(|v| v.to_str().map(String::from).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            headers.insert(name.to_string(), Json::String(values.join(", ")));
        }
        if response.content_length().is_some_and(|v| v > maximum) {
            return Err("response exceeds max_bytes".into());
        }
        let path = Path::new(&destination);
        let mut temporary =
            tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new("."))).map_err(io)?;
        let copied =
            std::io::copy(&mut response.by_ref().take(maximum + 1), &mut temporary).map_err(io)?;
        if copied > maximum {
            return Err("response exceeds max_bytes".into());
        }
        temporary.persist_noclobber(path).map_err(|e| e.to_string())?;
        Ok(json!({"status":status,"headers":headers,"bytes":copied,"path":destination}))
    })?;
    decoded(result)
}
struct LimitedReader<R> {
    inner: R,
    remaining: u64,
}
impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let length = bytes.len().min(self.remaining.saturating_add(1) as usize);
        let n = self.inner.read(&mut bytes[..length])?;
        if n as u64 > self.remaining {
            return Err(std::io::Error::other("expanded XML exceeded byte limit"));
        }
        self.remaining -= n as u64;
        Ok(n)
    }
}
fn xml_file_select(path: &Path, options: Json) -> Result<Value, String> {
    use quick_xml::{events::Event, NsReader, XmlVersion};
    let maximum = options["max_bytes"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= FILE_MAX)
        .ok_or("invalid max_bytes")?;
    let max_matches = options["max_matches"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= 1_000_000)
        .ok_or("invalid max_matches")?;
    let element = options["element"].as_str().ok_or("element must be string")?;
    let file = regular(path, FILE_MAX)?;
    let input: Box<dyn Read> = match options["compression"].as_str().unwrap_or("none") {
        "none" => Box::new(file),
        "gzip" => Box::new(flate2::read::MultiGzDecoder::new(file)),
        _ => return Err("invalid compression".into()),
    };
    let mut reader =
        NsReader::from_reader(BufReader::new(LimitedReader { inner: input, remaining: maximum }));
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();
    let mut depth = 0usize;
    let mut root = String::new();
    let mut values = Vec::new();
    let mut selected = None;
    let mut content = String::new();
    let mut closed = false;
    let mut matched = 0u64;
    let mut text_bytes = 0usize;
    let mut declaration_seen = false;
    let mut content_seen = false;
    loop {
        let event = reader.read_event_into(&mut buffer).map_err(|e| e.to_string())?;
        let empty = matches!(&event, Event::Empty(_));
        if !matches!(&event, Event::Decl(_) | Event::Eof) {
            content_seen = true;
        }
        match event {
            Event::Decl(declaration) => {
                if declaration_seen || content_seen {
                    return Err("invalid XML declaration position".into());
                }
                declaration_seen = true;
                if declaration.version().map_err(|e| e.to_string())?.as_ref() != "1.0" {
                    return Err("unsupported XML version".into());
                }
                if let Some(encoding) = declaration.encoding() {
                    let encoding = encoding.map_err(|e| e.to_string())?;
                    if !["utf-8", "utf8"].contains(&encoding.as_ref().to_ascii_lowercase().as_str())
                    {
                        return Err("unsupported XML encoding".into());
                    }
                }
                if let Some(standalone) = declaration.standalone() {
                    if !["yes", "no"].contains(&standalone.map_err(|e| e.to_string())?.as_ref()) {
                        return Err("invalid XML standalone declaration".into());
                    }
                }
            }
            Event::DocType(_) => return Err("XML document types are prohibited".into()),
            Event::Start(tag) | Event::Empty(tag) => {
                let name = tag.local_name().as_ref().to_string();
                super::xml::namespace_value(reader.resolver().resolve_element(tag.name()).0)?;
                let mut expanded_names = std::collections::BTreeSet::new();
                for attribute in tag.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|e| e.to_string())?;
                    let key = attribute.key.as_ref();
                    if key == "xmlns" || key.starts_with("xmlns:") {
                        continue;
                    }
                    let (namespace, local) = reader.resolver().resolve_attribute(attribute.key);
                    let namespace = super::xml::namespace_value(namespace)?;
                    if !expanded_names.insert((namespace, local.as_ref().to_string())) {
                        return Err("duplicate XML attribute".into());
                    }
                    attribute
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|e| e.to_string())?;
                }
                if depth == 0 {
                    if closed || !root.is_empty() {
                        return Err("multiple XML roots".into());
                    }
                    root = name.clone();
                }
                if depth >= 64 {
                    return Err("XML depth limit exceeded".into());
                }
                depth += 1;
                if name == element {
                    matched += 1;
                    if matched <= max_matches {
                        selected = Some(depth);
                        content.clear();
                    }
                }
                if empty {
                    if selected == Some(depth) {
                        values.push(Json::String(content.clone()));
                        selected = None;
                    }
                    depth -= 1;
                    if depth == 0 {
                        closed = true;
                    }
                }
            }
            Event::End(_) => {
                if depth == 0 {
                    return Err("unexpected XML end".into());
                }
                if selected == Some(depth) {
                    values.push(Json::String(std::mem::take(&mut content)));
                    selected = None;
                }
                depth -= 1;
                if depth == 0 {
                    closed = true;
                }
            }
            Event::Text(text) => {
                let value = text.xml10_content();
                let value = quick_xml::escape::unescape(&value).map_err(|e| e.to_string())?;
                if depth == 0 && !value.trim().is_empty() {
                    return Err("text outside XML root".into());
                }
                if selected == Some(depth) {
                    text_bytes += value.len();
                    if text_bytes > 32 * 1024 * 1024 {
                        return Err("XML selected text exceeds 32 MiB".into());
                    }
                    content.push_str(&value);
                }
            }
            Event::CData(text) => {
                if depth == 0 {
                    return Err("CDATA outside XML root".into());
                }
                let value = text.xml10_content();
                if selected == Some(depth) {
                    text_bytes += value.len();
                    if text_bytes > 32 * 1024 * 1024 {
                        return Err("XML selected text exceeds 32 MiB".into());
                    }
                    content.push_str(&value);
                }
            }
            Event::GeneralRef(reference) => {
                if depth == 0 {
                    return Err("entity outside XML root".into());
                }
                let encoded = format!("&{};", reference.xml10_content());
                let value = quick_xml::escape::unescape(&encoded).map_err(|e| e.to_string())?;
                if selected == Some(depth) {
                    text_bytes += value.len();
                    if text_bytes > 32 * 1024 * 1024 {
                        return Err("XML selected text exceeds 32 MiB".into());
                    }
                    content.push_str(&value);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    if root.is_empty() || depth != 0 {
        return Err("incomplete XML document".into());
    }
    decoded(json!({"root":root,"values":values,"truncated":matched>max_matches}))
}

fn regular_digest(path: &Path, maximum: u64) -> Result<Value, String> {
    use sha2::{Digest, Sha256};
    let file = regular(path, maximum)?;
    let mut reader = LimitedReader { inner: file, remaining: maximum };
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let count = reader.read(&mut buffer).map_err(io)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes += count as u64;
    }
    decoded(json!({"bytes":bytes,"sha256":format!("{:x}",hasher.finalize())}))
}
fn jsonl_wrap(
    source: &Path,
    destination: &Path,
    field: &str,
    metadata: Json,
    maximum: u64,
) -> Result<Value, String> {
    let metadata = metadata.as_object().ok_or("metadata must be an object")?;
    if metadata.contains_key(field) {
        return Err("array field conflicts with metadata".into());
    }
    let mut reader =
        BufReader::new(LimitedReader { inner: regular(source, maximum)?, remaining: maximum });
    let mut temporary =
        tempfile::NamedTempFile::new_in(destination.parent().unwrap_or(Path::new(".")))
            .map_err(io)?;
    {
        let mut writer =
            BoundedWriter { inner: BufWriter::new(temporary.as_file_mut()), remaining: maximum };
        writer.write_all(b"{").map_err(io)?;
        for (key, value) in metadata {
            serde_json::to_writer(&mut writer, key).map_err(|e| e.to_string())?;
            writer.write_all(b":").map_err(io)?;
            serde_json::to_writer(&mut writer, value).map_err(|e| e.to_string())?;
            writer.write_all(b",").map_err(io)?;
        }
        serde_json::to_writer(&mut writer, field).map_err(|e| e.to_string())?;
        writer.write_all(b":[").map_err(io)?;
        let mut first = true;
        while let Some(line) = line_read(&mut reader, maximum as usize)? {
            if line.trim().is_empty() {
                continue;
            }
            let record = parse_line(&line)?;
            if !first {
                writer.write_all(b",").map_err(io)?
            }
            serde_json::to_writer(&mut writer, &record).map_err(|e| e.to_string())?;
            first = false;
        }
        writer.write_all(b"]}\n").map_err(io)?;
        writer.flush().map_err(io)?;
    }
    temporary.persist_noclobber(destination).map_err(|e| e.to_string())?;
    Ok(Value::Bool(true))
}

pub fn handle(interp: &mut Interpreter, name: &str, args: &[Value]) -> Option<Value> {
    let (_, arity) = BUILTINS.iter().find(|(n, _)| *n == name)?;
    if args.len() != *arity {
        return Some(Value::Error(format!("{name} expects {arity} arguments")));
    }
    let result = (|| -> Result<Value, String> {
        match name {
            "regular_file_digest" => {
                regular_digest(Path::new(text(&args[0])?), size(&args[1], FILE_MAX)?)
            }
            "jsonl_wrap_array" => {
                interp
                    .require_capability(NativeCapability::FilesystemRead, name)
                    .map_err(|e| format!("{e:?}"))?;
                jsonl_wrap(
                    Path::new(text(&args[0])?),
                    Path::new(text(&args[1])?),
                    text(&args[2])?,
                    kujo_value_to_json(&args[3])?,
                    size(&args[4], FILE_MAX)?,
                )
            }
            "http_get_file_response" => {
                interp
                    .require_capability(NativeCapability::FilesystemWrite, name)
                    .map_err(|e| format!("{e:?}"))?;
                http_file(
                    text(&args[0])?.to_string(),
                    text(&args[1])?.to_string(),
                    kujo_value_to_json(&args[2])?,
                )
            }
            "xml_file_select" => {
                xml_file_select(Path::new(text(&args[0])?), kujo_value_to_json(&args[1])?)
            }
            "http_destination_check" => {
                match crate::network_policy::build_policy_http_client(
                    text(&args[0])?,
                    std::time::Duration::from_secs(15),
                    true,
                    true,
                    false,
                    "HTTP destination check",
                ) {
                    Ok(_) => decoded(json!({"ok":true,"error":""})),
                    Err(e) => decoded(json!({"ok":false,"error":e})),
                }
            }
            "rate_limit_wait" => {
                let channel = match &args[0] {
                    Value::Channel(c) => c,
                    _ => return Err("expected rate-limit channel".into()),
                };
                let delay = match args[1] {
                    Value::Int(n) => n as f64,
                    Value::Float(n) => n,
                    _ => return Err("expected interval milliseconds".into()),
                };
                if !delay.is_finite() || !(0.0..=60000.0).contains(&delay) {
                    return Err("interval must be finite and 0..60000 ms".into());
                }
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                let start = START.get_or_init(std::time::Instant::now);
                let guard = channel.lock().map_err(|_| "rate limiter lock poisoned")?;
                let last = match guard.1.try_recv() {
                    Ok(Value::Float(v))
                        if v.is_finite()
                            && v >= 0.0
                            && v <= start.elapsed().as_secs_f64() * 1000.0 =>
                    {
                        v
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => -60000.0,
                    _ => return Err("invalid rate limiter channel state".into()),
                };
                let wait = last + delay - start.elapsed().as_secs_f64() * 1000.0;
                if wait > 0.0 {
                    std::thread::sleep(std::time::Duration::from_secs_f64(wait / 1000.0))
                }
                guard
                    .0
                    .try_send(Value::Float(start.elapsed().as_secs_f64() * 1000.0))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            "process_usage" => {
                #[cfg(unix)]
                {
                    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
                    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
                        return Err(std::io::Error::last_os_error().to_string());
                    }
                    let cpu = usage.ru_utime.tv_sec as f64
                        + usage.ru_stime.tv_sec as f64
                        + (usage.ru_utime.tv_usec + usage.ru_stime.tv_usec) as f64 / 1_000_000.0;
                    let rss =
                        usage.ru_maxrss as u64 * if cfg!(target_os = "macos") { 1 } else { 1024 };
                    decoded(json!({"cpu_seconds":cpu,"peak_rss_bytes":rss}))
                }
                #[cfg(not(unix))]
                {
                    decoded(json!({"cpu_seconds":null,"peak_rss_bytes":null}))
                }
            }
            "html_tokens" => html(text(&args[0])?, size(&args[1], 1_000_000)? as usize),
            "url_normalize" => {
                normalize(text(&args[0])?, text(&args[1])?, kujo_value_to_json(&args[2])?)
            }
            "url_components" => {
                let u = reqwest::Url::parse(text(&args[0])?).map_err(|e| e.to_string())?;
                decoded(
                    json!({"scheme":u.scheme(),"host":u.host_str(),"port":u.port_or_known_default(),"origin":u.origin().ascii_serialization(),"path":u.path(),"query":u.query().unwrap_or("")}),
                )
            }
            "decode_text_lossy" => {
                let raw = match &args[0] {
                    Value::Bytes(b) => b.as_slice(),
                    _ => return Err("expected bytes".into()),
                };
                let max = size(&args[2], FILE_MAX)? as usize;
                if raw.len() > FILE_MAX as usize {
                    return Err("encoded input exceeds limit".into());
                }
                let label = text(&args[1])?;
                let enc = encoding_rs::Encoding::for_label(label.as_bytes())
                    .unwrap_or(encoding_rs::UTF_8);
                let mut value = String::new();
                if ["latin-1", "latin1", "iso-8859-1", "iso8859-1"]
                    .contains(&label.to_ascii_lowercase().as_str())
                {
                    for byte in raw {
                        let character = char::from(*byte);
                        if value.len() + character.len_utf8() > max {
                            return Err("decoded text exceeds limit".into());
                        }
                        value.push(character);
                    }
                } else {
                    let mut decoder = enc.new_decoder_without_bom_handling();
                    let mut offset = 0;
                    let mut buffer = [0u8; 4096];
                    loop {
                        let (status, read, written, _) =
                            decoder.decode_to_utf8(&raw[offset..], &mut buffer, true);
                        if value.len() + written > max {
                            return Err("decoded text exceeds limit".into());
                        }
                        value.push_str(
                            std::str::from_utf8(&buffer[..written]).map_err(|e| e.to_string())?,
                        );
                        offset += read;
                        if status == encoding_rs::CoderResult::InputEmpty {
                            break;
                        }
                    }
                }
                Ok(Value::Str(Arc::new(value)))
            }
            "text_file_read" => {
                let max = size(&args[1], FILE_MAX)?;
                let file = regular(Path::new(text(&args[0])?), max)?;
                let mut reader = LimitedReader { inner: file, remaining: max };
                let mut output = String::new();
                reader.read_to_string(&mut output).map_err(io)?;
                Ok(Value::Str(Arc::new(output)))
            }
            "json_file_read" => {
                let max = size(&args[1], FILE_MAX)?;
                let file = regular(Path::new(text(&args[0])?), max)?;
                let json = serde_json::from_reader(BufReader::new(LimitedReader {
                    inner: file,
                    remaining: max,
                }))
                .map_err(|e| e.to_string())?;
                decoded(json)
            }
            "json_file_write" => {
                let path = Path::new(text(&args[0])?);
                let max = size(&args[2], FILE_MAX)?;
                let value = kujo_value_to_json(&args[1])?;
                let mut tmp =
                    tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))
                        .map_err(io)?;
                {
                    let mut w =
                        BoundedWriter { inner: BufWriter::new(tmp.as_file_mut()), remaining: max };
                    serde_json::to_writer_pretty(&mut w, &value).map_err(|e| e.to_string())?;
                    w.write_all(b"\n").map_err(io)?;
                    w.flush().map_err(io)?;
                }
                tmp.persist_noclobber(path).map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            "jsonl_read_chunk" => {
                let path = Path::new(text(&args[0])?);
                let offset = match args[1] {
                    Value::Int(n) if n >= 0 => n as u64,
                    _ => return Err("invalid offset".into()),
                };
                let rows = size(&args[2], 100_000)?;
                let max = size(&args[3], FILE_MAX)?;
                let mut file = regular(path, FILE_MAX)?;
                let total = file.metadata().map_err(io)?.len();
                if offset > total {
                    return Err("offset beyond EOF".into());
                }
                file.seek(SeekFrom::Start(offset)).map_err(io)?;
                let mut reader = BufReader::new(file);
                let mut values = Vec::new();
                let mut used = 0;
                while (values.len() as u64) < rows && used < max {
                    match line_read(&mut reader, max as usize)? {
                        Some(line) => {
                            if used > 0 && used + line.len() as u64 > max {
                                break;
                            }
                            used += line.len() as u64;
                            if !line.trim().is_empty() {
                                values.push(parse_line(&line)?)
                            }
                        }
                        None => break,
                    }
                }
                decoded(json!({"rows":values,"offset":offset+used,"eof":offset+used>=total}))
            }
            "jsonl_write" => {
                let path = Path::new(text(&args[0])?);
                let rows = match &args[1] {
                    Value::Array(a) => a,
                    _ => return Err("records must be an array".into()),
                };
                let append = match args[2] {
                    Value::Bool(b) => b,
                    _ => return Err("append must be boolean".into()),
                };
                let max = size(&args[3], FILE_MAX)?;
                let mut opts = OpenOptions::new();
                opts.write(true);
                if append {
                    opts.append(true).create(true);
                } else {
                    opts.create_new(true);
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    opts.mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
                }
                if path.symlink_metadata().is_ok_and(|m| !m.is_file()) {
                    return Err("expected regular JSONL destination".into());
                }
                let file = opts.open(path).map_err(io)?;
                let metadata = file.metadata().map_err(io)?;
                if !metadata.is_file() {
                    return Err("JSONL destination must be a regular file".into());
                }
                let len = metadata.len();
                if len > max {
                    return Err("output byte limit exceeded".into());
                }
                let mut w = BoundedWriter { inner: BufWriter::new(file), remaining: max - len };
                for row in rows.iter() {
                    serde_json::to_writer(&mut w, &kujo_value_to_json(row)?)
                        .map_err(|e| e.to_string())?;
                    w.write_all(b"\n").map_err(io)?
                }
                w.flush().map_err(io)?;
                Ok(Value::Bool(true))
            }
            "jsonl_sort" => {
                interp
                    .require_capability(NativeCapability::FilesystemRead, name)
                    .map_err(|e| format!("{e:?}"))?;
                let fields = kujo_value_to_json(&args[2])?;
                let fields = fields
                    .as_array()
                    .ok_or("key_fields must be an array")?
                    .iter()
                    .map(|v| v.as_str().map(String::from).ok_or("key must be string".into()))
                    .collect::<Result<Vec<String>, String>>()?;
                sort_file(
                    Path::new(text(&args[0])?),
                    Path::new(text(&args[1])?),
                    fields,
                    size(&args[3], FILE_MAX)? as usize,
                )?;
                Ok(Value::Bool(true))
            }
            "create_temp_dir" => {
                let prefix = text(&args[1])?;
                if prefix.contains('/') || prefix.contains('\\') || prefix.len() > 100 {
                    return Err("invalid temporary directory prefix".into());
                }
                let tmp = tempfile::Builder::new()
                    .prefix(prefix)
                    .tempdir_in(text(&args[0])?)
                    .map_err(io)?;
                Ok(Value::Str(Arc::new(tmp.keep().to_string_lossy().into_owned())))
            }
            "publish_directory_noreplace" => {
                interp
                    .require_capability(NativeCapability::FilesystemDelete, name)
                    .map_err(|e| format!("{e:?}"))?;
                publish_dir(Path::new(text(&args[0])?), Path::new(text(&args[1])?))?;
                Ok(Value::Bool(true))
            }
            _ => unreachable!(),
        }
    })();
    Some(result.unwrap_or_else(Value::Error))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn s(value: &str) -> Value {
        Value::Str(Arc::new(value.into()))
    }
    fn call(name: &str, args: Vec<Value>) -> Result<Json, String> {
        match handle(&mut Interpreter::new(), name, &args).unwrap() {
            Value::Error(e) => Err(e),
            v => kujo_value_to_json(&v),
        }
    }
    #[test]
    fn rate_limit_rejects_corrupted_or_future_channel_state() {
        for timestamp in [f64::NAN, f64::INFINITY, -1.0, f64::MAX] {
            let mut interpreter = Interpreter::new();
            let state = interpreter.call_native_function_impl("channel", &[]);
            if let Value::Channel(channel) = &state {
                channel.lock().unwrap().0.try_send(Value::Float(timestamp)).unwrap();
            } else {
                panic!("channel constructor failed");
            }
            let result =
                handle(&mut interpreter, "rate_limit_wait", &[state, Value::Int(0)]).unwrap();
            assert!(
                matches!(result, Value::Error(message) if message.contains("invalid rate limiter channel state"))
            );
        }
    }

    #[test]
    fn html_events_decode_entities_preserve_raw_script_and_bound_events() {
        let value = kujo_value_to_json(
            &html("<title>A &amp; B</title><script>{\"x\":\"<b>\"}</script>", 100).unwrap(),
        )
        .unwrap();
        assert_eq!(value[1]["text"], "A & B");
        assert_eq!(value[4]["text"], "{\"x\":\"<b>\"}");
        assert!(html("<p>x</p>", 2).is_err());
    }
    #[test]
    fn url_policy_normalizes_and_rejects_unsafe_input() {
        assert_eq!(
            call(
                "url_normalize",
                vec![
                    s("HTTPS://BÜCHER.example:443//a?b=2&utm=x&a=1#fragment"),
                    s(""),
                    json_to_kujo_value(json!({"query_mode":"sort","deny_params":["utm"]}))
                ]
            )
            .unwrap(),
            "https://xn--bcher-kva.example/a?a=1&b=2"
        );
        for input in [
            "http://user:secret@example.com/",
            "https://exam\nple.com/",
            "http://example.com:99999/",
            "ftp://example.com/",
        ] {
            assert_eq!(
                call("url_normalize", vec![s(input), s(""), json_to_kujo_value(json!({}))])
                    .unwrap(),
                ""
            );
        }
    }
    #[test]
    fn json_files_cross_eight_mib_and_do_not_replace_or_follow_links() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        let name = s(path.to_str().unwrap());
        let value = json_to_kujo_value(json!({"text":"x".repeat(9*1024*1024)}));
        assert!(call(
            "json_file_write",
            vec![name.clone(), value.clone(), Value::Int(16 * 1024 * 1024)]
        )
        .is_ok());
        assert!(call("json_file_write", vec![name.clone(), value, Value::Int(16 * 1024 * 1024)])
            .is_err());
        assert_eq!(
            call("json_file_read", vec![name.clone(), Value::Int(16 * 1024 * 1024)]).unwrap()
                ["text"]
                .as_str()
                .unwrap()
                .len(),
            9 * 1024 * 1024
        );
        assert!(call("json_file_read", vec![name, Value::Int(1024)]).is_err());
        #[cfg(unix)]
        {
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(regular(&link, FILE_MAX).is_err());
        }
    }
    #[test]
    fn external_sort_multiple_generations_preserves_records_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::write(
            &input,
            (0..1100).rev().map(|i| format!("{{\"key\":\"{i:04}\"}}\n")).collect::<String>(),
        )
        .unwrap();
        sort_file(&input, &output, vec!["key".into()], 1).unwrap();
        let result = fs::read_to_string(&output).unwrap();
        assert_eq!(result.lines().count(), 1100);
        assert!(result.starts_with("{\"key\":\"0000\"}"));
        assert!(result.ends_with("{\"key\":\"1099\"}\n"));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
        assert!(sort_file(&input, &output, vec!["key".into()], 1).is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }
    #[test]
    fn publication_never_replaces_existing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(source.join("evidence"), "retained").unwrap();
        assert!(publish_dir(&source, &target).is_err());
        assert!(source.join("evidence").exists());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
        fs::remove_dir(&target).unwrap();
        publish_dir(&source, &target).unwrap();
        assert!(!source.exists());
        assert_eq!(fs::read_to_string(target.join("evidence")).unwrap(), "retained");
    }
    #[test]
    fn xml_projection_supports_large_gzip_and_rejects_expansion_and_doctype() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("xml.gz");
        let source=format!("<urlset><!--{}--><url><loc>https://example.com/?a=1&amp;b=2</loc></url><empty/></urlset>","x".repeat(9*1024*1024));
        let mut gzip = flate2::write::GzEncoder::new(
            File::create(&path).unwrap(),
            flate2::Compression::default(),
        );
        gzip.write_all(source.as_bytes()).unwrap();
        gzip.finish().unwrap();
        let options = json!({"compression":"gzip","max_bytes":16*1024*1024,"element":"loc","max_matches":100});
        let value = kujo_value_to_json(&xml_file_select(&path, options.clone()).unwrap()).unwrap();
        assert_eq!(value["root"], "urlset");
        assert_eq!(value["values"], json!(["https://example.com/?a=1&b=2"]));
        let mut small = options;
        small["max_bytes"] = json!(1024);
        assert!(xml_file_select(&path, small).is_err());
        fs::write(&path, "<!DOCTYPE a [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><a>&x;</a>")
            .unwrap();
        assert!(xml_file_select(
            &path,
            json!({"compression":"none","max_bytes":1024,"element":"a","max_matches":10})
        )
        .is_err());
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn xml_projection_rejects_malformed_structure_namespaces_and_declarations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("xml");
        for source in [
            "<a><b></a>",
            "<a/><b/>",
            "<x:a/>",
            "<a x:key='v'/>",
            "<a/><![CDATA[tail]]>",
            "<?xml version='1.0'?><?xml version='1.0'?><a/>",
            "<a><?xml version='1.0'?></a>",
            "<a x='1' x='2'/>",
            "<a>&missing;</a>",
        ] {
            fs::write(&path, source).unwrap();
            assert!(
                xml_file_select(
                    &path,
                    json!({"compression":"none","max_bytes":4096,"element":"a","max_matches":10})
                )
                .is_err(),
                "accepted {source}"
            );
        }
    }
    #[test]
    fn streaming_http_preserves_status_and_cleans_failed_body() {
        use std::net::TcpListener;
        for (response,success) in [("HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",true),("HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\nshort",false)] {
            let listener=TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();
            let server=std::thread::spawn(move||{let(mut socket,_)=listener.accept().unwrap();socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();let mut request=[0u8;4096];socket.read(&mut request).unwrap();socket.write_all(response.as_bytes()).unwrap();});
            let dir=tempfile::tempdir().unwrap();let path=dir.path().join("body");
            let result=http_file(format!("http://{address}/"),path.to_str().unwrap().into(),json!({"max_bytes":4096,"timeout_ms":5000,"redirects":"none","pin_dns":true,"destination_policy":"default"}));
            server.join().unwrap();
            if success {let value=kujo_value_to_json(&result.unwrap()).unwrap();assert_eq!(value["status"],302);assert_eq!(value["headers"]["location"],"/next");assert!(path.exists());}
            else {assert!(result.is_err());assert_eq!(fs::read_dir(dir.path()).unwrap().count(),0);}
        }
    }
}

#[cfg(test)]
mod url_compatibility_tests {
    use super::*;
    #[test]
    fn quoted_paths_and_queries_preserve_legacy_resource_identity() {
        for (input, expected) in [
            ("https://example.com//a%2F%2Fb?q=~*%20x", "https://example.com/a//b?q=~%2A+x"),
            ("https://example.com/%FF", "https://example.com/%EF%BF%BD"),
            ("https://example.com/?q=&a=one+two", "https://example.com/?q=&a=one+two"),
        ] {
            assert_eq!(
                kujo_value_to_json(&normalize(input, "", json!({})).unwrap()).unwrap(),
                expected
            );
        }
    }
}
