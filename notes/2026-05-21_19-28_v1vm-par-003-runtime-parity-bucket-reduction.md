# Kujo Field Notes — V1VM-PAR-003 Runtime Parity Bucket Reduction

**Date:** 2026-05-21
**Session:** 19:28 local
**Branch/Commit:** main / c767f86
**Scope:** Reduced the next mismatch bucket by fixing interpreter recursion for custom enum constructor tags and adding parity regression coverage.

---

## What I Changed
- Updated `src/interpreter/mod.rs` (`Expr::Tag` branch) so namespaced tags (for example `Result::Ok`) construct tagged values directly instead of recursively re-invoking generated constructor bindings.
- Added `vm_and_interpreter_match_custom_enum_constructor_calls_without_recursion` to `tests/vm_interpreter_parity_surfaces.rs`.
- Refreshed affected enum fixture snapshots:
  - `tests/test_enum_err.out`
  - `tests/test_enum_err_only.out`
  - `tests/test_enum_nested.out`
  - `tests/test_enum_none.out`
  - `tests/test_enum_ok.out`
- Regenerated mismatch inventory artifacts:
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
- Updated `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md` and marked `V1VM-PAR-003` complete with evidence.

## Gotchas (Read This Next Time)
- **Gotcha:** Generated enum constructors can recurse through `Expr::Tag` if namespaced tags are also environment bindings.
  - **Symptom:** Interpreter-only runtime error: `Maximum call stack depth ... while calling Result::Ok`.
  - **Root cause:** `Stmt::EnumDef` binds constructor names (for example `Result::Ok`) to functions that return `Expr::Tag(Result::Ok, ...)`; the old `Expr::Tag` path looked up and invoked that same binding again.
  - **Fix:** Short-circuit namespaced tags in `Expr::Tag` to direct tagged-value construction.
  - **Prevention:** Keep constructor-evaluation paths acyclic; avoid generic env lookup for namespaced enum tags unless there is a distinct constructor value type.

## Things I Learned
- This parity bucket had at least one high-leverage interpreter recursion defect that impacted multiple custom-enum fixtures.
- Fixing one semantic root cause and then refreshing only affected snapshots is a clean way to reduce `runtime-parity-bug` counts without broad refactors.
- Tracking both mismatch-bucket totals and `vm_primary`/`interpreter_fallback` split gives clearer progress signals than pass count alone.

## Debug Notes (Only if applicable)
- **Failing test / error:** `Runtime Error: Maximum call stack depth of 32 exceeded while calling Result::Ok` under `--interpreter`.
- **Repro steps:** `./target/debug/kujo run tests/test_enum_ok.kujo --interpreter`.
- **Breakpoints / logs used:** Focused fixture runs plus inventory delta checks after regeneration.
- **Final diagnosis:** Recursive constructor path in interpreter tag evaluation.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue runtime-parity-bug reduction for remaining vm-only mismatches (`dict_methods`, `spread_operator`, method/loop parity fixtures).
- [ ] Re-evaluate whether the next loop should target remaining runtime-parity or move to harness-debt bucket based on net unblock value.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `tests/test_enum_err.out`
  - `tests/test_enum_err_only.out`
  - `tests/test_enum_nested.out`
  - `tests/test_enum_none.out`
  - `tests/test_enum_ok.out`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
