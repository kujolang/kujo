# ER-P0-001 — Verification matrix refresh

Date: 2026-05-25
Item: ER-P0-001

## Summary

Re-ran core failing matrix gates on latest `main` to refresh blocker evidence.

## Commands and results

- `cargo test --test unsafe_inventory_contract` -> FAILED
  - `unsafe_inventory_enforces_current_executable_budget`
  - `expected <=55, got 59`
- `cargo run -- test --runtime vm` -> PASS summary `137/150`
- `cargo run -- test --runtime dual` -> PASS summary `136/150` with fixture drift still present
  - notable additional drift signal in dual: `tests/stdlib_test.kujo`

## Blocker status

`ER-P0-001` remains blocked pending closure of:
1. `ER-P0-006` unsafe executable-budget reduction (`59 -> <=55`).
2. `ER-P0-003` runtime parity/parser-fixture debt cleanup so VM/dual sweeps pass fully.
