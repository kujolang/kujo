# Kujo Field Notes — ER-P1-005 docs consistency pass

**Date:** 2026-05-25
**Session:** 23:31 local
**Branch/Commit:** main / working tree (pre-commit)
**Scope:** Closed root-to-docs consistency item by running docs contract suites, validating docs examples, and aligning root/operator-facing docs with runtime behavior and contract markers.

---

## What I Changed
- Ran docs consistency suites:
  - `cargo test --test readme_contracts`
  - `cargo test --test docs_policy_consistency_contract`
  - `cargo test --test architecture_docs_contract`
  - `cargo test --test release_process_docs_contract`
  - `cargo test --test runtime_path_matrix_contract`
- Ran docs examples suite:
  - `cargo test --test docs_examples`
- Kept docs aligned with runtime/contract expectations:
  - `README.md` (positioning + repository layout clarity)
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md` marker/date compatibility with runtime-path contract tests
  - `docs/STANDARD_LIBRARY.md` runtime builtin inventory sync (`__vm_for_iterable`, `substr`)
- Marked `ER-P1-005` complete in `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md` with command evidence.

## Gotchas (Read This Next Time)
- **Gotcha:** docs contracts fail on small marker/date drift, not just semantic contradictions.
  - **Symptom:** `runtime_path_matrix_contract` fails when expected section marker date text changes.
  - **Root cause:** contract test matches exact marker strings.
  - **Fix:** keep marker text stable and move recency updates to adjacent “updated evidence snapshot” fields.
  - **Prevention:** treat marker headings as compatibility surfaces.

## Things I Learned
- The docs suite is broad enough to catch README, policy, release-process, runtime-path, and runnable-example drift together.
- Inventory-level docs (`STANDARD_LIBRARY.md`) are part of runtime contract enforcement, not optional prose.

## Debug Notes (Only if applicable)
- **Failing test / error:** none in this loop after alignment.
- **Repro steps:** n/a.
- **Breakpoints / logs used:** command outputs only.
- **Final diagnosis:** ER-P1-005 acceptance criteria met.

## Follow-ups / TODO (For Future Agents)
- [ ] Re-run docs contract set whenever CLI/runtime summaries, marker headers, or inventory rows change.
- [ ] Keep README/runtime-path/inventory updates in the same PR when behavior-facing text changes.

## Links / References
- Files touched:
  - `README.md`
  - `docs/STANDARD_LIBRARY.md`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`
  - `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`
- Related docs:
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
