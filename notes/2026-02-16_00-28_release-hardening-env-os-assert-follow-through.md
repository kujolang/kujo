# Kujo Field Notes — Release hardening follow-through for env, os/path, and assertions

**Date:** 2026-02-16
**Session:** 00:28 local
**Branch/Commit:** main / a112ad9
**Scope:** Continued iterative v0.10 release-hardening by adding dispatcher-critical coverage and comprehensive contract tests for env/system APIs, os/path APIs, and assertion/testing builtins. Also synchronized release docs and tracked remaining uncovered builtin count.

---

## What I Changed
- Added a new release-hardening contract test in `src/interpreter/native_functions/mod.rs`:
  - `test_release_hardening_env_os_path_and_assert_contracts`
- Expanded critical dispatcher coverage list in `test_release_hardening_builtin_dispatch_coverage_for_recent_apis` for:
  - env/system: `env`, `env_or`, `env_int`, `env_float`, `env_bool`, `env_required`, `env_set`, `env_list`, `args`, `arg_parser`
  - os/path: `os_getcwd`, `os_chdir`, `os_rmdir`, `os_environ`, `dirname`, `basename`, `path_exists`, `path_absolute`, `path_is_dir`, `path_is_file`, `path_extension`
  - assertions/testing: `assert`, `debug`, `assert_equal`, `assert_true`, `assert_false`, `assert_contains`
- Added filesystem-backed contract checks in test flow using workspace-local temp paths under `tmp/`.
- Updated release docs for the completed slice:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Recomputed hardening coverage delta (declared builtin list vs critical hardening list) and tracked updated remaining count.

## Gotchas (Read This Next Time)
- **Gotcha:** `env_bool(...)` does not error on unrecognized values; it resolves to `false`.
  - **Symptom:** Contract test expected `Value::ErrorObject` for non-boolean env string, but test failed with `Value::Bool(false)`.
  - **Root cause:** `builtins::env_bool` returns `Ok(matches!(...))`, where only truthy literals map to `true`; everything else maps to `false`.
  - **Fix:** Updated contract assertion to expect `Value::Bool(false)` for invalid/non-truthy env value.
  - **Prevention:** Treat `env_bool` as permissive boolean parsing (truthy set vs default false), not strict parse validation.

- **Gotcha:** Comparing `Value::Str(Arc<String>)` with `String` directly inside `matches!` can fail type checks.
  - **Symptom:** Rust compile error `E0277`: can't compare `&String` with `String` in assertion guard.
  - **Root cause:** Pattern binding yields borrowed `Arc<String>`; guard compared mismatched reference/value types.
  - **Fix:** Compare with `path.as_ref().as_str()` against `String`/`&str` rather than direct `Arc<String>` vs `String` equality.
  - **Prevention:** In `Value::Str` test guards, normalize to `&str` before equality checks.

## Things I Learned
- Release-hardening contract tests should lock current runtime semantics, even when semantics are permissive (`env_bool` false fallback) rather than strict.
- For these contract slices, keep side effects deterministic by using workspace-local temp dirs and restoring CWD inside the same test.
- The critical dispatcher coverage list is effectively a migration ledger; every new contract slice should add names there or drift tracking loses value.
- Coverage accounting is best done with direct list-diff extraction from:
  - `Interpreter::get_builtin_names()` in `src/interpreter/mod.rs`
  - `critical_builtin_names` in `src/interpreter/native_functions/mod.rs`

## Debug Notes (Only if applicable)
- **Failing test / error:**
  - `assertion failed: matches!(env_bool_bad_parse, Value::ErrorObject { .. })`
  - Rust compile error before that: `E0277` (`&String` vs `String` comparison in `matches!` guard)
- **Repro steps:**
  - `cargo test test_release_hardening_env_os_path_and_assert_contracts -- --nocapture`
- **Breakpoints / logs used:**
  - Read `src/builtins.rs` implementation of `env_bool` to confirm parse contract.
  - Used failing assertion location in `src/interpreter/native_functions/mod.rs` and adjusted expectations.
- **Final diagnosis:**
  - Test expectation was wrong for `env_bool`; implementation intentionally treats non-truthy values as `false` and only errors when env var is missing.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue next hardening slice for remaining uncovered collection/higher-order APIs (`map`, `filter`, `reduce`, `find`, `sort`, `zip`, `chunk`, etc.).
- [ ] Keep recomputing declared-vs-critical builtin delta after each slice to show measurable progress.
- [ ] If strict bool parsing is desired in future, treat it as behavior change and document migration impact (do not change silently in hardening-only slices).

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
