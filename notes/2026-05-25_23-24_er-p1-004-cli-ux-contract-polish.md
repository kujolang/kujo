# Kujo Field Notes — ER-P1-004 CLI UX contract polish

**Date:** 2026-05-25
**Session:** 23:24 local
**Branch/Commit:** main / working tree (pre-commit)
**Scope:** Closed CLI UX contract polish by validating JSON/human CLI contract suites and documenting deterministic runtime summary/fallback markers for `kujo test`.

---

## What I Changed
- Ran CLI contract suite: `cargo test --test cli_contracts`.
- Ran CLI JSON contract suite: `cargo test --test cli_json_contracts`.
- Updated `docs/CLI_MACHINE_READABLE_CONTRACTS.md` with a dedicated section for deterministic human-readable `kujo test` summary contracts:
  - runtime strategy line presence
  - dual split counters (`vm_primary`, `interpreter_fallback`)
  - conditional fallback marker emission
- Marked `ER-P1-004` complete with evidence in `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** CLI UX contract scope includes both JSON payloads and human-readable summary invariants used by automation wrappers.
  - **Symptom:** runtime summary/fallback behavior can drift while JSON tests remain green.
  - **Root cause:** summary text contracts live in `cli_contracts`, not `cli_json_contracts`.
  - **Fix:** keep both suites in the acceptance set for CLI UX closure.
  - **Prevention:** if `kujo test` output semantics change, update docs and `cli_contracts` together.

## Things I Learned
- Current dual-mode fixture behavior is VM-primary with no fallback on the drift fixture currently used in CLI contracts.
- Documenting human-readable summary invariants directly in the machine-readable contract doc removes ambiguity for operator tooling.

## Debug Notes (Only if applicable)
- **Failing test / error:** none in this loop.
- **Repro steps:** n/a.
- **Breakpoints / logs used:** command output only.
- **Final diagnosis:** ER-P1-004 acceptance criteria met.

## Follow-ups / TODO (For Future Agents)
- [ ] If runtime strategy defaults change, update the deterministic summary section and `cli_contracts` assertions in the same PR.
- [ ] Keep fallback-marker semantics synced with `src/parser.rs` test harness messaging.

## Links / References
- Files touched:
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`
- Related docs:
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
