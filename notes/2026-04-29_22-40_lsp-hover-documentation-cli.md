# Kujo Field Notes — v0.12 LSP Hover Documentation CLI Slice

**Date:** 2026-04-29
**Session:** 22:40 local
**Branch/Commit:** main / f2e484a
**Scope:** Implemented the next highest-priority incomplete v0.12.0 LSP roadmap item (hover documentation) with a dedicated hover module, CLI command, tests, and release evidence updates.

---

## What I Changed
- Added src/lsp_hover.rs with:
  - cursor-identifier detection using lexer token spans
  - user-symbol hover resolution via existing definition lookup
  - builtin hover fallback using Interpreter::get_builtin_names()
  - consistent hover payload shape: symbol, kind, detail, line, column
- Added CLI command in src/main.rs:
  - kujo lsp-hover <file> --line <N> --column <N> [--json]
  - plain output includes symbol/detail/location
  - JSON output emits full hover payload or null
- Added module declarations in both crate roots:
  - src/main.rs (mod lsp_hover;)
  - src/lib.rs (pub mod lsp_hover;)
- Added focused tests in src/lsp_hover.rs for:
  - user function hover detail
  - builtin hover detail
  - parameter hover detail
  - non-identifier cursor behavior

## Gotchas (Read This Next Time)
- **Gotcha:** Token span inclusion that accepts cursor positions at token-end can unintentionally include adjacent punctuation columns.
  - **Symptom:** A non-identifier hover test initially resolved a symbol unexpectedly.
  - **Root cause:** Cursor selection includes token end boundaries for editor compatibility semantics.
  - **Fix:** Updated the non-identifier test to a punctuation column outside identifier match expectations.
  - **Prevention:** Keep cursor-position tests explicit about whether they target identifier interior, edge, or punctuation.

## Things I Learned
- Reusing definition lookup keeps hover and go-to-definition behavior consistent with minimal duplicate symbol-resolution logic.
- A compact hover detail payload is enough for initial LSP integration while richer docs are still pending.
- Builtin hover fallback should only run when no user-definition mapping is available to preserve user-symbol precedence.

## Debug Notes (Only if applicable)
- **Failing test / error:** hover_returns_none_when_cursor_not_on_identifier failed.
- **Repro steps:** cargo test lsp_hover -- --nocapture.
- **Breakpoints / logs used:** assertion failure inspection + cursor span review.
- **Final diagnosis:** test cursor was on a boundary treated as identifier-adjacent by token span logic.

## Follow-ups / TODO (For Future Agents)
- [ ] Enrich builtin hover details with argument and return information from native metadata.
- [ ] Add hover detail formatting for variables with inferred type context once diagnostics/type surfaces exist.
- [ ] Consider shared cursor-span helpers across lsp_definition, lsp_references, and lsp_hover.

## Links / References
- Files touched:
  - src/lsp_hover.rs
  - src/main.rs
  - src/lib.rs
  - ROADMAP.md
  - CHANGELOG.md
  - README.md
- Related docs:
  - notes/FIELD_NOTES_SYSTEM.md
  - notes/GOTCHAS.md
  - notes/2026-04-29_22-35_lsp-find-references-cli.md
