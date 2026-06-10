# V1X-VM-001 - External Project VM Smoke Gates

Date: 2026-05-27
Owner: AI agent loop execution

## Summary

Completed `V1X-VM-001` by adding deterministic external-project VM smoke gates for imported callable execution in both required forms:

1. `from module import symbol` then `symbol(...)`
2. `import module` then `module.symbol(...)`

## Implementation

### VM import behavior hardening

Updated VM import-all path in `src/vm.rs`:

- `__vm_import_all` now defines a module namespace binding (derived from imported module name) in addition to existing import-all symbol bindings.
- Added method-call-compatible export wrapping for module namespace callable fields so VM-compiled method-call lowering (`obj.method(...)` receiver + args) executes imported callables correctly.

New helper logic added in `src/vm.rs`:

- `module_binding_name(...)`
- `wrap_module_export_for_method_call(...)`
- `module_namespace_value(...)`

### External-project smoke gates

Added new integration suite:

- `tests/vm_external_project_smoke.rs`

Coverage:

- `vm_external_project_smoke_from_import_symbol_call`
- `vm_external_project_smoke_import_module_symbol_call`

Both tests create isolated temp projects, run `kujo run` (VM default path), and assert deterministic marker output and success status.

## Regression Class Now Gated

The suite now catches the imported-call regression class where a module import appears valid but callable invocation fails at call site on VM path (historically observed as non-callable/undefined or arity-mismatched behavior depending on lowering path).

## Validation Commands And Results

```bash
cargo test --test vm_external_project_smoke
```

- PASS (`2 passed; 0 failed`)

```bash
cargo test --test vm_interpreter_parity_surfaces
```

- PASS (`100 passed; 0 failed`)

```bash
cargo run -- test --runtime vm
```

- PASS (`EXIT_CODE:0`, runtime summary `Passed 134/150 tests`, `Runtime strategy: vm (vm_primary=134)`)
- Existing parser-invalid fixtures (for example `tests/generators_test.kujo`, `tests/destructuring.kujo`) remain outside this item scope and are unchanged by this loop.

## Notes

- Changes are additive and backward-compatible: existing `import module` symbol-injection behavior remains intact; module namespace binding is added for explicit `module.symbol(...)` call surfaces.
- Release/tag/publish actions were not executed.
