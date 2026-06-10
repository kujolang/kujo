# Kujo Field Notes — Field notes system hardening

**Date:** 2026-04-03
**Session:** 09:50 local
**Branch/Commit:** main / 0601b47
**Scope:** Re-aligned Kujo's notes workflow artifacts to a strict operational-memory format. Replaced oversized, mixed-content curation files with concise, deduplicated guidance and a focused index.

---

## What I Changed
- Replaced `notes/GOTCHAS.md` with a concise, deduplicated curated gotchas document organized by parser/runtime/compiler/tooling sections.
- Replaced `notes/README.md` with a minimal index that points to `GOTCHAS.md`, `FIELD_NOTES_SYSTEM.md`, and selected high-signal session notes.
- Kept `notes/FIELD_NOTES_SYSTEM.md` as the canonical workflow/template source (already matched required policy shape).

## Gotchas (Read This Next Time)
- **Gotcha:** Curated gotchas can silently become a second "session log" if left unconstrained.
  - **Symptom:** `notes/GOTCHAS.md` ballooned into a very large mixed historical dump with repeated patterns and low-signal detail.
  - **Root cause:** Session-level narrative content accumulated in the curated file instead of staying in per-session notes.
  - **Fix:** Rebuilt `GOTCHAS.md` as short, rule-oriented, deduplicated guidance with discovery references.
  - **Prevention:** Treat `GOTCHAS.md` as a curated contract surface; put stories and raw chronology only in timestamped session files.

- **Gotcha:** Notes index files drift into full changelog format over time.
  - **Symptom:** `notes/README.md` became long-form chronological prose rather than a quick index.
  - **Root cause:** Session summaries were appended without pruning/index discipline.
  - **Fix:** Replaced with a minimal "read first" index and a short list of high-signal sessions.
  - **Prevention:** Keep `notes/README.md` as navigation only; avoid full historical narrative there.

## Things I Learned
- A practical split works best: `FIELD_NOTES_SYSTEM.md` = policy, `GOTCHAS.md` = deduplicated rules, session files = raw operational memory.
- "Curated" only stays curated if entries are compact rules with bounded scope and explicit discovery references.
- Fast onboarding improves when `notes/README.md` is intentionally short and opinionated about where to start.

## Debug Notes (Only if applicable)
- **Failing test / error:** N/A
- **Repro steps:** N/A
- **Breakpoints / logs used:** N/A
- **Final diagnosis:** This was a documentation-structure correction task, not a runtime/code bug investigation.

## Follow-ups / TODO (For Future Agents)
- [ ] Keep adding new discoveries to timestamped session notes first, then promote only high-impact repeated patterns into `notes/GOTCHAS.md`.
- [ ] Revisit curated gotchas periodically to merge duplicates and prune stale items as subsystems evolve.

## Links / References
- Files touched:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `notes/2026-04-03_09-50_field-notes-system-hardening.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `README.md`
  - `ROADMAP.md`
