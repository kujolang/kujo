# Kujo Field Notes — Spawn parent-binding snapshot for concurrency

**Date:** 2026-02-14
**Session:** 17:08 local
**Branch/Commit:** main / cb3a206
**Scope:** Implemented the next highest-priority incomplete roadmap item for concurrency by enabling transferable parent-binding snapshots in `spawn` workers, then added integration coverage and updated user-facing docs.

---

## What I Changed
- Added spawn-time parent binding capture + transfer in `src/interpreter/mod.rs`:
  - Introduced `SpawnCapturedValue` and conversion logic to transfer only supported value shapes.
  - Added `capture_spawn_bindings()` and preloaded captured bindings into the spawned interpreter before `eval_stmts(...)`.
- Added integration tests in `tests/interpreter_tests.rs`:
  - `test_spawn_can_read_parent_scalar_bindings_snapshot`
  - `test_spawn_can_use_parent_defined_shared_key_variable`
  - `test_spawn_snapshot_mutations_do_not_write_back_to_parent_scope`
- Updated docs to reflect completion and behavior:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Committed in atomic steps and pushed:
  - `:package: NEW: capture transferable parent bindings for spawn`
  - `:ok_hand: IMPROVE: add spawn parent-binding capture integration tests`
  - `:book: DOC: document spawn parent-binding snapshot concurrency milestone`

## Gotchas (Read This Next Time)
- **Gotcha:** `spawn` parent visibility is snapshot-based, not shared mutable state.
  - **Symptom:** Spawn workers can read parent-defined values, but parent variables do not reflect spawned reassignment updates.
  - **Root cause:** Spawn creates a new interpreter and copies transferable values at spawn time; environments are not shared references.
  - **Fix:** Use parent binding snapshot only for worker inputs; use `shared_set/get/has/delete/add_int` for cross-thread coordination and observed updates.
  - **Prevention:** Treat spawn-captured values as immutable-by-copy semantics relative to parent scope. Do not assume lexical write-through.

- **Gotcha:** Not all runtime `Value` variants are safe/portable to transfer into `spawn` workers.
  - **Symptom:** Designing capture for arbitrary values risks thread-safety problems or non-transferable resources crossing thread boundaries.
  - **Root cause:** Kujo `Value` includes runtime handles/resources (promises, DB handles, channels, images, task handles, etc.) that should not be blindly cloned across interpreter threads.
  - **Fix:** Capture only explicitly supported transferable variants in `SpawnCapturedValue::from_value(...)`; skip unsupported variants.
  - **Prevention:** When adding new `Value` variants, explicitly decide whether they are spawn-transferable. If unknown, keep them non-transferable by default.

- **Gotcha:** `cargo test` accepts one name filter argument, not multiple positional test names.
  - **Symptom:** `cargo test <name1> <name2>` fails with “unexpected argument”.
  - **Root cause:** Cargo CLI expects a single test-name filter token (plus optional `--` test harness args).
  - **Fix:** Use one filter at a time or use a shared substring filter (`cargo test --test interpreter_tests spawn_can_`).
  - **Prevention:** For targeted runs, prefer module/test-file + one substring filter.

## Things I Learned
- Rule: `spawn` in Kujo is now “parent input capture + isolated execution + explicit shared-state APIs,” not full lexical environment sharing.
- Parent-variable-derived shared keys are now practical and cleaner than hardcoded shared store keys in spawn-heavy code.
- The correct safety boundary is explicit transferability per value variant; this avoids subtle thread/resource bugs.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error: unexpected argument 'test_spawn_can_use_parent_defined_shared_key_variable' found`
- **Repro steps:** Run `cargo test test_spawn_can_read_parent_scalar_bindings_snapshot test_spawn_can_use_parent_defined_shared_key_variable ...`
- **Breakpoints / logs used:** CLI output only; switched to one-filter command form.
- **Final diagnosis:** Cargo positional filtering was used incorrectly; tests themselves were green once run with valid filter usage.

## Follow-ups / TODO (For Future Agents)
- [ ] Evaluate whether function/closure capture semantics in `spawn` should support additional safe variant(s) or remain intentionally restricted.
- [ ] Consider exposing an explicit language-level note/spec text describing snapshot capture vs parent write-back isolation.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `tests/interpreter_tests.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
