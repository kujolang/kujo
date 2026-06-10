use crate::docgen::model::DocSymbol;
use std::path::{Component, Path};

pub mod html;
pub mod json;
pub mod markdown;

pub(crate) fn symbol_source_location(symbol: &DocSymbol) -> String {
    format!("{}:{}", symbol.source_path.display(), symbol.line)
}

#[derive(Debug, Clone)]
pub(crate) enum SourceLinkProvider {
    None,
    Template(String),
}

pub(crate) fn source_link_provider(
    source_links: bool,
    template: Option<&str>,
) -> SourceLinkProvider {
    if !source_links {
        return SourceLinkProvider::None;
    }
    match template.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => SourceLinkProvider::Template(value.to_string()),
        None => SourceLinkProvider::None,
    }
}

pub(crate) fn symbol_source_href(
    symbol: &DocSymbol,
    provider: &SourceLinkProvider,
) -> Option<String> {
    let SourceLinkProvider::Template(template) = provider else {
        return None;
    };
    let normalized_path = normalized_relative_path(&symbol.source_path)?;
    let encoded_path = percent_encode_path(&normalized_path);
    Some(template.replace("{path}", &encoded_path).replace("{line}", &symbol.line.to_string()))
}

fn normalized_relative_path(path: &Path) -> Option<String> {
    if path.is_absolute() {
        return None;
    }
    let mut segments = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(segment) => segments.push(segment.to_string_lossy().to_string()),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if segments.is_empty() {
        return None;
    }
    Some(segments.join("/"))
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/');
        if keep {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{source_link_provider, symbol_source_href};
    use crate::docgen::model::{DocComment, DocSymbol, DocSymbolKind, DocVisibility};
    use std::path::PathBuf;

    fn test_symbol(path: &str, line: usize) -> DocSymbol {
        DocSymbol {
            id: "sym:test".to_string(),
            language: "kujo".to_string(),
            kind: DocSymbolKind::Function,
            name: "test".to_string(),
            qualified_name: "test".to_string(),
            signature: None,
            visibility: DocVisibility::Public,
            source_path: PathBuf::from(path),
            line,
            docs: DocComment::default(),
            examples: Vec::new(),
            gaps: Vec::new(),
            parent: None,
        }
    }

    #[test]
    fn source_link_template_renders_expected_url() {
        let symbol = test_symbol("src/space name.kujo", 42);
        let provider = source_link_provider(
            true,
            Some("https://github.com/acme/repo/blob/main/{path}#L{line}"),
        );
        let href = symbol_source_href(&symbol, &provider).expect("source link href should exist");
        assert_eq!(href, "https://github.com/acme/repo/blob/main/src/space%20name.kujo#L42");
    }

    #[test]
    fn source_link_template_rejects_unsafe_paths() {
        let provider = source_link_provider(
            true,
            Some("https://gitlab.com/acme/repo/-/blob/main/{path}#L{line}"),
        );
        let parent_escape = test_symbol("../escape.kujo", 3);
        let absolute_path = test_symbol("/tmp/escape.kujo", 7);
        assert!(symbol_source_href(&parent_escape, &provider).is_none());
        assert!(symbol_source_href(&absolute_path, &provider).is_none());
    }
}
