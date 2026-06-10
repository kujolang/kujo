# Kujo Field Notes — Field Notes CI Guard

**Date:** 2026-05-06
**Session:** 11:30 local
**Branch/Commit:** main / working tree
**Scope:** Added an automated CI guard that validates newly added session-note filenames and required template structure, then hardened the script for local macOS compatibility.

---

## What I Changed
- Added `./.github/scripts/check-new-field-notes.sh`:
  - Detects newly added files in `notes/` for a given commit range.
  - Enforces session-note filename contract: `YYYY-MM-DD_HH-mm_short-kebab-summary.md`.
  - Validates required template metadata and section headings.
  - Validates section ordering for required headings.
- Added `./.github/workflows/field-notes-guard.yml`:
  - Runs on `pull_request` and `push` to `main`.
  - Uses `fetch-depth: 0` and passes base/head SHAs to the guard script.
- Validated script behavior against real commit range (`f2ce159..c3b573f`).
- Updated `notes/GOTCHAS.md`, `notes/README.md`, and `CHANGELOG.md` to capture the new operational guard.

## Gotchas (Read This Next Time)
- **Gotcha:** `mapfile` looked convenient but failed locally on macOS.
  - **Symptom:** Running the guard script produced `mapfile: command not found`.
  - **Root cause:** macOS default Bash is 3.2, and `mapfile` is Bash-4+.
  - **Fix:** Replaced `mapfile` with Bash 3-compatible `while read` loops and array appends.
  - **Prevention:** Keep repo guard scripts Bash 3-compatible unless the project explicitly requires newer Bash.

- **Gotcha:** Grep can treat literal `---` as an option.
  - **Symptom:** Guard run failed with `grep: unrecognized option '---'` when checking section separators.
  - **Root cause:** Literal arguments that begin with `-` need `--` separator in grep calls.
  - **Fix:** Changed literal lookup helper to use `grep -nF -- "$literal"`.
  - **Prevention:** Always use `--` for grep pattern args when literals may start with dashes.

## Things I Learned
- Rule: It is safer to validate only *newly added* session-note files so the guard enforces forward quality without blocking on legacy historical variance.
- Rule: Workflow SHA handling needs explicit fallback behavior (`pull_request` base SHA vs `push` before SHA, plus zero-SHA bootstrap handling).
- Rule: Field-note structure is a contract surface and should be validated by CI the same way release metadata contracts are validated.

## Debug Notes (Only if applicable)
- **Failing test / error:**
  - `mapfile: command not found`
  - `grep: unrecognized option '---'`
- **Repro steps:**
  - `bash .github/scripts/check-new-field-notes.sh f2ce159 c3b573f`
- **Breakpoints / logs used:**
  - Script syntax check: `bash -n .github/scripts/check-new-field-notes.sh`
  - Iterative script re-run on fixed commit range.
- **Final diagnosis:** Local portability and literal-grep handling were the only blockers; core note-template validation logic was otherwise correct.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider adding a short contributor snippet in `CONTRIBUTING.md` that explains the session-note filename/template contract enforced by CI.
- [ ] Consider optionally validating minimum content bullets under `What I Changed` and `Gotchas` for stronger note quality gates.

## Links / References
- Files touched:
  - `.github/scripts/check-new-field-notes.sh`
  - `.github/workflows/field-notes-guard.yml`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `CHANGELOG.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `README.md`
  - `ROADMAP.md`
