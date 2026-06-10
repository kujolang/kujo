# Kujo Field Notes — v0.12 LSP Real-Time Diagnostics CLI Slice

**Date:** 2026-04-29
**Session:** 22:43 local
**Branch/Commit:** main / 75c7d8b
**Scope:** Implemented the next highest-priority incomplete v0.12.0 LSP roadmap item (real-time diagnostics) with diagnostics collection logic, CLI exposure, tests, and release evidence updates.

---

## What I Changed
- Added src/lsp_diagnostics.rs with:
  - delimiter-balance diagnostics for braces, parentheses, and brackets
  - parser panic capture converted into deterministic diagnostics
  - stable diagnostics payload shape: line, column, severity, message
- Added CLI command in src/main.rs:
  - kujo lsp-diagnostics <file> [--json]
  - plain output: severity + location + message
  - JSON output: array of diagnostic objects
- Added module declarations in both crate roots:
  - src/main.rs (mod lsp_diagnostics;)
  - src/lib.rs (pub mod lsp_diagnostics;)
- Added focused tests in src/lsp_diagnostics.rs for:
  - valid-program no-diagnostics behavior
  - unmatched closing brace
  - unclosed opening parenthesis
  - parser panic message capture for malformed generic annotations

## Gotchas (Read This Next Time)
- **Gotcha:** catch_unwind captures panics but default panic hooks still emit noisy stderr output.
  - **Symptom:** Diagnostics command printed panic trace text before JSON diagnostics.
  - **Root cause:** Panic hook runs even when unwind is caught.
  - **Fix:** Temporarily swapped panic hook to no-op while parsing diagnostics, then restored original hook.
  - **Prevention:** Any panic-as-data workflow should explicitly control hook behavior during capture.

- **Gotcha:** Unused enum variants can introduce build warnings that violate release-hardening expectations.
  - **Symptom:** Warning enum variant triggered dead-code compiler warning.
  - **Root cause:** Added Warning severity before implementing warning-producing checks.
  - **Fix:** Removed unused Warning variant.
  - **Prevention:** Add severity variants only when at least one code path emits them or gate behind allow attributes with explicit rationale.

## Things I Learned
- Syntax diagnostics can be delivered incrementally via deterministic structural checks while richer semantic diagnostics remain pending.
- Parser panic capture provides immediate practical value for malformed type annotations that currently panic.
- Clean diagnostics output for editor consumers requires both stable payload shape and suppression of panic-hook noise.

## Debug Notes (Only if applicable)
- **Failing test / error:** Initial diagnostics runs emitted panic messages in CLI output despite successful JSON diagnostics.
- **Repro steps:** cargo run -- lsp-diagnostics /tmp/lsp_diagnostics_smoke.kujo --json.
- **Breakpoints / logs used:** observed stderr panic output with expected diagnostics payload.
- **Final diagnosis:** Default panic hook behavior needed to be temporarily overridden during catch_unwind.

## Follow-ups / TODO (For Future Agents)
- [ ] Extend diagnostics beyond syntax shape to include semantic checks (undefined symbol checks, duplicate declarations, etc.).
- [ ] Add line/column precision for parser panic diagnostics once parser exposes explicit error locations instead of panicking.
- [ ] Consider replacing panic-based parser failure paths with structured parser errors to simplify diagnostics flow.

## Links / References
- Files touched:
  - src/lsp_diagnostics.rs
  - src/main.rs
  - src/lib.rs
  - ROADMAP.md
  - CHANGELOG.md
  - README.md
- Related docs:
  - notes/FIELD_NOTES_SYSTEM.md
  - notes/GOTCHAS.md
  - notes/2026-04-29_22-40_lsp-hover-documentation-cli.md
