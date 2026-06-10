# Kujo Field Notes — v0.12 LSP Go-To-Definition CLI Slice

**Date:** 2026-04-29
**Session:** 22:28 local
**Branch/Commit:** main / 7bdd512
**Scope:** Implemented the highest-priority incomplete active-cycle roadmap item for v0.12.0 LSP: go-to-definition. Added a CLI surface, tests, and release docs evidence.

---

## What I Changed
- Added `src/lsp_definition.rs` with:
  - symbol-under-cursor lookup using lexer tokens
  - definition collection for `func`, `let`, `const`, `for`, `except`, and function parameters
  - definition selection policy: nearest previous definition first, earliest matching fallback when reference appears before declaration
- Added CLI command in `src/main.rs`:
  - `kujo lsp-definition <file> --line <N> --column <N> [--json]`
  - plain output: `<name>\t<file>:<line>:<column>`
  - JSON output: `{ name, line, column, kind }` or `null` when not found
- Added module declarations to both crate roots:
  - `src/main.rs` (`mod lsp_definition;`)
  - `src/lib.rs` (`pub mod lsp_definition;`)
- Added focused tests in `src/lsp_definition.rs` for:
  - function call-site resolution
  - nearest shadowed-variable resolution
  - parameter resolution
  - future-definition fallback
  - non-identifier cursor handling
- Updated release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Lexer token `column` is end-exclusive, not start-inclusive.
  - **Symptom:** Returned definition columns were off unless corrected.
  - **Root cause:** Identifier tokens store `column` after scanning token text.
  - **Fix:** Compute start as `token.column - identifier_length`.
  - **Prevention:** Treat lexer token coordinates as end positions whenever deriving symbol spans.

- **Gotcha:** Definition-selection ordering can invert nearest-shadowed results if comparator direction is wrong.
  - **Symptom:** Shadowed `value` usage resolved to global `let value` instead of nearest local one.
  - **Root cause:** Earlier comparator was reused where later-in-file ordering was required.
  - **Fix:** Use separate ordering helpers (`is_earlier`, `is_later`) for fallback and nearest-prior selection.
  - **Prevention:** Keep two explicit policies in tests: "nearest prior" and "earliest fallback".

## Things I Learned
- A lexer-token approach can ship useful go-to-definition without parser location metadata, as long as token-span conventions are handled correctly.
- For editor-like features in this codebase, deterministic behavior with explicit tie-breaking is more important than perfect semantic scoping in the first increment.
- Dual crate-root module declarations (`main.rs` + `lib.rs`) remain a frequent source of avoidable compile failures.

## Debug Notes (Only if applicable)
- **Failing test / error:** `can't call method saturating_sub on ambiguous numeric type {integer}` in `src/lsp_definition.rs`.
- **Repro steps:** `cargo test lsp_definition -- --nocapture`.
- **Breakpoints / logs used:** compiler diagnostics + direct fix on local counter type.
- **Final diagnosis:** `paren_depth` needed explicit integer type (`usize`) before calling `saturating_sub`.

## Follow-ups / TODO (For Future Agents)
- [ ] Expand definition collection to destructuring patterns (`let {x, y}` and `let [a, b]`) for parity with completion symbol collection.
- [ ] Add method/struct/member definition support when parser location metadata is available.
- [ ] Reuse shared cursor-symbol extraction logic between `lsp_complete` and `lsp_definition` to avoid drift.

## Links / References
- Files touched:
  - `src/lsp_definition.rs`
  - `src/main.rs`
  - `src/lib.rs`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `README.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `notes/2026-04-29_23-35_v0.12-lsp-completion-engine.md`
