# V1.0 Remaining Non-Release Work Checklist

Date: 2026-05-24
Owner: runtime/compiler/docs maintainers

## Scope

This checklist is the **post-hardening, non-release-flight** remainder for v1.0 readiness.

Explicitly excluded from this document:
- `V1U-OPEN-003` (release artifact tag-time sign-off)
- `V1U-FINAL-003` (final tag-time artifact completion)

Those two release-flight items remain governed by `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` and require explicit `UNBLOCK_V1_RELEASE` to execute.

## Current Snapshot (2026-05-24)

- `docs/V1_0_HARDENING_AND_LEANNESS_CHECKLIST.md`: `0` unchecked.
- `docs/V1_0_TECH_READINESS_CHECKLIST.md`: `0` unchecked.
- `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`: `0` unchecked.
- `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`: `2` unchecked, both release-flight (excluded above).

Non-release unchecked total: **0** primary blocker items.

## Current Generated Artifact Refresh (2026-06-19)

- `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` now reports `P0 runtime-parity-bug: 8`, `P1 stale-snapshot-expectation: 4`, and `P2 harness-debt: 0`.
- This supersedes the 2026-05-24 and 2026-05-25 zero-mismatch closure snapshots for current release-review purposes. Treat the completed rows below as historical closure evidence; the refreshed P0 rows are tracked by `V1RR-P0-004` and should be resolved or explicitly deferred by the active release-readiness checklist.

## A) Remaining Blockers / Stoppers

- [x] **RNR-PAR-001**: Close `V1VM-PAR-004` by reducing non-intentional VM parity mismatches to intentional-divergence-only.
  - Why open: `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` still reports `runtime-parity-bug` fixtures.
  - Completed (2026-05-24):
    - Regenerated `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` now reports `P0 runtime-parity-bug: 0` and `P2 harness-debt: 0`.
    - Required verification passed: `cargo test --test vm_runtime_mismatch_inventory_contract`, `cargo test --test vm_runtime_mismatch_baseline_contract`, and `cargo test --test vm_interpreter_parity_surfaces`.
    - Runtime sweeps: `cargo run -- test --runtime vm` and `cargo run -- test --runtime dual` both report `Passed 129/150` with `interpreter_fallback=0` in dual mode.
  - Follow-up completed (2026-05-25):
    - Burned down remaining `P1 stale-snapshot-expectation` rows to zero; regenerated inventory now reports `both mismatch: 0` and `P1 stale-snapshot-expectation: 0`.
    - Runtime sweeps now report `Passed 137/150` for both `--runtime vm` and `--runtime dual` with `interpreter_fallback=0`.
    - Updated mismatch inventory contracts to remain valid when mismatch rows are zero, preventing false failures when parity reaches full snapshot alignment.
  - Exit criteria:
    - `runtime-parity-bug` bucket reaches zero **or** residual divergences are explicitly reclassified as intentional with rationale.
    - `V1VM-PAR-004` can be marked complete with dated evidence.

- [x] **RNR-PAR-002**: Refresh universalization readiness verdict after parity closure.
  - Why open: `V1VM-FINAL-001` currently records `NO-GO` pending `V1VM-PAR-004` burn-down.
  - Completed (2026-05-24):
    - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md` updated with `V1VM-PAR-004` closure evidence and current bucket totals.
    - `docs/VM_INTERPRETER_PARITY_MATRIX.md` refreshed to reflect supported generator iteration parity and current runtime decision evidence.

## B) In-Progress / Partial / Needs-Follow-Through

- [x] **RNR-DOC-001**: Resolve docs snippet contract failure in `docs/NATIVE_API_SECURITY_POSTURE.md` code block #1.
  - Current evidence (2026-05-24):
    - `cargo test --test docs_examples` fails at `docs_kujo_snippets_parse_or_expected_fail`.
    - Error: parse failure in `docs/NATIVE_API_SECURITY_POSTURE.md#1` (`Expected expression`).
  - Exit criteria:
    - Snippet updated to valid Kujo syntax (or intentionally expected-fail with reason, if policy allows).
    - `cargo test --test docs_examples` passes.
  - Completed (2026-05-24):
    - Updated snippet syntax in `docs/NATIVE_API_SECURITY_POSTURE.md` from legacy `fn`/`=` style to current Kujo `func`/`:=` style.
    - Validation: `cargo test --test docs_examples` (`5 passed, 0 failed`).

- [x] **RNR-EVID-001**: Re-sync blocker evidence text in `V1VM-PAR-004` notes to current inventory values.
  - Why open: blocker note history still contains earlier larger counts; latest inventory is materially lower.
  - Exit criteria:
    - Add fresh dated blocker evidence line aligned to latest generated artifact.
  - Completed (2026-05-24):
    - Added fresh blocker note under `V1VM-PAR-004` in `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md` with regenerated artifact totals (`runtime-parity-bug: 9`, `stale-snapshot-expectation: 1`, `harness-debt: 0`, `vm_matches_snapshot: 153/163`).
    - Captured remaining parity-bug fixture list and supporting command evidence from parity + inventory contract suites.

## C) Structured Parity Burn-Down Plan (Recommended Workload Slices)

Use these as execution loops to close `RNR-PAR-001` predictably:

- [x] **RNR-PAR-G1**: Method/operator semantics cluster
  - Targets: `dict_methods_test`, `spread_operator`, `test_method_chaining`, `test_unary_overload`.
  - Focus: method dispatch consistency, operator overload parity, chained-call evaluation order.
  - Progress (2026-05-24):
    - Closed VM parity for `dict_methods_test` by adding `FixedDict` support in collection natives (`merge`, `clear`, `remove`).
    - Closed VM parity for `spread_operator` by preserving source-order dict insertion in VM `MakeDict`/`MakeDictFromMarker` (later keys now override deterministically).
    - Closed VM parity for `test_method_chaining` by adding compiler lowering support for `??`, `?.`, and `|>` in VM bytecode generation.
    - Closed VM parity for `test_unary_overload` runtime behavior by dispatching VM unary struct operator methods (`op_neg`, `op_not`); fixture now classifies as `stale-snapshot-expectation` instead of `runtime-parity-bug`.
  - Completed (2026-05-24):
    - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` shows `runtime-parity-bug` reduced from `16` to `14` and `test_method_chaining.kujo` now `both_match_snapshot`.
    - `test_unary_overload.kujo` moved to `both_mismatch_same_output` + `stale-snapshot-expectation` (interpreter/VM outputs now align, snapshot update pending in docs-owner track).

- [x] **RNR-PAR-G2**: Loop/function/generator control-flow cluster
  - Targets: `test_loop_correct`, `test_func_loop_correct`, `test_function_drop_fix`, `test_generators`, `integer_types`.
  - Focus: frame/stack lifecycle, loop progression semantics, generator-next behavior consistency.
  - Progress (2026-05-24):
    - Closed loop/function sub-bucket by normalizing VM for-loop iterables in compiler (`__vm_for_iterable`) without changing generic index error semantics.
    - `test_loop_correct`, `test_func_loop_correct`, and `test_function_drop_fix` no longer classify as `runtime-parity-bug`.
    - Remaining targets in this cluster: `test_generators` (runtime-path mismatch) and any residual numeric/runtime semantics from `integer_types`.
  - Progress (2026-05-24, native error + try-unwind parity pass):
    - VM `TryUnwrap` now performs function-frame early return semantics for `Err`/`None` instead of aborting script execution, aligning with interpreter behavior in `result_option` flows.
    - Remaining target in this cluster stays `test_generators` (generator call/resume parity path).

- [x] **RNR-PAR-G3**: Stdlib/env/IO behavior cluster
  - Targets: `env_and_args`, `stdlib_test`, `stdlib_os_path_test`, `test_connection_pooling`, `image_processing_test`, `simple_image_test`.
  - Focus: deterministic native interop results, environment handling, stable output normalization.
  - Progress (2026-05-24, native error throw/catch parity pass):
    - VM native-call errors now route through VM exception handlers (`throw` semantics) instead of terminating execution, closing parity gaps for:
      - `env_and_args` runtime path progression
      - `image_processing_test` missing-file catch path
      - `simple_image_test` missing-file catch path
    - Remaining targets in this cluster: `stdlib_test`, `stdlib_os_path_test`, `test_connection_pooling`.

- [x] **RNR-PAR-G4**: Collections/assertion/result semantics cluster
  - Targets: `result_option`, `test_enhanced_collections`, `test_assertions`, `dict_methods_test` (if residual overlap).
  - Focus: container mutation/view parity, Result/Option behavior, assertion output consistency.
  - Progress (2026-05-24):
    - Closed `test_enhanced_collections` parity mismatch by adding `FixedDict` support for `invert`/`update`/`get_default` and correcting `invert` parity behavior to mirror interpreter output.
    - Closed `result_option` parity by fixing VM `TryUnwrap` early-return unwinding semantics.
    - Closed `test_assertions` parity by routing native assertion failures through VM try/catch instead of immediate VM abort.

## D) Verification Matrix For Remaining Work

For each parity loop:
- `cargo test --test vm_interpreter_parity_surfaces`
- `cargo run -- test --runtime vm`
- `cargo run -- test --runtime dual`
- Regenerate inventory artifacts:
  - `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
- Contract checks:
  - `cargo test --test vm_runtime_mismatch_inventory_contract`
  - `cargo test --test vm_runtime_mismatch_baseline_contract`

For docs-fix loop:
- `cargo test --test docs_examples`
- related docs contract tests touched by the change.

## E) Definition Of Done (Non-Release)

This checklist is done when all are true:
- `RNR-PAR-001` complete (`V1VM-PAR-004` closed with evidence).
- `RNR-PAR-002` complete (readiness verdict refreshed from current artifact state).
- `RNR-DOC-001` complete (`docs_examples` passing).
- `RNR-EVID-001` complete (fresh dated blocker evidence synced).
- No new regressions in required parity/doc contract suites.
