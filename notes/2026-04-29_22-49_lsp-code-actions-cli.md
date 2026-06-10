# Kujo Field Notes — v0.12 LSP Code Actions CLI Slice

**Date:** 2026-04-29
**Session:** 22:49 local
**Branch/Commit:** main / b0724b3
**Scope:** Implemented the final incomplete v0.12.0 LSP roadmap item (code actions) with diagnostics-driven quick-fix generation, CLI wiring, tests, and release evidence updates.

---

## What I Changed
- Added src/lsp_code_actions.rs with:
  - quick-fix action generation from diagnostics output
  - syntax action kinds for unmatched closing delimiters and unclosed opening delimiters
  - stable action payload fields: title, kind, line, column, replacement, description
- Added CLI command in src/main.rs:
  - kujo lsp-code-actions <file> [--json]
  - plain output lists action title/location/replacement
  - JSON output emits structured code-action entries
- Added module declarations in both crate roots:
  - src/main.rs (mod lsp_code_actions;)
  - src/lib.rs (pub mod lsp_code_actions;)
- Added focused tests in src/lsp_code_actions.rs for:
  - no actions for valid source
  - unmatched closing brace action
  - unclosed parenthesis action
  - unclosed bracket action

## Gotchas (Read This Next Time)
- **Gotcha:** Code actions without deterministic source coordinates quickly become unusable for editor integration.
  - **Symptom:** Suggestions can be generated but cannot be safely applied.
  - **Root cause:** Diagnostics-level actions need explicit insertion/removal points.
  - **Fix:** Standardized each action with line/column + replacement payload.
  - **Prevention:** Treat line/column/replacement as mandatory contract for all quick-fix actions.

## Things I Learned
- Diagnostics-driven quick-fix generation can deliver practical LSP value before full AST-aware refactor actions exist.
- Using diagnostics as the code-action source keeps feature layering straightforward: diagnose first, then fix.
- Minimal, explicit action kinds make it easier to add future semantic actions without breaking consumers.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** cargo test lsp_code_actions -- --nocapture; cargo run -- lsp-code-actions /tmp/lsp_code_actions_smoke.kujo --json.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Expand code actions to include parser-panic remediation suggestions where actionable edits can be inferred.
- [ ] Add semantic quick-fixes once richer diagnostics are available (for example undefined symbols or duplicate declarations).
- [ ] Introduce optional action IDs for stable editor-side deduplication and telemetry.

## Links / References
- Files touched:
  - src/lsp_code_actions.rs
  - src/main.rs
  - src/lib.rs
  - ROADMAP.md
  - CHANGELOG.md
  - README.md
- Related docs:
  - notes/FIELD_NOTES_SYSTEM.md
  - notes/GOTCHAS.md
  - notes/2026-04-29_22-47_lsp-rename-refactoring-cli.md
