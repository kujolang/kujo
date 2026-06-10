# Kujo Field Notes — v0.12 LSP Rename Refactoring CLI Slice

**Date:** 2026-04-29
**Session:** 22:47 local
**Branch/Commit:** main / 491a42e
**Scope:** Implemented the next highest-priority incomplete v0.12.0 LSP roadmap item (rename refactoring) with deterministic edit generation, CLI wiring, tests, and release evidence updates.

---

## What I Changed
- Added src/lsp_rename.rs with:
  - symbol selection from cursor identifier
  - identifier validation for rename targets
  - scope-aware target discovery using existing lsp_references behavior
  - deterministic edit generation and in-memory source rewrite output
- Added CLI command in src/main.rs:
  - kujo lsp-rename <file> --line <N> --column <N> --new-name <NAME> [--json]
  - plain output lists applied edit locations
  - JSON output includes edit_count, edits, and updated_source
- Added module declarations in both crate roots:
  - src/main.rs (mod lsp_rename;)
  - src/lib.rs (pub mod lsp_rename;)
- Added focused tests in src/lsp_rename.rs for:
  - function definition + call-site rename
  - shadow-scope-safe rename isolation
  - invalid identifier rejection
  - non-identifier cursor handling

## Gotchas (Read This Next Time)
- **Gotcha:** Applying multiple edits on the same line from left-to-right corrupts column offsets.
  - **Symptom:** Later replacements on the same line targeted shifted columns.
  - **Root cause:** Earlier replacements changed line length before later edits were applied.
  - **Fix:** Grouped edits per line and applied them in descending column order.
  - **Prevention:** Any multi-edit same-line transformer should process right-to-left unless edits are offset-adjusted.

- **Gotcha:** Zero-warning policy is easy to violate with minor implementation leftovers.
  - **Symptom:** Build emitted unused_mut warning in rename implementation.
  - **Root cause:** Trailing-newline tracking variable was marked mutable without mutation.
  - **Fix:** Removed unnecessary mut qualifier.
  - **Prevention:** Re-run targeted tests and build after warning fixes before committing feature rounds.

## Things I Learned
- Existing scope-aware references logic provides a strong foundation for rename refactoring with minimal duplicate symbol-resolution code.
- Returning both edit metadata and updated source text is sufficient for early editor integration before direct workspace-edit protocols are introduced.
- Identifier validation upfront keeps rename failure modes deterministic and avoids partial edit generation.

## Debug Notes (Only if applicable)
- **Failing test / error:** Build warning (`unused_mut`) violated zero-warning expectation.
- **Repro steps:** cargo test lsp_rename -- --nocapture and cargo build.
- **Breakpoints / logs used:** compiler warning output.
- **Final diagnosis:** rename source-rewrite helper had non-mutated mutable binding.

## Follow-ups / TODO (For Future Agents)
- [ ] Add optional write-back mode for direct file edits after preview workflow is validated.
- [ ] Expand rename safety checks for cases with parser ambiguities and generated symbol names.
- [ ] Add rename conflict diagnostics (for example: target name collides with existing declaration in same scope).

## Links / References
- Files touched:
  - src/lsp_rename.rs
  - src/main.rs
  - src/lib.rs
  - ROADMAP.md
  - CHANGELOG.md
  - README.md
- Related docs:
  - notes/FIELD_NOTES_SYSTEM.md
  - notes/GOTCHAS.md
  - notes/2026-04-29_22-43_lsp-real-time-diagnostics-cli.md
