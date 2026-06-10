# Kujo Field Notes — V1VM-IMP-002 Dotted Resolution Precedence

**Date:** 2026-05-21
**Session:** 18:24 local
**Branch/Commit:** main / 9ccdd48
**Scope:** Locked deterministic dotted-module precedence behavior with VM/interpreter parity coverage when both flat and nested candidates exist.

---

## What I Changed
- Added parity regression test in `tests/vm_interpreter_parity_surfaces.rs`:
  - `vm_and_interpreter_dotted_import_resolution_prefers_flat_module_before_nested_path`
- The test creates both candidates for the same dotted import (`modules/<name>.core.math.kujo` and `modules/<name>/core/math.kujo`) and asserts both runtimes import from the flat dotted filename first.

## Gotchas (Read This Next Time)
- **Gotcha:** Precedence bugs can hide unless both candidate file shapes exist simultaneously.
  - **Symptom:** Regular dotted-import tests pass but conflict-order behavior remains unverified.
  - **Root cause:** Non-conflict fixtures only exercise one candidate path.
  - **Fix:** Create explicit conflict fixtures with distinct exported values.
  - **Prevention:** Keep one parity test that intentionally sets conflicting flat-vs-nested modules and asserts the exact winner.

## Things I Learned
- Current resolver precedence is deterministic and parity-safe for VM/interpreter in this conflict class.
- Existing `docs/LANGUAGE_SPEC.md` precedence wording matches runtime behavior after this parity lock.

## Debug Notes (Only if applicable)
- **Failing test / error:** N/A.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue with `V1VM-IMP-003` boundary/security verification for dotted imports.

## Links / References
- Files touched:
  - `tests/vm_interpreter_parity_surfaces.rs`
- Related docs:
  - `docs/LANGUAGE_SPEC.md`
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`

## Command Evidence
- `cargo test --test vm_interpreter_parity_surfaces`
  - Result: `85 passed; 0 failed`
- `cargo test load_module_dotted_name_resolution_prefers_legacy_flat_filename_before_nested_path`
  - Result: targeted resolver unit tests passed in both `src/lib.rs` and `src/main.rs` test binaries
