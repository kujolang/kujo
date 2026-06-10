# ER-P1-003 — Type-checker ergonomics hardening

Date: 2026-05-26
Item: ER-P1-003

## Summary

Closed ER-P1-003 with focused type-checker inference improvements, targeted tests, refreshed TODO triage artifacts, and explicit docs for current inference boundaries.

## Code changes

- `src/type_checker.rs`
  - Added `infer_known_method_return_type(...)`.
  - Method-call inference now returns concrete types for known core method surfaces:
    - `string` methods (e.g., `len`, `to_upper`, `contains`, `split`)
    - `array` methods (`len`, `is_empty`, `contains`)
    - `dict` methods (`len`, `contains`, `has_key`)
  - Unknown method surfaces intentionally fall back to `Any`.

## Tests

- Added unit tests:
  - `test_method_call_infers_known_string_method_return_types`
  - `test_method_call_unknown_method_falls_back_to_any`
- Validation:
  - `cargo test type_checker::tests::` -> PASS
  - `cargo test --test v1_code_todo_triage_contract` -> PASS

## Artifacts and docs

- Regenerated:
  - `docs/generated/V1_CODE_TODO_TRIAGE.md`
  - `docs/generated/V1_CODE_TODO_TRIAGE.csv`
- Updated:
  - `docs/OPTIONAL_TYPING_DESIGN.md` with explicit current inference boundary language to avoid implying full static support.
