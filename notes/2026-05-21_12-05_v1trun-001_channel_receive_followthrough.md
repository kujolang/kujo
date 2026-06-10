# V1TRUN-001 - Channel Receive Runtime TODO Follow-through

Date: 2026-05-21
Checklist item: `V1TRUN-001`

## Implementation Summary

- Replaced non-blocking `Channel.receive` behavior in active interpreter method-call paths with a lock-safe blocking receive loop.
- Added shared channel helpers in `Interpreter` (`channel_send`, `channel_receive_blocking`) to keep channel send/receive behavior consistent across both method dispatch paths.
- Removed the production-path TODO for blocking receive semantics.

## Test Coverage Added

In `tests/interpreter_tests.rs`:
- `test_channel_receive_blocks_until_value_is_sent` (regression: receive unblocks on delayed sender thread)
- `test_channel_receive_and_send_preserve_fifo_order` (success path ordering)
- `test_channel_receive_arity_error_via_expression_method_call` (failure path)

## Commands Run

1. `cargo test --test interpreter_tests channel_receive` -> PASS
2. `cargo test --test vm_interpreter_parity_surfaces` -> PASS
3. `cargo test` -> PASS

## Outcome

`V1TRUN-001` is complete; active runtime-path channel receive behavior now blocks for values instead of returning immediate `null` on empty channel.
