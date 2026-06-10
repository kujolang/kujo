# Kujo Field Notes — V1-STD-002 JSON hardening

**Date:** 2026-05-15
**Session:** 17:35 local
**Branch/Commit:** main / 1092a6a
**Scope:** Implemented roadmap item `V1-STD-002` by hardening JSON parse/stringify behavior with explicit bounds, deterministic serialization, and regression coverage.

---

## What I Changed
- Hardened JSON builtins in `src/builtins.rs`:
  - Added parse bounds: max input size `1,048,576` bytes and max nesting depth `64`.
  - Added explicit nesting validation for parsed `serde_json::Value`.
  - Rejected non-finite floats (`NaN`, `+/-inf`) in `to_json(...)` instead of silently coercing.
  - Made dictionary-like JSON serialization deterministic by sorting keys.
  - Removed the silent JSON number fallback that could coerce unsupported cases to sentinel values.
- Added dedicated regression coverage in `tests/native_json.rs`:
  - Root-type parse success: object/array/string/number/bool/null.
  - Invalid JSON location errors.
  - Size/depth limit failures.
  - Primitive+nested stringify success.
  - Unsupported-type failures.
  - Non-finite float failures.
- Updated docs for the new JSON contract:
  - `docs/STANDARD_LIBRARY.md`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md` (marked `V1-STD-002` complete with verification notes)

## Gotchas (Read This Next Time)
- **Gotcha:** `serde_json::Number::from_f64(...).unwrap_or(...)` silently converts non-finite floats into fallback values when used carelessly.
  - **Symptom:** `to_json(NaN)` returned `"0"` instead of failing.
  - **Root cause:** JSON number conversion used a fallback sentinel path (`unwrap_or_else`) instead of rejecting non-finite floats.
  - **Fix:** Reject non-finite floats up front and return an explicit JSON conversion error.
  - **Prevention:** Never use sentinel fallback conversion for numeric serialization contracts; reject invalid numeric states explicitly.
- **Gotcha:** Kujo `DictMap` is hash-map-backed, so direct iteration order is not a deterministic JSON contract.
  - **Symptom:** JSON object output key order can drift between runs/platforms when serializing dictionary-like values directly from map iteration.
  - **Root cause:** Hash-map iteration order is not a stable external API contract.
  - **Fix:** Sort keys before building `serde_json::Map` for object serialization.
  - **Prevention:** Any user-visible serialized output surface should define explicit ordering policy.

## Things I Learned
- JSON hardening is best handled in `src/builtins.rs` so every call site (`parse_json`, `to_json`, JWT payload conversion, OAuth token JSON paths) benefits from one contract.
- A dedicated integration test file (`tests/native_json.rs`) is a cleaner boundary for stdlib behavior than relying only on broad dispatch tests.
- `cargo test` can fail intermittently in socket-backed serve integration due startup readiness races; rerun is useful to distinguish flaky infra from logic regressions.

## Debug Notes (Only if applicable)
- **Failing test / error:** `tests/native_json.rs` initially failed on three regressions:
  - missing parse size limit
  - missing parse depth limit
  - `to_json(NaN)` returning `"0"`
- **Repro steps:** `cargo test --test native_json`
- **Breakpoints / logs used:** Focused on `parse_json`, `to_json`, and conversion helpers in `src/builtins.rs`.
- **Final diagnosis:** JSON handling had no explicit parse bounds and used non-finite float fallback coercion during serialization.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider moving `parse_json` / `to_json` arity from handler-local checks into `Interpreter::native_callable_arity` metadata for fully centralized arity reporting.
- [ ] If large JSON payload support is needed later, add explicit CLI/runtime configuration for JSON size/depth limits instead of ad hoc constant changes.

## Links / References
- Files touched:
  - `src/builtins.rs`
  - `tests/native_json.rs`
  - `docs/STANDARD_LIBRARY.md`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/STANDARD_LIBRARY.md`
