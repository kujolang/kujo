# Kujo Architecture

Last updated: 2026-05-20  
Current crate version: `1.0.0` (pre-tag release-candidate metadata)

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

- Top-level generator iteration (`func*` + `yield`) is intentionally divergent:
  - interpreter path supports covered scenarios,
  - VM currently returns deterministic error `Yield can only be used inside generator functions`.
- Struct generator methods remain explicitly unsupported.

## 7. Release Posture

Kujo is in pre-tag `1.0.0` release-candidate readiness.

- `ROADMAP.md` is the release-readiness source of truth.
- `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` tracks unresolved pre-tag `v1.0.0` closure work.
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` tracks tag-time artifact evidence that must not be marked complete before real publication.

## 8. Related Docs

- `README.md`
- `ROADMAP.md`
- `docs/VM_INTERPRETER_PARITY_MATRIX.md`
- `docs/LANGUAGE_SPEC.md`
- `docs/STANDARD_LIBRARY.md`
- `docs/NATIVE_API_SECURITY_POSTURE.md`
- `docs/RELEASE_PROCESS.md`
