# Kujo Field Notes — V1VM-IMP-003 Dotted Import Boundary Security

**Date:** 2026-05-21
**Session:** 18:34 local
**Branch/Commit:** main / 8ce08c8
**Scope:** Added explicit dotted-from-import symlink-escape security regression coverage and verified deterministic rejection in VM and interpreter runtime paths.

---

## What I Changed
- Added `runtime_security_rejects_dotted_module_symlink_escape_in_vm_and_interpreter` in `tests/runtime_security.rs`.
- Test constructs nested dotted import path (`from src.core.math import answer`) where `src/core/math.kujo` is a symlink to a file outside module search root.
- Test verifies both runtime modes (`kujo run` and `kujo run --interpreter`) reject with deterministic boundary error.

## Gotchas (Read This Next Time)
- **Gotcha:** Existing symlink escape tests can miss dotted-path-specific file resolution flow.
  - **Symptom:** Flat-module symlink tests pass, but nested dotted path resolution still lacks explicit runtime-mode security assertion.
  - **Root cause:** Candidate path shape differs (`<seg>/<seg>/.../<module>.kujo`) and needs direct coverage.
  - **Fix:** Add a dotted import workflow fixture with a nested symlink target outside root and assert both modes reject.
  - **Prevention:** Keep at least one boundary/security test per import path family (flat + dotted).

## Things I Learned
- Current boundary enforcement correctly rejects dotted-path symlink escape attempts in both VM and interpreter execution paths.

## Debug Notes (Only if applicable)
- **Failing test / error:** N/A.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue with `V1VM-IMP-004` to add broader real-world nested-layout integration fixtures.

## Links / References
- Files touched:
  - `tests/runtime_security.rs`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`

## Command Evidence
- `cargo test --test runtime_security`
  - Result: `9 passed; 0 failed`
- `cargo test --test vm_interpreter_parity_surfaces`
  - Result: `85 passed; 0 failed`
