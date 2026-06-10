# Kujo Field Notes — Release hardening: builtin alias/API contract stabilization

**Date:** 2026-02-15
**Session:** 09:18 local
**Branch/Commit:** main / 1f4c9e2
**Scope:** Hardened external builtin API stability for v0.10.0 by fixing VM/interpreter builtin list drift, restoring missing modular handlers, and adding regression tests for alias/path/collection API contracts.

---

## What I Changed
- Synced VM-visible builtin list and interpreter registration parity in `src/interpreter/mod.rs`.
- Removed duplicate builtin names from `Interpreter::get_builtin_names()` in `src/interpreter/mod.rs`.
- Added missing native handlers in modular dispatcher:
  - `src/interpreter/native_functions/filesystem.rs` for OS/path APIs (`os_getcwd`, `os_chdir`, `os_rmdir`, `os_environ`, `join_path`, `path_join`, `dirname`, `basename`, `path_exists`, `path_absolute`, `path_is_dir`, `path_is_file`, `path_extension`).
  - `src/interpreter/native_functions/collections.rs` for `queue_size` and `stack_size`.
- Added/expanded integration hardening tests in `tests/interpreter_tests.rs`:
  - required builtin contract entries present
  - no duplicates in builtin list
  - alias parity (`to_upper`/`upper`, `to_lower`/`lower`, `replace_str`/`replace`)
  - path alias/core behavior
  - queue/stack size behavior.
- Updated release docs in `CHANGELOG.md`, `ROADMAP.md`, and `README.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** Builtin declaration and actual execution support can drift after modularization.
  - **Symptom:** Builtin appears in `get_builtin_names()` / `register_builtins()`, but runtime behavior is wrong or unavailable.
  - **Root cause:** The actual runtime dispatch is `native_functions::call_native_function(...)`; declarations in `mod.rs` alone do not implement behavior.
  - **Fix:** Add handler branches in the correct native module (`filesystem.rs`, `collections.rs`, etc.) and keep declarations in sync.
  - **Prevention:** Treat builtin addition as a 3-surface checklist: `get_builtin_names` + `register_builtins` + native module handler.

- **Gotcha:** Unknown builtin fallback is not loudly failing in modular dispatcher.
  - **Symptom:** Missing handler can silently degrade into unexpected return values instead of explicit “unknown function” errors.
  - **Root cause:** `src/interpreter/native_functions/mod.rs` returns `Value::Int(0)` for unknown names.
  - **Fix:** Add missing handler branches for declared APIs; add behavior tests for high-risk/alias APIs.
  - **Prevention:** Prefer contract tests that execute builtins, not just name-list checks.

- **Gotcha:** API list uniqueness matters for release hardening, not just correctness.
  - **Symptom:** New duplicate-check test failed (`left: 296`, `right: 294`).
  - **Root cause:** Duplicate names (`clear`, `remove`) were present in `get_builtin_names()`.
  - **Fix:** Removed duplicate entries.
  - **Prevention:** Keep a no-duplicate test on `Interpreter::get_builtin_names()` and run it in CI.

## Things I Learned
- `Interpreter::get_builtin_names()` is a compatibility surface (VM initialization + user expectations), not just an internal helper.
- In this codebase, alias stability is part of API stability; aliases must be tested as first-class behavior.
- “Looks registered” is not a valid acceptance criterion for builtins after modular dispatch split; execution-path tests are required.
- Path/OS APIs are especially prone to drift because names stayed in declarations while handlers moved into category modules.

## Debug Notes (Only if applicable)
- **Failing test / error:** `assertion 'left == right' failed: Duplicate names found in builtin API list (left: 296 right: 294)`.
- **Repro steps:**
  - `cargo test test_builtin_names_do_not_contain_duplicates`
- **Breakpoints / logs used:**
  - Scripted extraction + diff of builtin name lists from `src/interpreter/mod.rs`.
  - Checked native module handler coverage in `src/interpreter/native_functions/*.rs`.
- **Final diagnosis:** Declared builtin API had duplicate names and several declared names without modular handler implementations.

## Follow-ups / TODO (For Future Agents)
- [ ] Add signature/argument-shape contract tests for high-traffic APIs (async, filesystem, collections).
- [ ] Consider changing unknown-native fallback in `src/interpreter/native_functions/mod.rs` from `Value::Int(0)` to explicit error for easier drift detection.
- [ ] Add automated parity tooling that compares declared names against modular handler match arms.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/filesystem.rs`
  - `src/interpreter/native_functions/collections.rs`
  - `tests/interpreter_tests.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
