# Kujo Field Notes — V1-STD-001 Native API Inventory Contracts

**Date:** 2026-05-15
**Session:** 17:22 local
**Branch/Commit:** main / 838f003
**Scope:** Completed roadmap item V1-STD-001 by adding a canonical standard-library inventory table and enforcing runtime/docs parity for builtin names, arity labels, and capability requirements.

---

## What I Changed
- Added `docs/STANDARD_LIBRARY.md` with one inventory row per `Interpreter::get_builtin_names()` entry.
- Added interpreter-native metadata helpers in `src/interpreter/mod.rs`:
  - `Interpreter::canonical_native_function_name(...)`
  - `Interpreter::native_function_capability(...)`
  - `Interpreter::native_function_arity(...)`
- Updated native dispatch in `src/interpreter/native_functions/mod.rs` to use the centralized canonicalization helper and metadata wrappers.
- Replaced and expanded `tests/stdlib_reference_contract.rs` to enforce:
  - full runtime/docs builtin coverage
  - no duplicate documented builtins
  - required table-field presence
  - capability-column parity with runtime policy
  - arity-column parity with centralized metadata
  - alias edge-case parity for `println`, `str`, and `time`
- Updated `README.md`, `docs/NATIVE_API_SECURITY_POSTURE.md`, `CHANGELOG.md`, and `ROADMAP.md` for V1-STD-001 completion.

## Gotchas (Read This Next Time)
- **Gotcha:** Public helper methods in `Interpreter` can still trigger dead-code warnings in the `kujo` binary target.
  - **Symptom:** `cargo test --test stdlib_reference_contract` initially reported dead-code warnings for new helper methods.
  - **Root cause:** The methods were only consumed by integration tests, not by main runtime paths.
  - **Fix:** Route native dispatch capability/arity checks through the new helper methods in `src/interpreter/native_functions/mod.rs`.
  - **Prevention:** When adding test-facing helper APIs, wire them into production call paths when possible to avoid warning drift.

- **Gotcha:** Native alias handling (`println`/`str`/`time`) can drift between docs/tests/runtime if canonicalization is duplicated.
  - **Symptom:** Capability/arity checks need alias-aware parity logic in docs contract tests.
  - **Root cause:** Canonical alias mapping originally lived only in native dispatch.
  - **Fix:** Centralized canonicalization in `Interpreter::canonical_native_function_name(...)` and reused it for dispatch + tests.
  - **Prevention:** Keep alias normalization in one helper and call it from all metadata/dispatch surfaces.

## Things I Learned
- The most stable source-of-truth chain for docs parity is:
  1. `Interpreter::get_builtin_names()` for exposed inventory
  2. `Interpreter::native_function_capability(...)` for policy gate mapping
  3. `Interpreter::native_function_arity(...)` for centralized arity labels
- A table-first docs contract test catches both omissions (undocumented runtime API) and stale docs entries (documented but no longer registered).

## Debug Notes (Only if applicable)
- **Failing test / error:** Dead-code warnings for newly added helper methods in `src/interpreter/mod.rs`.
- **Repro steps:** Run `cargo test --test stdlib_reference_contract` after adding helper methods but before using them in native dispatch.
- **Breakpoints / logs used:** Compiler warning output and direct source inspection in `src/interpreter/native_functions/mod.rs`.
- **Final diagnosis:** The warning was expected because methods were not referenced by non-test runtime code; resolved by routing dispatch checks through those helpers.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider generating `docs/STANDARD_LIBRARY.md` from a dedicated repository script to reduce manual drift risk when adding/removing builtins.
- [ ] Evaluate whether additional centralized arity metadata should be added for currently `handler-defined` entries to tighten contracts further.

## Links / References
- Files touched:
  - `docs/STANDARD_LIBRARY.md`
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `tests/stdlib_reference_contract.rs`
  - `README.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `README.md`
  - `ROADMAP.md`
