# Kujo Field Notes — V1 docs and RC gate follow-through

**Date:** 2026-05-16
**Session:** 17:40 local
**Branch/Commit:** main / 4cc5eb9
**Scope:** Completed roadmap documentation and release-readiness items `V1-DOC-001` through `V1-REL-001`, including new docs contract tests and an executable release-candidate gate script.

---

## What I Changed
- Completed roadmap items and marked them done in `ROADMAP.md`:
  - `V1-DOC-001`
  - `V1-DOC-002`
  - `V1-DOC-003`
  - `V1-DOC-004`
  - `V1-REL-001`
- Expanded language-spec alignment docs and added semantic contract tests:
  - `docs/LANGUAGE_SPEC.md`
  - `tests/language_spec_contracts.rs`
- Reworked native security posture docs and added docs-to-CLI contract checks:
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `tests/security_posture_docs_contract.rs`
- Expanded release process/compatibility policy docs and added policy contract checks:
  - `docs/RELEASE_PROCESS.md`
  - `tests/release_process_docs_contract.rs`
- Updated README first-contact framing and added README contract test:
  - `README.md`
  - `tests/readme_contracts.rs`
- Added executable release-candidate gate script and tests:
  - `scripts/release_candidate_gate.sh`
  - `tests/release_candidate_gate_contract.rs`
- Updated `CHANGELOG.md` entries for each completed roadmap item.

## Gotchas (Read This Next Time)
- **Gotcha:** Full-suite verification can fail intermittently on local socket/timing-sensitive tests even when item-specific changes are docs-only.
  - **Symptom:** `cargo test` intermittently fails in `tests/serve_command_integration.rs` with connection refused/port allocation errors, and occasionally fails async timing thresholds (for example `test_concurrent_tasks`).
  - **Root cause:** Local environment contention/permission variability for short-lived socket startup and timing-sensitive expectations.
  - **Fix:** Re-run targeted suites (`serve_command_integration`, contract tests) and record explicit local blocker context in `ROADMAP.md` when full-run instability is unrelated to the implemented item.
  - **Prevention:** Prefer deterministic targeted suites during iteration and keep release-checklist status truthful about local full-suite instability.

- **Gotcha:** `scripts/release_gate.sh --full` can fail early on pre-existing formatting drift unrelated to the current item.
  - **Symptom:** `cargo fmt --check` reports diffs in unrelated tests.
  - **Root cause:** Repository contains formatting drift in files outside the current change scope.
  - **Fix:** Treat as explicit release-checklist blocker and document it rather than silently broad-formatting unrelated surfaces during scoped roadmap work.
  - **Prevention:** Track checklist status in roadmap explicitly (`fmt` unchecked) until maintainers decide on broad formatting sweep.

## Things I Learned
- The final roadmap checklist is useful as a status ledger even before tagging: we can mark verified items (`P0/P1 complete`, docs accuracy, targeted suites) while keeping gate blockers (`fmt`, `clippy`, nondeterministic full `cargo test`, version bump, clean-tree RC build) honestly unchecked.
- For release readiness work, small contract tests that parse docs and CLI help output are high-value because they prevent policy drift with low runtime cost.
- An executable RC gate wrapper (`release_candidate_gate.sh`) makes the release process less tribal and easier to audit than prose-only instructions.

## Debug Notes (Only if applicable)
- **Failing test / error:** `serve_blocks_dot_ds_store_with_403` and related `serve_command_integration` tests intermittently reported `failed to connect to serve process: Connection refused (os error 61)`; another run failed with `failed to allocate test port: Operation not permitted`.
- **Repro steps:** `cargo test` after repeated full-suite runs.
- **Breakpoints / logs used:** Focused reruns of `cargo test --test serve_command_integration -- --test-threads=1` and repeated `cargo test` attempts.
- **Final diagnosis:** Environment-sensitive local socket/timing instability, not tied to docs/RC-gate code paths.

## Follow-ups / TODO (For Future Agents)
- [ ] Run and stabilize `cargo fmt --check` + `cargo clippy --all-targets --all-features -- -D warnings` for clean release-checklist closure.
- [ ] Investigate and harden `tests/serve_command_integration.rs` startup race/port-allocation resiliency.
- [ ] Re-run full release candidate gate (`bash scripts/release_candidate_gate.sh --full`) on a low-contention environment and update checklist statuses.

## Links / References
- Files touched:
  - `CHANGELOG.md`
  - `README.md`
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `docs/RELEASE_PROCESS.md`
  - `scripts/release_candidate_gate.sh`
  - `tests/language_spec_contracts.rs`
  - `tests/security_posture_docs_contract.rs`
  - `tests/release_process_docs_contract.rs`
  - `tests/readme_contracts.rs`
  - `tests/release_candidate_gate_contract.rs`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `docs/RELEASE_PROCESS.md`
