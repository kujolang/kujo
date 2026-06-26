# Kujo Field Notes — V1-ERR-001 Diagnostic Unification

**Date:** 2026-05-07
**Session:** 07:22 local
**Branch/Commit:** main / 4315b22
**Scope:** Implemented V1-ERR-001 by introducing a shared diagnostic model and routing lexer/parser/runtime/VM/CLI/LSP surfaces through stable diagnostic codes and rendering helpers.

---

## What I Changed
- Added centralized diagnostics primitives in `src/errors.rs`:
  - `Diagnostic`
  - `DiagnosticSeverity`
  - `DiagnosticSubsystem`
  - stable diagnostic code constants (`KUJOLEX001`, `KUJOPARSE001`, `KUJORUN001`, `KUJOVM001`, `KUJOCLI001`, `KUJOLSP001`)
- Added shared human renderer (`render_human`) and JSON renderer (`to_json_value`) for diagnostics.
- Updated `KujoError` to carry diagnostic code/subsystem metadata and expose code-prefixed runtime rendering.
- Added lexer conversion path `LexerDiagnostic::to_diagnostic()` and parser conversion path `ParseDiagnostic::to_diagnostic(...)`.
- Updated `src/lsp_diagnostics.rs` to emit shared diagnostics (including subsystem/code metadata) for lexer/parser and delimiter/type checks.
- Updated CLI diagnostics handling in `src/main.rs` to print shared diagnostics for lexer/parser/CLI/VM failure paths and emit richer `lsp-diagnostics --json` entries.
- Added regression coverage:
  - `tests/diagnostics_contract.rs`
  - `tests/cli_json_contracts.rs` updates for diagnostic metadata fields
  - `tests/parser_diagnostics_contract.rs` updates for parse error code rendering
- Updated docs:
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md` (`lsp-diagnostics --json` schema)
  - `README.md` (`lsp-diagnostics` contract callout)
  - `CHANGELOG.md`
  - `ROADMAP.md` (mark V1-ERR-001 complete)

## Gotchas (Read This Next Time)
- **Gotcha:** Shared diagnostic coordinates are an LSP integration contract, not an internal-only detail.
  - **Symptom:** `lsp_code_actions` and `lsp_server` compile failures after changing diagnostic line/column fields to `Option<usize>`.
  - **Root cause:** Existing LSP helpers and code-action paths assume diagnostics always provide numeric line/column fields.
  - **Fix:** Keep `Diagnostic.line`/`Diagnostic.column` as numeric fields and use `0` when location is unknown.
  - **Prevention:** If changing shared diagnostic data shapes, grep all LSP helper modules (`lsp_*`) and CLI JSON serializers before finalizing the type change.

## Things I Learned
- A unified diagnostic model can be introduced incrementally without rewriting every runtime path at once by adding conversion helpers at subsystem boundaries.
- Stable diagnostic code prefixes (`KUJOLEX*`, `KUJOPARSE*`, etc.) are easiest to preserve when code generation lives close to the originating subsystem (lexer/parser), then is normalized by shared renderers.
- For Kujo’s current tooling, preserving simple numeric location fields is safer than optional location fields because downstream helpers treat location as required.

## Debug Notes (Only if applicable)
- **Failing test / error:** Compile errors in `src/lsp_code_actions.rs` and `src/lsp_server.rs` due to `Option<usize>` line/column fields.
- **Repro steps:**
  - `cargo test --test diagnostics_contract`
- **Breakpoints / logs used:** Rust compiler errors and targeted re-run loop for diagnostics and LSP suites.
- **Final diagnosis:** Shared diagnostics type shape drifted from LSP helper expectations.

## Follow-ups / TODO (For Future Agents)
- [ ] Expand shared diagnostic wrapping to remaining ad-hoc CLI `eprintln!` failure paths that still emit plain strings.
- [ ] Decide whether to introduce stricter per-kind runtime/VM diagnostic code families (for example `KUJORUN00x`) once diagnostics catalog docs are added.

## Links / References
- Files touched:
  - `src/errors.rs`
  - `src/lexer.rs`
  - `src/parser.rs`
  - `src/lsp_diagnostics.rs`
  - `src/main.rs`
  - `tests/diagnostics_contract.rs`
  - `tests/cli_json_contracts.rs`
  - `tests/parser_diagnostics_contract.rs`
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `notes/GOTCHAS.md`
