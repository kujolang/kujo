# Kujo Field Notes — Field Notes System Enforcement Pass

**Date:** 2026-05-06
**Session:** 11:18 local
**Branch/Commit:** main / working tree
**Scope:** Aligned Kujo notes artifacts with the mandatory field-notes policy, normalized the latest serve session note to the required section structure, and added a deduplicated curated gotcha from the recent serve hardening work.

---

## What I Changed
- Updated `notes/2026-05-06_11-06_cli-serve-universal-hardening-followthrough.md` to match required section structure:
  - Added `Things I Learned`.
  - Added `Debug Notes (Only if applicable)` with explicit failing-test evidence and diagnosis.
  - Normalized `Links / References` into `Files touched` and `Related docs` subsections.
- Updated `notes/GOTCHAS.md` with a high-impact, deduplicated serve-related pitfall:
  - `tiny_http` header field comparison type-sensitivity with `AsciiStr` vs `&str`.
- Updated `notes/README.md` index to include `2026-05-06_11-06_cli-serve-universal-hardening-followthrough.md` as a high-signal session note.

## Gotchas (Read This Next Time)
- **Gotcha:** It is easy to think an existing note is "close enough" to the template and skip structural normalization.
  - **Symptom:** Session note has useful content but misses required sections (for example `Things I Learned` or required links structure).
  - **Root cause:** Incremental edits over time drift from the mandated note schema.
  - **Fix:** Reconcile note content against the required heading sequence and subsection structure.
  - **Prevention:** Treat note structure as a contract and run a quick section-by-section checklist before closing work.

## Things I Learned
- Rule: Template compliance is not only about content quality; section ordering and naming are part of the project contract.
- Rule: Curated `notes/GOTCHAS.md` should only receive repeated or high-impact pitfalls, not raw narrative.
- Rule: If a behavior had to be justified in implementation (for example, an unrelated failing test), it should be explicitly captured in session notes.

## Debug Notes (Only if applicable)
- **Failing test / error:** Not applicable for this pass (documentation/process alignment only).
- **Repro steps:** Not applicable.
- **Breakpoints / logs used:** Inspected existing notes files and diffs only.
- **Final diagnosis:** Existing field-notes policy file already matched requirements; the actionable work was consistency enforcement across recent session artifacts.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider adding a lightweight CI/content check that validates required section headers for newly added session-note files.
- [ ] Consider a short helper script to scaffold note files using the mandatory template filename + heading set.

## Links / References
- Files touched:
  - `notes/2026-05-06_11-06_cli-serve-universal-hardening-followthrough.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `README.md`
  - `ROADMAP.md`
