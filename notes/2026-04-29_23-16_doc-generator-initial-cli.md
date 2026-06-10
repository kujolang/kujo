# Kujo Field Notes — v0.12 Documentation Generator Initial Slice

**Date:** 2026-04-29
**Session:** 23:16 local
**Branch/Commit:** main / pending
**Scope:** Implemented the final remaining v0.12.0 roadmap track (documentation generator) with HTML output from `///` comments, example extraction, builtin API reference generation, tests, and release evidence updates.

---

## What I Changed
- Added src/doc_generator.rs with:
  - extraction of documented `func` declarations from `///` comments
  - fenced Kujo example extraction from doc comments
  - HTML rendering for module docs and docs index
  - builtin/native API reference HTML generation from registered builtins
- Added `kujo docgen <file> [--out-dir <DIR>] [--no-builtins]` command surface in src/main.rs.
- Exported doc generator module from src/lib.rs.
- Added focused tests in src/doc_generator.rs for:
  - doc-comment/function extraction
  - example extraction behavior
  - generated artifact creation for module/index/builtin pages
- Verified command smoke flow with temporary sample source:
  - `cargo run -- docgen <sample.kujo> --out-dir <temp/out>`

## Gotchas (Read This Next Time)
- **Gotcha:** The lexer currently ignores `///` comments during tokenization, so docs generation cannot rely on AST tokens.
  - **Symptom:** No doc-comment metadata available from parser/AST pipeline.
  - **Root cause:** `///` comments are treated as skipped comments in lexical scanning.
  - **Fix:** Implemented direct source-line scanning in the doc generator module.
  - **Prevention:** Keep docs extraction decoupled from parser until explicit doc-comment token support is introduced.

## Things I Learned
- A source-line scanner is sufficient for initial `func`-level docs and example extraction without parser changes.
- A generated docs index page improves discoverability and keeps command output deterministic.
- Builtin API references can be bootstrapped quickly from `Interpreter::get_builtin_names()` and improved later with richer signatures/descriptions.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Validation run:** `cargo test doc_generator -- --nocapture` plus `cargo run -- docgen ...` smoke run.
- **Final diagnosis:** Initial docs-generator roadmap scope is complete and stable.

## Follow-ups / TODO (For Future Agents)
- [ ] Expand docs extraction to cover structs/enums/methods and module-level summaries.
- [ ] Attach richer builtin signatures and descriptions (not just names).
- [ ] Add support for cross-file docs generation and output theming/custom templates.

## Links / References
- Files touched:
  - src/doc_generator.rs
  - src/main.rs
  - src/lib.rs
  - ROADMAP.md
  - CHANGELOG.md
  - README.md
  - notes/README.md
- Related docs:
  - notes/FIELD_NOTES_SYSTEM.md
  - notes/GOTCHAS.md
  - notes/2026-04-29_23-08_repl-improvements-initial-cli.md
