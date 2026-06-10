# Kujo Field Notes — Set Constructor Dispatch Hardening

**Date:** 2026-02-15
**Session:** 20:53 local
**Branch/Commit:** main / e79b5ff
**Scope:** Closed the next v0.10.0 P1 release-hardening dispatch gap by implementing modular native dispatch for the declared `Set(...)` constructor, then synchronized tests and release docs.

---

## What I Changed
- Added modular native dispatch support for `Set` in `src/interpreter/native_functions/collections.rs`.
- Preserved constructor behavior:
  - `Set()` returns an empty set.
  - `Set([..])` deduplicates elements using `Interpreter::values_equal(...)` semantics.
  - Invalid shape returns explicit `Value::Error` (`non-array` input and `>1` arguments).
- Updated release-hardening contracts in `src/interpreter/native_functions/mod.rs`:
  - added `Set` to critical non-fallback coverage list,
  - removed `Set` from `expected_known_legacy_dispatch_gaps`,
  - added dedicated `Set` constructor contract test.
- Updated release documentation:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Closing one dispatch gap requires updating both coverage lists in the same test module.
  - **Symptom:** `test_release_hardening_builtin_dispatch_coverage_for_declared_builtins` fails even though handler code exists.
  - **Root cause:** `Set` was still present in `expected_known_legacy_dispatch_gaps` after adding modular dispatch support.
  - **Fix:** Remove `Set` from expected known gaps and add `Set` to critical non-fallback coverage in the same edit.
  - **Prevention:** Treat hardening list updates as atomic with each migrated builtin.

- **Gotcha:** `Set(...)` constructor behavior is not just "dispatch exists"; shape + dedup semantics are part of the contract.
  - **Symptom:** A migration can pass unknown-native checks while still regressing constructor behavior.
  - **Root cause:** Dispatch-level tests only prove routing unless constructor-shape cases are asserted explicitly.
  - **Fix:** Added dedicated constructor contract assertions for empty construction, array deduplication, invalid type, and invalid arity.
  - **Prevention:** Add a behavior-level contract test whenever moving constructor-style builtins into modular dispatch.

## Things I Learned
- Release-hardening confidence requires both drift-ledger synchronization and behavior-level API checks.
- Constructor builtins (`Set`, `Queue`, `Stack`) need explicit argument-shape tests during migration, not just non-fallback dispatch probes.
- `Interpreter::values_equal(...)` is the right equality source when enforcing set uniqueness semantics in native handlers.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error: unexpected argument 'test_release_hardening_set_constructor_dispatch_contracts' found`
- **Repro steps:** Run `cargo test test_release_hardening_builtin_dispatch_coverage_for_declared_builtins test_release_hardening_set_constructor_dispatch_contracts -- --nocapture`
- **Breakpoints / logs used:** None; diagnosed from cargo CLI usage output.
- **Final diagnosis:** `cargo test` accepts one test name filter per invocation; run each filtered test in separate commands.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue release-hardening follow-through by migrating next declared gaps (`zip_*`, `load_image`, or `network` APIs).
- [ ] Keep constructor-style builtin migrations paired with explicit argument-shape contract tests.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/collections.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `.github/AGENT_INSTRUCTIONS.md`
