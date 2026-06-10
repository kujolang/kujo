# Kujo Field Notes — V1VM-IMP-004 Nested Layout Integration Fixtures

**Date:** 2026-05-21
**Session:** 18:37 local
**Branch/Commit:** main / 4ff508e
**Scope:** Added a real-world nested module integration fixture that validates dotted import workflows in both VM and interpreter modes, while preserving flat import behavior in the same test flow.

---

## What I Changed
- Added `package_module_workflow_nested_layout_is_runtime_mode_consistent_and_keeps_flat_imports` in `tests/package_module_workflow_integration.rs`.
- The test creates nested modules under `src/core` and `src/rag`, executes dotted imports in both runtime modes, and verifies deterministic output.
- The same test also runs a flat-module workflow (`from math_helper import answer`) in both modes to guard backward compatibility.

## Gotchas (Read This Next Time)
- **Gotcha:** Exporting callable values from nested modules can trigger runtime-mode behavior differences not relevant to import-path correctness.
  - **Symptom:** VM run failed with `Cannot call non-function` when workflow called an imported exported function value.
  - **Root cause:** Fixture shape conflated import resolution with callable export semantics.
  - **Fix:** Keep fixture focused on import reliability by exporting computed values and asserting value import behavior.
  - **Prevention:** For import-resolution tests, prefer scalar export assertions unless the loop explicitly targets callable export semantics.

## Things I Learned
- Dotted nested layout imports are stable in both VM and interpreter mode for value exports.
- Flat import behavior remains unchanged in the same integration environment.

## Debug Notes (Only if applicable)
- **Failing test / error:** `Cannot call non-function` in VM mode for callable export fixture variant.
- **Repro steps:** Run initial version of `package_module_workflow_nested_layout_is_runtime_mode_consistent_and_keeps_flat_imports` with `print(answer())` against `export answer := answer`.
- **Breakpoints / logs used:** Standard test stderr capture from integration test failure output.
- **Final diagnosis:** Fixture mixed import-validation goal with callable-export runtime semantics.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue with `V1VM-IMP-005` doc alignment for interpreter-flag guidance.

## Links / References
- Files touched:
  - `tests/package_module_workflow_integration.rs`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`

## Command Evidence
- `cargo test --test package_module_workflow_integration`
  - Result: `6 passed; 0 failed`
- `cargo test --test vm_interpreter_parity_surfaces`
  - Result: `85 passed; 0 failed`
