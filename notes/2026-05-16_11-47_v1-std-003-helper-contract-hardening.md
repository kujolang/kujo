# Kujo Field Notes — V1-STD-003 Helper Contract Hardening

**Date:** 2026-05-16
**Session:** 11:47 local
**Branch/Commit:** main / ff05d33
**Scope:** Hardened standard-library helper contracts for math, string, collection, time/date, and environment paths to replace silent fallbacks with deterministic runtime errors plus regression coverage.

---

## What I Changed
- Tightened math helper behavior in `src/interpreter/native_functions/math.rs`:
  - Missing/wrong-type arguments now return structured `Value::Error`.
  - Added domain errors for `sqrt(<0)` and `log(<=0)`.
- Tightened core string helper behavior in `src/interpreter/native_functions/strings.rs`:
  - Removed empty-string/empty-array/bool fallback outputs for invalid types in core helpers.
  - Added explicit argument/type errors for `to_upper`/`to_lower`/`trim*`/`char_at`/`substring`/`split`/`join`/`repeat`/`count_chars` and numeric width/length checks for `pad_*`/`truncate`.
- Tightened key collection helper behavior in `src/interpreter/native_functions/collections.rs`:
  - `len`, `push`/`append`, `pop`, `slice`, `concat`, `clear`, and unsupported `remove` shapes now return deterministic errors instead of silent fallback values.
- Hardened date/env parsing in `src/builtins.rs` and `src/interpreter/native_functions/system.rs`:
  - `parse_date` now returns `Result<f64, String>` and enforces `YYYY-MM-DD` format with explicit parse errors.
  - `env_bool` now accepts explicit true/false token sets and errors on invalid values.
- Updated compatibility path in `src/interpreter/legacy_full.rs` for new `parse_date` result shape.
- Added/updated regression tests across:
  - `src/interpreter/native_functions/math.rs`
  - `src/interpreter/native_functions/strings.rs`
  - `src/interpreter/native_functions/collections.rs`
  - `src/interpreter/native_functions/system.rs`
  - `src/interpreter/native_functions/mod.rs`

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo test` only accepts one positional filter.
  - **Symptom:** `cargo test` rejected a command with multiple test names.
  - **Root cause:** Cargo CLI supports a single positional `TESTNAME` filter.
  - **Fix:** Run multiple `cargo test <filter>` commands (or one broader filter) instead of passing many positional test names.
  - **Prevention:** Keep targeted test invocations one-filter-per-command.

- **Gotcha:** `cargo fmt` can reflow unrelated files.
  - **Symptom:** Unrelated formatting-only diffs appeared in `tests/native_json.rs` and `tests/stdlib_reference_contract.rs`.
  - **Root cause:** Formatter touched existing long assertions while formatting changed files.
  - **Fix:** Reverted unrelated files with `git restore`.
  - **Prevention:** Always check `git diff --stat` after `cargo fmt` and revert unrelated formatting churn for scoped roadmap work.

## Things I Learned
- The current native dispatch has centralized arity checks for many builtins, but several helper modules still need explicit type/domain validation to avoid silent fallbacks.
- `parse_date` previously used `0.0` fallback semantics that collide with valid epoch output; converting to `Result` is the clean fix for unambiguous error behavior.
- `env_bool` coercing any unknown value to `false` hides configuration mistakes; explicit token parsing (`true/false/1/0/yes/no/on/off`) is a safer contract.

## Debug Notes (Only if applicable)
- **Failing test / error:** `cargo test` initially failed in legacy contract tests expecting old fallback behavior (`Int(0)` / `Bool(false)` / `Float(0.0)`).
- **Repro steps:** Run `cargo test` after helper-hardening changes.
- **Breakpoints / logs used:** Updated failing assertions in `src/interpreter/native_functions/mod.rs` to align with hardened runtime contracts.
- **Final diagnosis:** Behavior changes were correct; stale tests encoded legacy fallback semantics and needed contract updates.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider tightening remaining collection helper fallback paths (especially dense-dict `remove` missing-key sentinel behavior) if roadmap scope expands further.
- [ ] Evaluate whether adding full per-helper contract tables in `docs/STANDARD_LIBRARY.md` would reduce future drift for type/domain edge cases.

## Links / References
- Files touched:
  - `src/builtins.rs`
  - `src/interpreter/legacy_full.rs`
  - `src/interpreter/native_functions/math.rs`
  - `src/interpreter/native_functions/strings.rs`
  - `src/interpreter/native_functions/collections.rs`
  - `src/interpreter/native_functions/system.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `ROADMAP.md`
