# Kujo Field Notes — Release Hardening Strict-Arity Follow-Through Slices

**Date:** 2026-02-16
**Session:** 20:23 local
**Branch/Commit:** main / 46204ce
**Scope:** Implemented and shipped iterative v0.10 P1 strict-arity hardening slices for assertion/testing APIs, conversion+introspection APIs, math APIs, and data-format/base64 APIs. Added module + dispatcher coverage for each slice and synchronized release docs.

---

## What I Changed
- Hardened assertion/testing extra-argument rejection in `src/interpreter/native_functions/type_ops.rs` for:
  - `assert`, `assert_equal`, `assert_true`, `assert_false`, `assert_contains`
  - Preserved `debug(...)` variadic behavior intentionally
- Hardened conversion/introspection strict-arity in `src/interpreter/native_functions/type_ops.rs` for:
  - `parse_int`, `parse_float`, `to_int`, `to_float`, `to_string`, `to_bool`, `bytes`
  - `type`, `is_int`, `is_float`, `is_string`, `is_bool`, `is_array`, `is_dict`, `is_null`, `is_function`
- Hardened math strict-arity in `src/interpreter/native_functions/math.rs` for:
  - single-arg group: `abs`, `sqrt`, `floor`, `ceil`, `round`, `sin`, `cos`, `tan`, `log`, `exp`
  - two-arg group: `pow`, `min`, `max`
- Hardened data-format/base64 strict-arity in `src/interpreter/native_functions/json.rs` for:
  - `parse_json`, `to_json`, `parse_toml`, `to_toml`, `parse_yaml`, `to_yaml`, `parse_csv`, `to_csv`, `encode_base64`, `decode_base64`
- Expanded dispatcher integration contracts in `src/interpreter/native_functions/mod.rs` for each slice above.
- Updated release docs after each completed slice:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Used incremental commits and pushes per slice (code/tests/docs split).

## Gotchas (Read This Next Time)
- **Gotcha:** Many native handlers already encode compatibility fallbacks for missing/invalid argument shapes.
  - **Symptom:** It is tempting to convert all non-exact argument counts into hard errors while adding strict-arity.
  - **Root cause:** Existing public contracts intentionally mix strict extra-argument rejection with legacy fallback behavior for missing/invalid inputs in some APIs.
  - **Fix:** Harden only trailing-argument paths first (`len() > expected`) where contract says “reject extras”; preserve missing/invalid fallback semantics unless intentionally changing API contract.
  - **Prevention:** Before patching, read existing dispatcher tests in `src/interpreter/native_functions/mod.rs` and mirror their contract shape exactly.

- **Gotcha:** `cargo test` accepts only one test filter per invocation.
  - **Symptom:** Running `cargo test filter_a filter_b` fails with `unexpected argument`.
  - **Root cause:** Cargo CLI supports one positional test name filter; additional filters are treated as invalid arguments.
  - **Fix:** Chain separate invocations (`cargo test filter_a && cargo test filter_b`).
  - **Prevention:** When scripting targeted validation in this repo, run each focused test name as its own command.

## Things I Learned
- Strict-arity follow-through work is fastest when done as “small slice + module test + dispatcher test + docs sync + push”.
- The highest-value guardrail is dispatcher-level integration contracts in `src/interpreter/native_functions/mod.rs`; module tests alone are insufficient for release-hardening confidence.
- A practical rule for this codebase: tighten `extra args` first, keep fallback behavior stable unless changelog/docs explicitly declare a behavior contract change.
- “Justified behavior” that must be documented: keeping `debug(...)` variadic is intentional and should not be swept into strict-arity hardening.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error: unexpected argument 'test_debug_remains_variadic' found`
- **Repro steps:** Run `cargo test test_assertion_api_strict_arity_contracts test_debug_remains_variadic`
- **Breakpoints / logs used:** Shell output only; no code-level debugger required.
- **Final diagnosis:** Command shape was invalid (single-filter cargo test requirement), not a runtime/code regression.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue v0.10 P1 strict-arity follow-through for remaining handlers that still use `first()/get()` without explicit trailing-argument guards.
- [ ] Keep module-level and dispatcher-level strict-arity coverage synchronized whenever a handler contract changes.
- [ ] Re-run `cargo fmt --check` baseline cleanup in a dedicated formatting-focused pass (separate from feature slices).

## Links / References
- Files touched:
  - `src/interpreter/native_functions/type_ops.rs`
  - `src/interpreter/native_functions/math.rs`
  - `src/interpreter/native_functions/json.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
