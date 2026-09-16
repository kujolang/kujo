# In-process HTML-to-PDF

Status: production business-document profile (`kujo.pdf.html/v1`)

Kujo's `pdf_render_html` and `pdf_render_html_to_file` APIs compile a deliberately bounded HTML/CSS profile into PDF inside the Kujo process. They do not start a browser, execute a command, call a renderer service, resolve a URL, inspect the host filesystem, or download a font or asset.

## API

```kujo
let receipt := pdf_render_html(html, options, assets)
let stored := pdf_render_html_to_file(html, options, assets, "/absolute/private/quote.pdf")
```

The in-memory receipt includes `api_version`, `renderer_version`, `input_sha256`, `output_sha256`, `bytes`, `pages`, `page_width_mm`, `page_height_mm`, `warnings`, `warning_count`, `render_duration_ms`, and bounded `pdf_bytes`. The file variant omits `pdf_bytes`, adds `path`, creates a mode-0600 same-directory temporary file, syncs it, and atomically publishes without replacement. File output requires `filesystem-write`; in-memory rendering has no host effect.

Options are strict and reject unknown keys:

- `page_size`: `A4` (default) or `Letter`;
- `orientation`: `portrait` (default) or `landscape`;
- `margin_mm`, or side-specific `margin_top_mm`, `margin_right_mm`, `margin_bottom_mm`, `margin_left_mm`, each 0–50;
- `show_page_numbers`: Boolean;
- `header_text`, `footer_text`, and `title`: bounded strings;
- `max_output_bytes`: 1–33,554,432;
- `timeout_ms`: 100–30,000.

Assets are explicit byte dictionaries:

```kujo
let assets := {
  "images": {"tenant-logo.png": logo_bytes},
  "fonts": {"tenant-brand.ttf": approved_font_bytes}
}
```

Image `src` values are logical keys from `assets.images`; they are never paths or URLs. Font families come from supplied single-font TrueType/OpenType data or the renderer's compiled default font set. Applications own font licensing and should allow only reviewed tenant fonts.

## Supported document profile

Supported elements are `html`, `head`, `title`, `body`, `div`, `section`, `header`, `footer`, `main`, `article`, headings, paragraphs, spans, strong/bold/emphasis/italic/underline/small text, ordered and unordered lists, tables and table sections/rows/cells, horizontal rules, line breaks, images, and fragment-only links.

Supported inline CSS properties cover block/flex layout, dimensions, gap, padding, margin, borders, border collapse/radius, background and foreground colors, font family/size/style/weight, line height, text alignment/decoration, vertical alignment, whitespace, table layout, and explicit page/break controls. Stylesheets and arbitrary selectors are not supported. An unknown element, attribute, property, or unsafe value is an error rather than a silent fallback.

This profile is intended for quotations, estimates, invoices, customer details, line-item tables, terms, tenant branding, page headers/footers, page numbers, and explicit or automatic pagination. It is not browser-compatible HTML and does not support JavaScript, forms, SVG, embedded media, floats, CSS Grid, external stylesheets, or arbitrary web application markup.

## Bounds and isolation

- HTML: 1 MiB, 5,000 tokenizer tokens, nesting depth 128;
- assets: 24 total and 16 MiB combined;
- images: 5 MiB each, at most 4096×4096 and 16,777,216 decoded pixels;
- fonts: 4 MiB each and single TrueType/OpenType signatures only;
- output: 32 MiB and 256 pages;
- render deadline: 100 ms–30 s;
- concurrent in-process renders: four.

Active tags, event attributes, remote/file/data URLs, CSS imports, CSS URL expressions, JavaScript, behavior expressions, path traversal asset names, malformed assets, and decompression-dimension bombs fail closed. Renderer panics are caught and converted to content-free errors. A timed-out render thread cannot be force-killed safely by Rust and retains its concurrency permit until it exits; the four-permit ceiling bounds accumulation. Deployments must still apply process CPU/memory limits and restart policy.

Output identifiers are rewritten from the input digest, renderer metadata excludes wall-clock and host identity, and identical normalized inputs under the same renderer version produce identical bytes. Golden tests reparse output and verify commercial text, dimensions, pagination, digest repeatability, and absence of executable or remote PDF actions. `tests/pdf_external_validation.sh` additionally validates structure and extracted text with the independent Poppler tools.

## Engine and limitations

The implementation pins `printpdf` 0.12.8 (MIT) and `lopdf` 0.44.0 in `Cargo.lock`. `printpdf` supplies the in-process HTML layout and PDF writer; `lopdf` canonicalizes identifiers and reparses golden output. The feature is compiled as `runtime-pdf` and is enabled by default.

The direct renderer dependencies are MIT; the layout/font stack is MIT or Apache-2.0. `cargo audit` reports no known vulnerability. It does report RUSTSEC-2025-0141 because `hyphenation`, a build dependency of the pinned layout engine, uses unmaintained `bincode` 1.3.3. That advisory is not a vulnerability and the crate is not part of the runtime input path. The release gate ignores only that exact advisory while continuing to deny every vulnerability and other warning; replacing the upstream layout dependency remains tracked release maintenance.

Tagged-PDF accessibility is not implemented or claimed. PDF/A conformance and digital signatures are not part of this profile. Complex-script shaping and Unicode coverage depend on the explicitly supplied approved font. Repeated table header behavior follows the pinned engine and must be visually regression-tested for each application template before release.
