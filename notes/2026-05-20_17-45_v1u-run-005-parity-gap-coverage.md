# V1U-RUN-005 — Parity-Gap Coverage Audit

Date: 2026-05-20

## Scope

For each interpreter-flag dependency tagged `parity-gap`, ensure there is either
parity coverage or explicit documented divergence.

## Changes

1. Extended `scripts/generate_interpreter_flag_dependency_map.sh` with:
   - explicit `parity-gap` reason tag semantics,
   - generated `V1U-RUN-005` parity-gap status section,
   - generated closure-evidence pointers for tagged surfaces.
2. Tagged `src/parser.rs` interpreter fallback usage as `harness-legacy,parity-gap`.
3. Updated `tests/interpreter_flag_dependency_map_contract.rs` to enforce:
   - parity-gap markers are present in generated output,
   - `src/parser.rs` parity-gap tagging is preserved,
   - interpreter fallback execution path remains explicit in harness code.
4. Re-generated `docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md`.

## Current Tagged Surfaces

- `src/parser.rs` (`harness-legacy,parity-gap`)

## Closure Evidence For Tagged Surface

- Runtime fallback behavior is covered by `tests/cli_contracts.rs` (`--runtime dual|vm|interpreter` contracts).
- Generator divergence is explicit and tested in `tests/vm_interpreter_parity_surfaces.rs` (`generator_iteration_surface_is_intentionally_divergent_with_explicit_vm_error`).
- Canonical docs carry the divergence boundary in `README.md` and `docs/VM_INTERPRETER_PARITY_MATRIX.md`.

## Validation

```bash
cargo test --test interpreter_flag_dependency_map_contract
cargo test --test vm_interpreter_parity_surfaces
```

Both commands passed.
