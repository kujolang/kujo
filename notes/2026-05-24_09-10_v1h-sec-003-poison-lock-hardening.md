# V1H-SEC-003 Poison-Lock Hardening

Date: 2026-05-24
Item: `V1H-SEC-003`

## Scope

Replace panic-prone `lock().unwrap()` in production native surfaces (`network`, `database`, `concurrency`) with poison-aware error propagation.

## Changes

- `src/interpreter/native_functions/network.rs`
  - Added `lock_or_network_error` helper.
  - Replaced all network-surface `lock().unwrap()` callsites with helper-backed `Result` handling.
- `src/interpreter/native_functions/database.rs`
  - Added `lock_or_db_error!` macro.
  - Replaced all database-surface `lock().unwrap()` callsites with poison-aware error returns.
- `src/interpreter/native_functions/concurrency.rs`
  - Added `lock_or_concurrency_error` helper.
  - Replaced all concurrency-surface `lock().unwrap()` callsites with helper-backed error propagation.

## Verification

- Callsite inventory:
  - `rg -n "lock\(\)\.unwrap\(\)" src/interpreter/native_functions/network.rs src/interpreter/native_functions/database.rs src/interpreter/native_functions/concurrency.rs`
  - Result: no matches.
- Focused tests:
  - `cargo test --lib test_db_connect_execute_query_close_sqlite` (pass)
  - `cargo test --lib test_db_transaction_begin_commit_and_rollback_sqlite` (pass)
  - `cargo test --lib test_release_hardening_shared_state_and_task_pool_contracts` (pass)
  - `cargo test --lib test_release_hardening_network_module_dispatch_argument_contracts` (pass)
  - `cargo test --lib test_release_hardening_network_module_strict_arity_contracts` (pass)
  - `cargo test --lib test_release_hardening_network_module_size_limit_contracts` (pass)
  - `cargo test --lib test_release_hardening_network_module_round_trip_behaviors` (pass)
- Required security suites:
  - `cargo test --test runtime_security` (9 passed)
  - `cargo test --test native_api_security_boundaries` (48 passed)
- Parity sweep:
  - `cargo test --test vm_interpreter_parity_surfaces` (87 passed)

## Residual Follow-up

- Repository-wide `lock().unwrap()` occurrences remain in non-native-surface areas (`src/main.rs`, `src/builtins.rs`) and are outside the explicit `V1H-SEC-003` native-surface scope.
