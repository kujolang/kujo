# Kujo Field Notes — V1-BASE-001 Readiness Baseline Messaging

**Date:** 2026-05-06
**Session:** 23:08 local
**Branch/Commit:** main / ba66808
**Scope:** Completed roadmap item V1-BASE-001 by reconciling public readiness messaging and marking the item complete after verification.

---

## What I Changed
- Added a dedicated `1.0 Readiness Status` section to `README.md` and pointed it to `ROADMAP.md` as the single release gate.
- Updated draft-status wording in `docs/LANGUAGE_SPEC.md` to explicitly state the draft marker is not a release-ready claim.
- Updated draft-status wording in `docs/NATIVE_API_SECURITY_POSTURE.md` with the same readiness clarification and refreshed the `Last updated` date.
- Marked `V1-BASE-001` complete in `ROADMAP.md` with a completion note and verification command outcome.
- Added an Unreleased changelog entry in `CHANGELOG.md` for the readiness-language alignment.

## Gotchas (Read This Next Time)
- **Gotcha:** Running `git add` and `git commit` concurrently can race on `.git/index.lock`.
  - **Symptom:** `fatal: Unable to create '.git/index.lock': File exists` during commit.
  - **Root cause:** Parallel git index mutations are not safe for the same repository state.
  - **Fix:** Retry sequentially (`git add` then `git commit`) after confirming the lock is stale/cleared.
  - **Prevention:** Keep git staging/commit commands serialized even when other read-only shell operations are parallelized.

## Things I Learned
- `V1-BASE-001` was intentionally left open even after the roadmap rewrite because README/spec/security docs still needed explicit anti-overstatement language.
- The roadmap item can be completed cleanly with docs-only edits, but P0 policy still requires a full `cargo test` run before marking complete.

## Debug Notes (Only if applicable)
- **Failing test / error:** `fatal: Unable to create '/Users/robertdevore/Documents/Kujolang/kujo-repos/kujo/.git/index.lock': File exists.`
- **Repro steps:** Triggered by running `git add ...` and `git commit ...` in parallel.
- **Breakpoints / logs used:** `git status --short`, `ls -l .git/index.lock`.
- **Final diagnosis:** Temporary git index lock race from concurrent write operations.

## Follow-ups / TODO (For Future Agents)
- [ ] Start the next incomplete P0 roadmap item (`V1-LEX-001`) after confirming no new Phase 1 blockers were introduced.
- [ ] Keep draft-status wording in readiness-sensitive docs synchronized with roadmap status markers when future release messaging changes.

## Links / References
- Files touched:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `docs/LANGUAGE_SPEC.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
