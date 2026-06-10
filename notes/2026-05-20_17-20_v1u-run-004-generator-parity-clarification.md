# V1U-RUN-004 — Generator Surface Ambiguity Closure

Date: 2026-05-20

## Scope

Close ambiguity about generator support status across runtime behavior, parity tests,
and release-facing docs.

## What Changed

1. Added explicit divergence coverage for top-level generator iteration:
   - `generator_iteration_surface_is_intentionally_divergent_with_explicit_vm_error`
   - Asserts interpreter success and deterministic VM error (`Yield can only be used inside generator functions`).
2. Kept generator failure-path parity explicit through existing arity mismatch coverage:
   - `vm_and_interpreter_error_on_generator_arity_mismatch`
   - `vm_and_interpreter_error_on_generator_arity_too_many`
3. Updated canonical docs to remove ambiguous wording:
   - `README.md` known-boundary language now distinguishes generator parity coverage from interpreter fallback workflows.
   - `docs/VM_INTERPRETER_PARITY_MATRIX.md` now includes a dedicated generator-iteration row and updated timestamp.

## Outcome

Generator support status is now explicit and consistent:

- Top-level generator iteration is intentionally divergent today (interpreter-supported, VM-deterministic error).
- Struct generator methods remain intentionally unsupported (explicit divergence).
- Interpreter fallback wording now focuses on fixture/diagnostic workflow compatibility, not vague runtime-surface uncertainty.

## Validation

```bash
cargo test --test vm_interpreter_parity_surfaces
cargo test --test readme_contracts
```

Both commands passed.
