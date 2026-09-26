# Kujo Architecture

Last updated: 2026-09-25
Current stable release: `v1.5.0`

This document describes the current Kujo architecture as implemented in this repository.
It is intentionally execution-path and release-readiness oriented.

## 1. System Overview

Kujo is a Rust-hosted language runtime with these primary layers:

1. Frontend pipeline: lexer + parser + AST + diagnostics.
2. Runtime execution engines:
   - VM (default `kujo run` path).
   - Tree-walking interpreter (explicit fallback path).
3. Native function surfaces (filesystem, process, network, HTTP, crypto, etc.) with capability policy controls.
4. Tooling commands (check/test/test-run/lsp/docgen/format/lint/package) with deterministic manifest/lockfile workflows.

## 2. Source-to-Execution Pipeline

```text
.kujo source
  -> lexer (src/lexer.rs)
  -> parser (src/parser.rs)
  -> AST (src/ast.rs)
  -> optional compile path (src/compiler.rs + src/bytecode.rs)
  -> VM execution (src/vm.rs)   [default for kujo run]
       or interpreter execution (src/interpreter/*) [run --interpreter]
```

Notes:

- `kujo check` and `kujo lsp-diagnostics` use parse/diagnostic flows and do not execute runtime side effects.
- Runtime-path command coverage is tracked in `docs/VM_INTERPRETER_PARITY_MATRIX.md` under `Command-Level Runtime Path Matrix`.
- Package bootstrap and lockfile verification are tracked as separate tooling contracts, but their nested import examples still resolve through the same package-root-aware module loader used by `kujo run`.

## 3. Runtime Path Model

### VM lexical captures

Function compilers inherit the bindings visible at their definition site.
Capture discovery happens during lexical resolution, with locals taking
precedence over inherited bindings. A nested reference can cause an intermediate
function to forward an otherwise unused capture. Only referenced bindings are
retained; names are not resolved by searching a completed function's slot list.

`BytecodeChunk.capture_sources` describes each snapshot source: a parent local
slot, a parent capture index, a scoped script binding, or a runtime-created named
binding (including bare assignment, destructuring and legacy receiver fields).
Function-local selected imports register named bindings. Import-all exports
are discovered at runtime, so descendant references lazily request named
captures through the intervening compiler boundaries; absent names keep global
fallback. This does not retain the whole import environment.
Ordinary global reads remain global lookups. `LoadCapture` and `StoreCapture` address frame-local
capture indices. Runtime-created bindings retain the v1 rule that a bare
assignment may target an existing global instead of creating a local.

Each closure creation copies values into its own heap cells, preserving v1
snapshot semantics. Calls and aliases share those cells. Frames lazily build an
indexed view of the same cells used by the interpreter callback bridge; ordinary
uncaptured locals remain plain slots. No open stack pointers need closing:
return, scope exit and unwind release frame references while escaped closures
own their snapshots. Nested named recursion uses a call-local self binding,
avoiding a self-reference inside the capture environment.

Captured functions bypass JIT function caches until the JIT supports this
representation. The main VM, JIT-to-VM fallback and bounded generator dispatcher
handle indexed capture access. Generator scheduling and spawn behavior are not
redesigned. Legacy manually constructed chunks without capture descriptors
retain their compatibility path; the older VM-wide `Upvalue` opcodes are not the
compiler-produced closure mechanism. Bytecode is not a persistent public format.

See [the capture audit](CLOSURE_UPVALUE_AUDIT.md) for baseline evidence and known
interpreter limitations. This mechanism does not add a tracing collector for
arbitrary user-created cyclic value graphs.

### 3.1 `kujo run`

- Default: VM execution.
- Alternate: `kujo run --interpreter` for explicit interpreter fallback.

### 3.2 `kujo test`

- Supports `--runtime dual|vm|interpreter`.
- Default is `dual`: VM-primary with bounded interpreter fallback when VM output drifts from fixture snapshot expectations.

### 3.3 `kujo test-run`

- Uses the interpreter-hosted test framework path (`TestRunner`).

### 3.4 Security/diagnostics suites

- Several security and diagnostics integration suites intentionally exercise interpreter command paths to preserve deterministic boundary coverage.

### 3.5 Package workflow and lockfiles

- `kujo init` seeds a package manifest and source layout for new projects.
- `kujo package-add` edits dependency declarations in `kujo.toml`.
- `kujo package-install` regenerates `kujo.lock` deterministically from the manifest.
- `kujo package-install --frozen` verifies that `kujo.lock` is current without rewriting it.
- Nested source layouts under the project root resolve the same way on VM and interpreter paths, so ordinary package projects do not need `--interpreter` just to import `src/...` modules.
- When execution starts inside a Kennel project, the module loader reads the
  nearest `kennel.lock` and adds only its existing, canonical package install
  roots. This makes locked dependencies importable without environment setup
  while keeping unrelated or stale package directories out of resolution.
- `KUJO_MODULE_PATH` remains an explicit extension point and is resolved before
  lockfile-discovered package roots.

## 4. Core Components

### 4.1 Frontend and diagnostics

- `src/lexer.rs`: tokenization and lexical diagnostics.
- `src/parser.rs`: AST construction, parser diagnostics, and fixture test harness wiring for `kujo test`.
- `src/errors.rs`: shared diagnostic model.

### 4.2 Interpreter subsystem

- `src/interpreter/mod.rs`: interpreter runtime orchestration and native dispatch integration.
- `src/interpreter/value.rs`: runtime value model.
- `src/interpreter/environment.rs`: lexical scope environment model.
- `src/interpreter/native_functions/*`: native API implementations.

### 4.3 Compiler/VM subsystem

- `src/compiler.rs`: AST -> bytecode lowering.
- `src/bytecode.rs`: instruction definitions.
- `src/vm.rs`: bytecode execution runtime.

### 4.4 Tooling and service surfaces

- `src/main.rs`: CLI command parsing + dispatch.
- `src/lsp_*`: LSP command/service surfaces.
- `src/serve_http.rs`: static server path.
- `src/docgen/*`: universal doc generation pipeline.

## 5. Capability and Security Boundaries

Kujo is not a sandbox.

- Trusted/default runtime paths can access host-effect APIs.
- Untrusted execution should use `--untrusted` plus explicit `--allow-*` flags.
- Canonical policy details live in `docs/NATIVE_API_SECURITY_POSTURE.md`.

## 6. Known Runtime Divergences

Runtime parity is tracked centrally in `docs/VM_INTERPRETER_PARITY_MATRIX.md`.

Current explicit divergence examples include:

- Top-level generator creation and straight-line iteration work in covered cases.
  Full continuation remains incomplete: the VM resume dispatcher omits ordinary
  calls/handlers and truncates loops at their first yield; eager iteration also
  limits laziness. Interpreter continuation and alias identity have separate gaps.
- VM `spawn` currently discards its compiled body; interpreter spawn and native
  task callables have different limitations. The nonnegative-counter parity probe
  does not prove execution. See the [Phase-A characterization](GENERATOR_ASYNC_SPAWN_COMPLETION_PLAN.md)
  and [Phase-B handoff](GENERATOR_ASYNC_SPAWN_PHASE_B_HANDOFF.md).
- Struct generator methods remain explicitly unsupported.

## 7. Release Posture

Kujo `v1.5.0` is the current stable release.

- `ROADMAP.md` tracks historical v1 work and post-1.0 planning.
- `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` preserves historical pre-launch closure evidence.
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` records published artifact verification.

## 8. Related Docs

- `README.md`
- `ROADMAP.md`
- `docs/VM_INTERPRETER_PARITY_MATRIX.md`
- `docs/LANGUAGE_SPEC.md`
- `docs/STANDARD_LIBRARY.md`
- `docs/NATIVE_API_SECURITY_POSTURE.md`
- `docs/RELEASE_PROCESS.md`
