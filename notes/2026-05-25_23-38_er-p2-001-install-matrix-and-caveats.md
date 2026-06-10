# Kujo Field Notes — ER-P2-001 install matrix and caveats

**Date:** 2026-05-25
**Session:** 23:38 local
**Branch/Commit:** main / working tree (pre-commit)
**Scope:** Closed packaging/distribution ergonomics planning item by publishing a concrete install/distribution matrix and explicit platform caveats.

---

## What I Changed
- Added `docs/INSTALL_MATRIX.md` with:
  - install/distribution paths (source dev, release binary, cargo install, commit-pinned install, locked CI build)
  - platform caveats (macOS/Linux/Windows)
  - pre-v1 distribution guidance and verification commands
- Updated `README.md` core reference links to include `docs/INSTALL_MATRIX.md`.
- Updated checklist status and evidence in `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`.
- Ran `cargo test --test readme_contracts` to validate README contract stability.

## Gotchas (Read This Next Time)
- **Gotcha:** install/distribution docs are contract-adjacent for operator onboarding and can silently drift from CLI/runtime expectations.
  - **Symptom:** operators use stale install paths or assume package-manager availability not yet guaranteed.
  - **Root cause:** pre-v1 workflows evolve faster than top-level onboarding docs.
  - **Fix:** publish one canonical matrix and link it from README core references.
  - **Prevention:** update install matrix and README in the same commit whenever distribution workflow changes.

## Things I Learned
- Commit-pinned Cargo installs are the clearest reproducible pre-v1 operator path.
- Explicit platform caveats reduce support ambiguity for socket-bound test behavior and toolchain prerequisites.

## Debug Notes (Only if applicable)
- **Failing test / error:** none.
- **Repro steps:** n/a.
- **Breakpoints / logs used:** test output only.
- **Final diagnosis:** ER-P2-001 acceptance criteria met.

## Follow-ups / TODO (For Future Agents)
- [ ] Add package-manager-specific install rows once official distribution channels are live.
- [ ] Revisit matrix defaults at v1 release cutover.

## Links / References
- Files touched:
  - `docs/INSTALL_MATRIX.md`
  - `README.md`
  - `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`
- Related docs:
  - `docs/RELEASE_PROCESS.md`
  - `docs/V1_SCOPE.md`
