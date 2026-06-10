# Interpreter Flag Dependency Map

- Generated: 2026-05-22 10:14:32 EDT
- Command: `rg -n -- "--interpreter" src tests docs README.md ROADMAP.md examples notes .github`

Reason tags:
- `harness-legacy`: Existing harness behavior still forces interpreter mode.
- `parity-gap`: Runtime path currently depends on an explicitly tracked interpreter/VM parity or output-contract gap.
- `security-test-choice`: Security-boundary regression intentionally exercises interpreter path.
- `diagnostics-diff`: Diagnostic contract coverage currently pins interpreter output shape.
- `docs-smoke`: Docs/example smoke harness runs interpreter as canonical execution path.
- `package-workflow`: Package/module workflow integration still validated via interpreter runs.
- `docs-contract`: User-facing docs explicitly describe interpreter mode behavior.
- `benchmark-baseline`: Example/benchmark docs keep interpreter as baseline comparator.
- `archive-note`: Historical field notes mentioning interpreter usage.

| File | Category | Reason Tags | Usage Count | Line References |
| --- | --- | --- | --- | --- |
| `README.md` | documentation | `docs-contract` | 5 | 11,122,326,561,562 |
| `ROADMAP.md` | documentation | `docs-contract` | 1 | 1334 |
| `docs/ARCHITECTURE.md` | documentation | `docs-contract` | 2 | 29,42 |
| `docs/IMAGE_CONVERSION_AGENT_HANDOFF.md` | documentation | `docs-contract` | 1 | 52 |
| `docs/NATIVE_API_SECURITY_POSTURE.md` | documentation | `docs-contract` | 3 | 193,199,211 |
| `docs/OPTIONAL_TYPING_DESIGN.md` | documentation | `docs-contract` | 1 | 26 |
| `docs/PERFORMANCE.md` | documentation | `docs-contract` | 3 | 46,495,500 |
| `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` | documentation | `docs-contract` | 2 | 265,271 |
| `docs/KUJO_FEATURE_INVENTORY.md` | documentation | `docs-contract` | 2 | 31,37 |
| `docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md` | documentation | `docs-contract` | 4 | 1,16,20,21 |
| `docs/VM_INTERPRETER_PARITY_MATRIX.md` | documentation | `docs-contract` | 6 | 37,38,43,44,45,66 |
| `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md` | documentation | `docs-contract` | 9 | 3,6,149,164,169,359,361,374,379 |
| `examples/benchmarks/README_REAL_WORLD.md` | example-doc | `benchmark-baseline` | 1 | 150 |
| `notes/2026-01-27_20-54_phase5-tokio-async-runtime.md` | notes-history | `archive-note` | 2 | 78,83 |
| `notes/2026-04-29_17-02_image-method-dispatch-parity.md` | notes-history | `archive-note` | 1 | 63 |
| `notes/2026-05-06_10-09_cli-serve-command-holistic-preview.md` | notes-history | `archive-note` | 3 | 24,42,45 |
| `notes/2026-05-13_01-13_v1-mod-001-module-import-hardening.md` | notes-history | `archive-note` | 1 | 24 |
| `notes/2026-05-16_16-12_v1-perf-003-resource-exhaustion-safeguards.md` | notes-history | `archive-note` | 1 | 39 |
| `notes/2026-05-16_16-59_v1-test-003-runtime-native-security-regressions.md` | notes-history | `archive-note` | 1 | 18 |
| `notes/2026-05-16_17-10_v1-test-005-docs-examples-smoke-suite.md` | notes-history | `archive-note` | 1 | 20 |
| `notes/2026-05-20_11-44_pre-v1-master-unfinished-checklist-audit.md` | notes-history | `archive-note` | 1 | 31 |
| `notes/2026-05-20_16-10_v1u-run-002_kujo-test-interpreter-hardcoding-analysis.md` | notes-history | `archive-note` | 2 | 8,21 |
| `notes/2026-05-20_18-05_v1u-run-006-command-runtime-path-matrix.md` | notes-history | `archive-note` | 1 | 15 |
| `notes/2026-05-20_18-13_v1u-code-003-optional-typing-boundary.md` | notes-history | `archive-note` | 1 | 20 |
| `notes/2026-05-21_17-48_vm-universal-checklist-and-agent-prompt.md` | notes-history | `archive-note` | 2 | 6,17 |
| `notes/2026-05-21_18-34_v1vm-imp-003-dotted-import-boundary-security.md` | notes-history | `archive-note` | 1 | 13 |
| `notes/2026-05-21_18-40_v1vm-imp-005-module-import-guidance-alignment.md` | notes-history | `archive-note` | 1 | 11 |
| `notes/2026-05-21_19-28_v1vm-par-003-runtime-parity-bucket-reduction.md` | notes-history | `archive-note` | 2 | 37,38 |
| `notes/2026-05-22_09-52_v1vm-doc-001-vm-first-runtime-guidance-alignment.md` | notes-history | `archive-note` | 2 | 8,14 |
| `notes/README.md` | notes-history | `archive-note` | 1 | 9 |
| `notes/vm_performance.md` | notes-history | `archive-note` | 1 | 21 |
| `src/main.rs` | other | `manual-review` | 1 | 125 |
| `src/parser.rs` | cli-harness | `harness-legacy,parity-gap` | 1 | 2212 |
| `tests/diagnostics_golden.rs` | integration-test | `diagnostics-diff,harness-legacy` | 1 | 60 |
| `tests/docs_examples.rs` | integration-test | `docs-smoke,harness-legacy` | 1 | 256 |
| `tests/interpreter_flag_dependency_map_contract.rs` | integration-test | `harness-legacy` | 2 | 57,90 |
| `tests/native_api_security_boundaries.rs` | integration-test | `security-test-choice` | 34 | 134,211,307,327,371,398,407,421,450,459,468,477,500,544,573,582,591,609,643,682,700,736,776,807,842,882,918,954,987,996,1029,1038,1074,1083 |
| `tests/optional_typing_v1_contract.rs` | integration-test | `harness-legacy` | 1 | 112 |
| `tests/package_module_workflow_integration.rs` | integration-test | `harness-legacy,package-workflow` | 7 | 124,316,347,366,405,455,486 |
| `tests/readme_contracts.rs` | integration-test | `harness-legacy` | 1 | 27 |
| `tests/runtime_path_matrix_contract.rs` | integration-test | `harness-legacy` | 3 | 22,24,25 |
| `tests/runtime_security.rs` | integration-test | `security-test-choice` | 6 | 128,146,175,206,261,307 |

## V1U-RUN-005: Parity-Gap Coverage Status

- Current `parity-gap` tagged entries: 1
- Tagged surfaces:
- `src/parser.rs` (harness-legacy,parity-gap)
- Coverage expectation: each tagged surface must have parity tests or explicit documented divergence.
- Current closure evidence paths:
  - `tests/cli_contracts.rs` (bounded runtime fallback contracts)
  - `tests/vm_interpreter_parity_surfaces.rs` (generator divergence contract)
  - `README.md` and `docs/VM_INTERPRETER_PARITY_MATRIX.md` (canonical divergence docs)

## V1U-RUN-002: `kujo test` Runtime Strategy Status

Current state (`src/parser.rs::run_all_tests`): `kujo test` supports explicit runtime strategy selection via `--runtime dual|vm|interpreter` (default `dual`), with VM-primary execution and bounded interpreter fallback in dual mode.

Current rationale:

- Snapshot corpus compatibility still matters because many `tests/*.out` files were created under interpreter-first historical behavior.
- Runtime-path drift remains measurable for part of the legacy fixture corpus, but the harness is no longer blanket interpreter-pinned.
- Command-level runtime strategy behavior is tracked in `docs/VM_INTERPRETER_PARITY_MATRIX.md`.

Import-reliability clarification:

- Dotted and flat module imports are supported in both VM and interpreter runtime paths.
- `--interpreter` is not required for ordinary multi-module import layouts; it remains an explicit fallback/debug mode while fixture parity burn-down continues.

VM-first practical recommendations:

- Use `kujo run <file>` as the default VM-first path for ordinary modular projects.
- Use `kujo test --runtime dual` for compatibility sweeps where fallback visibility matters.
- Use `kujo test --runtime vm` for strict migration/parity gating.
- Use `--interpreter` only for explicit compatibility/debug isolation.
