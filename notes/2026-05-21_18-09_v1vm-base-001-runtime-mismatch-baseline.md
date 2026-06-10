# Kujo Field Notes — V1VM-BASE-001 Runtime Mismatch Baseline

**Date:** 2026-05-21
**Session:** 18:09 local
**Branch/Commit:** main / 7819d14
**Scope:** Added deterministic VM-vs-interpreter fixture baseline generation and captured initial mismatch inventory evidence.

---

## What I Changed
- Added `scripts/generate_vm_runtime_mismatch_inventory.sh`.
- Added contract coverage in `tests/vm_runtime_mismatch_inventory_contract.rs`.
- Generated baseline artifacts:
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`

## Gotchas (Read This Next Time)
- **Gotcha:** Shell line-continuation + escaped markdown backticks in generators are easy to get wrong.
  - **Symptom:** Fixture path lines were accidentally treated as commands (`Permission denied` / `command not found`).
  - **Root cause:** A continuation backslash had trailing whitespace and markdown backticks were over-escaped (`\\``), which allowed command substitution behavior.
  - **Fix:** Removed trailing-whitespace continuation and switched to correctly escaped literal markdown backticks (`\`` inside double-quoted strings).
  - **Prevention:** Check generated shell with `cat -vet` when odd command-execution errors reference data lines.

## Things I Learned
- The repo currently has substantial fixture drift between runtime modes and snapshot expectations.
- Capturing per-fixture exit/match deltas gives a better burn-down baseline than top-line pass/fail alone.

## Debug Notes (Only if applicable)
- **Failing test / error:** Script initially emitted `Permission denied`/`command not found` during CSV write loop.
- **Repro steps:** Run `bash scripts/generate_vm_runtime_mismatch_inventory.sh` before escape/continuation fixes.
- **Breakpoints / logs used:** `nl -ba scripts/generate_vm_runtime_mismatch_inventory.sh | sed -n '130,220p'` and `cat -vet` around problematic lines.
- **Final diagnosis:** Bad escaping and line continuation in markdown-row/CSV write block.

## Follow-ups / TODO (For Future Agents)
- [ ] Add mismatch-cause buckets and owner/priority metadata (`V1VM-BASE-002`).
- [ ] Expand contract tests to enforce required bucket columns (`V1VM-BASE-003`).

## Links / References
- Files touched:
  - `scripts/generate_vm_runtime_mismatch_inventory.sh`
  - `tests/vm_runtime_mismatch_inventory_contract.rs`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`

## Command Evidence
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
  - Result: success
  - Summary counts from generated markdown:
    - fixtures scanned: `163`
    - both match snapshot: `69`
    - VM-only mismatch: `19`
    - interpreter-only mismatch: `11`
    - both mismatch: `64`
