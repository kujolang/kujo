# Kujo Field Notes — V1VM-IMP-001 Parser Dotted From-Import Hardening

**Date:** 2026-05-21
**Session:** 18:21 local
**Branch/Commit:** main / 0e0ea58
**Scope:** Hardened parser diagnostics coverage for dotted `from ... import ...` syntax and revalidated module-import parity/runtime behavior across VM/interpreter surfaces.

---

## What I Changed
- Added parser regression tests in `tests/parser_diagnostics_contract.rs`:
  - `parser_reports_diagnostic_for_trailing_dot_in_from_import_path`
  - `parser_reports_diagnostic_for_invalid_from_import_token_order`
- Revalidated existing positive and regression parser coverage for:
  - single-level dotted path acceptance
  - multi-level dotted path acceptance
  - existing flat import syntax compatibility

## Gotchas (Read This Next Time)
- **Gotcha:** Dotted-import parser hardening can be mostly a diagnostics-contract problem after feature implementation lands.
  - **Symptom:** Feature appears complete, but malformed token-order and trailing-segment cases are easy to miss.
  - **Root cause:** Core parse path handled `src..util`; adjacent malformed shapes were not explicitly asserted.
  - **Fix:** Add explicit negative tests for trailing-dot and invalid-token-order forms.
  - **Prevention:** Treat acceptance + malformed variants + legacy-form regression as a minimum parser test bundle for syntax extensions.

## Things I Learned
- Current parser behavior for dotted from-imports is additive and stable under expanded malformed-path coverage.
- Existing runtime/module tests already provide solid end-to-end dotted import confidence when combined with parser contracts.

## Debug Notes (Only if applicable)
- **Failing test / error:** N/A.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Move to `V1VM-IMP-002` precedence hardening and ensure precedence rules are explicitly documented in checklist evidence.

## Links / References
- Files touched:
  - `tests/parser_diagnostics_contract.rs`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/LANGUAGE_SPEC.md`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`

## Command Evidence
- `cargo test --test parser_diagnostics_contract`
  - Result: `27 passed; 0 failed`
- `cargo test --test interpreter_tests dotted_from_import`
  - Result: `2 passed; 0 failed`
- `cargo test --test package_module_workflow_integration dotted_from_imports`
  - Result: `1 passed; 0 failed`
- `cargo test --test vm_interpreter_parity_surfaces`
  - Result: `84 passed; 0 failed`
