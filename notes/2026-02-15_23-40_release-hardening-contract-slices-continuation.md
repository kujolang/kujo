# Kujo Field Notes — Release hardening contract slices continuation

**Date:** 2026-02-15
**Session:** 23:40 local
**Branch/Commit:** main / 1d5e8a8
**Scope:** Continued v0.10 release-hardening follow-through by adding dispatcher critical coverage and compatibility contract tests for string utility/transform, random-time-date, math, and collections/format builtins. Kept roadmap/changelog/readme synchronized and used incremental feature+docs commits for each slice.

---

## What I Changed
- Expanded `test_release_hardening_builtin_dispatch_coverage_for_recent_apis` in `src/interpreter/native_functions/mod.rs` across four iterative slices:
  - String utility APIs (`starts_with`, `ends_with`, `repeat`, `char_at`, `is_empty`, `count_chars`)
  - String transform/tokenization APIs (`substring`, `capitalize`, `trim`, `trim_start`, `trim_end`, `split`, `join`)
  - Random/time/date APIs (`random`, `random_int`, `random_choice`, `set_random_seed`, `clear_random_seed`, `now`, `current_timestamp`, `performance_now`, `time_us`, `time_ns`, `format_duration`, `elapsed`, `format_date`, `parse_date`)
  - Math APIs (`abs`, `sqrt`, `pow`, `floor`, `ceil`, `round`, `min`, `max`, `sin`, `cos`, `tan`, `log`, `exp`)
  - Collections/format APIs (`range`, `keys`, `values`, `items`, `has_key`, `get`, `merge`, `invert`, `update`, `get_default`, `format`)
- Added dedicated contract tests in `src/interpreter/native_functions/mod.rs`:
  - `test_release_hardening_string_utility_behavior_and_fallback_contracts`
  - `test_release_hardening_string_transform_and_tokenization_contracts`
  - `test_release_hardening_system_random_and_time_contracts`
  - `test_release_hardening_math_behavior_and_fallback_contracts`
  - `test_release_hardening_collections_and_format_contracts`
- Updated release documentation for each slice:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Validated each slice with targeted tests and full `cargo build` + `cargo test`.

## Gotchas (Read This Next Time)
- **Gotcha:** `Value` does not implement `PartialEq`, so `assert_eq!(Value, Value)` fails at compile time in Rust tests.
  - **Symptom:** `error[E0369]: binary operation == cannot be applied to type interpreter::value::Value`
  - **Root cause:** `Value` enum intentionally lacks `PartialEq` derive due to complex runtime variants.
  - **Fix:** Replace direct equality with pattern matching and variant-specific value assertions.
  - **Prevention:** In native-function contract tests, use `matches!` or manual `match` blocks for structural comparisons.

- **Gotcha:** `format(...)` uses printf-style placeholders (`%s`, `%d`, `%f`), not `{}` interpolation.
  - **Symptom:** Contract test for `format("Hello {}, ...")` failed despite correct args.
  - **Root cause:** `builtins::format_string` in `src/builtins.rs` parses `%` tokens only.
  - **Fix:** Updated tests to use `%s`/`%d` templates.
  - **Prevention:** Treat brace-style format strings as invalid assumptions unless implementation changes in `format_string`.

- **Gotcha:** Full-suite `cargo test` can show transient failure in `interpreter::async_runtime::tests::test_concurrent_tasks` and pass on isolated rerun.
  - **Symptom:** Full run reported one async runtime failure while targeted rerun passed immediately.
  - **Root cause:** Timing/scheduling sensitivity under full-suite load (likely environmental flake, not deterministic regression in unrelated slice).
  - **Fix:** Re-ran the failing test in isolation, then re-ran full suite to confirm green state.
  - **Prevention:** For unrelated async failures, verify with isolated repro before attributing to feature slice; then require a subsequent full-suite pass.

## Things I Learned
- Release-hardening value comes from contracting existing compatibility behavior, not inventing stricter semantics mid-release.
- For this codebase, “fallback behavior is expected” is often a compatibility contract and must be explicitly tested (e.g., math invalid-shape returns `Int(0)`).
- Deterministic tests for random APIs should use seed-reset parity (`set_random_seed` twice + same call sequence) instead of asserting specific raw random values.
- `keys`/`values`/`items` order should be asserted with sorted deterministic expectations for dict behavior contracts.

## Debug Notes (Only if applicable)
- **Failing test / error:**
  - `error[E0369]: binary operation == cannot be applied to type interpreter::value::Value`
  - Panic in `test_release_hardening_collections_and_format_contracts` for `format(...)` expected output.
  - Full-suite transient failure: `interpreter::async_runtime::tests::test_concurrent_tasks ... FAILED`
- **Repro steps:**
  - `cargo test test_release_hardening_system_random_and_time_contracts -- --nocapture`
  - `cargo test test_release_hardening_collections_and_format_contracts -- --nocapture`
  - `cargo test`
- **Breakpoints / logs used:**
  - Inspected `src/builtins.rs::format_string` implementation and placeholder parser.
  - Isolated async failure with `cargo test interpreter::async_runtime::tests::test_concurrent_tasks -- --nocapture`.
- **Final diagnosis:**
  - `Value` comparison needed structural assertions, not `assert_eq!`.
  - `format(...)` contract is printf-style placeholders.
  - Async runtime full-suite failure was transient and not tied to collection/format hardening logic.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider documenting `format(...)` placeholder style (`%s/%d/%f`) more prominently in user-facing language docs/examples.
- [ ] Investigate potential flakiness in `interpreter::async_runtime::tests::test_concurrent_tasks` under full-suite contention.
- [ ] Continue next release-hardening follow-through slice for uncovered public builtins not yet in critical dispatcher list.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/2026-02-15_23-40_release-hardening-contract-slices-continuation.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/README.md`
  - `notes/GOTCHAS.md`
