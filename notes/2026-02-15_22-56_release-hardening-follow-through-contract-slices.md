# Kujo Field Notes — Release Hardening Contract Follow-Through Slices

**Date:** 2026-02-15
**Session:** 22:56 local
**Branch/Commit:** main / fb4e6ae
**Scope:** Implemented iterative v0.10.0 Release Hardening follow-through slices by expanding centralized dispatcher contract coverage and behavior/argument-shape tests in `src/interpreter/native_functions/mod.rs`, then synchronized roadmap/changelog/readme after each completed slice.

---

## What I Changed
- Expanded release-hardening critical-dispatch and compatibility contract coverage for multiple API slices in `src/interpreter/native_functions/mod.rs`:
  - Async alias + SSG contracts
  - Advanced HTTP auth/concurrency contracts
  - Core alias behavior parity contracts
  - Polymorphic `len(...)` contracts
  - `type(...)` + `is_*` introspection contracts
  - Conversion + `bytes(...)` contracts
- Ran focused validation first (targeted `cargo test` filters), then full validation (`cargo build && cargo test`) for each slice.
- Updated user-facing/project-tracking docs after each slice:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Used incremental, atomic commits and pushed after each major step.

## Gotchas (Read This Next Time)
- **Gotcha:** `src/interpreter/native_functions/mod.rs` is a high-churn centralized hardening surface.
  - **Symptom:** Fresh patch chunks can fail or accidentally land in the wrong nearby test block when the file changes between slices.
  - **Root cause:** Many sequential hardening tasks append tests into the same module, and neighboring contract tests share similar structure/text.
  - **Fix:** Re-read the current file before every new patch and anchor edits near exact function names.
  - **Prevention:** Treat each slice as independent: re-open `mod.rs`, patch only the intended test block, and re-run targeted tests immediately.

- **Gotcha:** `cargo fmt` can create unrelated spillover edits in native-function modules.
  - **Symptom:** `git status --short` shows modified files outside the intended slice (for example `async_ops.rs`, `crypto.rs`, `json.rs`, `system.rs`).
  - **Root cause:** Workspace formatting touches additional files during iterative runs.
  - **Fix:** Restore unrelated files before staging (`git restore ...`) and commit only slice-specific changes.
  - **Prevention:** Always run `git status --short` after formatter/test passes and before `git add`.

- **Gotcha:** `Value` does not support direct `assert_eq!` usage in tests.
  - **Symptom:** Compile error `binary operation == cannot be applied to type Value`.
  - **Root cause:** `Value` intentionally does not implement `PartialEq` across the full runtime surface.
  - **Fix:** Assert with `matches!` + explicit shape/content checks.
  - **Prevention:** Use structural assertions from the start when adding contract tests on runtime values.

## Things I Learned
- The highest-leverage release-hardening workflow is: **critical-dispatch list update + targeted contract test + full-suite validation + docs sync + atomic commit**.
- `src/interpreter/native_functions/mod.rs` is effectively the contract ledger for public builtin dispatch drift detection.
- Keeping each hardening slice narrowly scoped and independently committed reduces risk and makes rollback/review straightforward.
- Repeated “justified behavior” that must be treated as intentional:
  - Formatter spillover is expected maintenance overhead in this workspace.
  - Targeted test filters should run before full suite to catch contract regressions fast.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error[E0369]: binary operation == cannot be applied to type Value`
- **Repro steps:** Add `assert_eq!(value_a, value_b)` in `src/interpreter/native_functions/mod.rs` tests and run targeted `cargo test`.
- **Breakpoints / logs used:** Compiler output + immediate local file inspection around failing assertions.
- **Final diagnosis:** `Value` is not `PartialEq`; assertions must use `matches!`/shape checks.

## Follow-ups / TODO (For Future Agents)
- [ ] Add release-hardening contract coverage for `format`, `assert`, `assert_equal`, `assert_true`, `assert_false`, and `assert_contains`.
- [ ] Consider extracting repeated hardening assertion helpers in `mod.rs` tests to reduce boilerplate and patch-conflict risk.
- [ ] Keep `critical_builtin_names` aligned when new public aliases or helper builtins are introduced.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
