# Kujo v1.0 Release Readiness Gap Checklist

Audit date: 2026-06-19
Status: active release-readiness gap checklist
Owner: Kujo core/release maintainers

## Purpose

This document consolidates the remaining work that should be closed before starting a final release-readiness review for Kujo `1.0.0`.

It is based on a documentation sweep across top-level docs, `docs/`, `notes/`, `examples/`, `tools/`, and benchmark docs. Generated docs were used only where an active checklist referenced them as source-of-truth evidence.

## Current Verdict

Kujo appears to have completed most implementation hardening checklists, but it is not ready for final release sign-off until the release state, evidence, artifact publication, and stale critical-note surfaces are reconciled.

The highest-signal gaps found in the docs are:

- Release-state truth set is reconciled as pre-tag `1.0.0` release-candidate readiness, with `Cargo.toml` staged at `1.0.0` but final tag/publish/artifact evidence still open.
- `ROADMAP.md` has its final release checklist fully checked, but `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` and `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` still have tag-time publication/sign-off items unchecked.
- `CHANGELOG.md` only has `[Unreleased]`; there is no final `[1.0.0]` release section.
- Several stale docs/notes still describe fixed or superseded failures as current critical blockers, especially image method dispatch and older assignment/dict mutation bugs.

## Source Notes

Primary docs reviewed:

- `README.md`
- `ROADMAP.md`
- `CHANGELOG.md`
- `Cargo.toml`
- `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`
- `docs/PRE_V1_ACTION_CHECKLIST.md`
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`
- `docs/RELEASE_PROCESS.md`
- `docs/V1_SCOPE.md`
- `docs/UNFINISHED_AND_MVP_AUDIT.md`
- `docs/V1_0_REMAINING_NON_RELEASE_WORK_CHECKLIST.md`
- `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`
- `docs/V1_0_UNIVERSAL_USEFULNESS_EXPANSION_CHECKLIST.md`
- `docs/V1_0_HARDENING_AND_LEANNESS_CHECKLIST.md`
- `docs/V1_0_TECH_READINESS_CHECKLIST.md`
- `docs/VM_INTERPRETER_PARITY_MATRIX.md`
- `docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md`
- `docs/ARCHITECTURE.md`
- `docs/IMAGE_CONVERSION_AGENT_HANDOFF.md`
- `docs/PERFORMANCE.md`
- `notes/GOTCHAS.md`
- `notes/bug_dict_index_assignment_hangs.md`
- `notes/MUTATION_OPERATOR_BUG.md`
- `notes/2026-01-30_dict_optimization_investigation.md`
- `notes/2026-06-08_22-00_v1x-type-001-import-signature-resolution-and-loop-scope-fix.md`
- `notes/2026-06-08_22-26_v1x-checklist-evidence-refresh.md`

ShipCheck scan source:

- Command: `/Users/robertdevore/2026/Kujolang/kujo-repos/kujo/target/release/kujo run shipcheck.kujo scan --dir /Users/robertdevore/2026/Kujolang/kujo-repos/kujo`
- Result: gate passed with warnings.
- Warnings: no lint command detected, no format command detected, no `kennel.toml`, no clear entry point detected.

## P0 - Must Close Before Release Readiness Review

- [x] **V1RR-P0-001: Reconcile the release-state truth set.**
  - Problem: release-state docs disagree. `Cargo.toml` is `1.0.0`; `docs/RELEASE_PROCESS.md` says Kujo is now at `1.0.0`; `README.md` says Kujo is not ready for `1.0.0`; `docs/ARCHITECTURE.md` says current crate version is `0.14.0`; `docs/V1_SCOPE.md` says `v0.14.0 scope gate baseline`.
  - Acceptance:
    - Decide whether the repo is pre-tag `1.0.0`, an RC, or already `1.0.0`.
    - Update `README.md`, `ROADMAP.md`, `docs/ARCHITECTURE.md`, `docs/V1_SCOPE.md`, `docs/RELEASE_PROCESS.md`, and any version-state contract tests to tell one story.
    - Add a short dated evidence note explaining the chosen state and why `Cargo.toml` should remain `1.0.0` or be changed.
    - Run `cargo test --test readme_contracts`, `cargo test --test architecture_docs_contract`, `cargo test --test release_process_docs_contract`, and `cargo test --test docs_policy_consistency_contract`.
  - Evidence 2026-06-19:
    - Chosen state: pre-tag `1.0.0` release-candidate readiness. `Cargo.toml` remains `1.0.0` so release-candidate validation and release-state guards exercise final crate metadata, but the final tag, crate publication, and binary artifact sign-off remain incomplete until tag-time evidence exists.
    - Updated `README.md`, `ROADMAP.md`, `docs/ARCHITECTURE.md`, `docs/V1_SCOPE.md`, `docs/RELEASE_PROCESS.md`, canonical boundary wording in related policy docs, and release-state contract tests.
    - Added evidence note: `notes/2026-06-19_v1_0_release_state_reconciliation.md`.
    - Validation passed: `cargo test --test readme_contracts`; `cargo test --test architecture_docs_contract`; `cargo test --test release_process_docs_contract`; `cargo test --test docs_policy_consistency_contract`; `cargo test --test v1_maturity_boundary_alignment_contract`; `cargo test --test stdlib_reference_policy_contract`; `bash .github/scripts/check-release-state.sh`; `rustfmt --check tests/v1_scope_docs_alignment.rs tests/architecture_docs_contract.rs tests/docs_policy_consistency_contract.rs tests/v1_maturity_boundary_alignment_contract.rs tests/stdlib_reference_policy_contract.rs`.

- [ ] **V1RR-P0-002: Close tag-time artifact publication/sign-off blockers.**
  - Problem: `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` still has `V1U-OPEN-003` and `V1U-FINAL-003` unchecked, and `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` still has all tag-time sign-off rows unchecked.
  - Acceptance:
    - Publish or intentionally defer the actual `v1.0.0` release event with an explicit release exception.
    - Confirm attached Linux, macOS, and Windows assets.
    - Confirm per-asset `.sha256` files and `checksums.txt`.
    - Confirm `.github/workflows/release-published-artifact-smoke.yml` passes for the published release.
    - Record artifact URLs, checksum values, and command logs in a dated `notes/` evidence file.
    - Mark the relevant release-artifact checklist rows only after real evidence exists.
  - Blocker 2026-06-19:
    - This item requires the actual `v1.0.0` tag/publication event and post-publish artifact evidence.
    - It is intentionally not completed in this release-readiness round because the active instruction forbids tagging, publishing, or marking tag-time artifact sign-off complete unless explicitly given `UNBLOCK_V1_RELEASE`.
    - Evidence note: `notes/2026-06-19_v1_0_tag_time_artifact_blocker.md`.
    - Current file evidence: `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` still has tag-time rows unchecked, and `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` still tracks `V1U-OPEN-003`/`V1U-FINAL-003` as release-flight items.

- [x] **V1RR-P0-003: Run a fresh final gate bundle on the current tree.**
  - Problem: the active docs include strong pass evidence from 2026-05-26 and 2026-06-08, but release readiness needs evidence from the final candidate tree.
  - Acceptance:
    - Run and record:
      - `cargo fmt --check`
      - `cargo check`
      - `cargo clippy --all-targets --all-features -- -D warnings`
      - `cargo test`
      - `cargo test --test docs_examples`
      - `cargo test --test readme_contracts`
      - `cargo test --test cli_contracts`
      - `cargo test --test cli_json_contracts`
      - `cargo test --test diagnostics_golden`
      - `cargo test --test native_api_security_boundaries`
      - `cargo test --test runtime_security`
      - `cargo test --test vm_interpreter_parity_surfaces`
      - `cargo run -- test --runtime vm`
      - `cargo run -- test --runtime dual`
      - `bash scripts/release_candidate_gate.sh --full`
    - If socket-sensitive suites are intentionally skipped, capture the exact environment reason and run the documented alternate gate.
    - Archive command output paths and summary in a dated `notes/` evidence file.
  - Evidence 2026-06-19:
    - Final candidate-tree evidence note: `notes/2026-06-19_v1_0_final_gate_bundle_evidence.md`.
    - Command logs and exit status manifest: `notes/release_evidence/2026-06-19_p0-003-final/status.tsv`.
    - All required commands passed with exit code `0`, including `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, the focused contract suites, `cargo run -- test --runtime vm`, `cargo run -- test --runtime dual`, and `bash scripts/release_candidate_gate.sh --full`.
    - No socket-sensitive suite was skipped; the full release candidate gate ran `serve_command_integration` serially and passed.
    - Gate follow-up fixed formatting drift in two interpreter files, refreshed generated artifacts, documented missing stdlib entries, updated the lockfile for `tokio-postgres`/`postgres-protocol` advisories, and added an explicit `cargo audit --ignore RUSTSEC-2023-0071` exception for the no-fixed-upgrade `rsa` advisory.

- [x] **V1RR-P0-004: Refresh generated evidence and active checklist snapshots.**
  - Problem: multiple active checklists cite generated counts and dated evidence. The June notes already call out that generated artifacts can drift after source churn.
  - Acceptance:
    - Regenerate and validate at least:
      - `docs/generated/V1_CODE_TODO_TRIAGE.md`
      - `docs/generated/UNSAFE_INVENTORY.md`
      - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
    - Run:
      - `cargo test --test v1_code_todo_triage_contract`
      - `cargo test --test unsafe_inventory_contract`
      - `cargo test --test vm_runtime_mismatch_inventory_contract`
      - `cargo test --test generated_artifact_freshness_contract`
    - Update any checklist prose that quotes stale counts, line totals, or dates.
  - Evidence 2026-06-19:
    - Generated evidence note: `notes/2026-06-19_v1_0_generated_evidence_refresh.md`.
    - Command logs and exit status manifest: `notes/release_evidence/2026-06-19_p0-004/status.tsv`.
    - Regenerated `docs/generated/V1_CODE_TODO_TRIAGE.md`, `docs/generated/UNSAFE_INVENTORY.md`, `docs/generated/UNSAFE_INVENTORY.csv`, `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`, and `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`.
    - Current counts after `V1RR-P0-006`: TODO/FIXME/HACK triage has `29` markers and `0` unclassified; unsafe inventory has `65` total matches, `55` executable, `10` non-executable, and `0` unknown; VM mismatch inventory has `P0 runtime-parity-bug: 6`, `P1 stale-snapshot-expectation: 5`, and `P2 harness-debt: 0`.
    - Updated active checklist/docs prose that still quoted stale generated totals or zero-parity wording.

- [x] **V1RR-P0-005: Cut a real `CHANGELOG.md` release section.**
  - Problem: `CHANGELOG.md` currently only has `[Unreleased]`, while the release docs require concrete user-impact notes.
  - Acceptance:
    - Add `[1.0.0] - YYYY-MM-DD` with `Added`, `Changed`, `Fixed`, `Security`, `Performance`, and `Removed` sections where applicable.
    - Include compatibility-impacting changes, security hardening, VM-first runtime posture, deferred/experimental surfaces, and migration notes.
    - Ensure release docs and `docs/V1_SCOPE.md` point to the changelog as final release evidence.
  - Evidence 2026-06-19:
    - Added `CHANGELOG.md` section `[1.0.0] - 2026-06-19` with `Added`, `Changed`, `Fixed`, `Security`, `Performance`, and `Removed` subsections.
    - Release note explicitly states the pre-tag release-candidate boundary: crate metadata is staged at `1.0.0`, while final tag, crate publication, and binary artifact sign-off remain governed by tag-time release evidence.
    - Updated `docs/V1_SCOPE.md` handoff checklist to name `CHANGELOG.md` as the release-note evidence surface.
    - Evidence note: `notes/2026-06-19_v1_0_changelog_release_section.md`.
    - Command logs and exit status manifest: `notes/release_evidence/2026-06-19_p0-005/status.tsv`.

- [x] **V1RR-P0-006: Reconcile the `kujo test` fixture-count story.**
  - Problem: several active docs report `cargo run -- test --runtime vm`/`dual` as passing while also showing summaries like `137/150`. That can be correct if expected-fail fixtures are policy-governed, but it needs to be unmistakable before launch.
  - Acceptance:
    - Document why `137/150` is a passing release result, or reduce the remaining fixture misses if they are not intentionally expected-fail.
    - Cross-check `examples/README_examples.md` and `tests/docs_examples.rs` expected-fail policy.
    - Ensure `kujo test` output clearly distinguishes expected-fail, skipped, failed, and passed fixtures.
    - Run `cargo test --test docs_examples`, `cargo run -- test --runtime vm`, and `cargo run -- test --runtime dual`.
  - Evidence 2026-06-19:
    - Evidence note: `notes/2026-06-19_v1_0_kujo_test_fixture_count_reconciliation.md`.
    - Command logs and exit status manifest: `notes/release_evidence/2026-06-19_p0-006/status.tsv`.
    - `kujo test` now emits explicit fixture outcome counters: `passed`, `failed`, `skipped`, `expected_fail`, `runnable`, and `discovered`.
    - Current VM and dual sweeps both pass with `Passed 144/144 tests` and `Fixture outcomes: passed=144, failed=0, skipped=6, expected_fail=0, runnable=144, discovered=150`.
    - The six skipped fixtures are `kujo test-run` framework fixtures, not hidden failures or expected-fail release fixtures. Expected-fail policy remains scoped to examples/docs smoke coverage in `tests/docs_examples.rs` and `examples/README_examples.md`.
    - Stabilized `tests/test_stdlib_system.kujo` by removing host-dependent printed argument/timing values; regenerated VM mismatch inventory now reports `P0 runtime-parity-bug: 6`, `P1 stale-snapshot-expectation: 5`, and `P2 harness-debt: 0`.

- [x] **V1RR-P0-007: Resolve ShipCheck release warnings or document them as intentional.**
  - Problem: ShipCheck passes with warnings, but release readiness should not leave ambiguous metadata warnings unexplained.
  - Acceptance:
    - Make ShipCheck detect the existing format/lint commands, or document why `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` are intentionally outside its detection.
    - Decide whether absence of `kennel.toml` is expected for the language runtime repo.
    - Document the entry point explicitly (`src/main.rs` / binary `kujo`) in the place ShipCheck expects, or add a release exception.
    - Re-run ShipCheck `scan` and `checklist` and record the result.
  - Evidence 2026-06-19:
    - Added release exception doc: `docs/SHIPCHECK_RELEASE_EXCEPTIONS.md`.
    - Updated `docs/RELEASE_PROCESS.md` to link the exception note and name the Cargo-owned commands/entry point.
    - Decision: do not add a `kennel.toml` or Makefile solely to satisfy ShipCheck 0.1 detectors; the canonical release commands remain `scripts/release_gate.sh --full` / `scripts/release_candidate_gate.sh --full`, and the runtime entry point remains Cargo binary `kujo` from `src/main.rs`.
    - Re-ran ShipCheck `scan` and `checklist`; scan still passes with the same four intentional warnings, now documented as release exceptions.
    - Evidence note: `notes/2026-06-19_v1_0_shipcheck_release_exceptions.md`.
    - Command logs and exit status manifest: `notes/release_evidence/2026-06-19_p0-007/status.tsv`.


## P1 - Strongly Recommended Before Launch

- [ ] **V1RR-P1-001: Close or archive stale critical bug and handoff docs.**
  - Problem: stale docs still describe fixed or superseded issues as current blockers. Examples include:
    - `docs/IMAGE_CONVERSION_AGENT_HANDOFF.md` says `img.save(...)`/`img.resize(...)` fail, but `tests/image_conversion_integration.rs` now covers PNG/JPEG/WebP round trips in interpreter and VM.
    - `notes/bug_dict_index_assignment_hangs.md` says dict mutation is completely broken, while later docs identify syntax confusion and current tests cover index assignment/update paths.
    - `notes/MUTATION_OPERATOR_BUG.md` describes `=` mutation as a critical v1 blocker; current language docs and tests need to clarify whether `=` is assignment syntax, legacy syntax, or intentionally unsupported in favor of `:=`.
  - Acceptance:
    - Add current-status headers to stale notes or move their conclusions into an archive/stale-notes index.
    - Link each stale critical note to the modern test or doc that supersedes it.
    - For any still-real issue, create an active checklist item with current reproduction and tests.

- [ ] **V1RR-P1-002: Finish or explicitly defer the remaining optional-typing cluster.**
  - Problem: the June 8 type-checker note leaves follow-ups for destructuring inference, module existence checks, struct field type lookup, Promise unwrap typing, and the permissive callable fallback decision.
  - Acceptance:
    - Either close the cluster with tests and docs, or document it as post-v1 in `docs/OPTIONAL_TYPING_DESIGN.md` and `docs/V1_SCOPE.md`.
    - Run `cargo test type_checker::tests::`, `cargo test --test optional_typing_v1_contract`, and `cargo test --test v1_code_todo_triage_contract`.

- [ ] **V1RR-P1-003: Align JIT and performance docs with release posture.**
  - Problem: `docs/VM_INTERPRETER_PARITY_MATRIX.md` says JIT is experimental/opt-in via `kujo run --jit`, while `docs/PERFORMANCE.md` says JIT activates automatically after 100 iterations and needs no flags.
  - Acceptance:
    - Update `docs/PERFORMANCE.md`, `README.md`, and any JIT references to consistently describe default VM behavior, JIT feature flags, opt-in status, and unsupported-surface fallback.
    - Include current benchmark evidence and avoid unsupported performance promises.
    - Run relevant docs contract tests and JIT-focused tests.

- [ ] **V1RR-P1-004: Revalidate clean checkout and feature-matrix builds.**
  - Problem: release docs mention optional runtime features and reduced builds, but final release evidence should prove current combinations still compile.
  - Acceptance:
    - In a clean checkout or clean worktree, run:
      - `cargo build --release`
      - `cargo check --no-default-features`
      - `cargo check --no-default-features --features runtime-jit`
      - `cargo check --no-default-features --features runtime-db,runtime-image,runtime-archive`
    - Record binary sizes using `scripts/measure_binary_size.sh`.
    - Confirm install docs still match artifact names and features.

- [ ] **V1RR-P1-005: Revalidate untrusted-mode and host-effect examples.**
  - Problem: safety docs are strong, but final launch quality depends on examples not accidentally teaching unsafe defaults.
  - Acceptance:
    - Review README examples, `docs/NATIVE_API_SECURITY_POSTURE.md`, `docs/STANDARD_LIBRARY_REFERENCE.md`, and host-effect examples for trusted/untrusted wording.
    - Add/refresh negative-path tests for filesystem, process, HTTP/network, archive, image, and database surfaces where launch examples exercise them.
    - Run `cargo test --test native_api_security_boundaries` and `cargo test --test runtime_security`.

- [ ] **V1RR-P1-006: Final docs freshness pass for versioned baselines.**
  - Problem: several docs still carry older baseline labels (`v0.13.0`, `v0.14.0`) even when they may remain active guidance.
  - Acceptance:
    - Review `docs/ARCHITECTURE.md`, `docs/V1_SCOPE.md`, `docs/LSP_RELIABILITY.md`, `docs/TREE_SITTER_KUJO.md`, `docs/INSTALLATION_LSP_EDITORS.md`, and editor/tool docs.
    - Decide per document whether the old version label is intentional historical baseline or stale release-state text.
    - Update or annotate each one.

- [ ] **V1RR-P1-007: Normalize release-freeze/unblock language.**
  - Problem: `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` repeatedly says tag-time work is blocked until `UNBLOCK_V1_RELEASE`. That may have been session-specific rather than standing release policy.
  - Acceptance:
    - Decide whether `UNBLOCK_V1_RELEASE` remains the required explicit release directive.
    - Move that rule into `docs/RELEASE_PROCESS.md` if it is standing policy, or retire stale session-specific blocker language from active checklists.

## P2 - Post-Signoff Polish Or Explicit Deferral

- [ ] **V1RR-P2-001: Triage historical field-note TODOs into archive vs active backlog.**
  - Problem: `notes/` contains many unchecked follow-ups from old implementation sessions. Most are not v1 blockers, but their unchecked state makes broad doc review noisy.
  - Acceptance:
    - Create or update an index that classifies field-note follow-ups as `archive`, `post-v1`, or `active`.
    - Ensure active items point to a maintained checklist, not only a historical note.

- [ ] **V1RR-P2-002: Decide package registry/Kennel launch boundaries.**
  - Problem: ShipCheck warns about missing `kennel.toml`, and package workflow docs describe deterministic local package workflows. Registry/publish expectations should be explicit.
  - Acceptance:
    - Document whether v1 includes only local package workflows or any registry/publish story.
    - Align `docs/WORKFLOW_PACKS.md`, `docs/KENNEL_NAMESPACE_PLAN.md`, and release metadata docs.

- [ ] **V1RR-P2-003: Refresh benchmark publication strategy.**
  - Problem: SSG/cross-language benchmark docs contain many future measurement tasks and older host-specific notes.
  - Acceptance:
    - Decide which benchmark results are launch claims and which are internal regression signals.
    - Remove or mark old performance promises that are not backed by current reproducible runs.

- [ ] **V1RR-P2-004: Editor and LSP launch matrix pass.**
  - Problem: editor docs have older baseline versions and advanced LSP follow-ups in historical notes.
  - Acceptance:
    - Confirm VS Code/Cursor, Neovim, JetBrains, and generic LSP instructions are still accurate.
    - Run or document the current editor-adapter smoke path.

## Suggested Execution Order

1. `V1RR-P0-001`
2. `V1RR-P0-003`
3. `V1RR-P0-004`
4. `V1RR-P0-006`
5. `V1RR-P0-005`
6. `V1RR-P0-007`
7. `V1RR-P1-001`
8. `V1RR-P1-003`
9. `V1RR-P1-002`
10. `V1RR-P0-002`

`V1RR-P0-002` is last because it requires the real release publication event. Everything above it should be clean before the tag-time artifact checklist is touched.

## Definition Of Done

This checklist is complete when:

- every P0 item is checked with dated evidence,
- every P1 item is either checked or explicitly deferred with owner/rationale,
- release-state wording is consistent across root docs and release docs,
- final gate evidence exists for the candidate tree,
- tag-time artifact evidence exists or an explicit no-release decision is recorded,
- stale critical docs no longer present fixed/superseded bugs as current launch blockers.
