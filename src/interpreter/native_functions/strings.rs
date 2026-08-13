// File: src/interpreter/native_functions/strings.rs
//
// String manipulation native functions

use crate::builtins;
use crate::interpreter::{DictMap, Value};
use crate::runtime_limits;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

// Native port of the SSG's inline-markdown pass, with HTML/attribute escaping
// at the native boundary so untrusted Markdown cannot inject active content.
static MD_IMG: Lazy<Regex> = Lazy::new(|| Regex::new(r"!\[([^\]]*)\]\(([^\)]+)\)").unwrap());
static MD_LINK: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([^\]]+)\]\(([^\)]+)\)").unwrap());
static MD_BOLD: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*\*([^\*]+)\*\*").unwrap());
static MD_ITALIC: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*([^\*]+)\*").unwrap());
static MD_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`([^`]+)`").unwrap());

fn is_safe_markdown_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with('/')
        || lower.starts_with('#')
        || lower.starts_with("./")
        || lower.starts_with("../")
    {
        return true;
    }
    !lower.contains(':')
}

fn safe_markdown_url(url: &str) -> String {
    if is_safe_markdown_url(url) {
        // inline_markdown_native escapes the source before applying Markdown
        // substitutions, so safe captures are already attribute-safe.
        url.trim().to_string()
    } else {
        "#".to_string()
    }
}

fn inline_markdown_native(text: &str) -> String {
    let mut out = lc_escape(text);
    if out.contains("![") {
        out = MD_IMG
            .replace_all(&out, |captures: &regex::Captures| {
                format!(
                    "<img src=\"{}\" alt=\"{}\">",
                    safe_markdown_url(&captures[2]),
                    &captures[1]
                )
            })
            .into_owned();
    }
    if out.contains('[') {
        out = MD_LINK
            .replace_all(&out, |captures: &regex::Captures| {
                format!("<a href=\"{}\">{}</a>", safe_markdown_url(&captures[2]), &captures[1])
            })
            .into_owned();
    }
    if out.contains("**") {
        out = MD_BOLD.replace_all(&out, "<strong>$1</strong>").into_owned();
    }
    if out.contains('*') {
        out = MD_ITALIC.replace_all(&out, "<em>$1</em>").into_owned();
    }
    if out.contains('`') {
        out = MD_CODE.replace_all(&out, "<code>$1</code>").into_owned();
    }
    out
}

fn markdown_table_cells(line: &str) -> Vec<&str> {
    line.trim().trim_matches('|').split('|').map(str::trim).collect()
}

fn is_markdown_table_separator(line: &str) -> bool {
    let cells = markdown_table_cells(line);
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let delimiter = cell.trim().trim_start_matches(':').trim_end_matches(':');
            delimiter.len() >= 3 && delimiter.chars().all(|character| character == '-')
        })
}

// Native port of markdown_to_html: paragraphs, ATX headings (#/##/###),
// unordered lists (- / *), blockquotes (>), and pipe tables, with the inline pass above.
fn render_markdown_native(markdown: &str) -> String {
    let mut html_lines: Vec<String> = Vec::new();
    let mut in_list = false;
    let lines: Vec<&str> = markdown.split('\n').collect();
    let mut index = 0;
    while index < lines.len() {
        let raw_line = lines[index];
        let line = raw_line.trim();
        if index + 1 < lines.len()
            && line.contains('|')
            && is_markdown_table_separator(lines[index + 1].trim())
        {
            if in_list {
                html_lines.push("</ul>".to_string());
                in_list = false;
            }
            html_lines.push("<table><thead><tr>".to_string());
            for cell in markdown_table_cells(line) {
                html_lines.push(format!("<th>{}</th>", inline_markdown_native(cell)));
            }
            html_lines.push("</tr></thead><tbody>".to_string());
            index += 2;
            while index < lines.len() {
                let row = lines[index].trim();
                if row.is_empty() || !row.contains('|') {
                    break;
                }
                html_lines.push("<tr>".to_string());
                for cell in markdown_table_cells(row) {
                    html_lines.push(format!("<td>{}</td>", inline_markdown_native(cell)));
                }
                html_lines.push("</tr>".to_string());
                index += 1;
            }
            html_lines.push("</tbody></table>".to_string());
            continue;
        }
        if line.starts_with("- ") || line.starts_with("* ") {
            let item_text = inline_markdown_native(line[2..].trim());
            if !in_list {
                in_list = true;
                html_lines.push("<ul>".to_string());
            }
            html_lines.push(format!("<li>{}</li>", item_text));
            index += 1;
            continue;
        }
        if in_list {
            html_lines.push("</ul>".to_string());
            in_list = false;
        }
        if line.is_empty() {
            index += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            html_lines.push(format!("<h3>{}</h3>", inline_markdown_native(rest.trim())));
            index += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            html_lines.push(format!("<h2>{}</h2>", inline_markdown_native(rest.trim())));
            index += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            html_lines.push(format!("<h1>{}</h1>", inline_markdown_native(rest.trim())));
            index += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("> ") {
            html_lines.push(format!(
                "<blockquote><p>{}</p></blockquote>",
                inline_markdown_native(rest.trim())
            ));
            index += 1;
            continue;
        }
        html_lines.push(format!("<p>{}</p>", inline_markdown_native(line)));
        index += 1;
    }
    if in_list {
        html_lines.push("</ul>".to_string());
    }
    html_lines.join("\n")
}

// --- Native listing-card renderer ------------------------------------------
// Byte-identical port of build.kujo's listing_card_html + its helpers
// (route_to_path, listing_card_media_url, listing_card_tags_html). Called 25×
// per listing page over hundreds of pages on large sites.
fn lc_escape(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + 16);
    for c in source.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn lc_route_to_path(route: &str) -> String {
    let mut clean = route.trim().to_string();
    if clean.is_empty() {
        return "/".to_string();
    }
    if let Some(rest) = clean.strip_prefix('/') {
        clean = rest.to_string();
    }
    if !clean.ends_with('/') {
        clean.push('/');
    }
    format!("/{}", clean)
}

fn lc_media_url(path_value: &str) -> String {
    let mut media = path_value.trim().to_string();
    if media.is_empty() {
        return String::new();
    }
    loop {
        if let Some(rest) = media.strip_prefix("../") {
            media = rest.to_string();
            continue;
        }
        if let Some(rest) = media.strip_prefix("./") {
            media = rest.to_string();
            continue;
        }
        break;
    }
    if media.starts_with("http://")
        || media.starts_with("https://")
        || media.starts_with("//")
        || media.starts_with('/')
    {
        media
    } else {
        format!("/{}", media)
    }
}

fn render_listing_card_native(
    route: &str,
    title: &str,
    excerpt: &str,
    featured_image: &str,
    terms: &[String],
    action_label: &str,
) -> String {
    let safe_href = lc_escape(&lc_route_to_path(route));
    let safe_title = lc_escape(title);
    let safe_excerpt = lc_escape(excerpt);
    let media_url = lc_media_url(featured_image);
    let safe_media_url = lc_escape(&media_url);
    let title_attr = lc_escape(title);

    let mut tags_html = String::new();
    for term in terms.iter().take(3) {
        tags_html.push_str("<span class=\"tag listing-tag\">");
        tags_html.push_str(&lc_escape(term));
        tags_html.push_str("</span>");
    }
    let tags_block = if tags_html.is_empty() {
        String::new()
    } else {
        format!("<div class=\"listing-card-tags\">{}</div>", tags_html)
    };

    let trimmed_label = action_label.trim();
    let button_text = if trimmed_label.is_empty() { "Read More" } else { trimmed_label };
    let safe_button_text = lc_escape(button_text);

    let media_block = if media_url.is_empty() {
        format!(
            "<a class=\"listing-card-image-link\" href=\"{}\"><span class=\"listing-card-image-placeholder\" aria-hidden=\"true\"></span></a>",
            safe_href
        )
    } else {
        format!(
            "<a class=\"listing-card-image-link\" href=\"{}\"><img src=\"{}\" alt=\"Featured image for {}\" class=\"listing-card-image\" loading=\"lazy\" decoding=\"async\"></a>",
            safe_href, safe_media_url, title_attr
        )
    };

    format!(
        "<li class=\"listing-card\">{}<div class=\"listing-card-body\">{}<h2 class=\"listing-card-title\"><a href=\"{}\" class=\"text-links\">{}</a></h2><p class=\"listing-card-excerpt\">{}</p><a href=\"{}\" class=\"listing-card-button\">{}</a></div></li>",
        media_block, tags_block, safe_href, safe_title, safe_excerpt, safe_href, safe_button_text
    )
}

// --- Native page-layout renderer -------------------------------------------
// Byte-identical port of build.kujo's render_layout + its string helpers. This
// is the single largest interpreted per-page cost in the SSG; doing it in one
// native pass removes the ~18 ms/page the bytecode VM spent on escapes,
// JSON-LD assembly, dict building, and template fills.

fn rl_dict_get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Dict(d) => d.get(key),
        Value::FixedDict { keys, values } => {
            keys.iter().position(|k| k.as_ref() == key).and_then(|i| values.get(i))
        }
        _ => None,
    }
}

// to_string() for the value types the SSG stores in these dicts.
fn rl_to_string(v: &Value) -> String {
    match v {
        Value::Str(s) => s.as_ref().clone(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Null => "null".to_string(),
        _ => String::new(),
    }
}

// to_string(dict[key]) without trimming; "" when absent.
fn rl_raw(dict: &Value, key: &str) -> String {
    rl_dict_get(dict, key).map(rl_to_string).unwrap_or_default()
}

fn rl_bool(dict: &Value, key: &str) -> bool {
    matches!(rl_dict_get(dict, key), Some(Value::Bool(true)))
}

// meta_string(meta, key, fallback): trimmed non-empty value, else fallback.
fn rl_meta(meta: &Value, key: &str, fallback: &str) -> String {
    match rl_dict_get(meta, key) {
        Some(value) => {
            let s = rl_to_string(value);
            let t = s.trim();
            if t.is_empty() {
                fallback.to_string()
            } else {
                t.to_string()
            }
        }
        None => fallback.to_string(),
    }
}

fn rl_xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn rl_json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn rl_normalize_route(route: &str) -> String {
    let clean = route.trim();
    if clean.is_empty() {
        return String::new();
    }
    let mut s = clean.to_string();
    if let Some(stripped) = s.strip_prefix('/') {
        s = stripped.to_string();
    }
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

fn rl_route_depth(route: &str) -> usize {
    let clean = rl_normalize_route(route);
    if clean.is_empty() {
        return 0;
    }
    clean.split('/').filter(|p| !p.trim().is_empty()).count()
}

fn rl_route_prefix(route: &str) -> String {
    "../".repeat(rl_route_depth(route))
}

fn rl_strip_trailing_slash(s: &str) -> &str {
    s.strip_suffix('/').unwrap_or(s)
}

fn rl_route_absolute_url(site_url: &str, route: &str) -> String {
    let site = site_url.trim();
    if site.is_empty() {
        return String::new();
    }
    let site = rl_strip_trailing_slash(site);
    let clean_route = rl_normalize_route(route);
    if clean_route.is_empty() {
        format!("{}/", site)
    } else {
        format!("{}/{}", site, clean_route)
    }
}

fn rl_absolute_media_url(path_value: &str, site_url: &str) -> String {
    let media_path = path_value.trim();
    if media_path.is_empty() {
        return String::new();
    }
    if media_path.starts_with("http://")
        || media_path.starts_with("https://")
        || media_path.starts_with("//")
    {
        return media_path.to_string();
    }
    let site = site_url.trim();
    if site.is_empty() {
        return String::new();
    }
    let site = rl_strip_trailing_slash(site);
    if media_path.starts_with('/') {
        format!("{}{}", site, media_path)
    } else {
        format!("{}/{}", site, media_path)
    }
}

fn rl_resolve_media_url(path_value: &str, relative_prefix: &str) -> String {
    let media_path = path_value.trim();
    if media_path.is_empty() {
        return String::new();
    }
    if media_path.starts_with("http://")
        || media_path.starts_with("https://")
        || media_path.starts_with("//")
        || media_path.starts_with('/')
    {
        return media_path.to_string();
    }
    format!("{}{}", relative_prefix, media_path)
}

fn rl_build_page_title(seo_title: &str, site_title: &str) -> String {
    let base = seo_title.trim();
    let base = if base.is_empty() { site_title.trim() } else { base };
    let site = site_title.trim();
    if site.is_empty() || base == site {
        base.to_string()
    } else {
        format!("{} | {}", base, site)
    }
}

fn render_layout_native(
    layout_template: &str,
    settings: &Value,
    route: &str,
    page_title: &str,
    meta_description: &str,
    navigation: &str,
    content: &str,
    page_meta: &Value,
) -> String {
    let prefix = rl_route_prefix(route);
    let style_file = if rl_bool(settings, "minify") {
        "assets/css/style.min.css"
    } else {
        "assets/css/style.css"
    };

    let seo_title = rl_meta(page_meta, "seo_title", page_title);
    let mut seo_description = rl_meta(page_meta, "seo_description", meta_description);
    if seo_description.is_empty() {
        seo_description = rl_raw(settings, "site_tagline").trim().to_string();
    }
    let seo_keywords = rl_meta(page_meta, "seo_keywords", "SSG, Static Site Generator");
    let author_meta = rl_meta(page_meta, "author", "Robert DeVore");
    let mut lang = rl_meta(page_meta, "lang", "en").to_lowercase();
    if lang.is_empty() {
        lang = "en".to_string();
    }

    let site_url = rl_raw(settings, "site_url").trim().to_string();
    let mut canonical_url = rl_meta(page_meta, "canonical", "");
    if canonical_url.is_empty() {
        canonical_url = rl_route_absolute_url(&site_url, route);
    } else if !site_url.is_empty()
        && !canonical_url.starts_with("http://")
        && !canonical_url.starts_with("https://")
        && !canonical_url.starts_with("//")
    {
        let site_root = rl_strip_trailing_slash(site_url.trim());
        canonical_url = if canonical_url.starts_with('/') {
            format!("{}{}", site_root, canonical_url)
        } else {
            format!("{}/{}", site_root, canonical_url)
        };
    }

    let mut og_url = canonical_url.clone();
    if og_url.is_empty() {
        og_url = rl_route_absolute_url(&site_url, route);
    }

    let raw_featured = rl_meta(page_meta, "featured_image", "");
    let mut social_image = rl_absolute_media_url(&raw_featured, &site_url);
    if social_image.is_empty() {
        social_image = rl_resolve_media_url(&raw_featured, &prefix);
    }

    let canonical_tag = if canonical_url.is_empty() {
        String::new()
    } else {
        format!("<link rel=\"canonical\" href=\"{}\">", rl_xml_escape(&canonical_url))
    };

    let (og_image_tag, twitter_image_tag) = if social_image.is_empty() {
        (String::new(), String::new())
    } else {
        let safe_image = rl_xml_escape(&social_image);
        (
            format!("<meta property=\"og:image\" content=\"{}\">", safe_image),
            format!("<meta name=\"twitter:image\" content=\"{}\">", safe_image),
        )
    };

    let mut og_type = rl_meta(page_meta, "og_type", "website").to_lowercase();
    if og_type != "article" {
        og_type = "website".to_string();
    }
    let mut og_locale = lang.replace('-', "_");
    if og_locale == "en" {
        og_locale = "en_US".to_string();
    }

    let published_iso = rl_meta(page_meta, "published_iso", "");
    let mut og_article_tags = String::new();
    if og_type == "article" {
        if !published_iso.is_empty() {
            og_article_tags.push_str(&format!(
                "<meta property=\"article:published_time\" content=\"{}\">",
                rl_xml_escape(&published_iso)
            ));
        }
        if !author_meta.is_empty() {
            og_article_tags.push_str(&format!(
                "<meta property=\"article:author\" content=\"{}\">",
                rl_xml_escape(&author_meta)
            ));
        }
    }

    let rss_link_tag = if !site_url.is_empty() && !rl_bool(settings, "no_aux") {
        format!(
            "<link rel=\"alternate\" type=\"application/rss+xml\" title=\"{}\" href=\"{}feed/index.xml\">",
            rl_xml_escape(&rl_raw(settings, "site_title")),
            prefix
        )
    } else {
        String::new()
    };

    let favicon_tag =
        format!("<link rel=\"icon\" type=\"image/svg+xml\" href=\"{}favicon.svg\">", prefix);

    let schema_type = if og_type == "article" { "BlogPosting" } else { "WebSite" };
    let mut json_ld_parts: Vec<String> = vec![
        "\"@context\":\"https://schema.org\"".to_string(),
        format!("\"@type\":\"{}\"", schema_type),
        format!("\"headline\":\"{}\"", rl_json_escape(&seo_title)),
        format!("\"name\":\"{}\"", rl_json_escape(&seo_title)),
        format!("\"description\":\"{}\"", rl_json_escape(&seo_description)),
        format!("\"url\":\"{}\"", rl_json_escape(&og_url)),
    ];
    if !social_image.is_empty() {
        json_ld_parts.push(format!("\"image\":\"{}\"", rl_json_escape(&social_image)));
    }
    if og_type == "article" {
        json_ld_parts.push(format!(
            "\"author\":{{\"@type\":\"Person\",\"name\":\"{}\"}}",
            rl_json_escape(&author_meta)
        ));
        if !published_iso.is_empty() {
            json_ld_parts.push(format!("\"datePublished\":\"{}\"", rl_json_escape(&published_iso)));
        }
    }
    json_ld_parts.push(format!(
        "\"publisher\":{{\"@type\":\"Organization\",\"name\":\"{}\"}}",
        rl_json_escape(&rl_raw(settings, "site_title"))
    ));
    let json_ld =
        format!("<script type=\"application/ld+json\">{{{}}}</script>", json_ld_parts.join(","));

    let computed_title = rl_build_page_title(&seo_title, &rl_raw(settings, "site_title"));
    let safe_title = rl_xml_escape(&computed_title);
    let safe_seo_title = rl_xml_escape(&seo_title);
    let safe_description = rl_xml_escape(&seo_description);
    let safe_keywords = rl_xml_escape(&seo_keywords);
    let safe_author = rl_xml_escape(&author_meta);
    let safe_og_url = rl_xml_escape(&og_url);
    let safe_site_title = rl_xml_escape(&rl_raw(settings, "site_title"));
    let safe_site_tagline = rl_xml_escape(&rl_raw(settings, "site_tagline"));
    let safe_og_locale = rl_xml_escape(&og_locale);
    let home_path = format!("{}index.html", prefix);
    let stylesheet_path = format!("{}{}", prefix, style_file);

    // apply_template: sequential {{key}} substitution in the same key order as
    // build.kujo's dict literal (byte-identical to the interpreted path).
    let pairs: [(&str, &str); 25] = [
        ("site_name", &safe_site_title),
        ("site_title", &safe_site_title),
        ("site_tagline", &safe_site_tagline),
        ("navigation", navigation),
        ("relative_path", &prefix),
        ("home_path", &home_path),
        ("page_title", &safe_title),
        ("meta_description", &safe_description),
        ("seo_title", &safe_seo_title),
        ("seo_description", &safe_description),
        ("seo_keywords", &safe_keywords),
        ("author_meta", &safe_author),
        ("canonical_tag", &canonical_tag),
        ("og_url", &safe_og_url),
        ("og_type", &og_type),
        ("og_locale", &safe_og_locale),
        ("og_image_tag", &og_image_tag),
        ("og_article_tags", &og_article_tags),
        ("twitter_image_tag", &twitter_image_tag),
        ("rss_link_tag", &rss_link_tag),
        ("favicon_tag", &favicon_tag),
        ("json_ld", &json_ld),
        ("lang", &lang),
        ("stylesheet_path", &stylesheet_path),
        ("content", content),
    ];
    let mut out = layout_template.to_string();
    for (key, value) in pairs.iter() {
        out = out.replace(&format!("{{{{{}}}}}", key), value);
    }
    out
}

fn require_string_arg<'a>(
    args: &'a [Value],
    index: usize,
    function_name: &str,
    label: &str,
) -> Result<&'a str, Value> {
    match args.get(index) {
        Some(Value::Str(value)) => Ok(value.as_ref()),
        _ => Err(Value::Error(format!(
            "{}() requires a string argument for {}",
            function_name, label
        ))),
    }
}

fn require_number_arg(
    args: &[Value],
    index: usize,
    function_name: &str,
    label: &str,
) -> Result<f64, Value> {
    match args.get(index) {
        Some(Value::Int(n)) => Ok(*n as f64),
        Some(Value::Float(n)) => Ok(*n),
        _ => Err(Value::Error(format!("{}() {} must be a number", function_name, label))),
    }
}

fn require_non_negative_i64_arg(
    args: &[Value],
    index: usize,
    function_name: &str,
    label: &str,
) -> Result<i64, Value> {
    let value = require_number_arg(args, index, function_name, label)?;
    if !value.is_finite() {
        return Err(Value::Error(format!("{}() {} must be a finite number", function_name, label)));
    }
    if value < 0.0 {
        return Err(Value::Error(format!(
            "{}() {} must be greater than or equal to 0",
            function_name, label
        )));
    }
    if value > i64::MAX as f64 {
        return Err(Value::Error(format!("{}() {} is too large", function_name, label)));
    }
    Ok(value as i64)
}

fn ensure_generated_string_chars(function_name: &str, requested_chars: usize) -> Result<(), Value> {
    if requested_chars > runtime_limits::MAX_GENERATED_STRING_CHARS {
        return Err(Value::Error(format!(
            "{}() output would exceed maximum generated string length {}",
            function_name,
            runtime_limits::MAX_GENERATED_STRING_CHARS
        )));
    }
    Ok(())
}

pub fn handle(name: &str, args: &[Value]) -> Option<Value> {
    let result = match name {
        "len" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Int(builtins::str_len(&**s) as i64)
            } else {
                return None; // Let collections module handle other types
            }
        }

        "to_upper" | "upper" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::to_upper(&**s)))
            } else {
                Value::Error(format!("{}() requires a string argument", name))
            }
        }

        "to_lower" | "lower" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::to_lower(&**s)))
            } else {
                Value::Error(format!("{}() requires a string argument", name))
            }
        }

        "capitalize" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::capitalize(&**s)))
            } else {
                Value::Error("capitalize() requires a string argument".to_string())
            }
        }

        // Single-pass XML/HTML attribute+text escaper. Mirrors the canonical
        // five-entity escape (&, <, >, ", ') exactly, but in one native pass
        // instead of five interpreted string replacements — a hot path for SSG
        // rendering. Accepts any value and stringifies it for ergonomics.
        "escape_xml" => {
            if let Some(value) = args.first() {
                let source = match value {
                    Value::Str(s) => s.as_ref().clone(),
                    other => crate::interpreter::Interpreter::stringify_value(other),
                };
                let mut out = String::with_capacity(source.len() + 16);
                for c in source.chars() {
                    match c {
                        '&' => out.push_str("&amp;"),
                        '<' => out.push_str("&lt;"),
                        '>' => out.push_str("&gt;"),
                        '"' => out.push_str("&quot;"),
                        '\'' => out.push_str("&apos;"),
                        _ => out.push(c),
                    }
                }
                Value::Str(Arc::new(out))
            } else {
                Value::Error("escape_xml() requires one argument".to_string())
            }
        }

        // Native markdown renderer for the SSG hot path (one of the largest
        // per-page costs when interpreted). Byte-identical to the Kujo version.
        "render_markdown" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(render_markdown_native(s.as_ref())))
            } else {
                Value::Error("render_markdown() requires a string argument".to_string())
            }
        }

        // Native listing-card renderer (see render_listing_card_native above).
        // args: route, title, excerpt, featured_image, terms(array), action_label
        "render_listing_card" => {
            if args.len() != 6 {
                return Some(Value::Error("render_listing_card() expects 6 arguments".to_string()));
            }
            let s = |v: &Value| -> String {
                match v {
                    Value::Str(t) => t.as_ref().clone(),
                    other => crate::interpreter::Interpreter::stringify_value(other),
                }
            };
            let terms: Vec<String> = match &args[4] {
                Value::Array(arr) => arr.iter().map(s).collect(),
                _ => Vec::new(),
            };
            Value::Str(Arc::new(render_listing_card_native(
                &s(&args[0]),
                &s(&args[1]),
                &s(&args[2]),
                &s(&args[3]),
                &terms,
                &s(&args[5]),
            )))
        }

        // Native page-layout renderer (see render_layout_native above).
        // args: layout_template, settings, route, page_title, meta_description,
        //       navigation, content, page_meta
        "render_layout_native" => {
            if args.len() != 8 {
                return Some(Value::Error(format!(
                    "render_layout_native() expects 8 arguments, got {}",
                    args.len()
                )));
            }
            let s = |i: usize| -> &str {
                match args.get(i) {
                    Some(Value::Str(v)) => v.as_str(),
                    _ => "",
                }
            };
            Value::Str(Arc::new(render_layout_native(
                s(0),
                &args[1],
                s(2),
                s(3),
                s(4),
                s(5),
                s(6),
                &args[7],
            )))
        }

        "trim" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::trim(&**s)))
            } else {
                Value::Error("trim() requires a string argument".to_string())
            }
        }

        "trim_start" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::trim_start(&**s)))
            } else {
                Value::Error("trim_start() requires a string argument".to_string())
            }
        }

        "trim_end" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::trim_end(&**s)))
            } else {
                Value::Error("trim_end() requires a string argument".to_string())
            }
        }

        "char_at" => {
            let s = match require_string_arg(args, 0, "char_at", "value") {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            let index = match require_number_arg(args, 1, "char_at", "index") {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            Value::Str(Arc::new(builtins::char_at(s, index)))
        }

        "is_empty" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Bool(builtins::is_empty(&**s))
            } else {
                Value::Error("is_empty() requires a string argument".to_string())
            }
        }

        "count_chars" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Int(builtins::count_chars(&**s))
            } else {
                Value::Error("count_chars() requires a string argument".to_string())
            }
        }

        "contains" => {
            // Polymorphic: strings handled here, arrays delegated to collections.rs
            match args.first() {
                Some(Value::Array(_)) => return None,
                Some(Value::Dict(dict)) => match args.get(1) {
                    Some(Value::Str(key)) => Value::Bool(dict.contains_key(key.as_str())),
                    _ => Value::Error(
                        "contains() requires two arguments: string/array and substring/item"
                            .to_string(),
                    ),
                },
                Some(Value::FixedDict { keys, .. }) => match args.get(1) {
                    Some(Value::Str(key)) => {
                        Value::Bool(keys.iter().any(|existing| existing.as_ref() == key.as_str()))
                    }
                    _ => Value::Error(
                        "contains() requires two arguments: string/array and substring/item"
                            .to_string(),
                    ),
                },
                Some(Value::Str(s)) => match args.get(1) {
                    Some(Value::Str(substr)) => {
                        Value::Int(if builtins::contains(&**s, &**substr) { 1 } else { 0 })
                    }
                    _ => Value::Error(
                        "contains() requires two arguments: string/array and substring/item"
                            .to_string(),
                    ),
                },
                Some(_) => Value::Error(
                    "contains() first argument must be a string, array, or dictionary".to_string(),
                ),
                None => Value::Error(
                    "contains() requires two arguments: string/array and substring/item"
                        .to_string(),
                ),
            }
        }

        "substring" | "substr" => {
            let s = match require_string_arg(args, 0, "substring", "value") {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            let start = match require_non_negative_i64_arg(args, 1, "substring", "start") {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            let end = match require_non_negative_i64_arg(args, 2, "substring", "end") {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            if end < start {
                Value::Error("substring() end must be greater than or equal to start".to_string())
            } else {
                Value::Str(Arc::new(builtins::substring(s, start as f64, end as f64)))
            }
        }

        "replace_str" | "replace" => {
            if let (Some(Value::Str(s)), Some(Value::Str(old)), Some(Value::Str(new))) =
                (args.first(), args.get(1), args.get(2))
            {
                Value::Str(Arc::new(builtins::replace(&**s, &**old, &**new)))
            } else {
                Value::Error(
                    "replace() requires string, from-string, and to-string arguments".to_string(),
                )
            }
        }

        "starts_with" => {
            if let (Some(Value::Str(s)), Some(Value::Str(prefix))) = (args.first(), args.get(1)) {
                Value::Bool(builtins::starts_with(&**s, &**prefix))
            } else {
                Value::Error(
                    "starts_with() requires string and prefix string arguments".to_string(),
                )
            }
        }

        "ends_with" => {
            if let (Some(Value::Str(s)), Some(Value::Str(suffix))) = (args.first(), args.get(1)) {
                Value::Bool(builtins::ends_with(&**s, &**suffix))
            } else {
                Value::Error("ends_with() requires string and suffix string arguments".to_string())
            }
        }

        "index_of" => {
            // Polymorphic: strings handled here, arrays delegated to collections.rs
            match args.first() {
                Some(Value::Array(_)) => return None,
                Some(Value::Str(s)) => match args.get(1) {
                    Some(Value::Str(substr)) => {
                        Value::Int(builtins::index_of(&**s, &**substr) as i64)
                    }
                    _ => Value::Error(
                        "index_of() requires two arguments: string/array and substring/item"
                            .to_string(),
                    ),
                },
                Some(_) => {
                    Value::Error("index_of() first argument must be a string or array".to_string())
                }
                None => Value::Error(
                    "index_of() requires two arguments: string/array and substring/item"
                        .to_string(),
                ),
            }
        }

        "repeat" => {
            let s = match require_string_arg(args, 0, "repeat", "value") {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            let count = match require_non_negative_i64_arg(args, 1, "repeat", "count") {
                Ok(value) => value,
                Err(error) => return Some(error),
            };
            let requested_chars = s.chars().count().saturating_mul(count as usize);
            if let Err(error) = ensure_generated_string_chars("repeat", requested_chars) {
                return Some(error);
            }
            Value::Str(Arc::new(builtins::repeat(s, count as f64)))
        }

        "split" => {
            if let (Some(Value::Str(s)), Some(Value::Str(delimiter))) = (args.first(), args.get(1))
            {
                let parts = builtins::split(&**s, &**delimiter);
                let values: Vec<Value> =
                    parts.into_iter().map(|s| Value::Str(Arc::new(s))).collect();
                Value::Array(Arc::new(values))
            } else {
                Value::Error("split() requires string and delimiter string arguments".to_string())
            }
        }

        "regex_match" => {
            if let (Some(Value::Str(text)), Some(Value::Str(pattern))) = (args.first(), args.get(1))
            {
                Value::Bool(builtins::regex_match(text.as_ref(), pattern.as_ref()))
            } else {
                Value::Error(
                    "regex_match requires two string arguments (text, pattern)".to_string(),
                )
            }
        }

        "regex_find_all" => {
            if let (Some(Value::Str(text)), Some(Value::Str(pattern))) = (args.first(), args.get(1))
            {
                let matches = builtins::regex_find_all(text.as_ref(), pattern.as_ref());
                let values: Vec<Value> =
                    matches.into_iter().map(|s| Value::Str(Arc::new(s))).collect();
                Value::Array(Arc::new(values))
            } else {
                Value::Error(
                    "regex_find_all requires two string arguments (text, pattern)".to_string(),
                )
            }
        }

        "regex_replace" => {
            if let (
                Some(Value::Str(text)),
                Some(Value::Str(pattern)),
                Some(Value::Str(replacement)),
            ) = (args.first(), args.get(1), args.get(2))
            {
                Value::Str(Arc::new(builtins::regex_replace(
                    text.as_ref(),
                    pattern.as_ref(),
                    replacement.as_ref(),
                )))
            } else {
                Value::Error(
                    "regex_replace requires three string arguments (text, pattern, replacement)"
                        .to_string(),
                )
            }
        }

        "regex_split" => {
            if let (Some(Value::Str(text)), Some(Value::Str(pattern))) = (args.first(), args.get(1))
            {
                let parts = builtins::regex_split(text.as_ref(), pattern.as_ref());
                let values: Vec<Value> =
                    parts.into_iter().map(|s| Value::Str(Arc::new(s))).collect();
                Value::Array(Arc::new(values))
            } else {
                Value::Error(
                    "regex_split requires two string arguments (text, pattern)".to_string(),
                )
            }
        }

        "join" => {
            if let (Some(Value::Array(arr)), Some(Value::Str(separator))) =
                (args.first(), args.get(1))
            {
                // Convert array elements to strings
                let strings: Vec<String> = arr
                    .iter()
                    .map(|v| match v {
                        Value::Str(s) => (&**s).to_string(),
                        Value::Int(n) => n.to_string(),
                        Value::Float(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => format!("{:?}", v),
                    })
                    .collect();
                Value::Str(Arc::new(builtins::join(&strings, &**separator)))
            } else {
                Value::Error("join() requires array and separator string arguments".to_string())
            }
        }

        "ssg_render_pages" => {
            // ssg_render_pages(source_pages: Array<String>) -> Dict
            // Returns { "pages": Array<String>, "checksum": Int }
            if args.len() != 1 {
                Value::Error(format!(
                    "ssg_render_pages() expects 1 argument (array of source pages), got {}",
                    args.len()
                ))
            } else {
                match args.first() {
                    Some(Value::Array(source_pages)) => {
                        let mut rendered_pages = Vec::with_capacity(source_pages.len());
                        let mut checksum: i64 = 0;

                        for (index, page) in source_pages.iter().enumerate() {
                            let source_body = match page {
                                Value::Str(body) => body,
                                _ => {
                                    return Some(Value::Error(format!(
                                        "ssg_render_pages() source page at index {} must be a string",
                                        index
                                    )));
                                }
                            };

                            let index_str = index.to_string();
                            let mut html = String::with_capacity(source_body.len() + 64);
                            html.push_str("<html><body><h1>Post ");
                            html.push_str(index_str.as_str());
                            html.push_str("</h1><article>");
                            html.push_str(&lc_escape(source_body.as_ref()));
                            html.push_str("</article></body></html>");

                            checksum += html.len() as i64;
                            rendered_pages.push(Value::Str(Arc::new(html)));
                        }

                        let mut result = DictMap::default();
                        result.insert("pages".into(), Value::Array(Arc::new(rendered_pages)));
                        result.insert("checksum".into(), Value::Int(checksum));
                        Value::Dict(Arc::new(result))
                    }
                    _ => Value::Error(
                        "ssg_render_pages() requires an array of source page strings".to_string(),
                    ),
                }
            }
        }

        "ssg_build_output_paths" => {
            // ssg_build_output_paths(output_dir: String, file_count: Int, extension?: String)
            //   -> Array<String>
            let args_len = args.len();
            if args_len != 2 && args_len != 3 {
                Value::Error(format!(
                    "ssg_build_output_paths() expects 2 or 3 arguments (output_dir, file_count, optional extension), got {}",
                    args_len
                ))
            } else {
                let output_dir = match args.first() {
                    Some(Value::Str(dir)) => dir,
                    _ => {
                        return Some(Value::Error(
                            "ssg_build_output_paths() output_dir must be a string".to_string(),
                        ));
                    }
                };

                let file_count = match args.get(1) {
                    Some(Value::Int(n)) if *n >= 0 => {
                        let count = usize::try_from(*n).unwrap_or(usize::MAX);
                        if count > runtime_limits::MAX_GENERATED_SEQUENCE_ITEMS {
                            return Some(Value::Error(format!(
                                "ssg_build_output_paths() would generate {} items, exceeding maximum generated sequence length {}",
                                n,
                                runtime_limits::MAX_GENERATED_SEQUENCE_ITEMS
                            )));
                        }
                        count
                    }
                    Some(Value::Int(n)) => {
                        return Some(Value::Error(format!(
                            "ssg_build_output_paths() file_count must be >= 0, got {}",
                            n
                        )));
                    }
                    _ => {
                        return Some(Value::Error(
                            "ssg_build_output_paths() file_count must be an integer".to_string(),
                        ));
                    }
                };

                let extension = match args.get(2) {
                    Some(Value::Str(ext)) => ext.as_ref().clone(),
                    Some(_) => {
                        return Some(Value::Error(
                            "ssg_build_output_paths() optional extension must be a string"
                                .to_string(),
                        ));
                    }
                    None => ".html".to_string(),
                };

                let mut output_paths = Vec::with_capacity(file_count);
                for index in 0..file_count {
                    let index_str = index.to_string();
                    let mut output_path =
                        String::with_capacity(output_dir.len() + extension.len() + 24);
                    output_path.push_str(output_dir.as_ref());
                    output_path.push_str("/post_");
                    output_path.push_str(index_str.as_str());
                    output_path.push_str(extension.as_str());
                    output_paths.push(Value::Str(Arc::new(output_path)));
                }

                Value::Array(Arc::new(output_paths))
            }
        }

        "pad_left" | "pad_start" => {
            if let (Some(Value::Str(s)), Some(width_val), Some(Value::Str(pad_char))) =
                (args.first(), args.get(1), args.get(2))
            {
                let width = match width_val {
                    Value::Int(n) => *n,
                    Value::Float(n) if n.is_finite() => *n as i64,
                    Value::Float(_) => {
                        return Some(Value::Error(format!("{}() width must be finite", name)))
                    }
                    _ => return Some(Value::Error(format!("{}() width must be a number", name))),
                };
                if width < 0 {
                    return Some(Value::Error(format!(
                        "{}() width must be greater than or equal to 0",
                        name
                    )));
                }
                if let Err(error) = ensure_generated_string_chars(name, width as usize) {
                    return Some(error);
                }
                Value::Str(Arc::new(builtins::str_pad_left(&**s, width, &**pad_char)))
            } else {
                Value::Error(format!("{}() requires 3 arguments: string, width, char", name))
            }
        }

        "pad_right" | "pad_end" => {
            if let (Some(Value::Str(s)), Some(width_val), Some(Value::Str(pad_char))) =
                (args.first(), args.get(1), args.get(2))
            {
                let width = match width_val {
                    Value::Int(n) => *n,
                    Value::Float(n) if n.is_finite() => *n as i64,
                    Value::Float(_) => {
                        return Some(Value::Error(format!("{}() width must be finite", name)))
                    }
                    _ => return Some(Value::Error(format!("{}() width must be a number", name))),
                };
                if width < 0 {
                    return Some(Value::Error(format!(
                        "{}() width must be greater than or equal to 0",
                        name
                    )));
                }
                if let Err(error) = ensure_generated_string_chars(name, width as usize) {
                    return Some(error);
                }
                Value::Str(Arc::new(builtins::str_pad_right(&**s, width, &**pad_char)))
            } else {
                Value::Error(format!("{}() requires 3 arguments: string, width, char", name))
            }
        }

        "lines" => {
            if let Some(Value::Str(s)) = args.first() {
                let lines = builtins::str_lines(&**s);
                Value::Array(Arc::new(lines.into_iter().map(|s| Value::Str(Arc::new(s))).collect()))
            } else {
                Value::Error("lines() requires a string argument".to_string())
            }
        }

        "words" => {
            if let Some(Value::Str(s)) = args.first() {
                let words = builtins::str_words(&**s);
                Value::Array(Arc::new(words.into_iter().map(|s| Value::Str(Arc::new(s))).collect()))
            } else {
                Value::Error("words() requires a string argument".to_string())
            }
        }

        "str_reverse" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::str_reverse(&**s)))
            } else {
                Value::Error("str_reverse() requires a string argument".to_string())
            }
        }

        "slugify" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::str_slugify(&**s)))
            } else {
                Value::Error("slugify() requires a string argument".to_string())
            }
        }

        "truncate" => {
            if let (Some(Value::Str(s)), Some(len_val), Some(Value::Str(suffix))) =
                (args.first(), args.get(1), args.get(2))
            {
                let max_len = match len_val {
                    Value::Int(n) => *n,
                    Value::Float(n) if n.is_finite() => *n as i64,
                    Value::Float(_) => {
                        return Some(Value::Error("truncate() length must be finite".to_string()))
                    }
                    _ => {
                        return Some(Value::Error("truncate() length must be a number".to_string()))
                    }
                };
                if max_len < 0 {
                    return Some(Value::Error(
                        "truncate() length must be greater than or equal to 0".to_string(),
                    ));
                }
                Value::Str(Arc::new(builtins::str_truncate(&**s, max_len, &**suffix)))
            } else {
                Value::Error("truncate() requires 3 arguments: string, length, suffix".to_string())
            }
        }

        "to_camel_case" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::str_to_camel_case(&**s)))
            } else {
                Value::Error("to_camel_case() requires a string argument".to_string())
            }
        }

        "to_snake_case" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::str_to_snake_case(&**s)))
            } else {
                Value::Error("to_snake_case() requires a string argument".to_string())
            }
        }

        "to_kebab_case" => {
            if let Some(Value::Str(s)) = args.first() {
                Value::Str(Arc::new(builtins::str_to_kebab_case(&**s)))
            } else {
                Value::Error("to_kebab_case() requires a string argument".to_string())
            }
        }

        _ => return None, // Not a string function
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn str_value(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    #[test]
    fn test_ssg_render_pages_success_returns_pages_and_checksum() {
        let args = vec![Value::Array(Arc::new(vec![
            str_value("# Post 0\n\nGenerated page 0"),
            str_value("# Post 1\n\nGenerated page 1"),
        ]))];

        let result = handle("ssg_render_pages", &args).unwrap();
        match result {
            Value::Dict(dict) => {
                let pages = dict.get("pages").expect("pages key missing");
                let checksum = dict.get("checksum").expect("checksum key missing");

                match pages {
                    Value::Array(values) => {
                        assert_eq!(values.len(), 2);
                        assert!(
                            matches!(&values[0], Value::Str(s) if s.as_ref() == "<html><body><h1>Post 0</h1><article># Post 0\n\nGenerated page 0</article></body></html>")
                        );
                        assert!(
                            matches!(&values[1], Value::Str(s) if s.as_ref() == "<html><body><h1>Post 1</h1><article># Post 1\n\nGenerated page 1</article></body></html>")
                        );
                    }
                    _ => panic!("Expected pages to be an Array"),
                }

                let expected_checksum = "<html><body><h1>Post 0</h1><article># Post 0\n\nGenerated page 0</article></body></html>".len() as i64
                    + "<html><body><h1>Post 1</h1><article># Post 1\n\nGenerated page 1</article></body></html>".len() as i64;

                match checksum {
                    Value::Int(value) => assert_eq!(*value, expected_checksum),
                    _ => panic!("Expected checksum to be an Int"),
                }
            }
            _ => panic!("Expected Dict result from ssg_render_pages"),
        }
    }

    #[test]
    fn test_ssg_render_pages_escapes_source_html() {
        let args = vec![Value::Array(Arc::new(vec![str_value("<script>alert(1)</script>")]))];

        let result = handle("ssg_render_pages", &args).unwrap();
        match result {
            Value::Dict(dict) => match dict.get("pages") {
                Some(Value::Array(values)) => {
                    assert!(
                        matches!(&values[0], Value::Str(s) if s.contains("&lt;script&gt;alert(1)&lt;/script&gt;"))
                    );
                    assert!(
                        matches!(&values[0], Value::Str(s) if !s.contains("<script>")),
                        "rendered page should not contain raw script"
                    );
                }
                other => panic!("Expected pages array, got {:?}", other),
            },
            other => panic!("Expected Dict result from ssg_render_pages, got {:?}", other),
        }
    }

    #[test]
    fn test_ssg_render_pages_requires_array_argument() {
        let args = vec![str_value("not-an-array")];
        let result = handle("ssg_render_pages", &args).unwrap();

        match result {
            Value::Error(message) => {
                assert!(message.contains("requires an array of source page strings"));
            }
            _ => panic!("Expected Value::Error for non-array input"),
        }
    }

    #[test]
    fn test_ssg_build_output_paths_success_default_extension() {
        let result =
            handle("ssg_build_output_paths", &[str_value("tmp/out"), Value::Int(3)]).unwrap();

        match result {
            Value::Array(paths) => {
                assert_eq!(paths.len(), 3);
                assert!(
                    matches!(&paths[0], Value::Str(path) if path.as_ref() == "tmp/out/post_0.html")
                );
                assert!(
                    matches!(&paths[1], Value::Str(path) if path.as_ref() == "tmp/out/post_1.html")
                );
                assert!(
                    matches!(&paths[2], Value::Str(path) if path.as_ref() == "tmp/out/post_2.html")
                );
            }
            _ => panic!("Expected Array result from ssg_build_output_paths"),
        }
    }

    #[test]
    fn test_ssg_build_output_paths_success_custom_extension() {
        let result = handle(
            "ssg_build_output_paths",
            &[str_value("tmp/out"), Value::Int(2), str_value(".txt")],
        )
        .unwrap();

        match result {
            Value::Array(paths) => {
                assert_eq!(paths.len(), 2);
                assert!(
                    matches!(&paths[0], Value::Str(path) if path.as_ref() == "tmp/out/post_0.txt")
                );
                assert!(
                    matches!(&paths[1], Value::Str(path) if path.as_ref() == "tmp/out/post_1.txt")
                );
            }
            _ => panic!("Expected Array result from ssg_build_output_paths"),
        }
    }

    #[test]
    fn test_ssg_build_output_paths_validates_argument_contracts() {
        let wrong_arity = handle("ssg_build_output_paths", &[str_value("tmp/out")]).unwrap();
        assert!(
            matches!(wrong_arity, Value::Error(message) if message.contains("expects 2 or 3 arguments"))
        );

        let bad_dir = handle("ssg_build_output_paths", &[Value::Int(1), Value::Int(2)]).unwrap();
        assert!(
            matches!(bad_dir, Value::Error(message) if message.contains("output_dir must be a string"))
        );

        let bad_count =
            handle("ssg_build_output_paths", &[str_value("tmp/out"), Value::Int(-1)]).unwrap();
        assert!(
            matches!(bad_count, Value::Error(message) if message.contains("file_count must be >= 0"))
        );

        let bad_count_type =
            handle("ssg_build_output_paths", &[str_value("tmp/out"), str_value("2")]).unwrap();
        assert!(
            matches!(bad_count_type, Value::Error(message) if message.contains("file_count must be an integer"))
        );

        let oversized_count =
            handle("ssg_build_output_paths", &[str_value("tmp/out"), Value::Int(i64::MAX)])
                .unwrap();
        assert!(
            matches!(oversized_count, Value::Error(message) if message.contains("maximum generated sequence length"))
        );

        let bad_extension =
            handle("ssg_build_output_paths", &[str_value("tmp/out"), Value::Int(2), Value::Int(1)])
                .unwrap();
        assert!(
            matches!(bad_extension, Value::Error(message) if message.contains("optional extension must be a string"))
        );
    }

    #[test]
    fn test_regex_match_and_find_all() {
        let match_result =
            handle("regex_match", &[str_value("hello123"), str_value("^[a-z]+\\d+$")]).unwrap();
        assert!(matches!(match_result, Value::Bool(true)));

        let find_all_result =
            handle("regex_find_all", &[str_value("a1 b22 c333"), str_value("\\d+")]).unwrap();

        match find_all_result {
            Value::Array(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(&values[0], Value::Str(s) if s.as_ref() == "1"));
                assert!(matches!(&values[1], Value::Str(s) if s.as_ref() == "22"));
                assert!(matches!(&values[2], Value::Str(s) if s.as_ref() == "333"));
            }
            _ => panic!("Expected Value::Array from regex_find_all"),
        }
    }

    #[test]
    fn test_regex_replace_and_split() {
        let replace_result =
            handle("regex_replace", &[str_value("a1 b22"), str_value("\\d+"), str_value("#")])
                .unwrap();
        assert!(matches!(&replace_result, Value::Str(s) if s.as_ref() == "a# b#"));

        let split_result =
            handle("regex_split", &[str_value("a, b; c"), str_value("[,;]\\s*")]).unwrap();

        match split_result {
            Value::Array(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(&values[0], Value::Str(s) if s.as_ref() == "a"));
                assert!(matches!(&values[1], Value::Str(s) if s.as_ref() == "b"));
                assert!(matches!(&values[2], Value::Str(s) if s.as_ref() == "c"));
            }
            _ => panic!("Expected Value::Array from regex_split"),
        }
    }

    #[test]
    fn test_regex_argument_validation_errors() {
        let match_error = handle("regex_match", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(match_error, Value::Error(message) if message.contains("regex_match requires two string arguments"))
        );

        let replace_error = handle("regex_replace", &[str_value("a"), str_value("b")]).unwrap();
        assert!(
            matches!(replace_error, Value::Error(message) if message.contains("regex_replace requires three string arguments"))
        );
    }

    #[test]
    fn test_render_markdown_escapes_html_and_unsafe_link_targets() {
        let rendered = handle(
            "render_markdown",
            &[str_value(
                "# Hi <script>alert(1)</script>\n[click](javascript:alert(1))\n[q](https://example.test?a=1&b=2)",
            )],
        )
        .unwrap();

        match rendered {
            Value::Str(html) => {
                assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
                assert!(!html.contains("<script>"));
                assert!(html.contains("<a href=\"#\">click</a>"));
                assert!(!html.contains("javascript:alert"));
                assert!(html.contains("<a href=\"https://example.test?a=1&amp;b=2\">q</a>"));
                assert!(!html.contains("&amp;amp;"));
            }
            other => panic!("Expected rendered markdown string, got {:?}", other),
        }
    }

    #[test]
    fn test_render_markdown_renders_pipe_tables() {
        let rendered = handle(
            "render_markdown",
            &[str_value(
                "| Path | Start with |\n| --- | :---: |\n| Learn | [Kujo](/learn/) |\n| Build | `kujo run` |",
            )],
        )
        .unwrap();

        match rendered {
            Value::Str(html) => {
                assert!(html.contains("<table><thead><tr>"));
                assert!(html.contains("<th>Path</th>"));
                assert!(html.contains("<td><a href=\"/learn/\">Kujo</a></td>"));
                assert!(html.contains("<td><code>kujo run</code></td>"));
                assert!(html.ends_with("</tbody></table>"));
            }
            other => panic!("Expected rendered markdown table string, got {:?}", other),
        }
    }

    #[test]
    fn test_contains_and_index_of_string_behavior() {
        let contains_true =
            handle("contains", &[str_value("kujo language"), str_value("lang")]).unwrap();
        assert!(matches!(contains_true, Value::Int(1)));

        let contains_false =
            handle("contains", &[str_value("kujo language"), str_value("python")]).unwrap();
        assert!(matches!(contains_false, Value::Int(0)));

        let index_found = handle("index_of", &[str_value("abcabc"), str_value("ca")]).unwrap();
        assert!(matches!(index_found, Value::Int(2)));

        let index_missing = handle("index_of", &[str_value("abcabc"), str_value("zz")]).unwrap();
        assert!(matches!(index_missing, Value::Int(-1)));
    }

    #[test]
    fn test_contains_supports_dictionary_keys() {
        let mut dict = DictMap::default();
        dict.insert(Arc::<str>::from("alpha"), Value::Int(1));
        dict.insert(Arc::<str>::from("beta"), Value::Int(2));

        let contains_dict =
            handle("contains", &[Value::Dict(Arc::new(dict)), str_value("beta")]).unwrap();
        assert!(matches!(contains_dict, Value::Bool(true)));

        let fixed_dict = Value::FixedDict {
            keys: Arc::new(vec![Arc::<str>::from("alpha"), Arc::<str>::from("beta")]),
            values: vec![Value::Int(1), Value::Int(2)],
        };
        let contains_fixed = handle("contains", &[fixed_dict.clone(), str_value("alpha")]).unwrap();
        assert!(matches!(contains_fixed, Value::Bool(true)));

        let missing_fixed = handle("contains", &[fixed_dict, str_value("gamma")]).unwrap();
        assert!(matches!(missing_fixed, Value::Bool(false)));
    }

    #[test]
    fn test_substr_alias_matches_substring_behavior() {
        let substring =
            handle("substring", &[str_value("kujolang"), Value::Int(1), Value::Int(4)]).unwrap();
        let substr = handle("substr", &[str_value("kujolang"), Value::Int(1), Value::Int(4)])
            .expect("substr alias should be available");

        assert!(matches!(&substring, Value::Str(value) if value.as_ref() == "ujo"));
        assert!(matches!(&substr, Value::Str(value) if value.as_ref() == "ujo"));
    }

    #[test]
    fn test_contains_and_index_of_argument_shape_errors() {
        let contains_missing = handle("contains", &[str_value("kujo")]).unwrap();
        assert!(
            matches!(contains_missing, Value::Error(message) if message.contains("contains() requires two arguments"))
        );

        let index_missing = handle("index_of", &[str_value("kujo")]).unwrap();
        assert!(
            matches!(index_missing, Value::Error(message) if message.contains("index_of() requires two arguments"))
        );

        let contains_invalid_type = handle("contains", &[Value::Int(1), str_value("x")]).unwrap();
        assert!(
            matches!(contains_invalid_type, Value::Error(message) if message.contains("first argument must be a string, array, or dictionary"))
        );

        let index_invalid_type = handle("index_of", &[Value::Bool(true), str_value("x")]).unwrap();
        assert!(
            matches!(index_invalid_type, Value::Error(message) if message.contains("first argument must be a string or array"))
        );
    }

    #[test]
    fn test_contains_and_index_of_delegate_array_case_to_collections() {
        let array_args =
            [Value::Array(Arc::new(vec![Value::Int(1), Value::Int(2)])), Value::Int(2)];
        assert!(handle("contains", &array_args).is_none());
        assert!(handle("index_of", &array_args).is_none());
    }

    #[test]
    fn test_core_string_helpers_reject_wrong_types_instead_of_silent_fallbacks() {
        let upper_wrong_type = handle("to_upper", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(upper_wrong_type, Value::Error(message) if message.contains("to_upper() requires a string argument"))
        );

        let char_at_wrong_index =
            handle("char_at", &[str_value("kujo"), Value::Bool(true)]).unwrap();
        assert!(
            matches!(char_at_wrong_index, Value::Error(message) if message.contains("char_at() index must be a number"))
        );

        let substring_wrong_bound = handle(
            "substring",
            &[str_value("kujo"), Value::Int(1), Value::Str("x".to_string().into())],
        )
        .unwrap();
        assert!(
            matches!(substring_wrong_bound, Value::Error(message) if message.contains("substring() end must be a number"))
        );

        let split_wrong_delimiter = handle("split", &[str_value("a,b"), Value::Int(1)]).unwrap();
        assert!(
            matches!(split_wrong_delimiter, Value::Error(message) if message.contains("split() requires string and delimiter string arguments"))
        );

        let repeat_wrong_count = handle("repeat", &[str_value("ha"), str_value("3")]).unwrap();
        assert!(
            matches!(repeat_wrong_count, Value::Error(message) if message.contains("repeat() count must be a number"))
        );

        let count_chars_wrong_type = handle("count_chars", &[Value::Null]).unwrap();
        assert!(
            matches!(count_chars_wrong_type, Value::Error(message) if message.contains("count_chars() requires a string argument"))
        );
    }

    #[test]
    fn test_core_string_helpers_reject_invalid_lengths_without_panicking() {
        let substring_negative =
            handle("substring", &[str_value("kujo"), Value::Int(-1), Value::Int(2)]).unwrap();
        assert!(
            matches!(substring_negative, Value::Error(message) if message.contains("substring() start must be greater than or equal to 0"))
        );

        let substring_reversed =
            handle("substring", &[str_value("kujo"), Value::Int(3), Value::Int(1)]).unwrap();
        assert!(
            matches!(substring_reversed, Value::Error(message) if message.contains("substring() end must be greater than or equal to start"))
        );

        let repeat_negative = handle("repeat", &[str_value("kujo"), Value::Int(-1)]).unwrap();
        assert!(
            matches!(repeat_negative, Value::Error(message) if message.contains("repeat() count must be greater than or equal to 0"))
        );

        let repeat_too_large = handle(
            "repeat",
            &[str_value("x"), Value::Int(runtime_limits::MAX_GENERATED_STRING_CHARS as i64 + 1)],
        )
        .unwrap();
        assert!(
            matches!(repeat_too_large, Value::Error(message) if message.contains("repeat() output would exceed maximum generated string length"))
        );

        let pad_start_negative =
            handle("pad_start", &[str_value("kujo"), Value::Int(-1), str_value("0")]).unwrap();
        assert!(
            matches!(pad_start_negative, Value::Error(message) if message.contains("pad_start() width must be greater than or equal to 0"))
        );

        let pad_end_too_large = handle(
            "pad_end",
            &[
                str_value("kujo"),
                Value::Int(runtime_limits::MAX_GENERATED_STRING_CHARS as i64 + 1),
                str_value("."),
            ],
        )
        .unwrap();
        assert!(
            matches!(pad_end_too_large, Value::Error(message) if message.contains("pad_end() output would exceed maximum generated string length"))
        );

        let truncate_negative =
            handle("truncate", &[str_value("kujo"), Value::Int(-1), str_value("...")]).unwrap();
        assert!(
            matches!(truncate_negative, Value::Error(message) if message.contains("truncate() length must be greater than or equal to 0"))
        );
    }

    #[test]
    fn test_ssg_render_pages_requires_string_elements() {
        let args = vec![Value::Array(Arc::new(vec![Value::Int(1)]))];
        let result = handle("ssg_render_pages", &args).unwrap();

        match result {
            Value::Error(message) => {
                assert!(message.contains("source page at index 0 must be a string"));
            }
            _ => panic!("Expected Value::Error for non-string source page"),
        }
    }

    #[test]
    fn test_ssg_render_pages_validates_argument_count() {
        let args = vec![Value::Array(Arc::new(vec![])), Value::Array(Arc::new(vec![]))];
        let result = handle("ssg_render_pages", &args).unwrap();

        match result {
            Value::Error(message) => {
                assert!(message.contains("expects 1 argument"));
            }
            _ => panic!("Expected Value::Error for invalid argument count"),
        }
    }
}
