# Kujo Field Notes — V1-HTTP-007 MIME registry coverage and precompressed extension behavior

**Date:** 2026-05-15
**Session:** 14:21 local
**Branch/Commit:** main / eb9c092
**Scope:** Expanded `kujo serve` MIME coverage for real-world asset families, added regression tests for fallback and dotfile interaction, and refined `.gz`/`.br` handling so archive MIME behavior remains predictable.

---

## What I Changed
- Expanded `src/serve_http.rs` MIME extension registry for `tif`/`tiff`, `mov`, `eot`, `zip`, `tar`, `gz`, `tgz`, and `7z`.
- Refined `split_content_path_and_encoding(...)` so `.gz`/`.br` are treated as `Content-Encoding` only for multi-extension precompressed assets (for example `index.html.gz`), while single-extension archive files keep extension-based MIME handling.
- Added/expanded unit tests in `src/serve_http.rs` for required mapping families, double-extension fallback, case-insensitive matching, and precompressed split behavior.
- Expanded live subprocess coverage in `tests/serve_command_integration.rs` for required MIME families, `application/octet-stream` fallback contracts, and dotfile-block precedence over MIME resolution.
- Updated `CHANGELOG.md`, `README.md`, and `ROADMAP.md` for completed `V1-HTTP-007` scope.

## Gotchas (Read This Next Time)
- **Gotcha:** Treating every `.gz` file as precompressed content encoding breaks archive MIME expectations.
  - **Symptom:** `.gz` assets can lose extension-based type mapping and be served as generic payloads with `Content-Encoding: gzip` even when they are archive files.
  - **Root cause:** Previous split logic stripped `.gz`/`.br` unconditionally before MIME lookup.
  - **Fix:** Only strip `.gz`/`.br` and set encoding when the stem still has an extension (multi-extension precompressed pattern).
  - **Prevention:** Keep precompressed-asset detection explicit and test both `index.html.gz`-style responses and direct archive extension behavior.

## Things I Learned
- MIME registry breadth and precompressed-sibling behavior are coupled; changes to one can silently affect the other.
- Double-extension regressions are a cheap way to lock safe fallback semantics (`payload.js.unknown` should stay `application/octet-stream`).
- Dotfile/private blocking should be validated as a higher-priority policy than MIME detection.

## Debug Notes (Only if applicable)
- **Failing test / error:** `serve_http::tests::guess_content_type_covers_v1_http_007_required_mappings` initially failed on missing `image/tiff` mapping.
- **Repro steps:** `cargo test serve_http::tests::guess_content_type_ -- --nocapture`
- **Breakpoints / logs used:** Focused test output plus direct inspection of `known_mime_for_extension(...)` and `split_content_path_and_encoding(...)`.
- **Final diagnosis:** MIME registry lacked required `V1-HTTP-007` entries, and split behavior needed tightening for `.gz`/`.br` edge cases.

## Follow-ups / TODO (For Future Agents)
- [ ] Add explicit serve integration checks for direct `.gz` and `.br` request header behavior if archive-vs-precompressed semantics evolve again.
- [ ] Consider documenting `.tgz`/`.tar.gz` response-shape examples in a dedicated serve contract doc if users request it frequently.

## Links / References
- Files touched:
  - `src/serve_http.rs`
  - `tests/serve_command_integration.rs`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
