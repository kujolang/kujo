use crate::docgen::model::DocProject;
use crate::docgen::render::{source_link_provider, symbol_source_href, symbol_source_location};

pub fn render(
    project: &DocProject,
    source_links: bool,
    source_link_template: Option<&str>,
) -> String {
    let provider = source_link_provider(source_links, source_link_template);
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    html.push_str("<title>Kujo DocGen</title>");
    html.push_str("<style>");
    html.push_str(":root{--bg:#f8fafc;--card:#ffffff;--ink:#102a43;--muted:#627d98;--line:#d9e2ec;--accent:#0f766e;}*{box-sizing:border-box}body{margin:0;background:linear-gradient(160deg,#f8fafc,#eef2ff);font-family:Charter,Georgia,serif;color:var(--ink)}main{max-width:1100px;margin:0 auto;padding:2rem 1rem 4rem}header{background:var(--card);border:1px solid var(--line);padding:1rem 1.25rem;border-radius:14px;box-shadow:0 8px 25px rgba(15,23,42,.06)}h1{margin:.1rem 0 .2rem;font-size:1.8rem}p.meta{margin:.2rem 0;color:var(--muted)}section.symbol{margin-top:1rem;background:var(--card);border:1px solid var(--line);border-left:6px solid var(--accent);padding:1rem 1.1rem;border-radius:10px}.sig{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.88rem;background:#f1f5f9;padding:.35rem .5rem;border-radius:6px;display:inline-block}.placeholder{color:#9f1239;font-style:italic}.lang-tag{display:inline-block;margin-right:.35rem;background:#e0f2fe;color:#075985;padding:.15rem .45rem;border-radius:999px;font-size:.74rem}.filters{margin-top:.75rem;display:flex;flex-wrap:wrap;gap:.6rem}.filters label{font-size:.85rem;color:var(--muted)}</style>");
    html.push_str("<script>function filterLang(){const checked=[...document.querySelectorAll('.lang-filter:checked')].map(e=>e.value);document.querySelectorAll('[data-lang]').forEach(e=>{e.style.display=checked.includes(e.dataset.lang)?'block':'none';});}</script>");
    html.push_str("</head><body><main>");

    html.push_str("<header>");
    html.push_str("<h1>Universal Kujo DocGen</h1>");
    html.push_str(&format!(
        "<p class=\"meta\">Root: <code>{}</code></p>",
        escape_html(&project.root.display().to_string())
    ));
    html.push_str(&format!(
        "<p class=\"meta\">{} symbols across {} languages. {} unresolved documentation gaps.</p>",
        project.symbols.len(),
        project.languages.len(),
        project.gaps.len()
    ));
    html.push_str("<div class=\"filters\">");
    for language in &project.languages {
        html.push_str(&format!("<label><input class=\"lang-filter\" type=\"checkbox\" value=\"{}\" checked onchange=\"filterLang()\"> {}</label>", escape_html(language), escape_html(language)));
    }
    html.push_str("</div>");
    html.push_str("</header>");

    for symbol in &project.symbols {
        html.push_str(&format!(
            "<section class=\"symbol\" data-lang=\"{}\">",
            escape_html(&symbol.language)
        ));
        html.push_str(&format!("<div class=\"lang-tag\">{}</div>", escape_html(&symbol.language)));
        html.push_str(&format!("<h2>{}</h2>", escape_html(&symbol.qualified_name)));
        html.push_str(&format!(
            "<p><strong>Kind:</strong> {:?} · <strong>Visibility:</strong> {:?}</p>",
            symbol.kind, symbol.visibility
        ));
        if let Some(signature) = &symbol.signature {
            html.push_str(&format!("<p class=\"sig\">{}</p>", escape_html(signature)));
        }

        let source_location = symbol_source_location(symbol);
        if let Some(href) = symbol_source_href(symbol, &provider) {
            html.push_str(&format!(
                "<p><strong>Source:</strong> <a href=\"{}\"><code>{}</code></a></p>",
                escape_html(&href),
                escape_html(&source_location)
            ));
        } else {
            html.push_str(&format!(
                "<p><strong>Source:</strong> <code>{}</code></p>",
                escape_html(&source_location)
            ));
        }

        if symbol.docs.placeholder {
            html.push_str("<p class=\"placeholder\">Documentation needed. This symbol was discovered from the source code, but no human-authored documentation was found.</p>");
        } else {
            html.push_str("<div>");
            for line in &symbol.docs.lines {
                html.push_str(&format!("<p>{}</p>", escape_html(line)));
            }
            html.push_str("</div>");
        }

        html.push_str("</section>");
    }

    html.push_str("</main></body></html>");
    html
}

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
