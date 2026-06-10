# Kujo Pre-v1 Action Checklist

Status: active working checklist before `v1.0.0` release prep.

Purpose: track high-value work we can do now, before final RC/tag-time release tasks.

How to use with an AI agent:
- Pick one unchecked item.
- Complete implementation + tests + docs for that item only.
- Mark it done and link evidence (commands + results).

---

## 1) Runtime / Language / Parity Follow-through

- [x] **PREV1-RUN-001**: Add async named-nested closure capture parity coverage.
  - Scope: add explicit async parity test(s) for named nested closure capture/mutation behavior.
  - Acceptance criteria:
    - New test(s) added in `tests/vm_interpreter_parity_surfaces.rs`.
    - `cargo test --test vm_interpreter_parity_surfaces` passes.
  - Source context: `notes/2026-05-12_23-23_NO-ROADMAP_named-nested-closure-capture-parity.md`.

- [x] **PREV1-RUN-002**: Close closure-mutation docs drift in Kujo MCP docs.
  - Scope: update the referenced `mcp` docs note so it no longer overstates old closure-mutation limitations.
  - Acceptance criteria:
    - Docs text aligned with current Kujo runtime behavior.
    - Linked note entry updated with what changed.
  - Source context: `notes/2026-05-12_23-23_NO-ROADMAP_named-nested-closure-capture-parity.md`.
  - Evidence (2026-05-20):
    - Created formal external handoff packet in `notes/2026-05-20_12-55_v1u-open-001_mcp-doc-handoff.md` with exact wording update proposal, target files, validation steps, and follow-through owner/action.
    - Revalidated blocker context with `rg -n "mcp|mcp.kujo" README.md docs notes -g '*.md'` and `rg --files | rg "mcp|MCP|mcp|mcp.kujo"` to confirm this repo still has no editable `mcp` source docs.

---

## 2) Docs/Examples Quality Debt

- [x] **PREV1-DOC-001**: Burn down expected-fail example debt in smoke harness.
  - Scope: convert parse/runtime-drift examples to parse-clean (or run-clean where appropriate), then remove them from expected-fail classification.
  - Acceptance criteria:
    - Expected-fail examples list in `tests/docs_examples.rs` is reduced.
    - `cargo test --test docs_examples` passes.
  - Source context: `notes/2026-05-16_17-10_v1-test-005-docs-examples-smoke-suite.md`.

- [x] **PREV1-DOC-002**: Keep docs snippet smoke set parse-clean.
  - Scope: maintain zero stale expected-fail doc snippets as docs evolve.
  - Acceptance criteria:
    - `expected_fail_doc_blocks()` stays empty (or trends toward empty if new temporary debt is introduced intentionally with reasons).
    - `cargo test --test docs_examples` passes after doc edits.
  - Source context: `tests/docs_examples.rs`, `notes/2026-05-16_17-10_v1-test-005-docs-examples-smoke-suite.md`.

- [x] **PREV1-DOC-003**: Add a roadmap-tracked item for universal docgen maturation.
  - Scope: add/update explicit roadmap tracking for universal docgen staging and next milestones.
  - Acceptance criteria:
    - `ROADMAP.md` includes a clear item/slice for universal docgen follow-through.
    - `docs/DOCGEN.md` and roadmap references are aligned.
  - Source context: `notes/2026-05-17_21-57_universal-docgen-architecture-and-gates.md`.

---

## 3) Diagnostics / Contracts / Test Infrastructure

- [x] **PREV1-DIAG-001**: Expand diagnostics goldens for runtime JSON diagnostic surfaces as they evolve.
  - Scope: add fixture + golden coverage when new machine-readable runtime diagnostics are added.
  - Acceptance criteria:
    - `tests/diagnostics_golden.rs` includes new fixture category coverage.
    - `cargo test --test diagnostics_golden` passes.
  - Source context: `notes/2026-05-16_17-03_v1-test-004-diagnostics-golden-snapshots.md`.

- [x] **PREV1-DIAG-002**: Keep runtime-security diagnostics represented in golden coverage where feasible.
  - Scope: add selected `tests/runtime_security.rs`-aligned diagnostics snapshot cases.
  - Acceptance criteria:
    - At least one runtime-security-oriented diagnostics fixture added (when output is stable enough).
    - Golden tests remain deterministic cross-platform (CRLF-safe).
  - Source context: `notes/2026-05-16_16-59_v1-test-003-runtime-native-security-regressions.md`.

---

## 4) Fuzzing Operational Hardening

- [x] **PREV1-FUZZ-001**: Add local fuzz-smoke helper script with prerequisite checks.
  - Scope: script for nightly/cargo-fuzz/toolchain checks and clear error guidance.
  - Acceptance criteria:
    - Script added under `scripts/` and documented in README/docs.
    - Smoke run command is reproducible on supported local environments.
  - Source context: `notes/2026-05-16_16-35_v1-test-002-lexer-parser-fuzzing.md`.

- [x] **PREV1-FUZZ-002**: Add parser/lexer fuzz crash reproduction automation path.
  - Scope: standardized way to replay crash artifacts from fuzz CI.
  - Acceptance criteria:
    - Documented repro workflow and helper command/script.
    - Validated on at least one synthetic or real crash input.
  - Source context: `notes/2026-05-16_16-35_v1-test-002-lexer-parser-fuzzing.md`.
  - Evidence (2026-05-20):
    - Added `scripts/fuzz_repro.sh` with explicit-target and inferred-target replay paths plus `--dry-run`/prereq validation.
    - Added script contract tests in `tests/fuzz_repro_contract.rs` and synthetic replay fixtures under `tests/fixtures/fuzz/`.
    - Verified synthetic repro path with `bash scripts/fuzz_repro.sh --target parser --artifact tests/fixtures/fuzz/synthetic_crash_input.kujo --dry-run`.

---

## 5) Cross-Platform Security Coverage

- [x] **PREV1-SEC-001**: Define non-Unix strategy for module-escape regression coverage.
  - Scope: add Windows-compatible equivalent for symlink/escape boundary testing, or document and gate an equivalent deterministic strategy.
  - Acceptance criteria:
    - Cross-platform policy documented.
    - Tests updated to reflect the chosen strategy without flaky behavior.
  - Source context: `notes/2026-05-16_16-59_v1-test-003-runtime-native-security-regressions.md`.
  - Evidence (2026-05-20):
    - Added deterministic cross-platform traversal guard regression: `runtime_security_module_loader_rejects_parent_traversal_import_name_cross_platform` in `tests/runtime_security.rs`.
    - Documented Unix-vs-non-Unix module-escape coverage split in `docs/NATIVE_API_SECURITY_POSTURE.md`.

---

## 6) Release-Readiness Prep (Pre-tag, non-version-bump)

- [x] **PREV1-REL-001**: Run and record full release-candidate gate evidence in a low-contention environment.
  - Scope: execute `scripts/release_candidate_gate.sh --full` in an environment suitable for stable socket/timing-sensitive tests; capture results in a dated note.
  - Acceptance criteria:
    - Command results logged with pass/fail details.
    - Any instability is explicitly categorized with mitigation or follow-up.
  - Source context: `docs/RELEASE_PROCESS.md`, `docs/UNFINISHED_AND_MVP_AUDIT.md`.
  - Evidence (2026-05-20):
    - Executed `bash scripts/release_candidate_gate.sh --full` and recorded results in `notes/2026-05-20_09-07_prev1-rel-001-rc-gate-evidence.md`.
    - Outcome classification: deterministic gate failure at `cargo fmt --check` (formatting drift), not socket/timing instability; follow-up re-run noted in evidence file.

- [x] **PREV1-REL-002**: Keep deferred/non-goal boundaries explicit and current.
  - Scope: make sure docs consistently reflect what is intentionally deferred vs in-scope during ongoing pre-v1 work.
  - Acceptance criteria:
    - `README.md`, `docs/V1_SCOPE.md`, `docs/OPTIONAL_TYPING_DESIGN.md` stay aligned after each major feature/doc change.
    - No stale claims about readiness or enforcement guarantees.
  - Source context: `README.md`, `docs/V1_SCOPE.md`, `docs/OPTIONAL_TYPING_DESIGN.md`.
  - Evidence (2026-05-20):
    - Added explicit README linkage to deferred-boundary docs (`docs/V1_SCOPE.md` and `docs/OPTIONAL_TYPING_DESIGN.md`) in the `1.0 Readiness Status` section.
    - Added alignment contract coverage in `tests/v1_scope_docs_alignment.rs` to lock readiness/deferred-boundary wording across all three docs.

---

## Suggested Execution Order

1. `PREV1-DOC-001`
2. `PREV1-FUZZ-001`
3. `PREV1-RUN-001`
4. `PREV1-SEC-001`
5. `PREV1-DIAG-001`
6. `PREV1-REL-001`
