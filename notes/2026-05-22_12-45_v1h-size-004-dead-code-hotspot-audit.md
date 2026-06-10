# V1H-SIZE-004 - Dead Code Hotspot Audit

Date: 2026-05-22
Item: V1H-SIZE-004

## Summary

Completed a dead-code allowance hotspot audit and classified major concentration files into keep/defer/remove buckets.

Applied low-risk production cleanup:
- `src/path_security.rs`: converted `reject_url_encoded_parent_traversal` from runtime `#[allow(dead_code)]` to `#[cfg(test)] pub(crate)` because it is only referenced by unit tests.

## Evidence

- Inventory/classification doc: `docs/generated/DEAD_CODE_ALLOW_HOTSPOT_AUDIT.md`
- Checklist row updated with validation commands and rationale.

## Validation

- `cargo test reject_url_encoded_parent_traversal`
- `cargo test path_security`
- `cargo test --test vm_interpreter_parity_surfaces`
