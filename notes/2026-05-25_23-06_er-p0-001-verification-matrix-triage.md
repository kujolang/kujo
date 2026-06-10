# Kujo Field Notes — ER-P0-001 verification matrix triage

**Date:** 2026-05-25
**Session:** 23:06 local
**Branch/Commit:** main / working tree (pre-commit)
**Scope:** Ran the ER-P0-001 verification matrix commands, captured pass/fail evidence, and triaged blockers preventing checklist closure.

---

## What I Changed
- Ran VM fixture sweep: `cargo run -- test --runtime vm`.
- Ran dual fixture sweep: `cargo run -- test --runtime dual`.
- Ran security suites:
  - `cargo test --test native_api_security_boundaries`
  - `cargo test --test runtime_security`
- Ran docs/release contract checks:
  - `cargo test --test docs_policy_consistency_contract`
  - `bash scripts/release_candidate_gate.sh --roadmap-only`
- Ran unsafe budget gate check:
  - `cargo test --test unsafe_inventory_contract`
- Updated checklist blocker state in `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** ER-P0-001 cannot be closed even when most matrix checks are green.
  - **Symptom:** `cargo test --test unsafe_inventory_contract` fails with `expected <= 55, got 59`.
  - **Root cause:** current executable unsafe inventory exceeds hard contract budget.
  - **Fix:** complete unsafe reduction work (ER-P0-006) to bring executable unsafe count back under budget without weakening the gate.
  - **Prevention:** run `cargo test --test unsafe_inventory_contract` early in any “full verification” loop before spending time on broader sweeps.

- **Gotcha:** VM/dual runtime fixture sweeps currently report 137/150 passing and still return success for the command surface.
  - **Symptom:** multiple fixtures intentionally/known-fail via parser errors while runtime summary still reports pass ratio metadata.
  - **Root cause:** outstanding parser-fixture debt/mismatch inventory not fully burned down yet (ER-P0-003 scope).
  - **Fix:** treat these fixture misses as unresolved parity debt and keep item blocked.
  - **Prevention:** include VM mismatch inventory references in every verification note.

## Things I Learned
- Security suites (`native_api_security_boundaries`, `runtime_security`) are stable and green in this workspace snapshot.
- Roadmap-only release gate script currently passes and is not the release blocker.
- Unsafe executable budget is the dominant hard gate failure for ER-P0-001.

## Debug Notes (Only if applicable)
- **Failing test / error:** `unsafe_inventory_enforces_current_executable_budget` failed with `executable unsafe budget regression: expected <= 55, got 59`.
- **Repro steps:**
  1. `cargo test --test unsafe_inventory_contract`
- **Breakpoints / logs used:** command output only.
- **Final diagnosis:** checklist item ER-P0-001 is blocked until ER-P0-006 reduces executable unsafe count.

## Follow-ups / TODO (For Future Agents)
- [ ] Complete ER-P0-006 and rerun `cargo test --test unsafe_inventory_contract`.
- [ ] Burn down remaining VM/dual fixture misses under ER-P0-003 and refresh mismatch artifacts.
- [ ] Re-run full `cargo test` matrix after ER-P0-006 + ER-P0-003 to attempt ER-P0-001 closure.

## Links / References
- Files touched:
  - `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`
- Related docs:
  - `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`
  - `docs/generated/UNSAFE_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
