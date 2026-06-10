# Kujo Field Notes — Release hardening for Set/Queue/Stack contracts

**Date:** 2026-02-16
**Session:** 00:46 local
**Branch/Commit:** main / 44b6040
**Scope:** Expanded release-hardening coverage for collection constructor/method APIs (Set/Queue/Stack), added comprehensive dispatcher/runtime contract tests, and synchronized changelog/roadmap/readme documentation.

---

## What I Changed
- Expanded critical dispatch hardening coverage list in `src/interpreter/native_functions/mod.rs` to include Set/Queue/Stack constructor and method APIs.
- Added `test_release_hardening_set_queue_stack_method_contracts` in `src/interpreter/native_functions/mod.rs` with behavior and argument-shape coverage for:
  - `set_add`, `set_has`, `set_remove`, `set_union`, `set_intersect`, `set_difference`, `set_to_array`
  - `Queue`, `queue_enqueue`, `queue_dequeue`, `queue_peek`, `queue_is_empty`, `queue_to_array`
  - `Stack`, `stack_push`, `stack_pop`, `stack_peek`, `stack_is_empty`, `stack_to_array`
  - explicit type checks for `queue_size(...)` and `stack_size(...)`
- Updated `CHANGELOG.md`, `ROADMAP.md`, and `README.md` with a new Release Hardening follow-through slice for Set/Queue/Stack APIs.
- Created three incremental commits instead of one combined commit:
  - dispatch-coverage expansion
  - comprehensive contract tests
  - documentation updates

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo test` accepts only one positional test-name filter.
  - **Symptom:** Running `cargo test test_a test_b -- --nocapture` failed with `unexpected argument 'test_b' found`.
  - **Root cause:** Cargo CLI treats the positional test name as a single optional filter, not a list.
  - **Fix:** Run filtered tests sequentially (`cargo test test_a ... && cargo test test_b ...`) or use one regex/name filter.
  - **Prevention:** For multi-test targeted validation, chain separate `cargo test` commands; do not pass multiple positional test names.

- **Gotcha:** `cargo fmt` reflowed unrelated lines in large native-function test modules.
  - **Symptom:** Focused test work in `src/interpreter/native_functions/mod.rs` also produced formatting churn in unrelated files and broad in-file reflow noise.
  - **Root cause:** Formatter pass applies globally and can normalize long assertions/layout broadly.
  - **Fix:** Restored unrelated files with `git restore ...` and committed only the intended slice.
  - **Prevention:** Always run `git status --short` immediately after `cargo fmt`; explicitly de-scope non-feature files before staging.

## Things I Learned
- Release-hardening coverage is now broad enough that adding a new API slice should include both dispatcher critical-list coverage and deep behavior/shape contracts in the same session.
- For Kujo `Value` assertions, shape-based `matches!` checks remain the stable pattern; direct equality still isn’t the right strategy for many composite/runtime variants.
- Keeping hardening work split into `feature -> tests -> docs` commits makes backtracking/review much easier than one monolithic commit.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error: unexpected argument 'test_release_hardening_set_queue_stack_method_contracts' found`
- **Repro steps:** Run `cargo test test_release_hardening_builtin_dispatch_coverage_for_recent_apis test_release_hardening_set_queue_stack_method_contracts -- --nocapture`
- **Breakpoints / logs used:** CLI output only (no runtime breakpoints needed)
- **Final diagnosis:** Cargo supports a single positional test-name filter; this was a command-shape issue, not a runtime failure.

## Follow-ups / TODO (For Future Agents)
- [ ] Add equivalent release-hardening contract slices for any newly introduced builtins/aliases immediately after registration/dispatch changes.
- [ ] Consider moving oversized hardening tests in `src/interpreter/native_functions/mod.rs` into smaller module-local test files if review noise increases.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/2026-02-16_00-46_release-hardening-set-queue-stack-contracts.md`
  - `notes/README.md`
  - `notes/GOTCHAS.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
