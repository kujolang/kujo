# V1TRUN-002 - Iterator TODO Follow-through (filter/map/generator-next)

Date: 2026-05-21
Checklist item: `V1TRUN-002`

## Implementation Summary

- Replaced TODO placeholders for iterator `filter`/`map` chaining by layering deterministic iterator stages that preserve method-call order.
- Added nested-iterator source materialization support in iterator runtime paths so chained stages execute correctly for both `.collect()` and `.next()`.
- Fixed generator-backed iterator `.next()` filtering behavior to continue scanning yields after filtered-out values instead of prematurely returning `None`.
- Regenerated `docs/generated/V1_CODE_TODO_TRIAGE.md` and `docs/generated/V1_CODE_TODO_TRIAGE.csv` after removing active-runtime TODO markers from `src/interpreter/mod.rs`.

## Tests Added/Updated

Added in `tests/interpreter_tests.rs`:
- `test_iterator_filter_chain_preserves_both_predicates`
- `test_iterator_map_chain_applies_transformers_in_order`
- `test_generator_iterator_next_skips_filtered_items_until_match`

Updated in `tests/v1_code_todo_triage_contract.rs`:
- Contract expectations now match the updated triage state after `src/interpreter/mod.rs` runtime TODO removal.

## Commands Run

1. `cargo test --test interpreter_tests iterator_` -> PASS
2. `cargo test --test vm_interpreter_parity_surfaces` -> PASS
3. `cargo test --test v1_code_todo_triage_contract` -> PASS
4. `cargo test` -> PASS

## Outcome

`V1TRUN-002` is complete. Iterator chaining TODOs and generator-next filter TODO behavior in active runtime paths are closed with regression coverage.
