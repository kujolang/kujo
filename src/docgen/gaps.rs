use crate::docgen::model::{DocGap, DocGapKind, DocProject, DocSymbol, DocVisibility};
#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkValidationOptions {
    pub validate_local_anchors: bool,
    pub validate_external_links: bool,
    pub external_link_timeout_ms: u64,
    pub external_link_allowlist: BTreeSet<String>,
    pub allow_private_network_links: bool,
    pub max_link_checks: Option<usize>,
    pub max_external_link_checks: Option<usize>,
    pub max_total_validation_time_ms: Option<u64>,
}

impl Default for LinkValidationOptions {
    fn default() -> Self {
        Self {
            validate_local_anchors: false,
            validate_external_links: false,
            external_link_timeout_ms: 1500,
            external_link_allowlist: BTreeSet::new(),
            allow_private_network_links: false,
            max_link_checks: None,
            max_external_link_checks: None,
            max_total_validation_time_ms: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkValidationBudgetSkipCounts {
    pub max_link_checks: usize,
    pub max_external_checks: usize,
    pub max_total_time: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkValidationReport {
    pub broken_links: Vec<BrokenLinkFinding>,
    pub skip_counts: LinkValidationBudgetSkipCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokenLinkKind {
    LocalFileMissing,
    LocalAnchorMissing,
    ExternalUnreachable,
    ExternalRedirectDisallowed,
    ExternalPrivateAddressBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLinkFinding {
    pub symbol: String,
    pub target: String,
    pub line: usize,
    pub kind: BrokenLinkKind,
}

#[cfg(test)]
static LOCAL_ANCHOR_FILE_READ_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static EXTERNAL_HTTP_CLIENT_BUILD_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
thread_local! {
    static CALL_SITE_INDEX_LINE_SCAN_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
fn increment_local_anchor_file_read_count() {
    LOCAL_ANCHOR_FILE_READ_COUNT.fetch_add(1, Ordering::Relaxed);
}

#[cfg(not(test))]
fn increment_local_anchor_file_read_count() {}

#[cfg(test)]
fn increment_external_http_client_build_count() {
    EXTERNAL_HTTP_CLIENT_BUILD_COUNT.fetch_add(1, Ordering::Relaxed);
}

#[cfg(not(test))]
fn increment_external_http_client_build_count() {}

#[cfg(test)]
fn increment_call_site_index_line_scan_count() {
    CALL_SITE_INDEX_LINE_SCAN_COUNT.with(|count| count.set(count.get() + 1));
}

#[cfg(not(test))]
fn increment_call_site_index_line_scan_count() {}

#[cfg(test)]
fn reset_link_validation_test_counters() {
    LOCAL_ANCHOR_FILE_READ_COUNT.store(0, Ordering::Relaxed);
    EXTERNAL_HTTP_CLIENT_BUILD_COUNT.store(0, Ordering::Relaxed);
}

#[cfg(test)]
fn link_validation_test_counters() -> (usize, usize) {
    (
        LOCAL_ANCHOR_FILE_READ_COUNT.load(Ordering::Relaxed),
        EXTERNAL_HTTP_CLIENT_BUILD_COUNT.load(Ordering::Relaxed),
    )
}

#[cfg(test)]
fn reset_call_site_index_test_counter() {
    CALL_SITE_INDEX_LINE_SCAN_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn call_site_index_test_counter() -> usize {
    CALL_SITE_INDEX_LINE_SCAN_COUNT.with(Cell::get)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct AnchorValidationIndex {
    html_id_values: BTreeSet<String>,
    html_name_values: BTreeSet<String>,
    markdown_heading_slugs: BTreeSet<String>,
}

impl AnchorValidationIndex {
    fn contains_normalized_anchor(&self, anchor: &str) -> bool {
        self.html_id_values.contains(anchor)
            || self.html_name_values.contains(anchor)
            || self.markdown_heading_slugs.contains(anchor)
    }
}

pub fn build_gaps(project: &mut DocProject, source_map: &BTreeMap<String, String>) {
    let call_site_index = build_known_call_site_index(
        source_map,
        project.symbols.iter().map(|symbol| symbol.name.as_str()),
        6,
    );
    let mut gaps = Vec::new();

    for symbol in &project.symbols {
        if symbol.visibility == DocVisibility::Private {
            continue;
        }

        if symbol.gaps.is_empty() {
            continue;
        }

        let key = symbol.source_path.display().to_string();
        let context = source_map
            .get(&key)
            .map(|source| bounded_context(source, symbol.line, 2))
            .unwrap_or_default();

        let call_sites = call_site_index.get(&symbol.name).cloned().unwrap_or_default();

        gaps.push(DocGap {
            id: format!("gap:{}:{}:{}", symbol.language, symbol.qualified_name, symbol.line),
            language: symbol.language.clone(),
            symbol_id: symbol.id.clone(),
            symbol_name: symbol.qualified_name.clone(),
            symbol_kind: symbol.kind.clone(),
            signature: symbol.signature.clone(),
            source_path: symbol.source_path.clone(),
            line: symbol.line,
            missing_sections: symbol.gaps.clone(),
            existing_docs: symbol.docs.lines.clone(),
            bounded_source_context: context,
            known_call_sites: call_sites,
            suggested_ai_prompt: ai_prompt(symbol),
        });
    }

    gaps.sort_by(|a, b| {
        a.language
            .cmp(&b.language)
            .then(a.source_path.cmp(&b.source_path))
            .then(a.line.cmp(&b.line))
            .then(a.symbol_name.cmp(&b.symbol_name))
    });

    project.gaps = gaps;
}

fn bounded_context(source: &str, line: usize, radius: usize) -> Vec<String> {
    if line == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let start = line.saturating_sub(radius + 1) + 1;
    let end = line + radius;

    for (idx, text) in source.lines().enumerate() {
        let line_no = idx + 1;
        if line_no < start || line_no > end {
            continue;
        }
        out.push(format!("{}: {}", line_no, text));
    }

    out
}

fn build_known_call_site_index<'a, I>(
    source_map: &BTreeMap<String, String>,
    symbol_names: I,
    limit: usize,
) -> BTreeMap<String, Vec<String>>
where
    I: IntoIterator<Item = &'a str>,
{
    if limit == 0 {
        return BTreeMap::new();
    }

    let unique_names: Vec<String> = symbol_names
        .into_iter()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    if unique_names.is_empty() {
        return BTreeMap::new();
    }

    let patterns = unique_names
        .iter()
        .map(|name| format!(r"{}\(", regex::escape(name)))
        .collect::<Vec<String>>();
    let Ok(regex_set) = regex::RegexSet::new(patterns) else {
        return BTreeMap::new();
    };

    let mut calls_by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (path, source) in source_map {
        for (idx, line) in source.lines().enumerate() {
            increment_call_site_index_line_scan_count();
            let matches = regex_set.matches(line);
            if matches.matched_any() {
                for matched_idx in matches.iter() {
                    let symbol_name = &unique_names[matched_idx];
                    let calls = calls_by_name.entry(symbol_name.clone()).or_default();
                    if calls.len() < limit {
                        calls.push(format!("{}:{}: {}", path, idx + 1, line.trim()));
                    }
                }
            }
        }
    }

    calls_by_name
}

fn ai_prompt(symbol: &DocSymbol) -> String {
    let missing = if symbol.gaps.is_empty() {
        "none".to_string()
    } else {
        symbol.gaps.iter().map(gap_name).collect::<Vec<&'static str>>().join(", ")
    };

    format!(
        "Document this {} '{}' with missing sections: {}. Use only the provided source context. Do not invent behavior. Mark uncertainty. Keep docs concise. Prefer examples only when the source supports them.",
        kind_name(&symbol.kind),
        symbol.qualified_name,
        missing
    )
}

fn gap_name(kind: &DocGapKind) -> &'static str {
    match kind {
        DocGapKind::MissingSummary => "summary",
        DocGapKind::MissingExamples => "examples",
        DocGapKind::MissingDocs => "docs",
    }
}

fn kind_name(kind: &crate::docgen::model::DocSymbolKind) -> &'static str {
    match kind {
        crate::docgen::model::DocSymbolKind::Module => "module",
        crate::docgen::model::DocSymbolKind::Function => "function",
        crate::docgen::model::DocSymbolKind::Method => "method",
        crate::docgen::model::DocSymbolKind::Class => "class",
        crate::docgen::model::DocSymbolKind::Struct => "struct",
        crate::docgen::model::DocSymbolKind::Enum => "enum",
        crate::docgen::model::DocSymbolKind::EnumVariant => "enum variant",
        crate::docgen::model::DocSymbolKind::Interface => "interface",
        crate::docgen::model::DocSymbolKind::Trait => "trait",
        crate::docgen::model::DocSymbolKind::TypeAlias => "type alias",
        crate::docgen::model::DocSymbolKind::Constant => "constant",
        crate::docgen::model::DocSymbolKind::Variable => "variable",
        crate::docgen::model::DocSymbolKind::Property => "property",
        crate::docgen::model::DocSymbolKind::Builtin => "builtin",
        crate::docgen::model::DocSymbolKind::Unknown => "symbol",
    }
}

pub fn detect_broken_doc_links(
    root: &Path,
    project: &DocProject,
    options: LinkValidationOptions,
) -> LinkValidationReport {
    let mut broken = Vec::new();
    let mut skip_counts = LinkValidationBudgetSkipCounts::default();
    let started_at = Instant::now();
    let mut checked_links = 0usize;
    let mut checked_external_links = 0usize;
    let mut local_anchor_cache: BTreeMap<PathBuf, Option<AnchorValidationIndex>> = BTreeMap::new();
    let external_client =
        if options.validate_external_links && !options.external_link_allowlist.is_empty() {
            build_external_link_client(options.external_link_timeout_ms)
        } else {
            None
        };

    for symbol in &project.symbols {
        for (idx, line) in symbol.docs.lines.iter().enumerate() {
            for link in extract_markdown_links(line) {
                if link.starts_with("mailto:") {
                    continue;
                }
                if link.starts_with("http://") || link.starts_with("https://") {
                    if !options.validate_external_links
                        || options.external_link_allowlist.is_empty()
                    {
                        continue;
                    }

                    if !external_link_in_allowlist(&link, &options.external_link_allowlist) {
                        continue;
                    }

                    if link_validation_time_budget_exhausted(
                        started_at,
                        options.max_total_validation_time_ms,
                    ) {
                        skip_counts.max_total_time += 1;
                        continue;
                    }
                    if let Some(max_link_checks) = options.max_link_checks {
                        if checked_links >= max_link_checks {
                            skip_counts.max_link_checks += 1;
                            continue;
                        }
                    }
                    if let Some(max_external_link_checks) = options.max_external_link_checks {
                        if checked_external_links >= max_external_link_checks {
                            skip_counts.max_external_checks += 1;
                            continue;
                        }
                    }
                    checked_links += 1;
                    checked_external_links += 1;

                    match external_link_check(
                        external_client.as_ref(),
                        &link,
                        &options.external_link_allowlist,
                        options.allow_private_network_links,
                    ) {
                        ExternalLinkCheck::Reachable => {}
                        ExternalLinkCheck::Unreachable => {
                            broken.push(BrokenLinkFinding {
                                symbol: symbol.qualified_name.clone(),
                                target: link.clone(),
                                line: idx + 1,
                                kind: BrokenLinkKind::ExternalUnreachable,
                            });
                        }
                        ExternalLinkCheck::RedirectDisallowed { next_url, blocked_host } => {
                            broken.push(BrokenLinkFinding {
                                symbol: symbol.qualified_name.clone(),
                                target: format!(
                                    "{} (redirected to non-allowlisted host '{}' via '{}')",
                                    link, blocked_host, next_url
                                ),
                                line: idx + 1,
                                kind: BrokenLinkKind::ExternalRedirectDisallowed,
                            });
                        }
                        ExternalLinkCheck::PrivateAddressBlocked {
                            url,
                            host,
                            blocked_addresses,
                        } => {
                            let blocked = blocked_addresses.join(", ");
                            broken.push(BrokenLinkFinding {
                                symbol: symbol.qualified_name.clone(),
                                target: format!(
                                    "{} (host '{}' resolves to blocked address(es): {})",
                                    url, host, blocked
                                ),
                                line: idx + 1,
                                kind: BrokenLinkKind::ExternalPrivateAddressBlocked,
                            });
                        }
                    }
                    continue;
                }

                let fragment = link.split_once('#').map(|(_, anchor)| anchor).unwrap_or_default();
                let link_without_fragment = link.split('#').next().unwrap_or_default();
                let link_without_query =
                    link_without_fragment.split('?').next().unwrap_or_default();
                if link_without_query.is_empty() {
                    continue;
                }

                if link_validation_time_budget_exhausted(
                    started_at,
                    options.max_total_validation_time_ms,
                ) {
                    skip_counts.max_total_time += 1;
                    continue;
                }
                if let Some(max_link_checks) = options.max_link_checks {
                    if checked_links >= max_link_checks {
                        skip_counts.max_link_checks += 1;
                        continue;
                    }
                }
                checked_links += 1;

                let target = root.join(link_without_query);
                if !target.exists() {
                    broken.push(BrokenLinkFinding {
                        symbol: symbol.qualified_name.clone(),
                        target: target.display().to_string(),
                        line: idx + 1,
                        kind: BrokenLinkKind::LocalFileMissing,
                    });
                    continue;
                }

                if options.validate_local_anchors
                    && !fragment.is_empty()
                    && !local_anchor_exists(&target, fragment, &mut local_anchor_cache)
                {
                    broken.push(BrokenLinkFinding {
                        symbol: symbol.qualified_name.clone(),
                        target: format!("{}#{}", target.display(), fragment),
                        line: idx + 1,
                        kind: BrokenLinkKind::LocalAnchorMissing,
                    });
                }
            }
        }
    }
    LinkValidationReport { broken_links: broken, skip_counts }
}

fn link_validation_time_budget_exhausted(
    started_at: Instant,
    max_total_validation_time_ms: Option<u64>,
) -> bool {
    let Some(max_ms) = max_total_validation_time_ms else {
        return false;
    };
    started_at.elapsed() >= Duration::from_millis(max_ms)
}

fn local_anchor_exists(
    target: &Path,
    fragment: &str,
    cache: &mut BTreeMap<PathBuf, Option<AnchorValidationIndex>>,
) -> bool {
    let anchor = normalize_anchor(fragment);
    if anchor.is_empty() {
        return true;
    }

    let key = target.to_path_buf();
    if !cache.contains_key(&key) {
        increment_local_anchor_file_read_count();
        let parsed = match std::fs::read_to_string(target) {
            Ok(content) => Some(parse_anchor_index(&content)),
            Err(_) => None,
        };
        cache.insert(key.clone(), parsed);
    }

    let Some(index) = cache.get(&key).and_then(|entry| entry.as_ref()) else {
        return false;
    };

    index.contains_normalized_anchor(&anchor)
}

fn parse_anchor_index(content: &str) -> AnchorValidationIndex {
    let mut index = AnchorValidationIndex::default();

    for line in content.lines() {
        extract_quoted_attr_values(line, "id", &mut index.html_id_values);
        extract_quoted_attr_values(line, "name", &mut index.html_name_values);

        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            continue;
        }
        let heading = trimmed.trim_start_matches('#').trim();
        let slug = markdown_anchor_slug(heading);
        if !slug.is_empty() {
            index.markdown_heading_slugs.insert(slug);
        }
    }

    index
}

fn extract_quoted_attr_values(line: &str, attr_name: &str, values: &mut BTreeSet<String>) {
    let needle = format!(r#"{attr_name}=""#);
    let mut rest = line;
    while let Some(start) = rest.find(&needle) {
        let after = &rest[start + needle.len()..];
        let Some(end_quote) = after.find('"') else {
            break;
        };
        let value = &after[..end_quote];
        if !value.is_empty() {
            values.insert(value.to_string());
        }
        rest = &after[end_quote + 1..];
    }
}

fn normalize_anchor(anchor: &str) -> String {
    anchor.trim().trim_start_matches('#').trim().to_ascii_lowercase()
}

fn markdown_anchor_slug(heading: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;

    for ch in heading.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }

    out.trim_matches('-').to_string()
}

fn external_link_in_allowlist(link: &str, allowlist: &BTreeSet<String>) -> bool {
    let Ok(url) = reqwest::Url::parse(link) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    allowlist.contains(&host.to_ascii_lowercase())
}

enum ExternalLinkCheck {
    Reachable,
    Unreachable,
    RedirectDisallowed { next_url: String, blocked_host: String },
    PrivateAddressBlocked { url: String, host: String, blocked_addresses: Vec<String> },
}

fn build_external_link_client(timeout_ms: u64) -> Option<reqwest::blocking::Client> {
    increment_external_http_client_build_count();
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_millis(timeout_ms.max(1)))
        .build()
        .ok()
}

fn external_link_check(
    client: Option<&reqwest::blocking::Client>,
    link: &str,
    allowlist: &BTreeSet<String>,
    allow_private_network_links: bool,
) -> ExternalLinkCheck {
    let Some(client) = client else {
        return ExternalLinkCheck::Unreachable;
    };

    let mut current_url = match reqwest::Url::parse(link) {
        Ok(url) => url,
        Err(_) => return ExternalLinkCheck::Unreachable,
    };

    for _ in 0..10usize {
        if !allow_private_network_links {
            match blocked_private_addresses_for_url(&current_url) {
                Ok(blocked_addresses) if !blocked_addresses.is_empty() => {
                    return ExternalLinkCheck::PrivateAddressBlocked {
                        url: current_url.to_string(),
                        host: current_url.host_str().unwrap_or_default().to_string(),
                        blocked_addresses,
                    };
                }
                Ok(_) => {}
                Err(_) => return ExternalLinkCheck::Unreachable,
            }
        }

        let response = match client.get(current_url.clone()).send() {
            Ok(response) => response,
            Err(_) => return ExternalLinkCheck::Unreachable,
        };
        let status = response.status();
        if status.is_success() {
            return ExternalLinkCheck::Reachable;
        }
        if !status.is_redirection() {
            return ExternalLinkCheck::Unreachable;
        }

        let Some(location) = response.headers().get(reqwest::header::LOCATION) else {
            return ExternalLinkCheck::Unreachable;
        };
        let Ok(location) = location.to_str() else {
            return ExternalLinkCheck::Unreachable;
        };
        let next_url = match current_url.join(location) {
            Ok(url) => url,
            Err(_) => return ExternalLinkCheck::Unreachable,
        };
        let Some(host) = next_url.host_str() else {
            return ExternalLinkCheck::Unreachable;
        };
        let blocked_host = host.to_ascii_lowercase();
        if !allowlist.contains(&blocked_host) {
            return ExternalLinkCheck::RedirectDisallowed {
                next_url: next_url.to_string(),
                blocked_host,
            };
        }
        current_url = next_url;
    }

    ExternalLinkCheck::Unreachable
}

fn blocked_private_addresses_for_url(url: &reqwest::Url) -> Result<Vec<String>, ()> {
    let Some(host) = url.host_str() else {
        return Err(());
    };

    let mut blocked = BTreeSet::new();
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_external_ip(ip) {
            blocked.insert(ip.to_string());
        }
        return Ok(blocked.into_iter().collect());
    }

    let port = url.port_or_known_default().ok_or(())?;
    let addrs = (host, port).to_socket_addrs().map_err(|_| ())?;
    for addr in addrs {
        if is_blocked_external_ip(addr.ip()) {
            blocked.insert(addr.ip().to_string());
        }
    }

    Ok(blocked.into_iter().collect())
}

fn is_blocked_external_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_private() || ipv4.is_loopback() || ipv4.is_link_local() || ipv4.is_multicast()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_unique_local()
                || ipv6.is_loopback()
                || ipv6.is_unicast_link_local()
                || ipv6.is_multicast()
        }
    }
}

fn extract_markdown_links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find(')') {
            let link = &after[..end];
            out.push(link.to_string());
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docgen::model::{
        DocComment, DocGapKind, DocProject, DocSymbol, DocSymbolKind, DocVisibility,
    };
    use std::fs;
    use std::iter;
    use std::sync::{Mutex, MutexGuard};
    use std::time::{SystemTime, UNIX_EPOCH};

    static LINK_VALIDATION_COUNTER_TEST_MUTEX: Mutex<()> = Mutex::new(());
    static CALL_SITE_INDEX_COUNTER_TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("kujo_docgen_gaps_{prefix}_{nanos}"));
        fs::create_dir_all(&path).expect("failed to create temp directory");
        path
    }

    fn lock_test_mutex(mutex: &'static Mutex<()>) -> MutexGuard<'static, ()> {
        // Keep later tests runnable even if a prior test panicked while holding this mutex.
        mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn doc_project_with_symbol(root: &Path, doc_lines: Vec<String>) -> DocProject {
        DocProject {
            name: Some("test".to_string()),
            root: root.to_path_buf(),
            languages: vec!["kujo".to_string()],
            modules: Vec::new(),
            symbols: vec![DocSymbol {
                id: "symbol:test".to_string(),
                language: "kujo".to_string(),
                kind: DocSymbolKind::Function,
                name: "test_symbol".to_string(),
                qualified_name: "test_symbol".to_string(),
                signature: Some("func test_symbol()".to_string()),
                visibility: DocVisibility::Public,
                source_path: root.join("module.kujo"),
                line: 1,
                docs: DocComment { lines: doc_lines, summary: None, placeholder: false },
                examples: Vec::new(),
                gaps: Vec::new(),
                parent: None,
            }],
            gaps: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn gap_symbol(
        root: &Path,
        id: &str,
        name: &str,
        source_rel_path: &str,
        source_line: usize,
        visibility: DocVisibility,
    ) -> DocSymbol {
        DocSymbol {
            id: id.to_string(),
            language: "kujo".to_string(),
            kind: DocSymbolKind::Function,
            name: name.to_string(),
            qualified_name: name.to_string(),
            signature: Some(format!("func {name}()")),
            visibility,
            source_path: root.join(source_rel_path),
            line: source_line,
            docs: DocComment { lines: Vec::new(), summary: None, placeholder: false },
            examples: Vec::new(),
            gaps: vec![DocGapKind::MissingDocs],
            parent: None,
        }
    }

    fn legacy_known_call_sites(
        source_map: &BTreeMap<String, String>,
        symbol_name: &str,
        limit: usize,
    ) -> Vec<String> {
        if symbol_name.is_empty() {
            return Vec::new();
        }

        let mut calls = Vec::new();
        let needle = format!("{}(", symbol_name);
        for (path, source) in source_map {
            for (idx, line) in source.lines().enumerate() {
                if line.contains(&needle) {
                    calls.push(format!("{}:{}: {}", path, idx + 1, line.trim()));
                    if calls.len() >= limit {
                        return calls;
                    }
                }
            }
        }
        calls
    }

    #[test]
    fn repeated_local_anchor_checks_use_cached_file_index_per_path() {
        let _guard = lock_test_mutex(&LINK_VALIDATION_COUNTER_TEST_MUTEX);
        reset_link_validation_test_counters();

        let root = unique_temp_dir("anchor_cache");
        let anchor_target = root.join("target.md");
        fs::write(&anchor_target, "# API Reference\n\n## Usage\n")
            .expect("failed to write anchor target");
        let project = doc_project_with_symbol(
            &root,
            vec![
                "[first](target.md#api-reference) [second](target.md#api-reference)".to_string(),
                "[third](target.md#usage)".to_string(),
            ],
        );
        let options = LinkValidationOptions { validate_local_anchors: true, ..Default::default() };

        let report = detect_broken_doc_links(&root, &project, options);
        assert!(report.broken_links.is_empty(), "expected no broken links");

        let (local_anchor_reads, external_client_builds) = link_validation_test_counters();
        assert_eq!(
            local_anchor_reads, 1,
            "repeated local-anchor checks should read/parse each target file once"
        );
        assert_eq!(
            external_client_builds, 0,
            "local-anchor validation should not build external HTTP clients"
        );
    }

    #[test]
    fn repeated_external_host_checks_reuse_single_http_client() {
        let _guard = lock_test_mutex(&LINK_VALIDATION_COUNTER_TEST_MUTEX);
        reset_link_validation_test_counters();

        let root = unique_temp_dir("external_client_cache");
        let project = doc_project_with_symbol(
            &root,
            vec![
                "[one](http://localhost:9/service-a)".to_string(),
                "[two](http://localhost:9/service-b)".to_string(),
            ],
        );
        let mut allowlist = BTreeSet::new();
        allowlist.insert("localhost".to_string());
        let options = LinkValidationOptions {
            validate_external_links: true,
            external_link_allowlist: allowlist,
            allow_private_network_links: true,
            external_link_timeout_ms: 200,
            ..Default::default()
        };

        let report =
            match std::panic::catch_unwind(|| detect_broken_doc_links(&root, &project, options)) {
                Ok(report) => report,
                Err(_) => {
                    eprintln!(
                        "skipping repeated_external_host_checks_reuse_single_http_client: \
                     host runtime does not support reqwest/system-configuration initialization"
                    );
                    return;
                }
            };
        assert_eq!(
            report.broken_links.len(),
            2,
            "both unreachable links should be reported when host is allowlisted"
        );
        assert!(report
            .broken_links
            .iter()
            .all(|finding| finding.kind == BrokenLinkKind::ExternalUnreachable));

        let (local_anchor_reads, external_client_builds) = link_validation_test_counters();
        assert_eq!(local_anchor_reads, 0);
        assert_eq!(
            external_client_builds, 1,
            "all external checks in a run should reuse one HTTP client"
        );
    }

    #[test]
    fn gap_call_site_index_matches_legacy_order_and_limit_semantics() {
        let _guard = CALL_SITE_INDEX_COUNTER_TEST_MUTEX.lock().expect("test mutex lock");
        reset_call_site_index_test_counter();

        let source_map = BTreeMap::from([
            (
                "a.kujo".to_string(),
                ["foo();", "prefixfoo();", "bar();", "foo();", "foo();", "foo();"].join("\n"),
            ),
            (
                "b.kujo".to_string(),
                ["bar();", "foo();", "foo();", "foo();", "foo();", "foo();"].join("\n"),
            ),
        ]);

        let indexed = build_known_call_site_index(&source_map, ["foo", "bar", "missing"], 6);

        for symbol in ["foo", "bar", "missing"] {
            let expected = legacy_known_call_sites(&source_map, symbol, 6);
            let actual = indexed.get(symbol).cloned().unwrap_or_default();
            assert_eq!(actual, expected, "indexed and legacy call sites must match for {symbol}");
        }
    }

    #[test]
    fn build_gaps_uses_indexed_call_sites_and_preserves_known_call_sites_output() {
        let _guard = CALL_SITE_INDEX_COUNTER_TEST_MUTEX.lock().expect("test mutex lock");
        reset_call_site_index_test_counter();

        let root = unique_temp_dir("gap_indexed_callsites");
        let alpha_path = root.join("alpha.kujo");
        let beta_path = root.join("beta.kujo");
        fs::write(&alpha_path, "func alpha() {}\n").expect("write alpha source");
        fs::write(&beta_path, "func beta() {}\n").expect("write beta source");

        let mut project = DocProject {
            name: Some("test".to_string()),
            root: root.clone(),
            languages: vec!["kujo".to_string()],
            modules: Vec::new(),
            symbols: vec![
                gap_symbol(&root, "symbol:alpha", "foo", "alpha.kujo", 1, DocVisibility::Public),
                gap_symbol(&root, "symbol:beta", "bar", "beta.kujo", 1, DocVisibility::Public),
                gap_symbol(&root, "symbol:empty", "", "beta.kujo", 2, DocVisibility::Public),
                gap_symbol(
                    &root,
                    "symbol:private",
                    "hidden",
                    "alpha.kujo",
                    2,
                    DocVisibility::Private,
                ),
            ],
            gaps: Vec::new(),
            diagnostics: Vec::new(),
        };

        let source_map = BTreeMap::from([
            (
                alpha_path.display().to_string(),
                ["foo();", "bar();", "foo();", "prefixfoo();", "foo();", "foo();", "foo();"]
                    .join("\n"),
            ),
            (
                beta_path.display().to_string(),
                ["foo();", "bar();", "foo();", "foo();", "foo();"].join("\n"),
            ),
        ]);

        build_gaps(&mut project, &source_map);

        assert_eq!(project.gaps.len(), 3, "private symbols must not produce gaps");
        let by_symbol_id = project
            .gaps
            .iter()
            .map(|gap| (gap.symbol_id.as_str(), gap))
            .collect::<BTreeMap<&str, &crate::docgen::model::DocGap>>();

        let foo_gap = by_symbol_id.get("symbol:alpha").expect("foo gap should exist");
        let bar_gap = by_symbol_id.get("symbol:beta").expect("bar gap should exist");
        let empty_gap = by_symbol_id.get("symbol:empty").expect("empty-name gap should exist");

        assert_eq!(foo_gap.known_call_sites.len(), 6, "foo call sites should be limited to 6");
        assert_eq!(
            foo_gap.known_call_sites,
            legacy_known_call_sites(&source_map, "foo", 6),
            "foo known call sites should preserve legacy deterministic ordering"
        );
        assert_eq!(
            bar_gap.known_call_sites,
            legacy_known_call_sites(&source_map, "bar", 6),
            "bar known call sites should preserve legacy deterministic ordering"
        );
        assert!(
            empty_gap.known_call_sites.is_empty(),
            "empty symbol names should not produce known call sites"
        );
    }

    #[test]
    fn large_input_call_site_index_scans_each_source_line_once() {
        let _guard = CALL_SITE_INDEX_COUNTER_TEST_MUTEX.lock().expect("test mutex lock");
        reset_call_site_index_test_counter();

        let root = unique_temp_dir("gap_index_large");
        let file_count = 4usize;
        let lines_per_file = 150usize;
        let symbol_count = 48usize;
        let mut source_map = BTreeMap::new();

        for file_idx in 0..file_count {
            let path = root.join(format!("file_{file_idx}.kujo"));
            let mut lines = Vec::with_capacity(lines_per_file);
            for line_idx in 0..lines_per_file {
                if line_idx % 10 == 0 {
                    lines.push(format!("sym{}();", line_idx % symbol_count));
                } else {
                    lines.push("let value = 1;".to_string());
                }
            }
            let content = lines.join("\n");
            fs::write(&path, &content).expect("write large input source");
            source_map.insert(path.display().to_string(), content);
        }

        let symbols = (0..symbol_count)
            .map(|idx| format!("sym{idx}"))
            .chain(iter::once(String::new()))
            .collect::<Vec<String>>();
        let name_refs = symbols.iter().map(String::as_str).collect::<Vec<&str>>();

        let indexed = build_known_call_site_index(&source_map, name_refs, 6);
        assert!(
            indexed.contains_key("sym0"),
            "large-input index should record known call sites for matching symbols"
        );

        let scanned = call_site_index_test_counter();
        assert_eq!(
            scanned,
            file_count * lines_per_file,
            "call-site index should scan each source line once for the whole pass"
        );
    }
}
