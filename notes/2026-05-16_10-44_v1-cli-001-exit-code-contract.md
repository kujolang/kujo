# Kujo Field Notes — V1-CLI-001 Exit-Code Contract

**Date:** 2026-05-16
**Session:** 10:44 local
**Branch/Commit:** main / pending
**Scope:** Implemented `V1-CLI-001` by defining categorized CLI exit codes, wiring command failure paths in `src/main.rs`, and locking behavior with CLI contract tests and docs updates.

---

## What I Changed
- Added centralized CLI exit-code categories in `src/main.rs`:
  - usage `2`, lex/parse `3`, runtime `4`, IO `5`, internal `6`
- Routed parser/lexer diagnostics through the lex/parse exit code and runtime VM/interpreter failures through runtime exit code.
- Replaced panic-prone command-path `expect(...)` calls in `init`, package commands, and profile flamegraph writing with deterministic stderr diagnostics + categorized exits.
- Added `tests/cli_contracts.rs` for help/version success, usage error, missing-file IO error, parse error, runtime error, and JSON diagnostics JSON-validity/stdout-stderr behavior.
- Updated existing assertions in:
  - `tests/cli_json_contracts.rs`
  - `tests/parser_diagnostics_contract.rs`
  - `tests/native_api_security_boundaries.rs`
- Updated docs and roadmap state:
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `README.md`
  - `ROADMAP.md` (marked `V1-CLI-001` complete)
  - `CHANGELOG.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Exit-code contract changes ripple into many integration tests beyond CLI-focused suites.
  - **Symptom:** `cargo test` initially failed in `tests/native_api_security_boundaries.rs` with dozens of `expected ... code 1` assertions.
  - **Root cause:** Those security tests validate runtime-boundary subprocess failures and were pinned to the old generic `1` code.
  - **Fix:** Updated runtime-boundary expectations to code `4`.
  - **Prevention:** After any CLI exit-code semantic change, run a repository-wide search for `Some(1)` in integration tests and classify each expectation before final `cargo test`.

- **Gotcha:** `cargo test` can show transient serve integration readiness failures.
  - **Symptom:** `serve_command_integration` intermittently reported `Connection refused` during one full-suite run.
  - **Root cause:** Subprocess readiness race under local timing/load variance.
  - **Fix:** Re-ran the suite and full `cargo test`; tests passed without code changes.
  - **Prevention:** Treat isolated connection-refused serve failures as potential timing flake first; confirm with immediate targeted rerun before changing server code.

## Things I Learned
- `src/main.rs` had broad command-surface `expect(...)` usage that undermined predictable CLI contract behavior; replacing those with structured exits was necessary for stable automation-facing semantics.
- Categorized exit codes improve test clarity significantly: assertions can now distinguish parse vs runtime vs IO regressions directly.
- Keeping generic gate failures on `1` (for check-style command outcomes) avoids breaking expected command semantics while still adding richer failure classification.

## Debug Notes (Only if applicable)
- **Failing test / error:** `tests/native_api_security_boundaries.rs` expected `Some(1)` but received `Some(4)` after runtime exit-code categorization.
- **Repro steps:** `cargo test` after wiring new CLI exit codes.
- **Breakpoints / logs used:** direct assertion output from test failure logs and `rg -n "Some\(1\)" tests` scan.
- **Final diagnosis:** test expectations reflected legacy generic failure code and needed runtime-code updates.

## Follow-ups / TODO (For Future Agents)
- [ ] Add a shared integration-test helper for exit-code constants to avoid duplicated numeric literals across test files.
- [ ] Consider extending CLI contract tests to cover explicit internal-error surfaces with deterministic fixture hooks.

## Links / References
- Files touched:
  - `src/main.rs`
  - `tests/cli_contracts.rs`
  - `tests/cli_json_contracts.rs`
  - `tests/parser_diagnostics_contract.rs`
  - `tests/native_api_security_boundaries.rs`
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `README.md`
  - `ROADMAP.md`
