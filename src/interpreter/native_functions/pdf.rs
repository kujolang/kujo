//! Bounded, in-process HTML-to-PDF rendering for business documents.
//!
//! This surface deliberately implements a strict HTML/CSS profile. It never
//! performs network I/O or resolves filesystem paths from document input.

use crate::interpreter::{DictMap, Value};
use html5ever::tokenizer::{
    BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};
use printpdf::{Base64OrRaw, GeneratePdfOptions, PdfDocument, PdfSaveOptions};
use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const API_VERSION: &str = "kujo.pdf.html/v1";
const RENDERER_VERSION: &str = "printpdf/0.12.8-kujo-profile-v2";
const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_HTML_TOKENS: usize = 5_000;
const MAX_HTML_DEPTH: usize = 128;
const MAX_ASSET_COUNT: usize = 24;
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_FONT_BYTES: usize = 4 * 1024 * 1024;
const MAX_ASSET_BYTES: usize = 16 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_PAGES: usize = 256;
const MAX_RENDER_SECONDS: u64 = 30;
const MAX_CONCURRENT_RENDERS: usize = 4;

static ACTIVE_RENDERS: AtomicUsize = AtomicUsize::new(0);

fn reserve_render_slot(counter: &AtomicUsize, limit: usize) -> Result<(), String> {
    counter
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            (current < limit).then_some(current + 1)
        })
        .map(|_| ())
        .map_err(|_| "pdf renderer concurrency limit reached".to_string())
}

#[derive(Clone)]
struct RenderOptions {
    page_width_mm: f32,
    page_height_mm: f32,
    margin_top_mm: f32,
    margin_right_mm: f32,
    margin_bottom_mm: f32,
    margin_left_mm: f32,
    show_page_numbers: bool,
    header_text: Option<String>,
    footer_text: Option<String>,
    title: String,
    max_output_bytes: usize,
    timeout: Duration,
}

struct RenderedPdf {
    bytes: Vec<u8>,
    pages: usize,
    width_mm: f32,
    height_mm: f32,
    warning_count: usize,
    warning_codes: Vec<String>,
}

struct RenderPermit;

impl RenderPermit {
    fn acquire() -> Result<Self, String> {
        reserve_render_slot(&ACTIVE_RENDERS, MAX_CONCURRENT_RENDERS).map(|_| Self)
    }
}

fn receive_before_deadline<T>(receiver: mpsc::Receiver<T>, wait: Duration) -> Result<T, String> {
    receiver.recv_timeout(wait).map_err(|_| "pdf render deadline exceeded".to_string())
}

impl Drop for RenderPermit {
    fn drop(&mut self) {
        ACTIVE_RENDERS.fetch_sub(1, Ordering::AcqRel);
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn canonicalize_pdf_identifier(bytes: &[u8], source: &[u8]) -> Result<Vec<u8>, String> {
    use lopdf::Object::{Array, String as PdfString};
    use lopdf::StringFormat::Literal;

    let mut document = lopdf::Document::load_mem(bytes)
        .map_err(|_| "pdf renderer produced an unreadable document".to_string())?;
    let identifier = sha256_hex(source).into_bytes();
    document.trailer.set(
        "ID",
        Array(vec![PdfString(identifier.clone(), Literal), PdfString(identifier, Literal)]),
    );
    let mut canonical = Vec::with_capacity(bytes.len());
    document
        .save_to(&mut canonical)
        .map_err(|_| "pdf renderer could not finalize document".to_string())?;
    Ok(canonical)
}

fn number(value: Option<&Value>, name: &str, default: f32) -> Result<f32, String> {
    let result = match value {
        None => default,
        Some(Value::Int(value)) => *value as f32,
        Some(Value::Float(value)) => *value as f32,
        _ => return Err(format!("pdf option '{name}' must be a number")),
    };
    if !result.is_finite() {
        return Err(format!("pdf option '{name}' must be finite"));
    }
    Ok(result)
}

fn bounded_text(
    value: Option<&Value>,
    name: &str,
    maximum: usize,
) -> Result<Option<String>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Str(value)) if value.len() <= maximum => Ok(Some(value.as_ref().clone())),
        Some(Value::Str(_)) => Err(format!("pdf option '{name}' exceeds {maximum} bytes")),
        _ => Err(format!("pdf option '{name}' must be a string")),
    }
}

fn dictionary(value: &Value, description: &str) -> Result<DictMap, String> {
    match value {
        Value::Dict(values) => Ok(values.as_ref().clone()),
        Value::FixedDict { keys, values } => Ok(keys
            .iter()
            .zip(values.iter())
            .map(|(key, value)| (Arc::<str>::from(key.as_ref()), value.clone()))
            .collect()),
        _ => Err(format!("{description} must be a dictionary")),
    }
}

fn parse_options(value: &Value) -> Result<RenderOptions, String> {
    let options = dictionary(value, "pdf options")?;
    let allowed: HashSet<&str> = [
        "page_size",
        "orientation",
        "margin_mm",
        "margin_top_mm",
        "margin_right_mm",
        "margin_bottom_mm",
        "margin_left_mm",
        "show_page_numbers",
        "header_text",
        "footer_text",
        "title",
        "max_output_bytes",
        "timeout_ms",
    ]
    .into_iter()
    .collect();
    for key in options.keys() {
        if !allowed.contains(key.as_ref()) {
            return Err(format!("unknown pdf option '{}'", key));
        }
    }

    let page_size = match options.get("page_size") {
        None => "A4",
        Some(Value::Str(value)) if value.as_ref() == "A4" || value.as_ref() == "Letter" => value,
        _ => return Err("pdf option 'page_size' must be 'A4' or 'Letter'".to_string()),
    };
    let orientation = match options.get("orientation") {
        None => "portrait",
        Some(Value::Str(value))
            if value.as_ref() == "portrait" || value.as_ref() == "landscape" =>
        {
            value
        }
        _ => return Err("pdf option 'orientation' must be 'portrait' or 'landscape'".to_string()),
    };
    let (mut page_width_mm, mut page_height_mm) =
        if page_size == "Letter" { (215.9, 279.4) } else { (210.0, 297.0) };
    if orientation == "landscape" {
        std::mem::swap(&mut page_width_mm, &mut page_height_mm);
    }

    let all_margin = number(options.get("margin_mm"), "margin_mm", 12.0)?;
    let margin_top_mm = number(options.get("margin_top_mm"), "margin_top_mm", all_margin)?;
    let margin_right_mm = number(options.get("margin_right_mm"), "margin_right_mm", all_margin)?;
    let margin_bottom_mm = number(options.get("margin_bottom_mm"), "margin_bottom_mm", all_margin)?;
    let margin_left_mm = number(options.get("margin_left_mm"), "margin_left_mm", all_margin)?;
    for (name, value) in [
        ("margin_top_mm", margin_top_mm),
        ("margin_right_mm", margin_right_mm),
        ("margin_bottom_mm", margin_bottom_mm),
        ("margin_left_mm", margin_left_mm),
    ] {
        if !(0.0..=50.0).contains(&value) {
            return Err(format!("pdf option '{name}' must be between 0 and 50"));
        }
    }

    let show_page_numbers = match options.get("show_page_numbers") {
        None => false,
        Some(Value::Bool(value)) => *value,
        _ => return Err("pdf option 'show_page_numbers' must be a boolean".to_string()),
    };
    let max_output_bytes = match options.get("max_output_bytes") {
        None => MAX_OUTPUT_BYTES,
        Some(Value::Int(value)) if *value > 0 && (*value as usize) <= MAX_OUTPUT_BYTES => {
            *value as usize
        }
        _ => {
            return Err(format!(
                "pdf option 'max_output_bytes' must be between 1 and {MAX_OUTPUT_BYTES}"
            ))
        }
    };
    let timeout_ms = match options.get("timeout_ms") {
        None => 10_000_u64,
        Some(Value::Int(value))
            if *value >= 100 && *value <= (MAX_RENDER_SECONDS * 1000) as i64 =>
        {
            *value as u64
        }
        _ => {
            return Err(format!(
                "pdf option 'timeout_ms' must be between 100 and {}",
                MAX_RENDER_SECONDS * 1000
            ))
        }
    };

    Ok(RenderOptions {
        page_width_mm,
        page_height_mm,
        margin_top_mm,
        margin_right_mm,
        margin_bottom_mm,
        margin_left_mm,
        show_page_numbers,
        header_text: bounded_text(options.get("header_text"), "header_text", 256)?,
        footer_text: bounded_text(options.get("footer_text"), "footer_text", 256)?,
        title: bounded_text(options.get("title"), "title", 256)?
            .unwrap_or_else(|| "Kujo business document".to_string()),
        max_output_bytes,
        timeout: Duration::from_millis(timeout_ms),
    })
}

fn valid_asset_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
        && !name.contains("..")
}

fn validate_image(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!("pdf image assets must be 1..{MAX_IMAGE_BYTES} bytes"));
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "pdf image asset has an unsupported format".to_string())?;
    let (width, height) =
        reader.into_dimensions().map_err(|_| "pdf image asset is malformed".to_string())?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("pdf image dimensions must be between 1 and 4096 pixels".to_string());
    }
    if u64::from(width) * u64::from(height) > 16_777_216 {
        return Err("pdf image decompressed pixel limit exceeded".to_string());
    }
    Ok(())
}

fn validate_font(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 12 || bytes.len() > MAX_FONT_BYTES {
        return Err(format!("pdf font assets must be 12..{MAX_FONT_BYTES} bytes"));
    }
    let signature = &bytes[..4];
    if signature != [0, 1, 0, 0] && signature != b"OTTO" {
        return Err("pdf font asset must be a single OpenType or TrueType font".to_string());
    }
    let mut warnings = Vec::new();
    if printpdf::ParsedFont::from_bytes(bytes, 0, &mut warnings).is_none() {
        return Err("pdf font asset is malformed or unsupported".to_string());
    }
    Ok(())
}

type PdfAssets = (BTreeMap<String, Base64OrRaw>, BTreeMap<String, Base64OrRaw>);

fn parse_assets(value: &Value) -> Result<PdfAssets, String> {
    let assets = dictionary(value, "pdf assets")?;
    for key in assets.keys() {
        if key.as_ref() != "images" && key.as_ref() != "fonts" {
            return Err(format!("unknown pdf asset group '{}'", key));
        }
    }

    let mut images = BTreeMap::new();
    let mut fonts = BTreeMap::new();
    let mut total_count = 0_usize;
    let mut total_bytes = 0_usize;

    for (group_name, destination, validator) in [
        ("images", &mut images, validate_image as fn(&[u8]) -> Result<(), String>),
        ("fonts", &mut fonts, validate_font as fn(&[u8]) -> Result<(), String>),
    ] {
        let Some(group) = assets.get(group_name) else {
            continue;
        };
        let group = dictionary(group, &format!("pdf asset group '{group_name}'"))?;
        for (name, value) in group.iter() {
            if !valid_asset_name(name) {
                return Err("pdf asset names must be bounded alphanumeric identifiers".to_string());
            }
            let Value::Bytes(bytes) = value else {
                return Err(format!("pdf asset '{name}' must contain bytes"));
            };
            validator(bytes)?;
            total_count += 1;
            total_bytes = total_bytes
                .checked_add(bytes.len())
                .ok_or_else(|| "pdf asset byte total overflow".to_string())?;
            destination.insert(name.to_string(), Base64OrRaw::Raw(bytes.clone()));
        }
    }
    if total_count > MAX_ASSET_COUNT || total_bytes > MAX_ASSET_BYTES {
        return Err("pdf asset count or total byte limit exceeded".to_string());
    }
    Ok((images, fonts))
}

fn style_allowed(style: &str) -> bool {
    if style.len() > 4096 {
        return false;
    }
    let lowered = style.to_ascii_lowercase();
    if ["url(", "@import", "expression(", "javascript:", "file:", "data:", "behavior:"]
        .iter()
        .any(|needle| lowered.contains(needle))
    {
        return false;
    }
    let allowed: HashSet<&str> = [
        "display",
        "flex-direction",
        "justify-content",
        "align-items",
        "gap",
        "width",
        "height",
        "min-width",
        "max-width",
        "min-height",
        "max-height",
        "padding",
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
        "margin",
        "margin-top",
        "margin-right",
        "margin-bottom",
        "margin-left",
        "border",
        "border-width",
        "border-style",
        "border-color",
        "border-top",
        "border-right",
        "border-bottom",
        "border-left",
        "border-collapse",
        "border-radius",
        "background",
        "background-color",
        "color",
        "font-family",
        "font-size",
        "font-style",
        "font-weight",
        "line-height",
        "text-align",
        "text-decoration",
        "vertical-align",
        "white-space",
        "table-layout",
        "page-break-before",
        "page-break-after",
        "page-break-inside",
        "break-before",
        "break-after",
        "break-inside",
    ]
    .into_iter()
    .collect();
    style.split(';').all(|declaration| {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            return true;
        }
        let Some((property, value)) = declaration.split_once(':') else {
            return false;
        };
        allowed.contains(property.trim().to_ascii_lowercase().as_str())
            && !value.trim().is_empty()
            && value.len() <= 512
    })
}

struct PdfHtmlSink<'a> {
    images: &'a HashSet<String>,
    failed: RefCell<Option<String>>,
    tokens: Cell<usize>,
    depth: Cell<usize>,
    roots: RefCell<Vec<SafeNode>>,
    stack: RefCell<Vec<SafeElement>>,
}

#[derive(Clone)]
enum SafeNode {
    Element(SafeElement),
    Text(String),
}

#[derive(Clone)]
struct SafeElement {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<SafeNode>,
}

fn escape_html_text(value: &str, attribute: bool) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' if attribute => escaped.push_str("&quot;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn is_void_element(name: &str) -> bool {
    matches!(name, "br" | "hr" | "img")
}

fn serialize_attrs(attrs: &[(String, String)], excluded: Option<&str>) -> String {
    let mut rendered = String::new();
    for (name, value) in attrs.iter().filter(|(name, _)| excluded != Some(name.as_str())) {
        rendered.push(' ');
        rendered.push_str(name);
        rendered.push_str("=\"");
        rendered.push_str(&escape_html_text(value, true));
        rendered.push('"');
    }
    rendered
}

fn serialize_node(node: &SafeNode, output: &mut String) -> Result<(), String> {
    match node {
        SafeNode::Text(text) => output.push_str(&escape_html_text(text, false)),
        SafeNode::Element(element) => serialize_element(element, output)?,
    }
    Ok(())
}

fn only_whitespace(nodes: &[SafeNode]) -> bool {
    nodes.iter().all(|node| matches!(node, SafeNode::Text(text) if text.trim().is_empty()))
}

fn direct_elements<'a>(element: &'a SafeElement, name: &str) -> Vec<&'a SafeElement> {
    element
        .children
        .iter()
        .filter_map(|node| match node {
            SafeNode::Element(child) if child.name == name => Some(child),
            _ => None,
        })
        .collect()
}

fn serialize_repeating_table(
    element: &SafeElement,
    every: usize,
    output: &mut String,
) -> Result<(), String> {
    let headers = direct_elements(element, "thead");
    let bodies = direct_elements(element, "tbody");
    if headers.len() != 1 || bodies.len() != 1 {
        return Err("pdf repeating tables require exactly one direct thead and one direct tbody"
            .to_string());
    }
    let header = headers[0];
    let body = bodies[0];
    let rows: Vec<&SafeNode> = body
        .children
        .iter()
        .filter(|node| matches!(node, SafeNode::Element(child) if child.name == "tr"))
        .collect();
    let body_extras: Vec<SafeNode> = body
        .children
        .iter()
        .filter(|node| !matches!(node, SafeNode::Element(child) if child.name == "tr"))
        .cloned()
        .collect();
    if !only_whitespace(&body_extras) || rows.is_empty() {
        return Err("pdf repeating table tbody must contain direct tr rows only".to_string());
    }
    let table_extras: Vec<SafeNode> = element
        .children
        .iter()
        .filter(|node| {
            !matches!(node, SafeNode::Element(child) if matches!(child.name.as_str(), "thead" | "tbody" | "tfoot"))
        })
        .cloned()
        .collect();
    if !only_whitespace(&table_extras) || direct_elements(element, "tfoot").len() > 1 {
        return Err("pdf repeating table has unsupported direct content".to_string());
    }
    let foot = direct_elements(element, "tfoot").into_iter().next();
    for (index, chunk) in rows.chunks(every).enumerate() {
        if index > 0 {
            output.push_str("<div style=\"page-break-before:always\"></div>");
        }
        output.push_str("<table");
        output.push_str(&serialize_attrs(&element.attrs, Some("data-repeat-header-every")));
        output.push('>');
        serialize_element(header, output)?;
        output.push_str("<tbody");
        output.push_str(&serialize_attrs(&body.attrs, None));
        output.push('>');
        for row in chunk {
            serialize_node(row, output)?;
        }
        output.push_str("</tbody>");
        if index + 1 == rows.chunks(every).len() {
            if let Some(foot) = foot {
                serialize_element(foot, output)?;
            }
        }
        output.push_str("</table>");
    }
    Ok(())
}

fn serialize_element(element: &SafeElement, output: &mut String) -> Result<(), String> {
    if element.name == "table" {
        if let Some((_, value)) =
            element.attrs.iter().find(|(name, _)| name == "data-repeat-header-every")
        {
            let every = value
                .parse::<usize>()
                .map_err(|_| "pdf repeating table row count is invalid".to_string())?;
            return serialize_repeating_table(element, every, output);
        }
    }
    output.push('<');
    output.push_str(&element.name);
    output.push_str(&serialize_attrs(&element.attrs, None));
    output.push('>');
    if !is_void_element(&element.name) {
        for child in &element.children {
            serialize_node(child, output)?;
        }
        output.push_str("</");
        output.push_str(&element.name);
        output.push('>');
    }
    Ok(())
}

impl PdfHtmlSink<'_> {
    fn fail(&self, error: impl Into<String>) {
        if self.failed.borrow().is_none() {
            *self.failed.borrow_mut() = Some(error.into());
        }
    }

    fn append(&self, node: SafeNode) {
        let mut stack = self.stack.borrow_mut();
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        } else {
            drop(stack);
            self.roots.borrow_mut().push(node);
        }
    }
}

impl TokenSink for PdfHtmlSink<'_> {
    type Handle = ();

    fn process_token(&self, token: Token, _line: u64) -> TokenSinkResult<()> {
        if self.failed.borrow().is_some() {
            return TokenSinkResult::Continue;
        }
        self.tokens.set(self.tokens.get() + 1);
        if self.tokens.get() > MAX_HTML_TOKENS {
            *self.failed.borrow_mut() = Some("pdf HTML token limit exceeded".to_string());
            return TokenSinkResult::Continue;
        }
        if let Token::TagToken(tag) = token {
            let name = tag.name.to_string().to_ascii_lowercase();
            let allowed = [
                "html", "head", "title", "body", "div", "section", "header", "footer", "main",
                "article", "h1", "h2", "h3", "h4", "h5", "h6", "p", "span", "strong", "b", "em",
                "i", "u", "small", "ul", "ol", "li", "table", "thead", "tbody", "tfoot", "tr",
                "th", "td", "hr", "br", "img", "a",
            ];
            if !allowed.contains(&name.as_str()) {
                *self.failed.borrow_mut() = Some(format!("unsupported pdf HTML tag '{name}'"));
                return TokenSinkResult::Continue;
            }
            let mut safe_attrs = Vec::with_capacity(tag.attrs.len());
            for attribute in tag.attrs {
                let key = attribute.name.local.to_string().to_ascii_lowercase();
                let value = attribute.value.to_string();
                if key.starts_with("on") {
                    *self.failed.borrow_mut() =
                        Some("event handler attributes are not supported in pdf HTML".to_string());
                    return TokenSinkResult::Continue;
                }
                let valid = match key.as_str() {
                    "style" => style_allowed(&value),
                    "class" | "id" | "alt" => value.len() <= 256,
                    "colspan" | "rowspan" | "width" | "height" => {
                        value.parse::<u16>().is_ok_and(|number| number > 0 && number <= 4096)
                    }
                    "src" if name == "img" => self.images.contains(&value),
                    "href" if name == "a" => value.starts_with('#') && value.len() <= 256,
                    "data-repeat-header-every" if name == "table" => {
                        value.parse::<usize>().is_ok_and(|number| (1..=100).contains(&number))
                    }
                    _ => false,
                };
                if !valid {
                    *self.failed.borrow_mut() = Some(format!(
                        "unsupported or unsafe pdf HTML attribute '{key}' on '{name}'"
                    ));
                    return TokenSinkResult::Continue;
                }
                safe_attrs.push((key, value));
            }
            safe_attrs.sort();
            if tag.kind == TagKind::StartTag {
                let element =
                    SafeElement { name: name.clone(), attrs: safe_attrs, children: Vec::new() };
                if tag.self_closing || is_void_element(&name) {
                    self.append(SafeNode::Element(element));
                } else {
                    self.depth.set(self.depth.get() + 1);
                    if self.depth.get() > MAX_HTML_DEPTH {
                        self.fail("pdf HTML nesting limit exceeded");
                        return TokenSinkResult::Continue;
                    }
                    self.stack.borrow_mut().push(element);
                }
            } else {
                if is_void_element(&name) {
                    self.fail("pdf HTML void elements must not have closing tags");
                    return TokenSinkResult::Continue;
                }
                let Some(element) = self.stack.borrow_mut().pop() else {
                    self.fail("pdf HTML has an unmatched closing tag");
                    return TokenSinkResult::Continue;
                };
                if element.name != name {
                    self.fail("pdf HTML tags must be explicitly and correctly nested");
                    return TokenSinkResult::Continue;
                }
                self.depth.set(self.depth.get().saturating_sub(1));
                self.append(SafeNode::Element(element));
            }
        } else if let Token::CharacterTokens(text) = token {
            self.append(SafeNode::Text(text.to_string()));
        } else if matches!(token, Token::NullCharacterToken) {
            self.fail("pdf HTML contains a null character");
        } else if matches!(token, Token::ParseError(_)) {
            self.fail("pdf HTML is malformed");
        }
        TokenSinkResult::Continue
    }
}

fn normalize_html(html: &str, image_names: &HashSet<String>) -> Result<String, String> {
    if html.is_empty() || html.len() > MAX_HTML_BYTES {
        return Err(format!("pdf HTML must be 1..{MAX_HTML_BYTES} bytes"));
    }
    if html.contains('\0') {
        return Err("pdf HTML contains a null character".to_string());
    }
    let sink = PdfHtmlSink {
        images: image_names,
        failed: RefCell::new(None),
        tokens: Cell::new(0),
        depth: Cell::new(0),
        roots: RefCell::new(Vec::new()),
        stack: RefCell::new(Vec::new()),
    };
    let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
    let queue = BufferQueue::default();
    queue.push_back(html.into());
    let _ = tokenizer.feed(&queue);
    tokenizer.end();
    if let Some(error) = tokenizer.sink.failed.into_inner() {
        return Err(error);
    }
    if !tokenizer.sink.stack.into_inner().is_empty() {
        return Err("pdf HTML tags must be explicitly closed".to_string());
    }
    let mut normalized = String::with_capacity(html.len());
    for node in tokenizer.sink.roots.into_inner() {
        serialize_node(&node, &mut normalized)?;
    }
    if normalized.is_empty() || normalized.len() > MAX_HTML_BYTES {
        return Err("pdf normalized HTML exceeds the byte limit".to_string());
    }
    Ok(normalized)
}

#[allow(dead_code)]
pub(crate) fn validate_html_for_fuzz(html: &str) -> Result<(), String> {
    normalize_html(html, &HashSet::new()).map(|_| ())
}

fn render(
    html: String,
    options: RenderOptions,
    images: BTreeMap<String, Base64OrRaw>,
    fonts: BTreeMap<String, Base64OrRaw>,
) -> Result<RenderedPdf, String> {
    let permit = RenderPermit::acquire()?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let wait = options.timeout;
    std::thread::spawn(move || {
        let _permit = permit;
        let result = std::panic::catch_unwind(|| {
            let font_bytes: BTreeMap<String, Vec<u8>> = fonts
                .iter()
                .filter_map(|(name, value)| match value {
                    Base64OrRaw::Raw(bytes) => Some((name.clone(), bytes.clone())),
                    Base64OrRaw::B64(_) => None,
                })
                .collect();
            let pool = printpdf::html::build_font_pool(&font_bytes, Some(&[]));
            let generate = GeneratePdfOptions {
                font_embedding: Some(true),
                page_width: Some(options.page_width_mm),
                page_height: Some(options.page_height_mm),
                margin_top: Some(options.margin_top_mm),
                margin_right: Some(options.margin_right_mm),
                margin_bottom: Some(options.margin_bottom_mm),
                margin_left: Some(options.margin_left_mm),
                image_optimization: None,
                show_page_numbers: Some(options.show_page_numbers),
                header_text: options.header_text.clone(),
                footer_text: options.footer_text.clone(),
                skip_first_page: Some(false),
            };
            let mut warnings = Vec::new();
            let mut document = PdfDocument::from_html_with_cache(
                &html,
                &images,
                &fonts,
                &generate,
                &mut warnings,
                Some(pool),
            )?;
            if document.pages.is_empty() {
                return Err("pdf renderer produced no pages".to_string());
            }
            if document.pages.len() > MAX_PAGES {
                return Err(format!("pdf output page limit of {MAX_PAGES} exceeded"));
            }
            document.metadata.info.document_title = options.title;
            document.metadata.info.creator = "Kujo".to_string();
            document.metadata.info.producer = RENDERER_VERSION.to_string();
            let pages = document.pages.len();
            let mut save_warnings = Vec::new();
            let serialized = document.save(&PdfSaveOptions::default(), &mut save_warnings);
            let bytes = canonicalize_pdf_identifier(&serialized, html.as_bytes())?;
            let warning_count = warnings.len() + save_warnings.len();
            if bytes.len() > options.max_output_bytes {
                return Err("pdf output byte limit exceeded".to_string());
            }
            if !bytes.starts_with(b"%PDF-") {
                return Err("pdf renderer produced an invalid header".to_string());
            }
            Ok(RenderedPdf {
                bytes,
                pages,
                width_mm: options.page_width_mm,
                height_mm: options.page_height_mm,
                warning_count,
                warning_codes: vec!["renderer_warning".to_string(); warning_count],
            })
        })
        .unwrap_or_else(|_| Err("pdf renderer failed safely".to_string()));
        let _ = sender.send(result);
    });
    receive_before_deadline(receiver, wait)?
}

fn receipt(
    rendered: RenderedPdf,
    input_sha256: String,
    elapsed: Duration,
    include_bytes: bool,
) -> Value {
    let output_sha256 = sha256_hex(&rendered.bytes);
    let mut result = DictMap::default();
    result.insert("ok".into(), Value::Bool(true));
    result.insert("api_version".into(), Value::Str(Arc::new(API_VERSION.to_string())));
    result.insert("renderer_version".into(), Value::Str(Arc::new(RENDERER_VERSION.to_string())));
    result.insert("input_sha256".into(), Value::Str(Arc::new(input_sha256)));
    result.insert("output_sha256".into(), Value::Str(Arc::new(output_sha256)));
    result.insert("bytes".into(), Value::Int(rendered.bytes.len() as i64));
    result.insert("pages".into(), Value::Int(rendered.pages as i64));
    result.insert("page_width_mm".into(), Value::Float(rendered.width_mm as f64));
    result.insert("page_height_mm".into(), Value::Float(rendered.height_mm as f64));
    result.insert("warning_count".into(), Value::Int(rendered.warning_count as i64));
    result.insert(
        "warnings".into(),
        Value::Array(Arc::new(
            rendered
                .warning_codes
                .into_iter()
                .map(|warning| Value::Str(Arc::new(warning)))
                .collect(),
        )),
    );
    result.insert(
        "render_duration_ms".into(),
        Value::Int(elapsed.as_millis().min(i64::MAX as u128) as i64),
    );
    if include_bytes {
        result.insert("pdf_bytes".into(), Value::Bytes(rendered.bytes));
    }
    Value::Dict(Arc::new(result))
}

fn destination(value: &Value) -> Result<PathBuf, String> {
    let Value::Str(value) = value else {
        return Err("pdf destination must be a string path".to_string());
    };
    let path = PathBuf::from(value.as_ref());
    if !path.is_absolute() || path.file_name().is_none() {
        return Err("pdf destination must be an absolute file path".to_string());
    }
    Ok(path)
}

fn publish_private_noreplace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "pdf destination has no parent".to_string())?;
    if !parent.is_dir() {
        return Err("pdf destination parent must be an existing directory".to_string());
    }
    let mut temporary = tempfile::Builder::new()
        .prefix(".kujo-pdf-")
        .tempfile_in(parent)
        .map_err(|_| "could not create private pdf output".to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| "could not secure private pdf output".to_string())?;
    }
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.flush())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| "could not write private pdf output".to_string())?;
    temporary
        .persist_noclobber(path)
        .map_err(|_| "pdf destination already exists or could not be published".to_string())?;
    Ok(())
}

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    if name != "pdf_render_html" && name != "pdf_render_html_to_file" {
        return None;
    }
    let expected = if name == "pdf_render_html" { 3 } else { 4 };
    if arg_values.len() != expected {
        return Some(Value::Error(format!("{name} expects {expected} arguments")));
    }
    let Value::Str(html) = &arg_values[0] else {
        return Some(Value::Error("pdf HTML input must be a string".to_string()));
    };
    let options = match parse_options(&arg_values[1]) {
        Ok(value) => value,
        Err(error) => return Some(Value::Error(error)),
    };
    let (images, fonts) = match parse_assets(&arg_values[2]) {
        Ok(value) => value,
        Err(error) => return Some(Value::Error(error)),
    };
    let image_names = images.keys().cloned().collect();
    let normalized_html = match normalize_html(html, &image_names) {
        Ok(value) => value,
        Err(error) => return Some(Value::Error(error)),
    };
    let input_sha256 = sha256_hex(normalized_html.as_bytes());
    let started = Instant::now();
    let rendered = match render(normalized_html, options, images, fonts) {
        Ok(value) => value,
        Err(error) => return Some(Value::Error(error)),
    };

    if name == "pdf_render_html_to_file" {
        let path = match destination(&arg_values[3]) {
            Ok(value) => value,
            Err(error) => return Some(Value::Error(error)),
        };
        if let Err(error) = publish_private_noreplace(&path, &rendered.bytes) {
            return Some(Value::Error(error));
        }
        let mut value = receipt(rendered, input_sha256, started.elapsed(), false);
        if let Value::Dict(fields) = &mut value {
            Arc::make_mut(fields)
                .insert("path".into(), Value::Str(Arc::new(path.to_string_lossy().to_string())));
        }
        Some(value)
    } else {
        Some(receipt(rendered, input_sha256, started.elapsed(), true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    static PDF_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        PDF_TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn empty_dict() -> Value {
        Value::Dict(Arc::new(DictMap::default()))
    }

    fn render_html(html: &str) -> Result<Value, String> {
        match handle(
            "pdf_render_html",
            &[Value::Str(Arc::new(html.to_string())), empty_dict(), empty_dict()],
        ) {
            Some(Value::Error(error)) => Err(error),
            Some(value) => Ok(value),
            None => Err("pdf handler declined request".to_string()),
        }
    }

    fn receipt_bytes(value: &Value) -> &[u8] {
        let Value::Dict(fields) = value else { panic!("expected pdf receipt") };
        let Some(Value::Bytes(bytes)) = fields.get("pdf_bytes") else {
            panic!("expected pdf bytes")
        };
        bytes
    }

    fn page_size_points(document: &lopdf::Document, page: lopdf::ObjectId) -> (f32, f32) {
        let mut current = page;
        loop {
            let dictionary = document
                .get_object(current)
                .expect("page object")
                .as_dict()
                .expect("page dictionary");
            if let Ok(media_box) = dictionary.get(b"MediaBox") {
                let values = media_box.as_array().expect("MediaBox array");
                return (
                    values[2].as_float().expect("page width"),
                    values[3].as_float().expect("page height"),
                );
            }
            current = dictionary
                .get(b"Parent")
                .expect("inherited MediaBox parent")
                .as_reference()
                .expect("parent reference");
        }
    }

    #[test]
    fn renders_parseable_branded_business_document() {
        let _guard = test_guard();
        let html = include_str!("../../../tests/fixtures/pdf/hvac-estimate.html");
        let first = render_html(html).expect("render succeeds");
        let second = render_html(html).expect("repeat render succeeds");
        let first_bytes = receipt_bytes(&first);
        let second_bytes = receipt_bytes(&second);
        assert!(first_bytes.starts_with(b"%PDF-"));
        assert_eq!(sha256_hex(first_bytes), sha256_hex(second_bytes));

        let document =
            lopdf::Document::load_mem(first_bytes).expect("independent parser accepts PDF");
        let pages = document.get_pages();
        assert!(!pages.is_empty());
        let (width, height) = page_size_points(&document, pages[&1]);
        assert!((width - 595.28).abs() < 1.0);
        assert!((height - 841.89).abs() < 1.0);
        let text = document.extract_text(&[1]).expect("text extraction succeeds");
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(normalized.contains("Northstar Heating & Air"));
        assert!(normalized.contains("$9,850.00"));
        assert!(normalized.contains("José"));
        let debug = format!("{:?}{:?}", document.trailer, document.objects);
        assert!(!debug.contains("/JavaScript"));
        assert!(!debug.contains("/Launch"));
        assert!(!debug.contains("/URI"));
    }

    #[test]
    fn renders_letter_landscape_with_expected_dimensions() {
        let _guard = test_guard();
        let mut options = DictMap::default();
        options.insert("page_size".into(), Value::Str(Arc::new("Letter".into())));
        options.insert("orientation".into(), Value::Str(Arc::new("landscape".into())));
        let value = handle(
            "pdf_render_html",
            &[
                Value::Str(Arc::new("<h1>Landscape quotation</h1>".into())),
                Value::Dict(Arc::new(options)),
                empty_dict(),
            ],
        )
        .expect("handler result");
        let document =
            lopdf::Document::load_mem(receipt_bytes(&value)).expect("generated PDF parses");
        let pages = document.get_pages();
        let (width, height) = page_size_points(&document, pages[&1]);
        assert!((width - 792.0).abs() < 1.0);
        assert!((height - 612.0).abs() < 1.0);
    }

    #[test]
    fn concurrent_business_document_renders_are_isolated() {
        let _guard = test_guard();
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let threads = (1..=4)
            .map(|number| {
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    let expected = format!("Customer {number} total ${number},000.00");
                    let html = format!("<h1>Quote {number}</h1><p>{expected}</p>");
                    let value = render_html(&html).expect("concurrent render succeeds");
                    let document = lopdf::Document::load_mem(receipt_bytes(&value))
                        .expect("concurrent PDF parses");
                    let text = document.extract_text(&[1]).expect("text extraction succeeds");
                    assert!(text.contains(&expected));
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().expect("render worker did not panic");
        }
    }

    #[test]
    fn rejects_active_content_and_external_resource_references() {
        let _guard = test_guard();
        for html in [
            "<script>alert(1)</script>",
            "<img src=\"https://attacker.test/a.png\">",
            "<img src=\"file:///etc/passwd\">",
            "<p onclick=\"steal()\">unsafe</p>",
            "<p style=\"background:url(https://attacker.test/x)\">unsafe</p>",
            "<p style=\"position:absolute\">unsupported</p>",
        ] {
            let error = render_html(html).expect_err("unsafe HTML must fail closed");
            assert!(
                error.contains("unsupported")
                    || error.contains("unsafe")
                    || error.contains("event handler"),
                "{error}"
            );
        }
    }

    #[test]
    fn enforces_strict_option_bounds() {
        let _guard = test_guard();
        let mut options = DictMap::default();
        options.insert("unexpected".into(), Value::Bool(true));
        let result = handle(
            "pdf_render_html",
            &[
                Value::Str(Arc::new("<p>test</p>".to_string())),
                Value::Dict(Arc::new(options)),
                empty_dict(),
            ],
        );
        assert!(
            matches!(result, Some(Value::Error(error)) if error.contains("unknown pdf option"))
        );
    }

    #[test]
    fn canonical_html_normalization_is_attribute_order_independent() {
        let _guard = test_guard();
        let first = render_html("<p id=\"quote\" class=\"total\">A &amp; B</p>")
            .expect("first render succeeds");
        let second = render_html("<p class=\"total\" id=\"quote\">A &amp; B</p>")
            .expect("second render succeeds");
        let Value::Dict(first) = first else { panic!("expected first receipt") };
        let Value::Dict(second) = second else { panic!("expected second receipt") };
        let Some(Value::Str(first_input)) = first.get("input_sha256") else {
            panic!("expected first input digest")
        };
        let Some(Value::Str(second_input)) = second.get("input_sha256") else {
            panic!("expected second input digest")
        };
        let Some(Value::Str(first_output)) = first.get("output_sha256") else {
            panic!("expected first output digest")
        };
        let Some(Value::Str(second_output)) = second.get("output_sha256") else {
            panic!("expected second output digest")
        };
        assert_eq!(first_input, second_input);
        assert_eq!(first_output, second_output);
    }

    #[test]
    fn malformed_or_implicitly_closed_html_fails_closed() {
        let _guard = test_guard();
        for html in ["<div><span>broken</div></span>", "<div>unclosed"] {
            assert!(render_html(html).unwrap_err().contains("pdf HTML"));
        }
    }

    #[test]
    fn file_publish_is_private_atomic_and_no_clobber() {
        let _guard = test_guard();
        let directory = tempfile::tempdir().expect("temp directory");
        let path = directory.path().join("estimate.pdf");
        let arguments = || {
            vec![
                Value::Str(Arc::new("<h1>Estimate</h1>".to_string())),
                empty_dict(),
                empty_dict(),
                Value::Str(Arc::new(path.to_string_lossy().to_string())),
            ]
        };
        let first = handle("pdf_render_html_to_file", &arguments());
        assert!(matches!(first, Some(Value::Dict(_))));
        assert!(fs::read(&path).expect("published PDF").starts_with(b"%PDF-"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let second = handle("pdf_render_html_to_file", &arguments());
        assert!(matches!(second, Some(Value::Error(error)) if error.contains("already exists")));
    }

    #[test]
    fn page_break_and_header_footer_create_multi_page_document() {
        let _guard = test_guard();
        let mut options = DictMap::default();
        options.insert("show_page_numbers".into(), Value::Bool(true));
        options.insert("header_text".into(), Value::Str(Arc::new("QuoteFlow quotation".into())));
        options.insert("footer_text".into(), Value::Str(Arc::new("Confidential".into())));
        let value = handle(
            "pdf_render_html",
            &[
                Value::Str(Arc::new(
                    "<section><h1>Page one</h1></section><section style=\"page-break-before:always\"><h1>Page two</h1></section>".into(),
                )),
                Value::Dict(Arc::new(options)),
                empty_dict(),
            ],
        )
        .expect("handler result");
        let Value::Dict(fields) = value else { panic!("expected receipt") };
        assert!(matches!(fields.get("pages"), Some(Value::Int(pages)) if *pages >= 2));
    }

    #[test]
    fn renders_caller_supplied_logo_without_external_resolution() {
        let _guard = test_guard();
        let png = base64::engine::general_purpose::STANDARD
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=")
            .expect("fixture PNG");
        let mut images = DictMap::default();
        images.insert("logo.png".into(), Value::Bytes(png));
        let mut assets = DictMap::default();
        assets.insert("images".into(), Value::Dict(Arc::new(images)));
        let value = handle(
            "pdf_render_html",
            &[
                Value::Str(Arc::new(
                    "<header><img src=\"logo.png\" width=\"32\" height=\"32\"><h1>Branded quotation</h1></header>".into(),
                )),
                empty_dict(),
                Value::Dict(Arc::new(assets)),
            ],
        )
        .expect("handler result");
        assert!(receipt_bytes(&value).starts_with(b"%PDF-"));
    }

    #[test]
    fn embeds_and_uses_caller_supplied_approved_unicode_font() {
        let _guard = test_guard();
        let mut fonts = DictMap::default();
        fonts.insert(
            "Sansation".into(),
            Value::Bytes(
                include_bytes!("../../../tests/fixtures/pdf/Sansation-Regular.ttf").to_vec(),
            ),
        );
        let mut assets = DictMap::default();
        assets.insert("fonts".into(), Value::Dict(Arc::new(fonts)));
        let value = handle(
            "pdf_render_html",
            &[
                Value::Str(Arc::new(
                    "<p style=\"font-family:Sansation\">José · naïve · €9,850</p>".into(),
                )),
                empty_dict(),
                Value::Dict(Arc::new(assets)),
            ],
        )
        .expect("handler result");
        let document =
            lopdf::Document::load_mem(receipt_bytes(&value)).expect("generated PDF parses");
        let text = document.extract_text(&[1]).expect("text extraction succeeds");
        assert!(text.contains("José"));
        assert!(text.contains("naïve"));
        assert!(text.contains("€9,850"));
        let objects = format!("{:?}", document.objects);
        assert!(objects.contains("Sansation"), "approved font name must be embedded");
        assert!(objects.contains("FontFile2"), "approved TrueType bytes must be embedded");
    }

    #[test]
    fn renders_multi_page_fabrication_quotation_fixture() {
        let _guard = test_guard();
        let value =
            render_html(include_str!("../../../tests/fixtures/pdf/fabrication-quotation.html"))
                .expect("fabrication quotation renders");
        let bytes = receipt_bytes(&value);
        let document = lopdf::Document::load_mem(bytes).expect("independent parser accepts PDF");
        let page_count = document.get_pages().len();
        assert!(page_count >= 4);
        let page_text = (1..=page_count as u32)
            .map(|page| {
                document
                    .extract_text(&[page])
                    .expect("text extraction succeeds")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>();
        let normalized = page_text.join(" ");
        assert!(normalized.contains("ABC Manufacturing Quotation"));
        assert!(normalized.contains("250 mounting brackets"));
        assert!(normalized.contains("Terms"));
        let terms_page =
            page_text.iter().position(|text| text.contains("Terms")).expect("terms page exists");
        assert!(terms_page >= 3, "fixture must contain at least three table pages");
        for (index, text) in page_text.iter().take(terms_page).enumerate() {
            assert!(
                text.contains("Specification") && text.contains("Quantity"),
                "repeating table header missing from fixture page {}",
                index + 1
            );
        }
    }

    #[test]
    fn repeats_table_header_on_every_automatically_paginated_page() {
        let _guard = test_guard();
        let rows = (1..=90)
            .map(|number| {
                format!(
                    "<tr><td style=\"border:1px solid #333;padding:6px\">{number}</td><td style=\"border:1px solid #333;padding:6px\">Bracket {number}</td></tr>"
                )
            })
            .collect::<String>();
        let html = format!(
            "<html><body><table data-repeat-header-every=\"20\" style=\"width:100%;border-collapse:collapse\"><thead><tr><th style=\"border:1px solid #333;padding:6px\">Line</th><th style=\"border:1px solid #333;padding:6px\">Part number</th></tr></thead><tbody>{rows}</tbody></table></body></html>"
        );
        let value = render_html(&html).expect("long table renders");
        let document =
            lopdf::Document::load_mem(receipt_bytes(&value)).expect("generated PDF parses");
        let pages = document.get_pages();
        assert!(pages.len() >= 2, "fixture must exercise automatic pagination");
        for page_number in 1..=pages.len() as u32 {
            let text = document.extract_text(&[page_number]).expect("page text extracts");
            let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                normalized.contains("Part number"),
                "table header missing from automatically paginated page {page_number}: {normalized}"
            );
        }
    }

    #[test]
    fn rejects_malformed_and_oversized_assets_before_rendering() {
        let _guard = test_guard();
        let mut malformed_images = DictMap::default();
        malformed_images.insert("logo.png".into(), Value::Bytes(vec![0_u8; 32]));
        let mut assets = DictMap::default();
        assets.insert("images".into(), Value::Dict(Arc::new(malformed_images)));
        let result = handle(
            "pdf_render_html",
            &[
                Value::Str(Arc::new("<img src=\"logo.png\">".into())),
                empty_dict(),
                Value::Dict(Arc::new(assets)),
            ],
        );
        assert!(matches!(result, Some(Value::Error(error)) if error.contains("image asset")));

        let mut oversized_fonts = DictMap::default();
        oversized_fonts.insert("tenant.ttf".into(), Value::Bytes(vec![0_u8; MAX_FONT_BYTES + 1]));
        let mut assets = DictMap::default();
        assets.insert("fonts".into(), Value::Dict(Arc::new(oversized_fonts)));
        let result = handle(
            "pdf_render_html",
            &[
                Value::Str(Arc::new("<p>bounded</p>".into())),
                empty_dict(),
                Value::Dict(Arc::new(assets)),
            ],
        );
        assert!(matches!(result, Some(Value::Error(error)) if error.contains("font assets")));
    }

    #[test]
    fn rejects_decompressed_image_dimension_bomb() {
        let _guard = test_guard();
        let image = image::DynamicImage::new_rgba8(4097, 1);
        let mut cursor = std::io::Cursor::new(Vec::new());
        image.write_to(&mut cursor, image::ImageFormat::Png).expect("encode image fixture");
        assert!(validate_image(cursor.get_ref()).unwrap_err().contains("dimensions"));
    }

    #[test]
    fn concurrency_reservation_is_atomic_and_bounded() {
        let _guard = test_guard();
        let counter = AtomicUsize::new(0);
        for _ in 0..MAX_CONCURRENT_RENDERS {
            reserve_render_slot(&counter, MAX_CONCURRENT_RENDERS).expect("slot available");
        }
        assert_eq!(counter.load(Ordering::Acquire), MAX_CONCURRENT_RENDERS);
        assert!(reserve_render_slot(&counter, MAX_CONCURRENT_RENDERS)
            .unwrap_err()
            .contains("concurrency limit"));
    }

    #[test]
    fn render_deadline_fails_closed() {
        let _guard = test_guard();
        let (_sender, receiver) = mpsc::sync_channel::<()>(1);
        assert!(receive_before_deadline(receiver, Duration::from_millis(5))
            .unwrap_err()
            .contains("deadline exceeded"));
    }

    #[test]
    fn malformed_font_fails_without_content_disclosure() {
        let _guard = test_guard();
        let mut fonts = DictMap::default();
        let mut fake = vec![0_u8; 64];
        fake[..4].copy_from_slice(&[0, 1, 0, 0]);
        fonts.insert("tenant.ttf".into(), Value::Bytes(fake));
        let mut assets = DictMap::default();
        assets.insert("fonts".into(), Value::Dict(Arc::new(fonts)));
        let result = handle(
            "pdf_render_html",
            &[
                Value::Str(Arc::new("<p>PRIVATE-CUSTOMER-TEXT</p>".into())),
                empty_dict(),
                Value::Dict(Arc::new(assets)),
            ],
        );
        assert!(
            matches!(result, Some(Value::Error(error)) if !error.contains("PRIVATE-CUSTOMER-TEXT"))
        );
    }
}
