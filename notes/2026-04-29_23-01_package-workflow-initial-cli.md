# Kujo Field Notes — v0.12 Package Workflow Initial CLI Slice

**Date:** 2026-04-29
**Session:** 23:01 local
**Branch/Commit:** main / 2613f8b
**Scope:** Implemented the next highest-priority incomplete v0.12.0 track (Package/project workflow) with initial manifest, init scaffold, dependency workflow commands, tests, and release evidence updates.

---

## What I Changed
- Added src/package_workflow.rs with:
  - default `kujo.toml` manifest generation
  - manifest parsing helpers
  - dependency metadata insertion/update helpers
- Added package/project CLI surfaces in src/main.rs:
  - `kujo init`
  - `kujo package-add`
  - `kujo package-install`
  - `kujo package-publish`
- Added module declarations in both crate roots:
  - src/main.rs (mod package_workflow;)
  - src/lib.rs (pub mod package_workflow;)
- Added focused tests in src/package_workflow.rs for:
  - manifest generation
  - dependency insert/update path
  - invalid dependency input handling
- Verified end-to-end smoke workflow in a temp directory:
  - init -> add -> install -> publish preview

## Gotchas (Read This Next Time)
- **Gotcha:** Destructive shell cleanup commands can be blocked by policy even for temp paths.
  - **Symptom:** Attempted temp-directory cleanup command was denied.
  - **Root cause:** `rm` is deny-listed in this environment.
  - **Fix:** Switched to using unique fresh temp directory paths per smoke run.
  - **Prevention:** Prefer unique temp paths over cleanup commands when running iterative smoke tests.

## Things I Learned
- A lightweight package workflow can be staged effectively by separating manifest data logic into a dedicated module and keeping CLI orchestration thin.
- Publish-preview output is useful for workflow validation before implementing full remote publish behavior.
- Using deterministic manifest shapes simplifies both command behavior and regression tests.

## Debug Notes (Only if applicable)
- **Failing test / error:** No code-level failure in this round; one denied shell command due policy.
- **Repro steps:** Attempted command included deny-listed removal operation.
- **Breakpoints / logs used:** terminal deny-list message.
- **Final diagnosis:** environment policy, not code defect.

## Follow-ups / TODO (For Future Agents)
- [ ] Add lockfile and deterministic resolution behavior for dependency installs.
- [ ] Add registry authentication and publish transport implementation behind `package-publish --publish`.
- [ ] Add dependency remove/update workflows for parity with add/install.

## Links / References
- Files touched:
  - src/package_workflow.rs
  - src/main.rs
  - src/lib.rs
  - ROADMAP.md
  - CHANGELOG.md
  - README.md
- Related docs:
  - notes/FIELD_NOTES_SYSTEM.md
  - notes/GOTCHAS.md
  - notes/2026-04-29_22-57_linter-initial-cli.md
