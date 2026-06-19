# VM/Interpreter/Compiler Parity Matrix (v1.0.0)

Last updated: 2026-05-24

This matrix tracks parity for roadmap item `V1-COMP-001`.

## Status Labels

- `supported`: Implemented and parity-covered by tests.
- `unsupported (explicit)`: Rejected intentionally with deterministic errors.
- `not yet parity-covered`: Implemented surface exists, but dedicated parity evidence is still pending.
- `intentionally divergent`: Behavior differs by design and is documented.

## Matrix

| Surface | Compiler | Interpreter | VM | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| Variable/identifier resolution (`let`/`mut`/`const`, undefined identifiers) | lowers locals/globals with mutability metadata | lexical scopes + undefined-variable runtime errors | matching load/store + undefined-variable runtime errors | supported | `vm_and_interpreter_resolve_defined_identifiers`, `vm_and_interpreter_error_on_undefined_top_level_identifier`, `vm_and_interpreter_error_on_undefined_identifier_inside_function`, `vm_and_interpreter_error_on_undefined_identifier_inside_closure` |
| Function/closure/method/async/generator arity | emits callable metadata used by runtime arity checks | shared arity validation | matching callable arity checks | supported | `vm_and_interpreter_error_on_function_arity_too_few`, `vm_and_interpreter_error_on_function_arity_too_many`, `vm_and_interpreter_error_on_closure_arity_mismatch`, `vm_and_interpreter_error_on_method_arity_mismatch`, `vm_and_interpreter_error_on_async_function_arity_mismatch`, `vm_and_interpreter_error_on_generator_arity_mismatch`, `vm_and_interpreter_match_callable_arity_success_paths` |
| Top-level generator iteration (`func*`, `yield`, `for ... in generator`) | lowers generator declarations and generator call sites | generator creation + iteration in interpreter runtime | matching generator creation/iteration behavior for parity-covered surfaces | supported | `vm_and_interpreter_match_generator_iteration_surface`, `vm_and_interpreter_error_on_generator_arity_mismatch`, `vm_and_interpreter_error_on_generator_arity_too_many` |
| Struct methods (`obj.method(...)`) | lowers `MethodCall` to field-get + call | explicit `self` method dispatch | bytecode method dispatch | supported | `vm_and_interpreter_match_struct_method_behavior_contract` |
| Struct generator methods (`func*` inside `struct`) | compile-time rejection with shared message helper | runtime rejection with same shared message helper | compile path returns same message | unsupported (explicit) | `vm_and_interpreter_error_on_unsupported_struct_generator_method` |
| Collections/indexing/mutation | lowers array/dict/index ops and in-place updates | runtime checked index/map semantics | matching checked index/map semantics | supported | `vm_and_interpreter_match_valid_index_assignment_success_path`, `vm_and_interpreter_error_on_invalid_index_assignment_target`, `vm_and_interpreter_error_on_out_of_bounds_array_index`, `vm_and_interpreter_error_on_missing_string_map_key`, `vm_and_interpreter_match_successful_local_map_update` |
| Spread literals + destructuring bindings | emits marker-based spread/dict construction | spread + destructuring execution | matching marker-based spread/dict execution | supported | `vm_and_interpreter_match_spread_destructuring_surface` |
| Match/tag bindings (`Result::`/`Option::`) | lowers tag pattern checks | tag-style binding support | matching tag pattern checks | supported | `vm_and_interpreter_match_enum_match_binding_surface` |
| Imports (`import`, `from ... import ...`) | emits VM import native opcodes (`__vm_import_all`, `__vm_import_symbol`) | module-loader-backed import resolution | VM import handlers use module loader and bind into active scope | supported | `vm_and_interpreter_match_import_export_surface`, `vm_and_interpreter_match_dotted_from_import_surface` |
| Control flow (`if`/`while`/`loop`/`break`/`continue`/top-level `return`) | control-flow opcodes with validation | matching runtime semantics | matching runtime semantics | supported | `vm_and_interpreter_allow_break_and_continue_inside_loop`, `vm_and_interpreter_error_on_break_outside_loop`, `vm_and_interpreter_allow_top_level_return_for_script_exit` |
| Truthiness + short-circuit boolean logic | short-circuit lowering | shared truthiness/short-circuit semantics | matching truthiness/jump semantics | supported | `vm_and_interpreter_match_truthiness_semantics_across_conditionals`, `vm_and_interpreter_short_circuit_logical_operators_skip_rhs_when_possible`, `vm_and_interpreter_short_circuit_logical_operators_evaluate_rhs_when_required` |
| Equality/comparison + numeric safety | equality/comparison opcodes and checked arithmetic | centralized equality/comparison helpers + overflow/zero checks | same helper-backed comparison + checked arithmetic | supported | `vm_and_interpreter_define_cross_type_numeric_and_string_ordering_contract`, `vm_and_interpreter_define_collection_and_callable_equality_contract`, `vm_and_interpreter_reject_integer_add_overflow`, `vm_and_interpreter_reject_float_division_by_zero` |
| Native function parity (VM-allowed natives) | native call opcodes | interpreter native dispatch | VM native dispatch + shared native impl | supported | `vm_and_interpreter_error_on_native_function_arity_mismatch`, `vm_and_interpreter_preserve_variadic_native_contracts` |
| Spawn surface (`spawn { ... }`) | lowered closure-based spawn path | background-thread spawn support | matching tested spawn scenario | supported | `vm_and_interpreter_match_spawn_surface` |

## Command-Level Runtime Path Matrix

| Command/Test Surface | Runtime Path | Alternate Path(s) | Why This Path Exists | Evidence |
| --- | --- | --- | --- | --- |
| `kujo run <file>` | VM (default) | `kujo run --interpreter` | Production/default execution path is VM-first. | `README.md` CLI table, `tests/cli_contracts.rs` |
| `kujo run --interpreter <file>` | Interpreter | VM via default `kujo run` | Explicit fallback/debug path for tree-walk runtime behavior. | `README.md` CLI table |
| `kujo init` / `kujo package-add` / `kujo package-install` / `kujo package-install --frozen` | manifest/lockfile tooling | none (tooling-only) | Deterministic package bootstrap, dependency editing, and lockfile verification. | `tests/package_module_workflow_integration.rs`, `docs/RELEASE_PROCESS.md` |
| `kujo test --runtime dual` | VM-primary with bounded interpreter fallback | `--runtime vm`, `--runtime interpreter` | Legacy fixture snapshots still need deterministic fallback while VM-first coverage expands. | `src/parser.rs::run_all_tests`, `tests/cli_contracts.rs` |
| `kujo test --runtime vm` | VM-only | `dual`, `interpreter` | Explicit strict mode for parity-safe fixture sweeps and drift discovery. | `tests/cli_contracts.rs` (`cli_test_runtime_vm_mode_reports_mismatch_for_vm_drift_fixture`) |
| `kujo test --runtime interpreter` | Interpreter-only | `dual`, `vm` | Explicit compatibility mode for legacy fixture baselines. | `src/main.rs` CLI arg wiring, `src/parser.rs::run_all_tests` |
| `kujo test-run <file>` | Interpreter-hosted test framework execution | none (today) | Framework execution still uses interpreter `TestRunner` surfaces. | `src/main.rs` `Commands::TestRun`, `tests/generators_test.kujo`, `tests/iterators_test.kujo` |
| `cargo test --test native_api_security_boundaries` | Interpreter-focused command execution (`run --interpreter`) | none (today) | Security boundary regressions intentionally pin interpreter host-effect pathways. | `tests/native_api_security_boundaries.rs` |
| `cargo test --test runtime_security` | Interpreter-focused command execution (`run --interpreter`) | none (today) | Runtime security regressions currently target interpreter threat-model enforcement paths. | `tests/runtime_security.rs` |
| `cargo test --test diagnostics_golden` | Interpreter diagnostics command coverage (`run --interpreter`) | parser/lexer diagnostics independent of runtime mode | Golden snapshots lock deterministic diagnostics shape for existing interpreter-bound fixtures. | `tests/diagnostics_golden.rs` |
| `kujo lsp-diagnostics <file>` | Parse/diagnostic pipeline (runtime-agnostic) | n/a | Uses lexer/parser diagnostics without executing VM/interpreter runtime. | `tests/cli_contracts.rs` (`cli_lsp_diagnostics_json_is_valid_json`) |
| `kujo check <file>` | Parse/compile validation (runtime-agnostic) | n/a | Validates source without runtime execution side effects. | `tests/cli_contracts.rs` (`cli_check_does_not_execute_script_side_effects`) |

### `kujo test` Default Runtime Decision (2026-05-21)

Updated evidence snapshot: 2026-06-19

- Decision: keep default `kujo test` runtime at `dual` for now.
- Evidence:
  - `cargo run -- test --runtime vm` and `cargo run -- test --runtime dual` passed in the 2026-06-19 `V1RR-P0-006` evidence bundle (`notes/release_evidence/2026-06-19_p0-006/status.tsv`).
  - Both modes report `Passed 144/144 tests` plus `Fixture outcomes: passed=144, failed=0, skipped=6, expected_fail=0, runnable=144, discovered=150`.
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` currently reports `P0 runtime-parity-bug: 6`, `P1 stale-snapshot-expectation: 5`, and `P2 harness-debt: 0`.
- Risk analysis:
  - The generated inventory no longer supports an intentional-divergence-only parity claim; the P0 runtime rows need burn-down or explicit reclassification before final release sign-off.
  - Keeping `dual` continues deterministic compatibility signaling while the release-readiness checklist continues mismatch ownership burn-down.
- VM-only workflows remain explicitly available through `--runtime vm` for strict validation and migration gating.

## VM-First Practical Recommendations

- Prefer `kujo run <file>` (VM default) for day-to-day script execution and modular project usage.
- Prefer `kujo package-install --frozen` when you need to verify manifests and lockfiles without rewriting them.
- Prefer `kujo test --runtime dual` for fixture-compatibility regression checks while parity burn-down continues.
- Prefer `kujo test --runtime vm` for strict VM-only gating in migration and parity workflows.
- Treat `--interpreter` as an explicit compatibility/debug tool, not a default requirement for ordinary module-import workflows.

## CI Gate

- CI now includes a dedicated parity job that runs:
  - `cargo test --test vm_interpreter_parity_surfaces`
- The full release gate still requires `cargo test`, but the dedicated parity job fails fast on runtime-path drift.

## Notes

- This matrix tracks interpreter/VM/compiler parity only. JIT remains an experimental, opt-in surface (`kujo run --jit`) with explicit unsupported-opcode detection and deterministic fallback messaging; see `V1-JIT-001` notes in `ROADMAP.md`.
- Any newly added language/runtime surface must update this matrix and add parity evidence in the same change.
