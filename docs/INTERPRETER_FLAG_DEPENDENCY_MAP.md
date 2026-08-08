# Interpreter Flag Dependency Map

- Generated: 2026-08-07 19:50:35 EDT
- Command: `rg -n -- "--interpreter" src tests docs README.md ROADMAP.md examples notes .github`

Reason tags:
- `harness-legacy`: Existing harness behavior still forces interpreter mode.
- `parity-gap`: Runtime path currently depends on an explicitly tracked interpreter/VM parity or output-contract gap.
- `security-test-choice`: Security-boundary regression intentionally exercises interpreter path.
- `diagnostics-diff`: Diagnostic contract coverage currently pins interpreter output shape.
- `docs-smoke`: Docs/example smoke harness runs interpreter as canonical execution path.
- `package-workflow`: Package/module workflow integration still validated via interpreter runs.
- `docs-contract`: User-facing docs explicitly describe interpreter mode behavior.
- `example-runtime-choice`: Example documentation intentionally selects the interpreter path.
- `archive-note`: Historical field notes mentioning interpreter usage.

| File | Category | Reason Tags | Usage Count | Line References |
| --- | --- | --- | --- | --- |
| `README.md` | documentation | `docs-contract` | 4 | 283,289,290,303 |
| `ROADMAP.md` | documentation | `docs-contract` | 1 | 1350 |
| `docs/ARCHITECTURE.md` | documentation | `docs-contract` | 3 | 29,43,64 |
| `docs/FIRST_TOOL_COOKBOOK.md` | documentation | `docs-contract` | 1 | 135 |
| `docs/KUJO_TOOL_ARTIFACT_IGNORE_INVENTORY.md` | documentation | `docs-contract` | 2 | 48,76 |
| `docs/NATIVE_API_SECURITY_POSTURE.md` | documentation | `docs-contract` | 1 | 253 |
| `docs/OPTIONAL_TYPING_DESIGN.md` | documentation | `docs-contract` | 2 | 26,128 |
| `docs/PERFORMANCE.md` | documentation | `docs-contract` | 2 | 19,123 |
| `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` | documentation | `docs-contract` | 2 | 269,275 |
| `docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md` | documentation | `docs-contract` | 4 | 1,17,21,22 |
| `docs/VM_INTERPRETER_PARITY_MATRIX.md` | documentation | `docs-contract` | 6 | 37,38,44,45,46,70 |
| `src/main.rs` | other | `manual-review` | 1 | 146 |
| `src/parser.rs` | cli-harness | `harness-legacy,parity-gap` | 1 | 2241 |
| `tests/diagnostics_golden.rs` | integration-test | `diagnostics-diff,harness-legacy` | 2 | 132,139 |
| `tests/docs_examples.rs` | integration-test | `docs-smoke,harness-legacy` | 1 | 269 |
| `tests/docs_policy_consistency_contract.rs` | integration-test | `harness-legacy` | 1 | 41 |
| `tests/http_route_callback_closure.rs` | integration-test | `harness-legacy` | 1 | 55 |
| `tests/interpreter_flag_dependency_map_contract.rs` | integration-test | `harness-legacy` | 2 | 57,88 |
| `tests/native_api_security_boundaries.rs` | integration-test | `security-test-choice` | 48 | 134,211,323,343,387,414,423,437,466,475,484,506,515,529,563,595,618,662,697,733,769,805,841,886,913,922,931,940,949,967,1001,1040,1058,1094,1134,1165,1200,1240,1276,1312,1345,1354,1387,1396,1432,1441,1481,1490 |
| `tests/optional_typing_v1_contract.rs` | integration-test | `harness-legacy` | 2 | 111,175 |
| `tests/package_module_workflow_integration.rs` | integration-test | `harness-legacy,package-workflow` | 7 | 124,471,501,520,559,604,631 |
| `tests/readme_contracts.rs` | integration-test | `harness-legacy` | 1 | 43 |
| `tests/runtime_path_matrix_contract.rs` | integration-test | `harness-legacy` | 3 | 22,24,25 |
| `tests/runtime_security.rs` | integration-test | `security-test-choice` | 7 | 128,146,174,205,296,347,388 |
| `tests/vm_interpreter_migration_playbook_contract.rs` | integration-test | `harness-legacy` | 2 | 15,20 |

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
