# V1U-RUN-006 — Command-Level Runtime Path Matrix

Date: 2026-05-20

## Scope

Add command-level runtime-path coverage so maintainers can see which surfaces depend on VM,
interpreter, dual fallback, or runtime-agnostic parse/diagnostics behavior.

## Changes

1. Extended `docs/VM_INTERPRETER_PARITY_MATRIX.md` with a new
   `Command-Level Runtime Path Matrix` section.
2. Added explicit rows for:
   - `kujo run`, `kujo run --interpreter`
   - `kujo test --runtime dual|vm|interpreter`
   - `kujo test-run`
   - runtime security suites
   - diagnostics/runtime-agnostic command modes (`kujo lsp-diagnostics`, `kujo check`)
3. Added `tests/runtime_path_matrix_contract.rs` to lock the required matrix markers and core rows.

## Validation

```bash
cargo test --test runtime_path_matrix_contract
cargo test --test vm_interpreter_parity_surfaces
```

Both commands passed.
