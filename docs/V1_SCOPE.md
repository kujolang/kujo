# Kujo v1.0.0 Scope Definition

Status: stable v1.0.0 scope baseline

This document defines what is in-scope for Kujo `v1.0.0`, what is explicitly out-of-scope, and what compatibility commitments apply.

Release boundary: Kujo `v1.2.3` is the current stable release; explicit deferrals and compatibility guarantees are governed by `docs/V1_SCOPE.md`.

## In-Scope For v1.0.0

- Stable core language syntax carried forward from the historical `v0.13.0`/`v0.14.0` stabilization baseline docs into the staged `1.0.0` crate metadata.
- Stable runtime behavior for currently documented core execution paths:
  - CLI script execution (`kujo run` VM default + interpreter fallback)
  - core control flow, function, collection, and error-flow semantics covered by tests
  - optional typing boundary remains explicit: interpreter mode may emit non-fatal type-check warnings, while VM/default mode keeps dynamic execution without a static type gate
- Stable machine-readable tooling surfaces for:
  - CLI JSON contracts documented in `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - LSP protocol contracts documented in `docs/PROTOCOL_CONTRACTS.md`
- Release process reproducibility and artifact/install validation flow documented under:
  - `docs/RELEASE_PROCESS.md`
  - `docs/RELEASE_ARTIFACT_VALIDATION.md`
- Editor adapter baseline policy and first-party extension baseline wiring (`kujo lsp`) documented and tested.

## Out-Of-Scope For v1.0.0

- New major language features that alter parser/runtime compatibility guarantees.
- Experimental runtime expansion that lacks stable CLI/LSP contract coverage.
- Editor-specific feature forks that duplicate Kujo parser/analyzer behavior.
- Platform/package-manager distribution channels not yet covered by release artifact validation evidence.

## Compatibility Commitments (v1.0.0)

Language/runtime commitments:

- Backward-compatible behavior for documented syntax/runtime contracts unless a major-version policy change is declared.
- No silent behavior drift for covered core language/runtime tests.
- Any intentional breaking language/runtime change must be release-noted and version-gated.

Machine-readable tooling commitments:

- CLI/LSP contract field removal, rename, or type changes are considered breaking.
- Additive optional fields are non-breaking when existing fields remain stable.
- Golden fixture and contract test updates must accompany any intentional contract change.

Release/process commitments:

- Version-state consistency between `Cargo.toml`, `README.md`, and `ROADMAP.md` remains CI-enforced.
- Artifact validation and checksum workflows remain part of release-gate evidence.

## Deferred Runtime Execution Backlog (Explicit v1 Deferrals)

The following runtime-path implementation backlogs are explicitly deferred and non-silent for `v1.0.0` scope tracking:

- `src/vm.rs`:
  - `Upvalue` full closure-capture implementation remains deferred while current closure behavior stays contract-locked by parity suites.
  - `GeneratorState` full restoration model remains deferred while current generator boundaries stay explicitly documented in `docs/VM_INTERPRETER_PARITY_MATRIX.md`.
- `src/compiler.rs`:
  - Dedicated VM `SpawnThread` opcode is deferred; current spawn lowering behavior remains explicit in compiler comments and roadmap-driven follow-up planning.
  - Enum and interpolated-string builder opcode optimizations are deferred as post-v1 performance/representation work (non-contract semantics).
- `src/interpreter/native_functions/async_ops.rs`:
  - `spawn_task` body execution with full interpreter-context evaluation is deferred; current placeholder behavior remains explicit in code and triage artifacts.

Deferral guardrails:

1. Every deferred runtime item must stay listed in `docs/generated/V1_CODE_TODO_TRIAGE.md` with owner + bucket.
2. Deferred runtime behavior must remain explicit in code comments (no silent TODO markers on high-risk paths).
3. Any future implementation of these items must add/update targeted runtime/parity/security tests before checklist closure.

## Deferred Post-1.0 Candidates (Non-Blocking)

The following items are explicitly tracked as post-1.0 backlog and are not blockers for `v1.0.0`:

- Generics
- FFI (foreign function interface)
- WASM target
- Macro system
- Optional typing precision follow-ups:
  - destructuring inference
  - module existence checks
  - struct field type lookup
  - Promise unwrap typing
  - permissive callable fallback policy tightening

These candidates should be tracked as roadmap backlog slices after `v1.0.0` release stabilization.

## v1.0.0 Launch Record

The `v1.0.0` launch confirmed:

1. Earlier stabilization checklist evidence remains linked from the roadmap and completed release checklist.
2. Contract docs/tests and release process docs are in sync.
3. `CHANGELOG.md` distinguishes guaranteed `v1.0.0` surfaces from deferred backlog.
