# Kujo Field Notes — V1VM-HAR-001 Dual Fallback Determinism

**Date:** 2026-05-21
**Session:** 19:46 local
**Branch/Commit:** main / 260f242
**Scope:** Hardened `kujo test --runtime dual` fallback observability so fallback passes are explicit per fixture and covered by CLI contracts.

---

## What I Changed
- Updated `src/parser.rs` dual harness reporting to append `[dual fallback: interpreter]` when VM mismatch recovers via interpreter fallback.
- Updated `tests/cli_contracts.rs`:
  - `cli_test_runtime_dual_mode_falls_back_to_interpreter_for_vm_drift_fixture` now asserts fallback marker presence.
  - `cli_test_runtime_vm_mode_reports_mismatch_for_vm_drift_fixture` now asserts fallback marker absence in VM mode.
- Updated `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`:
  - Added blocker note to `V1VM-PAR-004` with current evidence.
  - Marked `V1VM-HAR-001` complete with commands/results.

## Gotchas (Read This Next Time)
- **Gotcha:** Dual mode can look "quietly successful" without a per-fixture fallback marker.
  - **Symptom:** Before this change, a passing fixture in dual mode did not indicate whether VM-primary or interpreter fallback produced the pass.
  - **Root cause:** The harness only reported aggregate fallback counts at the end.
  - **Fix:** Add deterministic per-fixture marker on fallback pass lines.
  - **Prevention:** Keep both per-fixture and aggregate fallback signals in contract tests to prevent silent behavior drift.

## Things I Learned
- Runtime summary counts (`vm_primary`, `interpreter_fallback`) are necessary but not sufficient for fast triage; fixture-local fallback visibility reduces debugging time.
- The existing fallback trigger was already deterministic (VM first, fallback on mismatch); the key hardening gap was observability.
- Blocking `V1VM-PAR-004` explicitly in-checklist avoids premature "intentional divergences only" claims while unexplained buckets remain.

## Debug Notes (Only if applicable)
- **Failing test / error:** No functional failures; this was output-contract hardening.
- **Repro steps:** Run `kujo test --runtime dual` on a VM-drift fixture where interpreter matches snapshot.
- **Breakpoints / logs used:** CLI contract tests and dual runtime command output lines.
- **Final diagnosis:** Silent fallback ambiguity fixed by explicit pass-line marker and contracts.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue `V1VM-HAR-002` by reducing fallback-dependent fixtures and tracking `vm_primary` trend.
- [ ] After parity/harness buckets are reduced, return to `V1VM-PAR-004` for intentional-divergence-only documentation.

## Links / References
- Files touched:
  - `src/parser.rs`
  - `tests/cli_contracts.rs`
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `notes/2026-05-21_19-46_v1vm-har-001-dual-fallback-determinism.md`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
