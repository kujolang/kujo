use crate::docgen::model::{DocProject, DocSymbol};
use crate::docgen::render::{source_link_provider, symbol_source_href, symbol_source_location};

pub fn render(
    project: &DocProject,
    source_links: bool,
    source_link_template: Option<&str>,
) -> String {
    let provider = source_link_provider(source_links, source_link_template);
    let mut out = String::new();
    out.push_str("# Kujo DocGen\n\n");
    out.push_str(&format!("- Root: `{}`\n", project.root.display()));
    out.push_str(&format!("- Languages: {}\n", project.languages.join(", ")));
    out.push_str(&format!("- Symbols: {}\n", project.symbols.len()));
    out.push_str(&format!("- Gaps: {}\n\n", project.gaps.len()));

    for module in &project.modules {
        out.push_str(&format!("## {} ({})\n\n", module.name, module.language));
        for symbol in symbols_for_module(project, module.path.display().to_string().as_str()) {
            out.push_str(&format!("### {}\n\n", symbol.qualified_name));
            out.push_str(&format!("- Kind: {:?}\n", symbol.kind));
            out.push_str(&format!("- Visibility: {:?}\n", symbol.visibility));
            let source_location = symbol_source_location(symbol);
            if let Some(href) = symbol_source_href(symbol, &provider) {
                out.push_str(&format!("- Source: [`{}`]({})\n", source_location, href));
            } else {
                out.push_str(&format!("- Source: `{}`\n", source_location));
            }
            if let Some(signature) = &symbol.signature {
                out.push_str(&format!("- Signature: `{}`\n", signature));
            }
            out.push_str("\n");
            for line in &symbol.docs.lines {
                out.push_str(line);
                out.push('\n');
            }
            out.push('\n');
        }
    }

    out
}

fn symbols_for_module<'a>(project: &'a DocProject, module_path: &str) -> Vec<&'a DocSymbol> {
    let mut symbols: Vec<&DocSymbol> = project
        .symbols
        .iter()
        .filter(|symbol| symbol.source_path.display().to_string() == module_path)
        .collect();
    symbols.sort_by(|a, b| a.line.cmp(&b.line).then(a.qualified_name.cmp(&b.qualified_name)));
    symbols
}
