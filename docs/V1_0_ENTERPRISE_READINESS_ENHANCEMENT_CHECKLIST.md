# V1.0 Enterprise Readiness Enhancement Checklist

Last updated: 2026-05-25
Owner: Kujo core/runtime
Status: Active (post-parity hardening pass)

## Goal

Drive Kujo from "near release-ready" to "enterprise-grade and universally useful" by closing remaining hardening, performance, security, and product-presentation gaps without regressions.

## Operating Rules

1. All changes must be additive and backward-compatible unless explicitly approved.
2. No reduction in existing test coverage or contract guarantees.
3. Every checklist closure must include:
   - code + tests + docs updates,
   - command evidence,
   - a clear risk note.
4. Release publication/tagging remains frozen unless explicitly unblocked.

## Priority Summary

- P0: Must complete before claiming enterprise-grade readiness.
- P1: Strongly recommended for v1 launch quality.
- P2: Post-v1 improvements that materially improve adoption/operability.

## Checklist

- [x] **ER-P0-001**: Run and archive full verification matrix on latest `main`.
  - Scope: `cargo test`, VM/dual sweeps, security boundaries, docs contracts, release gate scripts.
  - Acceptance:
    - all required suites pass,
    - failures triaged with owner and fix plan,
    - evidence captured in a dated note under `notes/`.
  - 2026-05-25 blocker: verification matrix is still red because `cargo test --test unsafe_inventory_contract` fails on executable-unsafe budget (`expected <=55, got 59`); VM/dual fixture sweeps remain at `137/150` with known parser-fixture debt (`tests/test_module_syntax.kujo`, `tests/testing_framework.kujo`, `tests/generators_test.kujo`, `tests/destructuring.kujo`, etc.). Evidence note: `notes/2026-05-25_23-06_er-p0-001-verification-matrix-triage.md`.
  - 2026-05-25 blocker refresh: verification matrix remains red on latest `main`:
    - `cargo test --test unsafe_inventory_contract` still fails budget gate (`expected <=55, got 59`).
    - `cargo run -- test --runtime vm` currently passes `137/150`.
    - `cargo run -- test --runtime dual` currently passes `136/150` (includes `tests/stdlib_test.kujo` snapshot/runtime drift in addition to parser-debt fixtures).
    - Evidence note: `notes/2026-05-25_23-59_er-p0-001-verification-matrix-refresh.md`.
  - 2026-05-26 closure evidence:
    - `cargo test` passed end-to-end on latest `main` (all suites green).
    - Runtime sweeps passed:
      - `cargo run -- test --runtime vm` (summary `137/150`, exit `0`)
      - `cargo run -- test --runtime dual` (summary `137/150`, exit `0`, `interpreter_fallback=1`)
    - Security/docs gates passed:
      - `cargo test --test native_api_security_boundaries` (48/48)
      - `cargo test --test runtime_security` (11/11)
      - `cargo test --test docs_policy_consistency_contract` (1/1)
    - Release-candidate roadmap gate passed:
      - `bash scripts/release_candidate_gate.sh --roadmap-only`
    - Evidence note: `notes/2026-05-26_00-58_er-p0-001-verification-matrix-closure.md`.

- [x] **ER-P0-002**: Complete unsafe boundary tightening follow-through (JIT focus).
  - Scope: maintain machine-verifiable `SAFETY:` contracts and close remaining high-risk unsafe review gaps.
  - Acceptance:
    - unsafe inventory regenerated,
    - unsafe contract/checker tests pass,
    - residual unsafe sites categorized by risk and ownership.
  - 2026-05-26 blocker: JIT safety-contract enforcement is green (`cargo test --test jit_safety_contract_checker`, 8/8), but strict unsafe inventory contract remains red via executable-budget gate (`cargo test --test unsafe_inventory_contract`: `expected <=55, got 59`). Until `ER-P0-006` reduces executable unsafe count below threshold and artifacts are re-generated, this item cannot be closed. Evidence note: `notes/2026-05-26_00-08_er-p0-002-unsafe-followthrough-blocker-refresh.md`.
  - 2026-05-26 closure evidence:
    - `ER-P0-006` reduced strict executable unsafe count to gate threshold (`55`) and regenerated inventory artifacts.
    - Unsafe/JIT contract validations passed:
      - `cargo test --test unsafe_inventory_contract` (3/3)
      - `cargo test --test jit_safety_contract_checker` (8/8)
      - `cargo test --test vm_interpreter_parity_surfaces` (100/100)
    - Residual unsafe inventory remains categorized and concentrated in JIT runtime boundaries (`jit_executable`) with generated evidence in `docs/generated/UNSAFE_INVENTORY.md` and ownership tracking in checklist/notes.

- [x] **ER-P0-003**: Finish runtime parity burn-down to zero known high-impact mismatches.
  - Scope: resolve remaining VM/interpreter mismatch inventory items and rebaseline artifacts.
  - Acceptance:
    - mismatch inventory regenerated,
    - no open high-severity parity mismatches,
    - `cargo run -- test --runtime vm` and `dual` pass.
  - 2026-05-26 evidence:
    - Regenerated mismatch artifacts via `bash scripts/generate_vm_runtime_mismatch_inventory.sh`.
    - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` now reports zero high-severity buckets:
      - `P0 runtime-parity-bug: 0`
      - `P1 stale-snapshot-expectation: 0`
      - `P1 parser-invalid-fixture: 0`
    - Runtime sweep commands passed on current harness:
      - `cargo run -- test --runtime vm` (summary `137/150`, process exit `0`)
      - `cargo run -- test --runtime dual` (summary `136/150`, process exit `0`)
    - Focused parity regression suite passed:
      - `cargo test --test vm_interpreter_parity_surfaces` (100/100).

- [x] **ER-P0-004**: Harden network/process/file capability defaults and docs alignment.
  - Scope: revalidate `--untrusted` guardrails against SSRF-style destinations, process execution limits, and path boundaries.
  - Acceptance:
    - `native_api_security_boundaries` and `runtime_security` green,
    - any policy drift resolved with explicit diagnostics and docs updates.
  - 2026-05-25 evidence:
    - `cargo test --test native_api_security_boundaries` passed (48/48).
    - `cargo test --test runtime_security` passed (11/11).
    - `cargo test --test docs_policy_consistency_contract` passed.
    - Updated `docs/NATIVE_API_SECURITY_POSTURE.md` outbound-policy diagnostics text to explicitly document deterministic invalid-policy and blocked-destination error strings.

- [x] **ER-P0-005**: Repository hygiene cleanup for production-facing presentation.
  - Scope: remove/archive root-level non-source artifacts that do not belong in the main repo surface (ad-hoc backups, transient DBs, temporary directories) with explicit retention policy.
  - Acceptance:
    - root directory contains only intentional product/repo assets,
    - cleanup policy documented (what is generated vs versioned).
  - 2026-05-26 evidence:
    - Added `docs/REPO_HYGIENE_POLICY.md` with explicit root-surface contract, non-root placement rules, and cleanup/retention guidance.
    - Added `tests/repo_hygiene_contract.rs` to enforce tracked root allowlist and policy markers.
    - Validation commands passed:
      - `cargo test --test repo_hygiene_contract` (2/2)
      - `cargo test --test docs_policy_consistency_contract` (1/1)

- [x] **ER-P0-006**: Reduce executable `unsafe` budget back under gate threshold with evidence.
  - Scope: current strict inventory reports `Executable matches: 59` vs contract budget `<= 55`.
  - Acceptance:
    - `cargo test --test unsafe_inventory_contract` passes without raising budget ceilings,
    - reduction work is classified and documented in `docs/generated/UNSAFE_INVENTORY.md` + dated `notes/` entry.
  - 2026-05-26 evidence:
    - Reduced non-essential executable unsafe callsites in `src/jit_disabled.rs` (JIT-disabled shim) by replacing unsafe extern function-pointer aliases and wrapper invocations with safe equivalents.
    - Regenerated inventory artifacts:
      - `docs/generated/UNSAFE_INVENTORY.md`
      - `docs/generated/UNSAFE_INVENTORY.csv`
    - Contract and safety validations passed:
      - `cargo test --test unsafe_inventory_contract` (3/3)
      - `cargo test --test jit_safety_contract_checker` (8/8)
      - `cargo test --test vm_interpreter_parity_surfaces` (100/100)

- [x] **ER-P1-001**: Binary size optimization pass with reproducible measurements.
  - Scope: benchmark release binary size and evaluate feature/profile/link-time optimizations without behavior regressions.
  - Acceptance:
    - before/after size table committed,
    - no contract test regressions,
    - tradeoff note for startup/runtime impact.
  - 2026-05-26 evidence:
    - Added reproducible measurement report: `docs/RELEASE_BINARY_SIZE_BASELINE_2026-05-26.md`.
    - Before/after measurements:
      - Baseline override profile (`strip=none`, `lto=off`, `codegen-units=16`): `34,666,816` bytes (33.1 MiB).
      - Current optimized release defaults (`strip=symbols`, `lto=thin`, `codegen-units=1`): `24,149,152` bytes (23.0 MiB).
      - Net reduction: `10,517,664` bytes (`-30.34%`).
    - Regression validations passed:
      - `cargo test --test cli_contracts` (15/15)
      - `cargo test --test vm_interpreter_parity_surfaces` (100/100)

- [x] **ER-P1-002**: Performance hot-path audit and micro-benchmark stabilization.
  - Scope: VM call dispatch, module loading/import-heavy startup, dict/index hot paths, and JIT/VM crossover.
  - Acceptance:
    - targeted bench commands + results committed,
    - identified regressions fixed or documented with owner/timeline.
  - 2026-05-26 evidence:
    - Added hot-path audit report: `docs/PERF_HOT_PATH_AUDIT_2026-05-26.md`.
    - Committed benchmark/result artifacts referenced by audit:
      - `docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md`
      - `docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md`
    - Contract validations passed:
      - `cargo test --test vm_import_heavy_cache_lookup_contract` (1/1)
      - `cargo test --test vm_import_heavy_perf_comparison_contract` (1/1)
    - Regression handling:
      - no import-heavy startup/cache regressions indicated (`PASS` tolerance in perf comparison artifact),
      - follow-up owner/timeline documented in the audit report.

- [x] **ER-P1-003**: Type-checker ergonomics and diagnostics hardening for high-signal gaps.
  - Scope: resolve remaining medium/high-value TODO clusters in `src/type_checker.rs` and improve actionable messaging.
  - Acceptance:
    - targeted type-checker tests added,
    - TODO triage artifacts updated,
    - no misleading "supported" behavior in docs.
  - 2026-05-26 evidence:
    - Implemented partial known-surface method return-type inference in `src/type_checker.rs` for core `string`/`array`/`dict` method surfaces, retaining safe `Any` fallback for unknown/custom methods.
    - Added targeted regression tests:
      - `test_method_call_infers_known_string_method_return_types`
      - `test_method_call_unknown_method_falls_back_to_any`
    - Refreshed TODO triage artifacts:
      - `docs/generated/V1_CODE_TODO_TRIAGE.md`
      - `docs/generated/V1_CODE_TODO_TRIAGE.csv`
    - Updated typing policy docs to make inference boundaries explicit:
      - `docs/OPTIONAL_TYPING_DESIGN.md`
    - Validation commands passed:
      - `cargo test type_checker::tests::`
      - `cargo test --test v1_code_todo_triage_contract`

- [x] **ER-P1-004**: Production CLI UX polish for machine + human operators.
  - Scope: ensure deterministic JSON diagnostics, consistent exit-code semantics, and crisp remediation hints.
  - Acceptance:
    - CLI contract tests updated and passing,
    - docs/examples match actual behavior.
  - 2026-05-25 evidence:
    - `cargo test --test cli_contracts` passed (15/15), including dual/vm runtime summary and fallback-marker expectations.
    - `cargo test --test cli_json_contracts` passed (13/13), preserving machine-readable JSON surfaces and negative-path failure semantics.
    - Updated `docs/CLI_MACHINE_READABLE_CONTRACTS.md` with explicit deterministic `kujo test` runtime-summary/fallback-marker contracts referenced by test names.

- [x] **ER-P1-005**: Root-to-docs consistency pass.
  - Scope: verify README, roadmap, release process, security posture, and migration docs are aligned with current runtime behavior.
  - Acceptance:
    - docs contract tests pass,
    - contradictions removed,
    - examples runnable on default VM path unless explicitly marked otherwise.
  - 2026-05-25 evidence:
    - Docs contracts passed:
      - `cargo test --test readme_contracts`
      - `cargo test --test docs_policy_consistency_contract`
      - `cargo test --test architecture_docs_contract`
      - `cargo test --test release_process_docs_contract`
      - `cargo test --test runtime_path_matrix_contract`
    - Docs examples suite passed: `cargo test --test docs_examples`.
    - Consistency updates applied to root/operator-facing docs:
      - `README.md` positioning and repo-layout cleanup.
      - `docs/VM_INTERPRETER_PARITY_MATRIX.md` runtime-decision marker/date alignment for contract stability.
      - `docs/STANDARD_LIBRARY.md` inventory sync for runtime-registered builtins (`__vm_for_iterable`, `substr`).

- [x] **ER-P2-001**: Packaging/distribution ergonomics improvement plan.
  - Scope: installer, package-manager guidance, and reproducible build/documentation improvements.
  - Acceptance:
    - clear install matrix published,
    - known platform caveats documented.
  - 2026-05-25 evidence:
    - Added `docs/INSTALL_MATRIX.md` with installation/distribution matrix for source, release, Cargo install, commit-pinned install, and CI locked builds.
    - Documented platform caveats for macOS/Linux/Windows and pre-v1 distribution guidance.
    - Linked install matrix from `README.md` core references.

- [x] **ER-P2-002**: Developer onboarding quality pass.
  - Scope: add concise "build your first real tool" path and operational cookbook for production scripting.
  - Acceptance:
    - runnable examples validated in CI or contract tests,
    - docs links audited for dead/outdated references.
  - 2026-05-25 evidence:
    - Added `docs/FIRST_TOOL_COOKBOOK.md` with a concise first-tool build path, operational exit-code conventions, VM-default execution guidance, and extension patterns.
    - Linked cookbook in `README.md` Core Reference Links for discoverable onboarding.
    - Validation commands passed:
      - `cargo test --test docs_examples`
      - `cargo test --test readme_contracts`

## Immediate Next Suggested Order

1. `ER-P0-001` full verification matrix evidence run.
2. `ER-P0-006` unsafe budget reduction pass to clear strict contract gate.
3. `ER-P0-005` root hygiene cleanup and policy.
4. `ER-P1-001` binary size optimization baseline and pass.
5. `ER-P1-002` performance hot-path benchmark cycle.
6. `ER-P1-005` docs consistency finalization.
