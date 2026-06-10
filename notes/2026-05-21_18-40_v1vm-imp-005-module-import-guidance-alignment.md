# Kujo Field Notes — V1VM-IMP-005 Module Import Guidance Alignment

**Date:** 2026-05-21
**Session:** 18:40 local
**Branch/Commit:** main / fe5c5dd
**Scope:** Removed stale ambiguity in module-import guidance by explicitly documenting VM-default support for dotted imports and updating interpreter-flag dependency-map language to reflect current runtime strategy.

---

## What I Changed
- Updated `README.md` language overview to state that dotted module import workflows are supported on default VM path and `--interpreter` is optional fallback/debug mode.
- Updated `docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md` to replace stale interpreter-hardcoding narrative with current `kujo test --runtime dual|vm|interpreter` strategy wording.
- Updated generator source `scripts/generate_interpreter_flag_dependency_map.sh` to emit the new runtime-strategy guidance deterministically.
- Updated docs contract tests:
  - `tests/readme_contracts.rs`
  - `tests/interpreter_flag_dependency_map_contract.rs`

## Gotchas (Read This Next Time)
- **Gotcha:** Editing generated docs directly without updating generator scripts creates silent drift.
  - **Symptom:** Manual doc edits get reverted the next time generation script runs.
  - **Root cause:** Canonical content is script-produced.
  - **Fix:** Update both generated file and generator script/contracts in same loop.
  - **Prevention:** Treat `docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md` as generator-owned and always patch `scripts/generate_interpreter_flag_dependency_map.sh` + contract tests alongside doc wording changes.

## Things I Learned
- The right docs stance now is: VM-default supports dotted imports; interpreter is fallback/debug, not a required mode for ordinary modular layouts.

## Debug Notes (Only if applicable)
- **Failing test / error:** N/A.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue to `V1VM-PAR-001` runtime-output normalization harness work.

## Links / References
- Files touched:
  - `README.md`
  - `docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md`
  - `scripts/generate_interpreter_flag_dependency_map.sh`
  - `tests/readme_contracts.rs`
  - `tests/interpreter_flag_dependency_map_contract.rs`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`

## Command Evidence
- `bash scripts/generate_interpreter_flag_dependency_map.sh`
  - Result: success
- `cargo test --test readme_contracts --test interpreter_flag_dependency_map_contract`
  - Result: `3 passed; 0 failed`
- `cargo test --test vm_interpreter_parity_surfaces`
  - Result: `85 passed; 0 failed`
