# Kujo Field Notes — V1U-DG-003 External Baseline Refresh Cadence

**Date:** 2026-05-20
**Session:** 17:59 local
**Branch/Commit:** main / 5243cd8
**Scope:** Completed `V1U-DG-003` by codifying the external-repo strict baseline cadence in `docs/DOCGEN.md` and capturing a refreshed strict/include-private + strict/public-only baseline run.

---

## What I Changed
- Updated `docs/DOCGEN.md`:
  - Marked `DG-NEXT-003` complete.
  - Added `External-Repo Strict Baseline Refresh Cadence` with cadence, required command modes, evidence format, and mitigation playbook.
- Ran refreshed external-repo strict baseline commands against:
  - `/Users/robertdevore/Documents/Kujolang/kujo-repos/ai-sdk`
  - `/Users/robertdevore/Documents/Kujolang/kujo-repos/mcp`
  - `/Users/robertdevore/Documents/Kujolang/kujo-repos/scout`
- Captured JSON outputs under `/private/tmp/docgen_external_refresh_2026-05-20` for both modes per repo.

## Gotchas (Read This Next Time)
- **Gotcha:** `kujo docgen --json` strict runs can return success while still reporting gate failures in payload fields.
  - **Symptom:** CLI exit status remains zero even when strict gates should fail.
  - **Root cause:** DocGen reports strict-gate outcomes via JSON summary (`gate_failures`, `undocumented_count`, etc.) instead of failing process execution for this workflow.
  - **Fix:** Treat JSON counts/`gate_failures` as the source of truth in baseline refresh notes.
  - **Prevention:** Always parse and record `undocumented_count`, `broken_link_count`, `warning_count`, and `gate_failures` per mode.

## Things I Learned
- The representative external baseline set currently remains stable at zero strict/public-only drift counts across all three repos.
- The cadence documentation needs both mode requirements (`--include-private` and `--public-only`) to prevent blind spots in visibility-policy regressions.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** Baseline refresh succeeded with no drift.

## Follow-ups / TODO (For Future Agents)
- [ ] On the next adapter behavior change, re-run both strict modes and compare against `/private/tmp/docgen_external_refresh_2026-05-20` counts.
- [ ] If any repo regresses, add a fixture-backed regression test in `tests/docgen_universal.rs` before shipping adapter logic changes.

## Links / References
- Files touched:
  - `docs/DOCGEN.md`
  - `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`
  - `notes/2026-05-20_17-59_v1u-dg-003-external-baseline-refresh-cadence.md`
- Related docs:
  - `docs/DOCGEN_EXTERNAL_REPOS_EVALUATION_2026-05-18.md`
  - `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`
  - `docs/RELEASE_PROCESS.md`

## Refreshed Baseline Metrics (2026-05-20)

| Repo | Mode | undocumented_count | broken_link_count | warning_count | gate_failures |
| --- | --- | ---: | ---: | ---: | ---: |
| `ai-sdk` | `strict` | 0 | 0 | 0 | 0 |
| `ai-sdk` | `strict-public-only` | 0 | 0 | 0 | 0 |
| `mcp` | `strict` | 0 | 0 | 0 | 0 |
| `mcp` | `strict-public-only` | 0 | 0 | 0 | 0 |
| `scout` | `strict` | 0 | 0 | 0 | 0 |
| `scout` | `strict-public-only` | 0 | 0 | 0 | 0 |

Delta vs prior baseline note (`docs/DOCGEN_EXTERNAL_REPOS_EVALUATION_2026-05-18.md`): all recorded strict/public-only counts unchanged (0 drift).
