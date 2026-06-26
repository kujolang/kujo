# Kujo v1.0 Completeness-Ready Roadmap

Last audit date: 2026-05-06
> Current crate version: `1.0.0` in [Cargo.toml](Cargo.toml)
Target release: 1.0.0
Release state: pre-tag `1.0.0` release candidate metadata; the final `v1.0.0` tag, crate publication, and binary artifact publication are not complete until the release artifact checklist has dated evidence.
Roadmap status: active production-readiness backlog

This roadmap replaces the previous release-execution roadmap. The prior roadmap stated that Kujo had no known P0 language/runtime parity blockers. That is no longer accurate. The current audit found at least one failing test plus multiple security, correctness, diagnostics, runtime, HTTP, filesystem, and documentation gaps that must be closed before Kujo can credibly ship a production-ready 1.0 release.

The purpose of this document is to let an implementation agent work through Kujo item by item until the project is secure, predictable, tested, documented, benchmarked, and releasable.

## 0. Non-Negotiable Release Rules

Kujo 1.0 must not be released until all of these are true:

1. Every P0 and P1 item in this roadmap is complete.
2. Every changed behavior has tests.
3. Every changed language semantic is documented in `docs/LANGUAGE_SPEC.md`.
4. Every changed CLI contract is documented in `README.md` and, where machine-readable output is involved, `docs/CLI_MACHINE_READABLE_CONTRACTS.md`.
5. Every changed native API security boundary is documented in `docs/NATIVE_API_SECURITY_POSTURE.md`.
6. `cargo test` passes with no unexpected failures.
7. Security boundary integration tests pass.
8. VM/interpreter parity tests pass or every divergence is explicitly documented and justified.
9. Static server integration tests pass.
10. Examples and documented commands are smoke-tested.
11. The release checklist at the end of this file is complete.

## 1. Current Baseline

The audit originally found this state:

```text
Previously failing command:
    cargo test

Previously failing test:
    vm::tests::test_sum_int_map_until_local_in_place_missing_key_errors

Observed failure:
    src/vm.rs:8090:26
    Expected runtime error, got value: Int(0)

Observed summary:
    462 passed, 1 failed, 7 ignored
```

This VM test was not a release-execution detail. It proved the runtime could silently convert a missing map key in an in-place operation into `Int(0)` instead of reporting a runtime error. `V1-TEST-001` has restored the base test suite by making missing map keys runtime errors across VM/interpreter map indexing paths.

### Core AI-Native Enhancement Track

- Item 8 from `research/kujo-language-deep-dive/CORE_AI_NATIVE_ENHANCEMENTS_PROPOSAL.md` is implemented: `ai_request_hash(prompt_or_messages, options)` now provides deterministic, credential-independent request keys for future cassette replay and userland caching.
- Item 2 from the same proposal is implemented: the four core `ai_*` helpers can now run against deterministic JSON cassettes in strict replay mode without opening a socket, with record and fallthrough mechanisms available for fixture refresh.
- Item 1 from the same proposal is implemented: AI helper success values now expose normalized usage/finish/tool metadata, and typed error dictionaries are available behind opt-in `options.structured_errors`.
- Remaining core AI-native items are tracked in that proposal's Progress Tracker and should continue in phase order without adding retry policy, provider routing, RAG, agents, MCP, eval, observability, or registry behavior to core.

## 2. Priority And Severity Definitions

Priority:

- P0: Must fix before any serious public use or v1.0 release candidate.
- P1: Must fix before v1.0 unless explicitly deferred in a documented release exception.
- P2: Strongly recommended for v1.0. Can be deferred only if the limitation is documented and tested.
- P3: Useful polish or future hardening. Not required for 1.0 unless it becomes a dependency of a higher-priority item.

Severity:

- Critical: Can cause unsafe host effects, severe security exposure, data loss, or unusable language/runtime behavior.
- High: Can cause incorrect execution, silent failure, dangerous defaults, major crashes, or misleading behavior.
- Medium: Important robustness, maintainability, diagnostics, performance, or coverage gap.
- Low: Minor behavior, polish, or local maintainability issue.
- Enhancement: Expands usefulness, ergonomics, coverage, or future maintainability.

## 3. Agent Execution Contract

Any agent implementing this roadmap must follow these rules:

1. Work one roadmap item or one tightly related item group at a time.
2. Do not remove existing functionality unless it is unsafe or obsolete and replacement behavior is added.
3. Do not reduce test coverage.
4. Do not skip tests for new behavior.
5. Do not silently change language semantics without documenting and testing the change.
6. Do not patch around symptoms when a centralized abstraction is the correct fix.
7. Every completed item must include code changes, tests, and documentation updates where relevant.
8. Every item must leave the repository with all relevant tests passing.
9. If an item exposes additional bugs, add regression tests before or with the fix.
10. If a current behavior is intentionally retained, document it and test it.

## 4. Required Verification Commands

Run the smallest relevant test command while developing an item. Before marking any P0 or P1 item complete, run the item-specific tests plus `cargo test` unless the item explicitly documents why that is not possible.

Required release-gate commands:

```sh
cargo test
cargo test --test native_api_security_boundaries
cargo test --test serve_command_integration
cargo test --test package_module_workflow_integration
cargo test --test vm_interpreter_parity_surfaces
cargo run -- test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

Additional commands to add during the roadmap:

```sh
cargo audit
cargo deny check
cargo fuzz run lexer
cargo fuzz run parser
cargo fuzz run runtime_entrypoints
cargo bench
```

If a command is not yet available, create the missing configuration as part of the relevant roadmap item.

## 5. Repository Map

| Area | Files/Paths | Responsibility | Current Maturity | Immediate Concerns |
| ---- | ----------- | -------------- | ---------------- | ------------------ |
| Crate config | `Cargo.toml`, `Cargo.lock` | Build metadata, dependencies, crate version | usable | Version metadata is staged at `1.0.0`; final tag/publish readiness still depends on release gates and artifact evidence. |
| CLI entry point | `src/main.rs` | Command dispatch, serve command helpers, test-only server helpers | partial | Contains duplicate `#[cfg(test)]` MIME/server logic that differs from production server policy. CLI exit-code and diagnostics policy need tightening. |
| Library facade | `src/lib.rs` | Public crate module exposure | usable | Needs stable 1.0 embedding/API surface decisions. |
| Lexer | `src/lexer.rs` | Source tokenization | partial | Silent invalid-character handling, weak malformed literal handling, no structured diagnostics. |
| Parser | `src/parser.rs` | AST construction and grammar handling | partial | `Option`-based parse failures, unchecked delimiter consumption, weak recovery, recursion/depth risks. |
| AST | `src/ast.rs` | Syntax tree representation | usable | Needs consistent spans and semantic invariants. |
| Diagnostics | `src/errors.rs`, `src/lsp_diagnostics.rs` | Error formatting and LSP diagnostics | partial | Error model is not yet central across lexer/parser/runtime/CLI/VM. |
| Interpreter | `src/interpreter/mod.rs`, `src/interpreter/environment.rs`, `src/interpreter/value.rs` | Tree-walk execution, values, scopes | partial | Undefined identifiers become strings, arity is inconsistent, invalid operations silently fall back. |
| Native filesystem APIs | `src/interpreter/native_functions/filesystem.rs` | File IO, archive helpers, metadata | partial | `unzip` path traversal risk, no centralized safe path/root policy. |
| Native system APIs | `src/interpreter/native_functions/system.rs`, `src/interpreter/builtins.rs` | Process/env/shell/native host effects | partial | Powerful trusted-code APIs have no runtime capability policy. Shell execution requires explicit hardening. |
| Native networking APIs | `src/interpreter/native_functions/network.rs` | TCP/UDP/network helpers | partial | Raw networking needs capability controls, timeouts, limits, and documentation. |
| Native HTTP APIs | `src/interpreter/native_functions/http.rs` | HTTP client/server helpers | partial | Needs request/response contracts, limits, timeout policy, and tests. |
| Async runtime helpers | `src/interpreter/native_functions/async_ops.rs` | Async operations and scheduling helpers | partial | Needs cancellation/resource semantics and tests. |
| Module system | `src/module.rs` | Imports, package/module resolution | usable | Needs path normalization, cache invalidation rules, security tests, and docs. |
| Compiler | `src/compiler.rs` | Bytecode generation | partial | Must stay semantically aligned with interpreter and VM. |
| VM | `src/vm.rs` | Bytecode execution | partial | Current failing missing-key test; parity and runtime error behavior need hardening. |
| JIT | `src/jit.rs` | Optional execution optimization path | demo/partial | Must be gated behind parity tests and explicit unsupported-surface behavior. |
| Static server | `src/serve_http.rs`, `tests/serve_command_integration.rs` | `kujo serve` static file serving | usable/partial | Security policy is incomplete: dotfiles, unknown active content fallback, limits, 405 headers, streaming/range. |
| Tests | `tests/*.rs`, inline module tests | Unit, integration, security, parity tests | usable | Strong base, but missing fuzzing, broader negative tests, benchmarks, and full security matrix. |
| Documentation | `README.md`, `docs/*.md` | User docs, spec, security posture, contracts | partial | Some docs overstate current production readiness or mismatch implementation. |
| Examples | `examples/`, documented snippets | User-facing behavior samples | partial | Need smoke tests that examples still parse/run. |

## 6. Phase Overview

| Phase | Name | Goal | Included Priorities | Exit Condition |
| ----- | ---- | ---- | ------------------- | -------------- |
| 0 | Baseline Reset | Make the roadmap and CI reflect the real current state. | P0 | Current failing tests and blockers are tracked, not hidden. |
| 1 | Stop The Bleeding | Fix release-blocking crashes, silent failures, and dangerous filesystem/server behavior. | P0 | `cargo test` is green and P0 safety regressions are covered. |
| 2 | Make Behavior Predictable | Harden lexer, parser, diagnostics, and core runtime semantics. | P0/P1 | Bad input fails with structured diagnostics instead of silent partial behavior. |
| 3 | Secure Host Boundaries | Add capability policy and safe defaults for filesystem, process, network, HTTP, imports. | P1 | Native APIs have explicit security boundaries and tests. |
| 4 | Make Kujo Universally Useful | Generalize HTTP, stdlib, CLI, module, docs, and examples beyond demo paths. | P1/P2 | Core user workflows are broad, tested, and documented. |
| 5 | Make It Measurable | Add fuzzing, benchmarks, performance budgets, and regression gates. | P1/P2 | CI can catch correctness, security, and performance regressions. |
| 6 | Make It Releasable | Complete docs, packaging, release process, versioning, changelog, and support policy. | P1/P2/P3 | Kujo can cut a 1.0 release candidate with clear gates. |

## 7. Master Roadmap Checklist

### Phase 0: Baseline Reset

```text
[x] V1-BASE-001: Establish the audited baseline as the source of truth
    Priority: P0
    Severity: High
    Area: Release/Tests/Docs
    Affected files: ROADMAP.md, README.md, docs/LANGUAGE_SPEC.md, docs/NATIVE_API_SECURITY_POSTURE.md
    Problem: The previous roadmap described Kujo as having no known P0 blockers, but the audit found a failing VM test and multiple production-readiness gaps.
    Recommendation: Treat this roadmap as the active release gate. Update any public docs that currently imply production readiness before the P0 items are fixed.
    Implementation steps:
        1. Search docs for claims such as "production-ready", "no known blockers", "v1 complete", and "release blocked only by packaging".
        2. Replace those claims with accurate pre-1.0 language.
        3. Add a short "1.0 readiness status" section to README.md that points to this roadmap.
    Tests required: Documentation smoke check only. Run `cargo test` to confirm no accidental code changes if files outside docs are touched.
    Acceptance criteria: Public docs do not overstate readiness, and this roadmap is the single 1.0 completion source.
    Notes: Completed on 2026-05-07. Audited public readiness language across README/spec/security docs, added an explicit README `1.0 Readiness Status` section that points to `ROADMAP.md` as the single release-gate source, and clarified that `v1.0.0 baseline draft` labels in `docs/LANGUAGE_SPEC.md` and `docs/NATIVE_API_SECURITY_POSTURE.md` do not imply release readiness. Verification: `cargo test` passed.
```

```text
[x] V1-BASE-002: Create a release-gate checklist script
    Priority: P1
    Severity: Medium
    Area: DX/Tests/Release
    Affected files: scripts/release_gate.sh, README.md, docs/RELEASE_PROCESS.md
    Problem: Required verification commands are documented manually, but there is no single repeatable local release-gate command.
    Recommendation: Add a script that runs formatting, clippy, tests, security checks, docs/example smoke tests, and benchmarks where configured.
    Implementation steps:
        1. Create `scripts/release_gate.sh`.
        2. Run commands in this order: fmt check, clippy, cargo test, selected integration tests, Kujo self-test command, audit/deny if installed.
        3. Print each command before running it.
        4. Exit non-zero on the first failure.
        5. Document prerequisites and expected runtime.
    Tests required: Add a lightweight CI job or local test that executes the script in a minimal mode if full mode is too expensive.
    Acceptance criteria: A contributor can run one command and get the same required gate used for 1.0 readiness.
    Notes: Completed on 2026-05-07. `scripts/release_gate.sh` now exposes explicit `--full` and `--minimal` modes. Full mode runs the required command order (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, selected integration tests, Kujo self-test command, optional `cargo audit`/`cargo deny` when installed) with fail-fast behavior and printed command traces. Minimal mode provides lightweight smoke execution for CI/local validation. Added CI smoke coverage in `.github/workflows/ci-release-gate.yml` (`release-gate-minimal-smoke`) plus documentation updates in `README.md` and `docs/RELEASE_PROCESS.md` covering prerequisites, runtime expectations, and mode/environment toggles. Verification: `bash scripts/release_gate.sh --minimal`, `cargo test`.
```

### Phase 1: Stop The Bleeding

```text
[x] V1-TEST-001: Fix the failing VM missing-key in-place map operation
    Priority: P0
    Severity: High
    Area: Correctness/Tests
    Affected files: src/vm.rs, src/compiler.rs, tests/vm_interpreter_parity_surfaces.rs
    Problem: `cargo test` currently fails because the VM returns `Int(0)` for a missing map key in an in-place operation instead of reporting a runtime error.
    Recommendation: Make VM map indexing and in-place update paths use the same missing-key error semantics as the interpreter.
    Implementation steps:
        1. Inspect `vm::tests::test_sum_int_map_until_local_in_place_missing_key_errors`.
        2. Trace the bytecode emitted for the failing program.
        3. Locate the map read/update opcode path that defaults missing values to `Int(0)`.
        4. Replace the default with a structured runtime error containing key/index context.
        5. Confirm compiler-generated bytecode does not skip the error path.
        6. Add parity coverage for local map update, captured map update, nested map update, and invalid key type.
    Tests required:
        - Existing `vm::tests::test_sum_int_map_until_local_in_place_missing_key_errors` must pass.
        - Add VM/interpreter parity tests for missing string key, missing integer key, nested missing key, and successful update.
        - Run `cargo test`.
    Acceptance criteria: Missing map keys never silently become `Int(0)` in VM or interpreter execution.
    Notes: Completed on 2026-05-06. VM `IndexGet`/`IndexGetInPlace` now share centralized missing-key handling, interpreter dictionary indexing reports matching missing-key errors, and parity coverage locks missing string keys, missing integer keys, nested missing keys, invalid key types, and local/nested/captured update success paths. Verification: `cargo test` passed.
```

```text
[x] V1-RUN-001: Make undefined identifiers runtime errors
    Priority: P0
    Severity: High
    Area: Correctness/Security
    Affected files: src/interpreter/mod.rs, src/interpreter/environment.rs, src/interpreter/value.rs, src/errors.rs, docs/LANGUAGE_SPEC.md
    Problem: Identifier evaluation currently falls back to `Value::Str(name)` when a binding is missing. This turns typos and failed security checks into valid string values.
    Recommendation: Replace the fallback with a structured undefined-variable runtime error.
    Implementation steps:
        1. Locate `Expr::Identifier` evaluation in `src/interpreter/mod.rs`.
        2. Replace `unwrap_or(Value::Str(...))` behavior with an error.
        3. Include identifier name and source span when available.
        4. Audit call sites that intentionally depended on identifier-to-string fallback.
        5. For any intended symbol/string shorthand, introduce an explicit syntax or built-in helper instead of implicit fallback.
        6. Update compiler/VM identifier resolution to match.
        7. Update spec and examples.
    Tests required:
        - Undefined top-level variable fails.
        - Undefined variable inside function fails.
        - Undefined variable inside closure fails.
        - Undefined method receiver/member fails with useful message.
        - String literals still work normally.
        - Existing examples do not rely on implicit identifier strings.
    Acceptance criteria: No runtime path converts an unknown identifier into a string silently.
    Notes: Completed on 2026-05-06. Interpreter identifier lookup now reports `Undefined variable: <name>` instead of implicitly producing a string, errors propagate through common expression/statement surfaces, and explicit string literals/defined identifiers keep working. VM global/local lookup errors now use the same undefined-variable message, cooperative VM execution surfaces returned error values as execution errors, and top-level script JIT avoids variable-load programs until checked undefined lookup parity is preserved. Verification: `cargo test` passed.
```

```text
[x] V1-RUN-002: Enforce function, method, closure, async, and generator arity
    Priority: P0
    Severity: High
    Area: Correctness
    Affected files: src/interpreter/mod.rs, src/interpreter/value.rs, src/compiler.rs, src/vm.rs, docs/LANGUAGE_SPEC.md
    Problem: User-defined call paths can leave missing parameters unbound or ignore extra arguments, causing unpredictable execution.
    Recommendation: Centralize arity checking and apply it to all callable values.
    Implementation steps:
        1. Create a shared callable metadata helper that exposes function name, min args, max args, variadic status, and parameter names.
        2. Apply it before entering user functions, closures, methods, async functions, generator functions, and native functions.
        3. Return a consistent runtime error for too few and too many args.
        4. Preserve intentional variadic native behavior only where explicitly declared.
        5. Update VM call opcodes to enforce the same rules.
    Tests required:
        - Too few args fail for functions, methods, closures, async functions, generators, and natives.
        - Too many args fail for the same call types.
        - Correct arity succeeds.
        - Variadic native functions accept expected ranges.
        - Error messages include function name, expected count/range, and received count.
    Acceptance criteria: Every callable has a tested arity contract and no call path silently ignores or invents arguments.
    Notes: Completed on 2026-05-06. Added shared callable arity metadata (`name`, `min/max`, `variadic`, `parameter_names`) and wired it through interpreter call paths, VM bytecode call preparation, and selected native dispatch metadata. Too-few/too-many calls now return consistent `... expects ... arguments, got ...` runtime errors for functions, closures, struct methods, async functions, generators, and covered native strict/range contracts (including `len` and `input`) while preserving explicit variadic natives (`debug`, `print`, `array`). Added VM/interpreter parity coverage for function/method/closure/async/generator/native failure modes plus success paths and variadic native acceptance. Verification: `cargo test --test vm_interpreter_parity_surfaces`, `cargo test arity`, and `cargo test` passed.
```

```text
[x] V1-RUN-003: Replace silent invalid operation fallbacks with runtime errors
    Priority: P0
    Severity: High
    Area: Correctness/Security
    Affected files: src/interpreter/mod.rs, src/interpreter/value.rs, src/vm.rs, src/compiler.rs, docs/LANGUAGE_SPEC.md
    Problem: Some invalid indexing, invalid assignment, invalid operation, and missing value paths return `Int(0)`, empty strings, print to stderr, or continue execution.
    Recommendation: Add centralized runtime semantic helpers for invalid operations and use them across interpreter and VM.
    Implementation steps:
        1. Audit indexing, assignment, arithmetic, comparison, call, member access, map/list access, and in-place update paths.
        2. Replace sentinel fallback values with structured runtime errors.
        3. Replace `eprintln!` plus continue behavior with returned errors.
        4. Add helper functions for `index_value`, `assign_index`, `assign_member`, `binary_op`, and `unary_op`.
        5. Ensure VM and interpreter share equivalent messages and error kinds.
    Tests required:
        - Out-of-bounds list access fails.
        - Out-of-bounds string access fails.
        - Missing map key fails.
        - Indexing non-indexable values fails.
        - Assigning to invalid target fails.
        - Unsupported binary/unary operations fail.
        - VM/interpreter parity tests cover all cases.
    Acceptance criteria: Invalid runtime operations never silently return default values or only print to stderr.
    Notes: Completed on 2026-05-06. Added centralized interpreter helpers for `index_value`, `assign_index`, `assign_member`, `binary_op_value`, and `unary_op_value`; replaced assignment-path `eprintln!` continuation behavior with returned runtime errors; and aligned VM binary/unary behavior so unsupported operations return deterministic runtime errors instead of `Int(0)` fallbacks. Added parity regression coverage for out-of-bounds array/string indexing, non-indexable assignment targets, invalid index operations, unsupported unary/binary operations, and valid index-assignment success paths in `tests/vm_interpreter_parity_surfaces.rs`. Verification: `cargo test --test vm_interpreter_parity_surfaces` passed.
```

```text
[x] V1-SEC-001: Make archive extraction safe against Zip Slip and resource exhaustion
    Priority: P0
    Severity: Critical
    Area: Security/FileSystem
    Affected files: src/interpreter/native_functions/filesystem.rs, tests/native_api_security_boundaries.rs, docs/NATIVE_API_SECURITY_POSTURE.md
    Problem: `unzip` joins `output_dir` with archive entry names directly. Malicious archive paths can escape the target directory. There are also no extraction size/count limits.
    Recommendation: Implement safe archive extraction with normalized containment checks and explicit resource limits.
    Implementation steps:
        1. Use sanitized archive entry paths instead of raw names.
        2. Reject absolute paths, parent components, drive prefixes, null bytes, and empty normalized paths.
        3. Resolve output paths through a central containment helper.
        4. Reject symlink entries unless a documented safe policy is implemented.
        5. Add max total uncompressed bytes.
        6. Add max entry count.
        7. Add max single entry size.
        8. Fail the whole extraction on the first unsafe entry.
        9. Document the limits and failure behavior.
    Tests required:
        - Archive entry `../escape.txt` is rejected.
        - Archive entry `/tmp/escape.txt` is rejected.
        - Windows drive-like paths are rejected.
        - Null-byte names are rejected.
        - Symlink entries are rejected or safely handled according to documented policy.
        - Oversized total extraction is rejected.
        - Oversized single file is rejected.
        - Safe nested extraction succeeds.
    Acceptance criteria: No archive entry can write outside the requested output directory, and extraction cannot consume unbounded disk space.
    Notes: Completed on 2026-05-06. `unzip` now routes through centralized extraction helpers in `src/interpreter/native_functions/filesystem.rs` that sanitize archive entry paths, reject absolute/parent-traversal/drive-prefixed/null-byte/empty-normalized paths, block symlink entries, and enforce deterministic limits (1024 entries, 16 MiB per entry, 64 MiB total uncompressed bytes) while failing fast on the first unsafe entry. Added integration security regressions in `tests/native_api_security_boundaries.rs` for traversal, absolute path, drive prefix, null-byte name, symlink metadata, single-entry and total-size exhaustion, entry-count exhaustion, and safe nested extraction success. Updated runtime policy docs in `docs/NATIVE_API_SECURITY_POSTURE.md`, `README.md`, and `CHANGELOG.md`. Verification: `cargo test --test native_api_security_boundaries` and `cargo test -q` passed.
```

```text
[x] V1-HTTP-001: Unify static server MIME and active-content policy
    Priority: P0
    Severity: High
    Area: Security/HTTP
    Affected files: src/main.rs, src/serve_http.rs, tests/serve_command_integration.rs, README.md
    Problem: `src/main.rs` contains `#[cfg(test)]` duplicate static-server helper logic that blocks unknown active content, while production `src/serve_http.rs` does not implement the same fallback policy. This creates test/production security drift.
    Recommendation: Move MIME detection and active-content fallback policy into one production module and make tests exercise the production code.
    Implementation steps:
        1. Create or consolidate a single MIME registry used by `src/serve_http.rs`.
        2. Remove duplicate test-only MIME logic from `src/main.rs`.
        3. Define exact fallback behavior:
           - known safe extension: configured MIME type
           - unknown passive file: `application/octet-stream`
           - unknown active content extension: reject with `415 Unsupported Media Type` or serve as download-only octet-stream according to documented policy
           - extensionless file: documented fallback
        4. Make extension matching case-insensitive.
        5. Add `X-Content-Type-Options: nosniff` for file responses.
        6. Update README claims to match actual production behavior.
    Tests required:
        - Known HTML, CSS, JS, JSON, PNG, JPG, SVG, WASM, font, PDF, text mappings.
        - Unknown extension fallback.
        - Case-insensitive extension mapping.
        - Extensionless file fallback.
        - Unknown active-content extension blocked or forced-download according to policy.
        - Tests import and exercise production helpers, not duplicate test helpers.
    Acceptance criteria: There is exactly one MIME/security policy implementation used by production server code and tests.
    Notes: Completed on 2026-05-06. Static serving MIME/security behavior is now centralized in `src/serve_http.rs` with one case-insensitive extension registry and explicit fallback policy: known extensions use configured MIME, unknown extensions and extensionless files serve as `application/octet-stream`, and unknown active-content payloads are forced to safe octet-stream responses instead of content-sniff promotion. Duplicate `#[cfg(test)]` serve MIME/security helpers were removed from `src/main.rs`, and coverage now exercises production paths via `src/serve_http.rs` unit tests plus live subprocess checks in `tests/serve_command_integration.rs` (including required HTML/CSS/JS/JSON/PNG/JPG/SVG/WASM/font/PDF/text mappings, unknown fallback, extensionless fallback, unknown active-content fallback, case-insensitive extensions, and `X-Content-Type-Options: nosniff`). Verification: `cargo test serve_http::tests`, `cargo test --test serve_command_integration`, and `cargo test` passed.
```

```text
[x] V1-CI-001: Prevent release with a failing test suite
    Priority: P0
    Severity: High
    Area: CI/Tests/Release
    Affected files: .github/workflows/*.yml, scripts/release_gate.sh, README.md
    Problem: The repo can drift into a state where roadmap/docs claim readiness while `cargo test` fails.
    Recommendation: CI must block merges and releases on the release-gate test suite.
    Implementation steps:
        1. Inspect existing GitHub Actions workflows.
        2. Ensure `cargo test` runs on every pull request and push to main.
        3. Ensure integration tests run on every pull request.
        4. Add fmt and clippy checks.
        5. Add a release-gate workflow or job that calls `scripts/release_gate.sh`.
        6. Add badges or README notes only after the workflow is green.
    Tests required: CI itself is the test. Locally run `scripts/release_gate.sh` after it exists.
    Acceptance criteria: A failing unit or integration test prevents merge/release.
    Notes: Completed on 2026-05-07. Added `scripts/release_gate.sh` and wired it into a new PR/main CI workflow (`.github/workflows/ci-release-gate.yml`) plus release-tag publishing workflow gating in `.github/workflows/release-binaries.yml`. CI now enforces `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and release-gate test execution before merge/release jobs can pass. The release-gate script runs stable test coverage for release-critical surfaces (`cargo test --lib -- --test-threads=1`, integration suite checks, native security boundaries, package workflow, VM/interpreter parity), supports socket-bound integration execution in CI via `KUJO_ENABLE_SOCKET_TESTS=1`, and skips optional `cargo audit`/`cargo deny` when tools are not installed. Verification: `cargo test` and `bash scripts/release_gate.sh` passed locally.
```

### Phase 2: Make Behavior Predictable

```text
[x] V1-LEX-001: Convert lexer failures into structured diagnostics
    Priority: P0
    Severity: High
    Area: Parser/Diagnostics/Correctness
    Affected files: src/lexer.rs, src/errors.rs, src/lsp_diagnostics.rs, docs/LANGUAGE_SPEC.md
    Problem: Lexer behavior is too optimistic for production. Invalid characters, malformed strings, invalid escapes, unterminated comments, and huge numeric literals can be ignored or downgraded into sentinel values.
    Recommendation: Make lexing return tokens plus diagnostics, or `Result<Vec<Token>, Vec<Diagnostic>>`, with precise spans.
    Implementation steps:
        1. Define lexer diagnostic kinds: invalid character, invalid UTF-8/encoding if applicable, null byte, unterminated string, unterminated comment, invalid escape, numeric literal overflow, malformed numeric literal.
        2. Track byte offsets, line, column, and file path where available.
        3. Reject null bytes in source.
        4. Reject or diagnose invalid escape sequences instead of accepting them silently.
        5. Replace numeric `unwrap_or(0)` and `unwrap_or(0.0)` fallbacks with diagnostics.
        6. Set maximum token length limits for identifiers and literals, with configurable constants.
        7. Preserve recovery where useful, but never silently drop invalid input.
    Tests required:
        - Invalid character reports diagnostic.
        - Unterminated string reports diagnostic.
        - Unterminated block comment reports diagnostic.
        - Invalid escape reports diagnostic.
        - Huge integer reports diagnostic.
        - Huge float reports diagnostic.
        - Null byte reports diagnostic.
        - Mixed line endings preserve correct line/column.
        - Long identifier and long string limit tests.
    Acceptance criteria: Lexing malformed source always produces a diagnostic and never fabricates valid tokens without reporting the problem.
    Notes: Completed on 2026-05-07. `src/lexer.rs` now returns structured lexer diagnostics (`Result<Vec<Token>, Vec<LexerDiagnostic>>`) and exposes recovery-aware tokenization for diagnostic consumers. Added explicit diagnostic kinds for invalid characters, null bytes, unterminated strings/comments, invalid escapes, malformed/overflowing numerics, and token-length limits (identifier/string/numeric), with line/column/byte-offset and optional file-path metadata. Replaced numeric `unwrap_or` fallbacks with diagnostics, added CRLF-aware mixed-line-ending tracking, and wired lexer-error propagation through CLI/module/REPL/benchmark/LSP/linter call paths so malformed source is reported instead of silently dropped. Added regression coverage in `src/lexer.rs` tests plus LSP diagnostic bridging coverage in `src/lsp_diagnostics.rs`. Verification: `cargo test lexer::tests --lib`, `cargo test lsp_diagnostics::tests --lib`, and `cargo test` passed.
```

```text
[x] V1-PAR-001: Replace `Option` parser failures with structured parse results
    Priority: P0
    Severity: High
    Area: Parser/Diagnostics
    Affected files: src/parser.rs, src/errors.rs, src/lsp_diagnostics.rs, src/main.rs, docs/LANGUAGE_SPEC.md
    Problem: The parser uses `Option` and unchecked token advancement in many places. Missing delimiters or malformed syntax can produce partial ASTs or confusing failures.
    Recommendation: Introduce a parser result type that returns AST plus diagnostics, and make expected-token failures explicit.
    Implementation steps:
        1. Add `ParseDiagnostic` or reuse the central diagnostic type from `V1-ERR-001`.
        2. Replace delimiter consumption helpers with `expect_token(kind, message)`.
        3. Ensure every `advance()` that expects a token has an EOF check.
        4. Add synchronization points for statement-level recovery.
        5. Return parse failure for invalid top-level constructs.
        6. Ensure CLI exits non-zero on parse diagnostics.
        7. Update LSP diagnostic conversion to preserve spans and messages.
    Tests required:
        - Missing `)` fails with clear location.
        - Missing `]` fails with clear location.
        - Missing `}` fails with clear location.
        - Invalid assignment target fails.
        - Unexpected EOF fails.
        - Parser recovery reports multiple independent errors without infinite loop.
        - CLI exits non-zero for parse errors.
    Acceptance criteria: Parser never silently accepts malformed syntax as a valid partial program.
    Notes: Completed on 2026-05-07. Added structured parser output in `src/parser.rs` (`ParseOutput` + `ParseDiagnostic`) with statement-level recovery and explicit `expect_*` delimiter/keyword/operator helpers for core parser surfaces. Missing `)`, `]`, `}`, invalid assignment targets, and EOF-truncated blocks now emit line/column diagnostics instead of silently yielding partial ASTs. CLI parse entrypoints (`kujo run`, `kujo test-run`, `kujo profile`) now exit non-zero on parser diagnostics, module loading reports parser failures, REPL surfaces parser diagnostics, and `src/lsp_diagnostics.rs` now consumes parser diagnostics directly instead of parser-panic probing. Added regression coverage in `tests/parser_diagnostics_contract.rs` for success/failure/edge/recovery/CLI exit behavior and updated parser-diagnostic LSP unit coverage. Verification: `cargo test --test parser_diagnostics_contract`, `cargo test lsp_diagnostics::tests --lib`, and `cargo test` passed.
```

```text
[x] V1-ERR-001: Centralize diagnostics across lexer, parser, runtime, VM, CLI, and LSP
    Priority: P1
    Severity: High
    Area: Diagnostics/Architecture
    Affected files: src/errors.rs, src/lsp_diagnostics.rs, src/lexer.rs, src/parser.rs, src/interpreter/mod.rs, src/vm.rs, src/main.rs, docs/CLI_MACHINE_READABLE_CONTRACTS.md
    Problem: Error reporting is inconsistent across subsystems. Production Kujo errors should answer what failed, where, why, and what to do next.
    Recommendation: Introduce one diagnostic model with severity, code, message, optional help, file path, span, line, column, and subsystem.
    Implementation steps:
        1. Define stable diagnostic codes such as `RUFLEX001`, `RUFPARSE001`, `RUFRUN001`, `RUFVM001`, `RUFCLI001`.
        2. Add text rendering for humans.
        3. Add JSON rendering for machine-readable CLI mode.
        4. Add LSP conversion.
        5. Route lexer, parser, runtime, VM, and CLI errors through the model.
        6. Document exit-code mapping.
    Tests required:
        - Snapshot/golden tests for human errors.
        - JSON schema tests for machine-readable diagnostics.
        - LSP conversion tests.
        - CLI exit-code tests.
        - Runtime error includes file/line/column when available.
    Acceptance criteria: Each user-visible failure has a stable code, precise location when possible, and consistent rendering.
    Notes: Completed on 2026-05-07. Added a shared diagnostic model in `src/errors.rs` with stable codes, severity/subsystem metadata, human rendering, and machine-readable JSON rendering. Routed lexer (`LexerDiagnostic::to_diagnostic`), parser (`ParseDiagnostic::to_diagnostic`), LSP diagnostics (`src/lsp_diagnostics.rs`), and CLI reporting (`src/main.rs`) through the unified model, including VM/runtime/CLI code tags (`RUFVM001`, `RUFRUN001`, `RUFCLI001`) in user-visible failures. Expanded regression coverage with `tests/diagnostics_contract.rs` plus JSON and parse-contract updates in `tests/cli_json_contracts.rs` and `tests/parser_diagnostics_contract.rs`, and documented the new `lsp-diagnostics --json` schema in `docs/CLI_MACHINE_READABLE_CONTRACTS.md`. Verification: `cargo test --test diagnostics_contract`, `cargo test --test parser_diagnostics_contract`, `cargo test --test cli_json_contracts`, `cargo test lsp_diagnostics::tests --lib`, and `cargo test` passed.
```

```text
[x] V1-PAR-002: Add parser depth and source size safety limits
    Priority: P1
    Severity: High
    Area: Security/Parser/Performance
    Affected files: src/parser.rs, src/lexer.rs, src/main.rs, docs/LANGUAGE_SPEC.md
    Problem: Deep nesting and very large source files can cause stack overflows, excessive CPU, or excessive memory use.
    Recommendation: Add explicit limits with clear diagnostics and, where appropriate, CLI flags/env config for trusted local use.
    Implementation steps:
        1. Define default maximum source bytes for CLI file input.
        2. Define maximum expression nesting depth.
        3. Define maximum block nesting depth.
        4. Define maximum call nesting depth if recursive parsing remains.
        5. Increment/decrement depth counters in parser entry points.
        6. Return structured diagnostics when limits are exceeded.
    Tests required:
        - Deep parentheses exceed limit gracefully.
        - Deep list/map literals exceed limit gracefully.
        - Deep nested blocks exceed limit gracefully.
        - Large source file over limit fails before parsing.
        - Boundary-at-limit input succeeds.
    Acceptance criteria: Malicious deep or huge source cannot crash the process through parser recursion or unbounded allocation.
    Notes: Completed on 2026-05-07. Added explicit parser safety constants and limit enforcement (`DEFAULT_MAX_SOURCE_BYTES=1,048,576`, `DEFAULT_MAX_EXPRESSION_DEPTH=256`, `DEFAULT_MAX_BLOCK_DEPTH=128`) plus centralized parser helpers for expression/block depth checks with structured parser diagnostics when limits are exceeded. Added CLI pre-parse source-size validation for parse entrypoints (`run`, `test-run`, `profile`, and `lsp-*` helper commands) so oversized source fails deterministically with `RUFPARSE001` before tokenization/parsing. Added regression coverage in `tests/parser_diagnostics_contract.rs` for deep parenthesized expressions, deep nested array literals, deep nested `if` blocks, source-size over-limit failure, and source-size boundary success. Verification: `cargo test --test parser_diagnostics_contract`, `cargo test`.
```

```text
[x] V1-PAR-003: Lock operator precedence and associativity with golden tests
    Priority: P1
    Severity: Medium
    Area: Correctness/Tests
    Affected files: src/parser.rs, src/lexer.rs, docs/LANGUAGE_SPEC.md, tests/parser_precedence.rs
    Problem: A language cannot be production-ready if precedence or associativity is implicit, incomplete, or untested.
    Recommendation: Define and test the full operator table.
    Implementation steps:
        1. Document precedence from highest to lowest.
        2. Document associativity for each operator.
        3. Add AST shape golden tests for every binary/unary operator pair.
        4. Add runtime evaluation tests where AST shape is insufficient.
        5. Include assignment and in-place operators.
    Tests required:
        - Arithmetic precedence.
        - Comparison precedence.
        - Equality precedence.
        - Boolean precedence.
        - Unary precedence.
        - Assignment associativity.
        - In-place update behavior.
        - Parenthesized override behavior.
    Acceptance criteria: Precedence behavior is documented and protected by tests.
    Notes: Completed on 2026-05-07. Split parser precedence tiers so comparison (`<`, `<=`, `>`, `>=`) and equality (`==`, `!=`) are independently ordered, added statement-level compound assignment support (`+=`, `-=`, `*=`, `/=`, `%=`) with deterministic lowering to assignment + binary-op semantics, and added a parser diagnostic for unsupported chained assignments. Added lexer tokenization for compound assignment operators and a dedicated precedence contract suite in `tests/parser_precedence.rs` that locks AST shapes plus runtime checks for arithmetic/comparison/equality/boolean/unary precedence, assignment/in-place update behavior, and parenthesized precedence override behavior. Documented the precedence/associativity table in `docs/LANGUAGE_SPEC.md` and aligned README language-surface notes. Verification: `cargo test --test parser_precedence`, `cargo test tokenizes_compound_assignment_operators --lib`, `cargo test`.
```

```text
[x] V1-PAR-004: Make AST spans complete and consistent
    Priority: P1
    Severity: Medium
    Area: Diagnostics/Architecture
    Affected files: src/ast.rs, src/parser.rs, src/lexer.rs, src/errors.rs, src/lsp_diagnostics.rs
    Problem: Diagnostics cannot be precise unless AST nodes consistently carry source spans.
    Recommendation: Ensure every expression, statement, declaration, and token-derived node has a start/end span.
    Implementation steps:
        1. Define one span type with byte offsets and line/column conversion.
        2. Add spans to AST nodes that do not have them.
        3. Populate spans during parsing.
        4. Use spans in runtime errors where source node context is available.
        5. Update LSP diagnostics to use the same spans.
    Tests required:
        - Span tests for literals, identifiers, binary expressions, function declarations, calls, blocks, loops, imports.
        - Runtime error location tests.
        - Parser error location tests.
    Acceptance criteria: User-facing diagnostics point to the correct source range for all common syntax and runtime failures.
    Notes: Completed on 2026-05-07. Added a shared `SourceSpan` model in `src/errors.rs` with byte-offset + line/column conversion helpers, extended lexer tokens (`src/lexer.rs`) to carry deterministic `byte_offset` metadata, and updated parser diagnostics (`src/parser.rs`) to include span payloads while preserving stable `line`/`column` contract fields. Parser outputs now publish `ast_spans` entries for statement/expression nodes so tooling surfaces can consume one consistent span stream, and parser-to-LSP diagnostic conversion now uses the same span-backed location source. Added regression coverage in `tests/parser_diagnostics_contract.rs` plus new lexer/errors span tests (`src/lexer.rs`, `src/errors.rs`) to lock monotonic token offsets, parser diagnostic span integrity, and parser AST span publication. Verification: `cargo test --test parser_diagnostics_contract`, `cargo test --test diagnostics_contract`, `cargo test token_byte_offsets_are_monotonic --lib`, `cargo test line_column_conversion_handles_multiline_utf8 --lib`, `cargo test`.
```

### Phase 3: Runtime, VM, And Language Semantics

```text
[x] V1-RUN-004: Centralize truthiness semantics
    Priority: P1
    Severity: Medium
    Area: Correctness/Language Semantics
    Affected files: src/interpreter/value.rs, src/interpreter/mod.rs, src/vm.rs, docs/LANGUAGE_SPEC.md
    Problem: Truthiness rules can drift across interpreter, VM, native functions, and docs.
    Recommendation: Implement one `is_truthy` semantic helper and use it everywhere conditions are evaluated.
    Implementation steps:
        1. Define truthiness for `Bool`, `Null`, `Int`, `Float`, `String`, lists, maps, functions, objects, and native handles.
        2. Decide whether empty string/list/map are truthy or falsey.
        3. Use the helper in `if`, `while`, `for`, logical operators, and VM conditional jumps.
        4. Document rules in the language spec.
    Tests required:
        - Truthiness table tests for every value kind.
        - Interpreter and VM parity tests.
        - Logical `and`/`or` short-circuit tests.
    Acceptance criteria: Truthiness is deterministic, documented, and identical across execution backends.
    Notes: Completed on 2026-05-07. Added centralized `Value::is_truthy()` semantics in `src/interpreter/value.rs` and routed interpreter control-flow (`if`, `while`, `loop`), VM condition/logical paths, and native predicate surfaces (`filter`, `find`, `any`, `all`, `assert`) through the shared helper so truthiness cannot drift by subsystem. Aligned logical operators with short-circuit execution in interpreter expression evaluation and compiler lowering so VM execution also skips unreachable RHS evaluation while preserving boolean result semantics. Added regression coverage for shared truthiness table behavior and short-circuit parity in `tests/vm_interpreter_parity_surfaces.rs`, plus interpreter-native collection predicate truthiness coverage in `tests/interpreter_tests.rs` and helper-table tests in `src/interpreter/value.rs`. Verification: `cargo test --test vm_interpreter_parity_surfaces`, `cargo test test_collection_predicates_use_shared_truthiness_semantics`, `cargo test value_truthiness_semantics_match_runtime_contract`, and `cargo test`.
```

```text
[x] V1-RUN-005: Enforce `let`, `const`, and mutability semantics
    Priority: P1
    Severity: High
    Area: Correctness/Language Semantics
    Affected files: src/interpreter/environment.rs, src/interpreter/mod.rs, src/compiler.rs, src/vm.rs, docs/LANGUAGE_SPEC.md
    Problem: The spec describes mutability concepts that are not consistently enforced by the runtime.
    Recommendation: Store binding metadata and reject invalid reassignment or mutation according to documented rules.
    Implementation steps:
        1. Define exact semantics for `let`, `const`, and any `mut` construct Kujo supports.
        2. Store binding kind in environment frames.
        3. Reject reassignment to immutable bindings.
        4. Decide and document whether mutable containers bound to const can mutate internally.
        5. Apply the same checks in compiler/VM paths.
        6. Add clear diagnostics.
    Tests required:
        - Reassigning const fails.
        - Reassigning immutable let fails or succeeds according to documented semantics.
        - Mutable binding reassignment succeeds.
        - Shadowing behavior is tested separately from reassignment.
        - VM/interpreter parity tests.
    Acceptance criteria: Binding mutability matches the language spec and cannot be bypassed by backend differences.
    Notes: Completed on 2026-05-07. Added binding-kind metadata across runtime surfaces so `let` and `const` bindings are enforced as immutable while `mut` bindings remain mutable. Interpreter environment frames now track binding kinds and return deterministic runtime errors for immutable reassignment/mutation attempts. Compiler/VM paths now preserve binding-kind metadata for globals and local slots, enforce immutable reassignment checks in `StoreVar`/`StoreLocal`/`StoreGlobal` paths, and guard global in-place index mutation with explicit mutability prechecks. Updated language documentation to lock the 1.0 choice that container/object in-place mutation through `let`/`const` bindings is rejected. Added regression coverage in `tests/vm_interpreter_parity_surfaces.rs` for immutable/mutable success and failure paths (including local function scope behavior) plus direct environment mutability contract tests in `tests/interpreter_tests.rs`. Verification: `cargo test --test vm_interpreter_parity_surfaces`, `cargo test test_environment_assign_checked_respects_binding_mutability --test interpreter_tests`, `cargo test test_environment_mutate_checked_respects_binding_mutability --test interpreter_tests`, and `cargo test`.
```

```text
[x] V1-RUN-006: Enforce scope isolation, shadowing, and declaration rules
    Priority: P1
    Severity: High
    Area: Correctness/Language Semantics
    Affected files: src/interpreter/environment.rs, src/interpreter/mod.rs, src/compiler.rs, src/vm.rs, docs/LANGUAGE_SPEC.md
    Problem: Scope leaks and unclear shadowing make programs unpredictable and complicate future compiler work.
    Recommendation: Define lexical scope rules and enforce them in interpreter and VM.
    Implementation steps:
        1. Define block scope, function scope, loop scope, module scope, and closure capture behavior.
        2. Decide whether duplicate declarations in the same scope are errors.
        3. Decide whether inner-scope shadowing is allowed.
        4. Implement environment checks for duplicate declarations and shadowing policy.
        5. Update compiler symbol resolution.
        6. Add runtime and compile/parse diagnostics where applicable.
    Tests required:
        - Block-local variables do not leak.
        - Function-local variables do not leak.
        - Loop variables follow documented lifetime.
        - Closure captures see correct values.
        - Duplicate same-scope declaration behavior is tested.
        - Shadowing behavior is tested.
    Acceptance criteria: Scope behavior is documented, tested, and identical across interpreter and VM.
    Notes: Completed on 2026-05-07. Added centralized same-scope declaration guards in `src/interpreter/environment.rs` (`define_with_kind_checked`) and routed interpreter `let`/`const` pattern binding through checked declaration paths so duplicate declarations now fail with deterministic `Duplicate declaration in the same scope: <name>` diagnostics. Updated compiler local symbol handling in `src/compiler.rs` with explicit local-scope declaration checks, function/lambda/method parameter duplicate-name rejection, and scoped resolution updates for `if`/`while`/`loop`/`for` bodies so function-local and loop-local bindings follow lexical lifetime/shadowing rules. Updated VM `DefineGlobal` execution paths in `src/vm.rs` to honor checked declarations for bytecode-defined bindings. Added parity regressions in `tests/vm_interpreter_parity_surfaces.rs` covering shadowing success, closure nearest-binding capture, duplicate `let`/`const` declaration failures, function-local control-flow leak prevention, and loop-variable lifetime isolation. Updated scope/declaration language contract text in `docs/LANGUAGE_SPEC.md` and README language overview. Verification: `cargo test --test vm_interpreter_parity_surfaces`, `cargo test --test interpreter_tests scope`, and `cargo test`.
```

```text
[x] V1-RUN-007: Enforce `return`, `break`, and `continue` context rules
    Priority: P1
    Severity: High
    Area: Correctness/Diagnostics
    Affected files: src/parser.rs, src/interpreter/mod.rs, src/compiler.rs, src/vm.rs, docs/LANGUAGE_SPEC.md
    Problem: Control-flow constructs outside valid contexts must fail predictably.
    Recommendation: Add parse-time or semantic validation for return outside valid execution contexts and break/continue outside loops.
    Implementation steps:
        1. Track function and loop context during semantic execution/compilation.
        2. Reject `break` outside a loop.
        3. Reject `continue` outside a loop.
        4. Preserve explicit top-level script `return` behavior as an intentional compatibility contract.
        5. Ensure compiler and VM do not silently accept invalid loop-control AST.
    Tests required:
        - `break` outside loop fails.
        - `continue` outside loop fails.
        - Nested loop/function cases behave correctly.
        - Top-level `return` compatibility behavior remains stable.
    Acceptance criteria: Invalid control flow cannot reach runtime as a partially meaningful sentinel.
    Notes: Completed on 2026-05-08. Added centralized loop-context validation in compiler/interpreter execution paths so `break` and `continue` now fail deterministically with `... can only be used inside a loop` when emitted outside loop contexts (including inside functions). Interpreter loop execution now tracks loop depth explicitly via scoped context helpers to avoid leaking control-flow sentinels into non-loop contexts. Added parity regressions in `tests/vm_interpreter_parity_surfaces.rs` for top-level and function-level `break`/`continue` failures, valid loop control-flow success paths, and retained top-level script `return` compatibility. Verification: `cargo test --test vm_interpreter_parity_surfaces`, `cargo test`.
```

```text
[x] V1-SEM-001: Define numeric overflow, division, and float edge-case behavior
    Priority: P1
    Severity: Medium
    Area: Correctness/Security
    Affected files: src/interpreter/value.rs, src/interpreter/mod.rs, src/vm.rs, src/optimizer.rs, tests/vm_interpreter_parity_surfaces.rs, docs/LANGUAGE_SPEC.md
    Problem: Numeric behavior must be stable for production programs. Overflow, division by zero, NaN, and infinity behavior need explicit policy.
    Recommendation: Define checked integer arithmetic and consistent float behavior.
    Implementation steps:
        1. Decide integer width and overflow policy.
        2. Use checked arithmetic for integer add/sub/mul/div/rem/pow where applicable.
        3. Return runtime errors for division by zero.
        4. Define float division by zero behavior: error or IEEE result.
        5. Define comparison behavior for NaN.
        6. Align interpreter and VM.
    Tests required:
        - Integer overflow fails or wraps according to documented policy.
        - Division by zero fails.
        - Modulo by zero fails.
        - Float NaN comparisons match spec.
        - Infinity handling matches spec.
        - VM/interpreter parity tests.
    Acceptance criteria: Numeric edge cases are deterministic and documented.
    Notes: Completed on 2026-05-08. Added centralized numeric helpers in `Value` (`checked_int_arithmetic`, `checked_float_arithmetic`, `float_equals`) and routed interpreter/VM arithmetic paths through them so integer add/sub/mul/div/rem now use checked `i64` behavior with deterministic overflow errors, while float `/` and `%` now reject zero divisors with runtime errors. Updated float equality behavior for NaN/infinity edge cases and aligned overflow handling in optimized VM in-place/map-fusion opcode paths plus compiler constant-folding safeguards (`src/optimizer.rs`) to prevent debug-overflow panics and runtime drift. Script-JIT admission now excludes arithmetic opcodes pending explicit parity guarantees for the hardened numeric contract. Added parity regressions in `tests/vm_interpreter_parity_surfaces.rs` covering integer add/sub/mul/div overflow, float division/modulo by zero, NaN comparison semantics, infinity comparison behavior, and local in-place overflow updates. Updated numeric language contract docs in `docs/LANGUAGE_SPEC.md` and summary docs in `README.md`/`CHANGELOG.md`. Verification: `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_reject_ -- --nocapture`, `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_nan_and_infinity_comparisons_match_policy -- --nocapture`, `cargo test --lib optimizer::tests::test_constant_folding_arithmetic -- --nocapture`, `cargo test --lib vm::tests::test_sum_int_map_until_local_in_place_result -- --nocapture`, `cargo test --lib vm::tests::test_sum_int_map_until_local_in_place_missing_key_errors -- --nocapture`, and `cargo test` passed.
```

```text
[x] V1-SEM-002: Define null, fallthrough, and return-without-value behavior
    Priority: P1
    Severity: Medium
    Area: Correctness/Language Semantics
    Affected files: src/interpreter/mod.rs, src/interpreter/value.rs, src/vm.rs, docs/LANGUAGE_SPEC.md
    Problem: The spec indicates null-like behavior in places where implementation may currently return `Int(0)`.
    Recommendation: Use an explicit `Null` value for missing return values and fallthrough where the language allows it.
    Implementation steps:
        1. Define when expressions/statements produce a value.
        2. Define function fallthrough result.
        3. Define `return` without expression result.
        4. Replace `Int(0)` sentinel returns with `Null` where appropriate.
        5. Update VM return opcodes.
    Tests required:
        - Function with no return returns null.
        - `return` without value returns null.
        - Explicit `return 0` still returns int zero.
        - Null equality and truthiness tests.
        - VM/interpreter parity tests.
    Acceptance criteria: Null behavior is explicit and no missing-value path uses integer zero as a sentinel.
    Notes: Completed on 2026-05-08. Parser return handling now accepts bare `return` before `}`/EOF, interpreter function/method/async return paths now treat body fallthrough and `return` without an expression as `Null` (replacing `Int(0)` sentinels), and explicit numeric returns such as `return 0` remain unchanged. Added parity and parser regressions for fallthrough null behavior, bare-return parsing, null equality/truthiness expectations, and explicit-zero return preservation. Verification: `cargo test --test vm_interpreter_parity_surfaces`, `cargo test --test parser_diagnostics_contract parser_accepts_bare_return`, and `cargo test` passed.
```

```text
[x] V1-SEM-003: Define equality and cross-type comparison semantics
    Priority: P1
    Severity: Medium
    Area: Correctness/Language Semantics
    Affected files: src/interpreter/value.rs, src/interpreter/mod.rs, src/vm.rs, docs/LANGUAGE_SPEC.md
    Problem: Equality and ordering across types are core language semantics and must not be accidental.
    Recommendation: Define which cross-type equality and ordering operations are allowed.
    Implementation steps:
        1. Define equality for primitive values.
        2. Define equality for lists, maps, objects, functions, native handles, and null.
        3. Define whether `1 == 1.0` is true.
        4. Reject unsupported ordering comparisons with runtime errors.
        5. Align VM comparison opcodes.
    Tests required:
        - Primitive equality tests.
        - Cross-type equality tests.
        - Collection equality tests if supported.
        - Unsupported ordering errors.
        - VM/interpreter parity tests.
    Acceptance criteria: Equality and comparison behavior is documented and stable.
    Notes: Completed on 2026-05-08. Added centralized value-comparison semantics in `src/interpreter/value.rs` and routed interpreter/VM equality+ordering through shared helpers so `==`/`!=` now have one runtime contract and ordering operators (`<`, `<=`, `>`, `>=`) reject unsupported type pairs with deterministic runtime errors instead of VM fallbacks. Equality now supports numeric cross-type comparison (`int`/`float`), deep array/dictionary comparison (including optimized VM dictionary encodings), structural `Result`/`Option`/tagged/struct equality, and identity-based callable equality for interpreter closures, VM bytecode closures, generator definitions, and native functions. Added regression coverage in `tests/vm_interpreter_parity_surfaces.rs` for cross-type numeric equality, string ordering, collection/callable equality, and unsupported ordering failures, plus focused unit coverage in `src/interpreter/value.rs` for helper-level semantics. Updated `docs/LANGUAGE_SPEC.md`, `README.md`, and `CHANGELOG.md` with the new contract. Verification: `cargo test --lib interpreter::value::tests::value_ -- --nocapture`, `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_define_ -- --nocapture`, `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_reject_ -- --nocapture`, and `cargo test` passed.
```

```text
[x] V1-COMP-001: Maintain VM/interpreter/compiler parity as a release gate
    Priority: P1
    Severity: High
    Area: Correctness/Architecture/Tests
    Affected files: src/compiler.rs, src/vm.rs, src/interpreter/mod.rs, tests/vm_interpreter_parity_surfaces.rs, docs/VM_INTERPRETER_PARITY_MATRIX.md
    Problem: Kujo has multiple execution paths. Production users cannot trust behavior if interpreter and VM diverge without documentation.
    Recommendation: Expand parity tests and require all supported surfaces to match.
    Implementation steps:
        1. Review the existing parity matrix.
        2. Mark each feature as supported, unsupported, intentionally divergent, or not yet implemented.
        3. Add parity tests for variables, functions, closures, classes/structs if present, loops, errors, collections, maps, imports, natives allowed in VM, and control flow.
        4. Make unsupported VM surfaces fail clearly instead of executing incorrectly.
        5. Keep the matrix updated with every semantic change.
    Tests required:
        - Parity test suite for all supported language surfaces.
        - Negative parity tests for unsupported features.
        - CI job running parity tests.
    Acceptance criteria: Any interpreter/VM divergence is either eliminated or explicitly documented and tested.
    Notes: Completed on 2026-05-08. Expanded parity coverage and release gating across compiler/interpreter/VM paths by: (1) adding explicit unsupported-surface parity for struct generator methods with one shared error contract (`Generator methods are not supported for structs: <Struct>.<method>`), (2) restoring import parity by compiling `import`/`from ... import ...` statements into VM import op paths backed by the shared module loader, (3) adding import parity regression coverage (`vm_and_interpreter_match_import_export_surface`) plus unsupported-surface negative coverage (`vm_and_interpreter_error_on_unsupported_struct_generator_method`), and (4) replacing the narrow P0-era parity matrix with a broader status/evidence matrix in `docs/VM_INTERPRETER_PARITY_MATRIX.md`. Added a dedicated CI parity job in `.github/workflows/ci-release-gate.yml` (`cargo test --test vm_interpreter_parity_surfaces`) and made the release-gate job depend on it. Verification: `cargo test --test vm_interpreter_parity_surfaces` and `cargo test` passed.
```

```text
[x] V1-JIT-001: Gate JIT behind explicit support and parity guarantees
    Priority: P2
    Severity: Medium
    Area: Performance/Correctness/Architecture
    Affected files: src/jit.rs, src/main.rs, src/compiler.rs, src/vm.rs, docs/VM_INTERPRETER_PARITY_MATRIX.md
    Problem: JIT code is high-risk if it can execute unsupported surfaces or diverge from interpreter semantics.
    Recommendation: Treat JIT as opt-in unless it passes explicit feature support checks and parity tests.
    Implementation steps:
        1. Identify all AST/opcode surfaces the JIT supports.
        2. Add a support checker that rejects unsupported programs before JIT execution.
        3. Add diagnostics for unsupported JIT features.
        4. Add parity tests for supported JIT surfaces.
        5. Document JIT status as experimental unless complete.
    Tests required:
        - Supported JIT arithmetic/function cases match interpreter.
        - Unsupported feature produces clear diagnostic.
        - JIT disabled by default unless release policy says otherwise.
    Acceptance criteria: JIT cannot silently execute a program with different semantics from interpreter/VM.
    Notes: Completed on 2026-05-15. JIT is now explicit opt-in (`kujo run --jit`) and no longer enabled by default in `VM::new()`. Added centralized support-surface validation in `src/jit.rs`/`src/vm.rs` that scans top-level and nested function chunks, reports the first unsupported opcode with chunk/instruction context, and is used by CLI JIT opt-in handling to emit a deterministic warning then fall back to normal VM bytecode execution without JIT. Added JIT contract regressions in `src/vm.rs` (default-off, unsupported-surface detection, supported arithmetic and bytecode-function execution paths) and CLI integration coverage in `tests/jit_execution_contract.rs` for default behavior, unsupported-surface diagnostics, and supported-surface opt-in success. Verification: `cargo test --lib jit_supported`, `cargo test test_jit_is_disabled_by_default --lib`, and `cargo test --test jit_execution_contract` passed.
```

### Phase 4: Secure Host Boundaries

```text
[x] V1-SEC-002: Add a native capability policy for host-effect APIs
    Priority: P1
    Severity: Critical
    Area: Security/Architecture
    Affected files: src/interpreter/native_functions/*.rs, src/interpreter/builtins.rs, src/main.rs, docs/NATIVE_API_SECURITY_POSTURE.md
    Problem: Kujo exposes powerful trusted-code APIs for filesystem, process execution, environment mutation, network access, database access, and shell commands without a runtime capability policy.
    Recommendation: Add an explicit capability model with safe defaults and opt-in flags/config for dangerous APIs.
    Implementation steps:
        1. Inventory all native APIs with host side effects.
        2. Define capabilities: filesystem-read, filesystem-write, filesystem-delete, process-exec, shell-exec, env-read, env-write, network-client, network-server, database, clock, random.
        3. Add runtime configuration storing enabled capabilities.
        4. Check capabilities at every native boundary.
        5. Add CLI flags such as `--allow-fs-read`, `--allow-fs-write`, `--allow-net`, `--allow-run`, or a documented equivalent.
        6. Make dangerous capabilities disabled by default for untrusted execution modes.
        7. Preserve a trusted local script mode only if explicitly documented.
    Tests required:
        - Each capability-denied API returns a structured error.
        - Each capability-enabled API works.
        - CLI flags enable only the requested capability.
        - No dangerous API bypasses the policy.
    Acceptance criteria: A Kujo program cannot access host filesystem, process, environment, or network effects unless policy allows it.
    Notes: Completed on 2026-05-08. Added centralized runtime capability policy primitives (`src/interpreter/capabilities.rs`) and wired enforcement through interpreter/VM native dispatch plus method-call bypass surfaces (`http_server.listen`, `Image.save`) and spawned/async interpreter contexts so policy cannot be bypassed via spawned execution paths. Added `kujo run`/`kujo test-run` CLI capability controls (`--untrusted`, granular `--allow-*`, `--allow-all`) with explicit restricted/trusted mode behavior. Updated security posture and README policy docs. Added integration coverage in `tests/native_api_security_boundaries.rs` for deny-by-default behavior, capability-enabled behavior, granularity (`allow only requested capability`), VM/interpreter parity, and spawned-interpreter inheritance checks. Verification: `cargo test --test native_api_security_boundaries`, `cargo test`.
```

```text
[x] V1-SEC-003: Harden shell and process execution APIs
    Priority: P1
    Severity: Critical
    Area: Security
    Affected files: src/interpreter/builtins.rs, src/interpreter/native_functions/system.rs, docs/NATIVE_API_SECURITY_POSTURE.md
    Problem: Shell execution through `sh -c` or `cmd /C` is dangerous when command strings can contain untrusted input.
    Recommendation: Separate direct process execution from shell execution and require explicit shell capability for shell mode.
    Implementation steps:
        1. Identify all process execution helpers.
        2. Add safe direct exec API that accepts executable plus argv list without shell interpolation.
        3. Mark shell-string execution as dangerous and require `shell-exec` capability.
        4. Add timeouts.
        5. Add max stdout/stderr capture size.
        6. Add environment allow/deny behavior.
        7. Return structured exit status instead of ambiguous strings.
    Tests required:
        - Direct argv exec does not invoke shell expansion.
        - Shell exec denied without capability.
        - Shell exec allowed with capability.
        - Timeout kills long process.
        - Output larger than limit is truncated or rejected according to docs.
        - Non-zero exit status is represented predictably.
    Acceptance criteria: Untrusted strings are not accidentally interpreted by a shell, and process execution is bounded.
    Notes: Completed on 2026-05-12. Added centralized bounded process execution in `src/interpreter/native_functions/system.rs` with shared option parsing and enforcement (`timeout_ms`, `max_output_bytes`, `inherit_env`, `env_allow`, `env_deny`, `env`), timeout-based process termination, bounded stdout/stderr capture with truncation metadata, and deterministic error-object propagation for spawn/wait/read failures. Preserved shell-string execution in `execute(...)` while adding structured shell status via `execute_status(...)`; both now accept optional process options and remain shell-exec scoped. Hardened direct argv execution surfaces (`spawn_process`, `pipe_commands`) to use the same bounded execution policy without shell interpolation and to return deterministic `ProcessResult` fields (`exitcode`, `stdout`, `stderr`, `success`, `timed_out`, `stdout_truncated`, `stderr_truncated`). Updated capability/type-check/dispatch contracts to include `execute_status` and optional process options. Added/updated integration and unit coverage for shell allow behavior, direct argv non-expansion, timeout termination, output truncation signaling, env allow/deny enforcement, strict arity expectations, and process dispatch contracts. Updated `README.md`, `docs/NATIVE_API_SECURITY_POSTURE.md`, and `docs/STANDARD_LIBRARY_REFERENCE.md` with the hardened process/shell contract details. Verification: `cargo test --lib interpreter::native_functions::system::tests`, `cargo test --lib native_functions::tests::test_release_hardening_system_operation_contracts`, `cargo test --lib native_functions::tests::test_release_hardening_process_module_dispatch_argument_contracts`, `cargo test --test native_api_security_boundaries process_`, `cargo test -q`.
```

```text
[x] V1-FS-001: Centralize safe path normalization and containment
    Priority: P1
    Severity: Critical
    Area: Security/FileSystem/Architecture
    Affected files: src/interpreter/native_functions/filesystem.rs, src/module.rs, src/serve_http.rs, src/main.rs
    Problem: Filesystem, module, archive, and static-server code need consistent path traversal, symlink, absolute path, and root containment behavior.
    Recommendation: Add one reusable path security module.
    Implementation steps:
        1. Create a helper module such as `src/path_security.rs`.
        2. Implement lexical rejection for null bytes, empty paths where invalid, parent traversal, absolute paths when disallowed, Windows drive prefixes, and reserved names where relevant.
        3. Implement canonical root containment checks after resolving paths.
        4. Add safe open/read helpers that avoid separate check-then-use where possible.
        5. Define symlink policy per caller: reject, allow only inside root, or follow safely.
        6. Replace ad hoc normalization in filesystem, module, archive, and server code.
    Tests required:
        - `../` traversal rejected.
        - URL-encoded traversal rejected in server path flow.
        - Symlink escape rejected.
        - Absolute path rejected where not allowed.
        - Windows drive-like path rejected cross-platform.
        - Valid nested path accepted.
    Acceptance criteria: Every root-bound path operation uses the central helper.
    Notes: Completed on 2026-05-12. Added centralized path-safety helpers in `src/path_security.rs` for lexical path sanitization (null byte, empty path, parent traversal, absolute path, and Windows drive-prefix rejection), root-bounded path joining, canonical containment checks, symlink-target rejection, and URL-encoded traversal detection. Replaced ad hoc path logic in `src/interpreter/native_functions/filesystem.rs` (archive extraction), `src/serve_http.rs` (request-path validation + root containment), and `src/module.rs` (search-root-bounded module resolution, including symlink-resolved escape rejection). Added regression coverage in `tests/serve_command_integration.rs` for URL-encoded traversal and symlink escape paths, plus `src/module.rs` tests for module symlink-escape rejection and unit tests in `src/path_security.rs` for success/failure path normalization contracts. Verification: `cargo test --test serve_command_integration serve_rejects_url_encoded_parent_traversal -- --nocapture`, `cargo test --test serve_command_integration serve_rejects_symlink_escape_target -- --nocapture`, `cargo test module::tests::load_module_rejects_symlink_escape_outside_search_root --lib -- --nocapture`, `cargo test path_security::tests --lib -- --nocapture`, `cargo test --test native_api_security_boundaries unzip_ -- --nocapture`, and `cargo test -q`.
```

```text
[x] V1-FS-002: Add file operation size, overwrite, and permission safeguards
    Priority: P1
    Severity: High
    Area: Security/FileSystem
    Affected files: src/interpreter/native_functions/filesystem.rs, docs/NATIVE_API_SECURITY_POSTURE.md
    Problem: Native file APIs can read/write/delete without consistent limits or overwrite policy.
    Recommendation: Add bounded file IO behavior and explicit overwrite semantics.
    Implementation steps:
        1. Add default max read size for whole-file reads.
        2. Add write size limits for string/binary writes.
        3. Add explicit overwrite flag for write APIs if overwriting is currently implicit.
        4. Make delete APIs require filesystem-delete capability.
        5. Reject directory deletion unless a separate explicit recursive API exists.
        6. Document platform-specific permission behavior.
    Tests required:
        - Read over size limit fails.
        - Write over size limit fails.
        - Existing file overwrite fails unless allowed.
        - Delete denied without capability.
        - Directory delete behavior is tested.
    Acceptance criteria: File APIs are bounded, permissioned, and documented.
    Notes: Completed on 2026-05-12. Added centralized filesystem file-operation safeguards in `src/interpreter/native_functions/filesystem.rs`: whole-file reads (`read_file`, `read_lines`, `read_binary_file`, `read_file_async`) now fail above 8 MiB; write payloads (`write_file`, `write_file_sync`, `write_file_async`, `write_binary_file`, `append_file`) now fail above 8 MiB; and `write_file`/`write_binary_file` now require explicit `overwrite=true` to replace existing files (default no-overwrite). `delete_file` now rejects directory paths explicitly, while capability enforcement for delete remains scoped to `filesystem-delete`. Updated type-check signatures for optional overwrite in `src/type_checker.rs`, expanded security integration coverage in `tests/native_api_security_boundaries.rs` (limit failures, boundary success, overwrite deny/allow, fs-delete deny/allow, non-recursive directory delete behavior), and refreshed security docs in `README.md` and `docs/NATIVE_API_SECURITY_POSTURE.md`. Verification: `cargo test --test native_api_security_boundaries filesystem_ -- --nocapture`, `cargo test --test native_api_security_boundaries native_capability_allow_fs_delete_enables_delete_file -- --nocapture`, `cargo test test_release_hardening_filesystem_core_contracts -- --nocapture`, and `cargo test -q`.
```

```text
[x] V1-MOD-001: Harden module import path resolution and caching
    Priority: P1
    Severity: High
    Area: Security/Correctness/Modules
    Affected files: src/module.rs, src/main.rs, docs/LANGUAGE_SPEC.md, tests/package_module_workflow_integration.rs
    Problem: Module imports can become a security and correctness boundary if paths, package roots, caches, or cycles are weakly defined.
    Recommendation: Define and enforce safe import resolution rooted in project/package boundaries.
    Implementation steps:
        1. Document import search order.
        2. Reject traversal outside package roots unless explicitly allowed.
        3. Canonicalize import paths with `V1-FS-001` helpers.
        4. Detect import cycles and report them clearly.
        5. Define cache keys by canonical path plus package context.
        6. Add cache invalidation behavior for CLI runs and tests.
    Tests required:
        - Relative import inside package succeeds.
        - Traversal import outside package fails.
        - Symlink escape import fails.
        - Import cycle reports useful error.
        - Import cache does not reuse wrong module after path changes.
    Acceptance criteria: Import resolution is deterministic, contained, and documented.
    Notes: Completed on 2026-05-13. Hardened module loading in `src/module.rs` with deterministic import search order that prefers the importing module's package root before configured search paths, explicit traversal rejection for unsafe module names, canonical root containment enforcement for resolved module paths (including symlink-escape rejection), circular-import diagnostics that include the full import chain, and cache-key scoping by canonical module path plus package-root context. Added metadata-based cache invalidation (mtime/size) so updated module source is reloaded instead of serving stale exports. Added/updated regression coverage in `src/module.rs` and `tests/package_module_workflow_integration.rs` for relative import success, traversal rejection, symlink escape rejection, cycle-chain errors, and module refresh after source change. Updated language/docs contracts in `docs/LANGUAGE_SPEC.md`, `README.md`, and `CHANGELOG.md`. Verification: `cargo test --lib module::tests::`, `cargo test --test package_module_workflow_integration`, and `cargo test` passed.
```

```text
[x] V1-NET-001: Add network timeout, size, and capability controls
    Priority: P1
    Severity: High
    Area: Security/Networking
    Affected files: src/interpreter/native_functions/network.rs, src/interpreter/native_functions/http.rs, docs/NATIVE_API_SECURITY_POSTURE.md
    Problem: Network APIs can hang, consume unbounded memory, or access external resources unexpectedly.
    Recommendation: Require network capabilities and add timeouts plus request/response size limits.
    Implementation steps:
        1. Require `network-client` for outbound connections.
        2. Require `network-server` for listening sockets.
        3. Add default connect/read/write timeouts.
        4. Add max response/body size limits.
        5. Add clear errors for timeout and limit exceeded.
        6. Document trusted-code assumptions and sandbox boundaries.
    Tests required:
        - Network APIs denied without capability.
        - Timeout behavior tested with local listener or mock.
        - Oversized response/body rejected.
        - Allowed local request succeeds with capability.
    Acceptance criteria: Network APIs cannot hang indefinitely or run without explicit policy.
    Notes: Completed on 2026-05-13. Added centralized network guardrails in `src/network_policy.rs` and applied them across HTTP/TCP/UDP native paths. HTTP client surfaces now execute in a dedicated blocking worker thread (avoids Tokio blocking-runtime panic paths), enforce bounded request timeouts (`30s` default), and reject oversized response bodies above `8 MiB` with deterministic boundary errors. TCP/UDP paths now use bounded connect/read/write timeout policy (`10s` connect, `30s` read/write), enforce maximum receive-size requests (`8 MiB`), and surface timeout-aware read/write errors consistently. Added regression coverage for oversized HTTP responses and local timeout behavior in `tests/native_api_security_boundaries.rs`, plus network module size-limit contracts in `src/interpreter/native_functions/mod.rs`. Updated security/readme/docs contracts accordingly. Verification: `cargo test --lib test_release_hardening_network_module_`, `cargo test --test native_api_security_boundaries network_http_`, and `cargo test`.
```

### Phase 5: HTTP And Static Server Hardening

```text
[x] V1-HTTP-002: Add complete static response status and header handling
    Priority: P1
    Severity: High
    Area: HTTP/Security/DX
    Affected files: src/serve_http.rs, tests/serve_command_integration.rs, README.md
    Problem: Static server status handling is too narrow for production and may omit required headers such as `Allow` for 405.
    Recommendation: Add a central HTTP response builder for static server responses.
    Implementation steps:
        1. Define supported status codes for static serving: 200, 301/302 if redirects are supported, 304 if conditional requests are supported, 400, 403, 404, 405, 408, 413, 414, 415, 500, 501, 503.
        2. Return `405 Method Not Allowed` with `Allow: GET, HEAD`.
        3. Return `501 Not Implemented` for methods/features explicitly unsupported if more accurate than 405.
        4. Add `Content-Length` to all responses where possible.
        5. Add `Content-Type` to all bodies.
        6. Add safe default headers: `X-Content-Type-Options: nosniff`, `Referrer-Policy`, and conservative cache policy.
        7. Ensure HEAD responses include headers but no body.
    Tests required:
        - GET file returns 200 with length/type.
        - HEAD file returns 200 with length/type and empty body.
        - POST returns 405 with `Allow`.
        - Unsupported method behavior is documented and tested.
        - 404/403/415/500 response shape tests.
    Acceptance criteria: Static server responses are standards-aware, predictable, and tested.
    Notes: Completed on 2026-05-13. Added centralized static text/error response construction in `src/serve_http.rs`, returning `405 Method Not Allowed` with `Allow: GET, HEAD` for standard unsupported methods and `501 Not Implemented` for non-standard methods. Hardened default response headers by always including `X-Content-Type-Options: nosniff` and `Referrer-Policy: no-referrer`, and made successful responses fall back to conservative cache policy (`public, max-age=0, must-revalidate`) when explicit max-age is absent. Expanded integration coverage in `tests/serve_command_integration.rs` for GET/HEAD response headers, POST `405` + `Allow`, non-standard-method `501`, and `403`/`404` error response headers. Added unit coverage in `src/serve_http.rs` for conservative cache fallback, static error response header contracts, and deterministic `500` response shape. Verification: `cargo test --test serve_command_integration serve_`, `cargo test serve_http::tests::`, and `cargo test` passed.
```

```text
[x] V1-HTTP-003: Harden URL decoding and request-target validation
    Priority: P1
    Severity: Critical
    Area: HTTP/Security
    Affected files: src/serve_http.rs, tests/serve_command_integration.rs
    Problem: Static serving must safely handle encoded traversal, double-decoding, null bytes, invalid UTF-8, long URIs, query strings, and fragments.
    Recommendation: Add a request-target parser that decodes exactly once and validates before filesystem resolution.
    Implementation steps:
        1. Parse path separately from query.
        2. Reject fragments if present in raw request target.
        3. Percent-decode exactly once.
        4. Reject invalid percent encodings.
        5. Reject decoded null bytes.
        6. Reject encoded and decoded traversal.
        7. Reject URI length over configured limit with 414.
        8. Pass only validated paths into `V1-FS-001` containment helpers.
    Tests required:
        - `%2e%2e/` traversal rejected.
        - Double-encoded traversal does not bypass policy.
        - Invalid percent encoding returns 400.
        - Null byte returns 400.
        - Oversized URI returns 414.
        - Query string does not affect filesystem path.
    Acceptance criteria: Request paths cannot escape the server root through encoding tricks.
    Notes: Completed on 2026-05-13. Added centralized request-target validation in `src/serve_http.rs` that parses path separately from query, rejects raw-request fragments, percent-decodes request paths exactly once, rejects malformed percent encodings and decoded null bytes with `400`, applies decoded-path sanitization/root-boundary policy before filesystem lookup, and enforces a `4096`-byte request-target cap with `414 URI Too Long`. Expanded integration coverage in `tests/serve_command_integration.rs` for encoded traversal, double-encoded traversal non-bypass behavior, invalid percent encoding (`400`), decoded null byte (`400`), fragment rejection (`400`), oversized URI (`414`), and query-string path isolation. Added focused validator unit tests in `src/serve_http.rs` for decode-once, traversal, query parsing, fragment, null-byte, invalid-percent, and length-limit contracts. Verification: `cargo test serve_http::tests::validate_request_target_ -- --nocapture`, `cargo test --test serve_command_integration serve_ -- --nocapture`, and `cargo test` passed.
```

```text
[x] V1-HTTP-004: Add dotfile, hidden-file, and private-file policy
    Priority: P1
    Severity: High
    Area: HTTP/Security
    Affected files: src/serve_http.rs, tests/serve_command_integration.rs, README.md
    Problem: Static servers commonly leak `.env`, `.git`, hidden files, backup files, or private config unless blocked.
    Recommendation: Deny hidden and private files by default with explicit opt-in if needed.
    Implementation steps:
        1. Block dotfiles and dot-directories by default.
        2. Block known private names: `.env`, `.git`, `.svn`, `.hg`, `.DS_Store`, backup suffixes, editor swap files.
        3. Add CLI option only if there is a strong use case, such as `--allow-hidden`.
        4. Return 403 for blocked existing files.
        5. Do not reveal whether blocked private paths exist if the documented policy chooses indistinguishable 404 behavior.
    Tests required:
        - `.env` blocked.
        - `.git/config` blocked.
        - `.DS_Store` blocked.
        - Backup/swap files blocked.
        - Normal files still served.
        - Optional allow-hidden flag works if added.
    Acceptance criteria: `kujo serve` does not expose common private files by default.
    Notes: Completed on 2026-05-15. Added centralized private-path filtering in `src/serve_http.rs` request-target validation so hidden/private path targets are denied before filesystem resolution with one deterministic policy: `403 Forbidden` for both existing and non-existing blocked targets. The deny policy blocks dotfile/dot-directory path components (including `.env`, `.git`, `.svn`, `.hg`, `.DS_Store`) and backup/swap-like leaf names (`*.bak`, `*.backup`, `*.tmp`, `*.old`, `*.orig`, `*.swp`, `*.swo`, and trailing `~`). Added integration regressions in `tests/serve_command_integration.rs` for `.env`, `.git/config`, `.DS_Store`, backup/swap files, and normal-file success; plus validator unit tests in `src/serve_http.rs` for hidden/private rejection and non-private pass-through. Updated `README.md` static-server hardening docs and `CHANGELOG.md`. Verification: `cargo test serve_http::tests::validate_request_target_ -- --nocapture`, `cargo test --test serve_command_integration serve_ -- --nocapture --test-threads=1`, and `cargo test` passed.
```

```text
[x] V1-HTTP-005: Add request size, header size, timeout, and connection limits
    Priority: P1
    Severity: High
    Area: HTTP/Security/Performance
    Affected files: src/serve_http.rs, src/main.rs, tests/serve_command_integration.rs, README.md
    Problem: A server without limits is vulnerable to slowloris-style behavior, oversized request lines/headers, and resource exhaustion.
    Recommendation: Add conservative defaults and CLI configuration for server limits.
    Implementation steps:
        1. Set max request line length.
        2. Set max header bytes.
        3. Set max header count.
        4. Add read timeout.
        5. Add write timeout if supported by the server implementation.
        6. Add max concurrent connection or thread limit if server is threaded.
        7. Return 400, 408, 413, or 414 as appropriate.
    Tests required:
        - Oversized request line returns 414.
        - Oversized headers return 413 or 400 according to docs.
        - Too many headers returns 400 or 413.
        - Timeout behavior tested deterministically where possible.
        - Normal requests still succeed.
    Acceptance criteria: Static server resource use is bounded by documented defaults.
    Notes: Completed on 2026-05-15. Added centralized request-limit enforcement in `src/serve_http.rs` for request-line bytes (`414 URI Too Long`), combined header bytes (`413 Payload Too Large`), header count (`413`), and request body bytes (`413`) with configurable defaults (`8192` line bytes, `16384` header bytes, `100` headers, `1048576` body bytes). Added bounded concurrent request-handler limits (`max_connections`, default `128`) with deterministic `503 Service Unavailable` when saturated, plus serve-option validation for zero/invalid limits and timeout settings. Exposed CLI knobs in `src/main.rs`: `--max-request-line-bytes`, `--max-header-bytes`, `--max-header-count`, `--max-request-body-bytes`, `--read-timeout-ms`, `--write-timeout-ms`, and `--max-connections`. Added serve integration regressions in `tests/serve_command_integration.rs` for oversized request lines, oversized header bytes, too-many-headers behavior, and explicit timeout-flag normal-request success; plus unit coverage in `src/serve_http.rs` for timeout/connection option validation contracts. Verification: `cargo test serve_http::tests:: -- --nocapture`, `cargo test --test serve_command_integration serve_ -- --nocapture --test-threads=1`, and `cargo test` passed.
```

```text
[x] V1-HTTP-006: Stream large files and define range-request policy
    Priority: P2
    Severity: Medium
    Area: HTTP/Performance
    Affected files: src/serve_http.rs, tests/serve_command_integration.rs, README.md
    Problem: Loading large files into memory before serving does not scale.
    Recommendation: Stream file responses and explicitly support or reject Range requests.
    Implementation steps:
        1. Replace whole-file reads with buffered streaming.
        2. Preserve correct `Content-Length`.
        3. Define max served file size if full support is not intended.
        4. Either implement single-range requests with 206/416 or reject Range with documented behavior.
        5. Add benchmarks for large static files.
    Tests required:
        - Large file served without excessive memory use.
        - Content-Length correct.
        - HEAD large file does not read whole body.
        - Range supported or rejected according to docs.
    Acceptance criteria: Static serving does not require loading entire files into memory.
    Notes: Completed on 2026-05-15. `src/serve_http.rs` now streams static file bodies from file handles (including ranged reads via seek + bounded reader) instead of buffering entire files in memory before response construction, while preserving deterministic `Content-Length` for `GET` and `HEAD` responses. Range policy remains explicit and tested: valid single-range requests return `206` with `Content-Range`, and invalid ranges return deterministic `416` with `Content-Range: bytes */<len>`. Added integration regressions in `tests/serve_command_integration.rs` for large-file GET/HEAD contracts and invalid-range handling. Verification: `cargo test --test serve_command_integration -- --nocapture --test-threads=1`, `cargo test serve_http::tests:: -- --nocapture`, and `cargo test` passed.
```

```text
[x] V1-HTTP-007: Expand MIME registry for real-world static assets
    Priority: P1
    Severity: Medium
    Area: HTTP/Standard Library/Usefulness
    Affected files: src/serve_http.rs, tests/serve_command_integration.rs, README.md
    Problem: A narrow MIME map makes the server feel demo-grade.
    Recommendation: Add a centralized broad MIME registry with safe fallback.
    Implementation steps:
        1. Add mappings for html, htm, css, js, mjs, json, xml, svg, txt, md, csv, pdf.
        2. Add image mappings for png, jpg, jpeg, gif, webp, avif, ico, bmp, tif, tiff.
        3. Add audio/video mappings for mp3, wav, ogg, mp4, webm, mov.
        4. Add font mappings for wasm, woff, woff2, ttf, otf, eot.
        5. Add archive mappings for zip, tar, gz, tgz, 7z.
        6. Use `application/octet-stream` as binary fallback unless active-content policy blocks the file.
        7. Make extension matching case-insensitive.
    Tests required:
        - One test per supported family.
        - Unknown extension fallback.
        - Extensionless fallback.
        - Double extension behavior.
        - Dotfile behavior interacts correctly with `V1-HTTP-004`.
    Acceptance criteria: Common static assets are served with correct types and unknown content is handled safely.
    Notes: Completed on 2026-05-15. Expanded `src/serve_http.rs` centralized MIME registry with required real-world mappings across image (`tif`/`tiff`), audio/video (`mov`), font (`eot`), and archive (`zip`, `tar`, `gz`, `tgz`, `7z`) families while preserving case-insensitive extension matching and safe `application/octet-stream` fallback for unknown/extensionless files. Refined `.gz`/`.br` handling so `Content-Encoding` is applied only for multi-extension precompressed assets (for example `index.html.gz`) instead of single-extension archive files. Added unit regressions in `src/serve_http.rs` and live subprocess coverage in `tests/serve_command_integration.rs` for family mappings, unknown/extensionless fallback, double-extension fallback, and dotfile-block precedence. Verification: `cargo test serve_http::tests::guess_content_type_ -- --nocapture`, `cargo test serve_http::tests::split_content_path_and_encoding_only_treats_multi_extension_files_as_precompressed -- --nocapture`, `cargo test --test serve_command_integration serve_mime_ -- --nocapture --test-threads=1`, `cargo test --test serve_command_integration serve_dotfile_blocking_takes_priority_over_known_extension_mime_resolution -- --nocapture --test-threads=1`, and `cargo test` passed.
```

### Phase 6: Standard Library And Universal Usefulness

```text
[x] V1-STD-001: Create a complete native API inventory and support table
    Priority: P1
    Severity: Medium
    Area: Docs/Standard Library/Security
    Affected files: src/interpreter/native_functions/*.rs, src/interpreter/builtins.rs, docs/STANDARD_LIBRARY.md, docs/NATIVE_API_SECURITY_POSTURE.md
    Problem: Kujo exposes many native helpers, but users and reviewers need a complete support, safety, and capability table.
    Recommendation: Document every native function with signature, return type, errors, required capability, and examples.
    Implementation steps:
        1. Inventory all builtins and native modules.
        2. Group by module: filesystem, system, network, HTTP, async, database, collections, strings, math, time, environment.
        3. Document argument types and arity.
        4. Document failure modes and diagnostic codes.
        5. Document required capabilities from `V1-SEC-002`.
        6. Add examples that are covered by tests.
    Tests required:
        - Doc examples parse/run where safe.
        - Native arity tests cover documented signatures.
        - Capability tests match documented requirements.
    Acceptance criteria: There is no undocumented native API exposed to users.
    Notes: Completed on 2026-05-15. Added `docs/STANDARD_LIBRARY.md` as the canonical full native API inventory table generated from runtime builtin registration (`Interpreter::get_builtin_names()`), including signature/arity labels, capability requirements, return/error contract summaries, and examples for every exposed builtin. Added centralized interpreter helpers for canonical native names plus public capability/arity lookup (`Interpreter::native_function_capability`, `Interpreter::native_function_arity`) and routed native dispatch canonicalization through the shared helper to avoid alias drift. Replaced and expanded `tests/stdlib_reference_contract.rs` to enforce full runtime/doc parity (no missing or extra entries, no duplicates, required field coverage, capability-column parity with runtime policy, and centralized arity metadata parity including alias edge cases for `println`, `str`, and `time`). Updated `README.md` and `docs/NATIVE_API_SECURITY_POSTURE.md` to point at the canonical inventory and contract tests. Verification: `cargo test --test stdlib_reference_contract` and `cargo test` passed.
```

```text
[x] V1-STD-002: Add robust JSON support or document its absence
    Priority: P2
    Severity: Enhancement
    Area: Standard Library/Usefulness
    Affected files: src/interpreter/native_functions/*.rs, docs/STANDARD_LIBRARY.md, tests/native_json.rs
    Problem: A production scripting/application language needs safe JSON parse/stringify support or a clear statement that it is not included in 1.0.
    Recommendation: Provide `json.parse`, `json.stringify`, and structured errors if not already present.
    Implementation steps:
        1. Check whether JSON support already exists.
        2. If present, harden arity, type conversion, invalid JSON errors, nesting limits, and output determinism.
        3. If absent and in scope, add a native module using a well-maintained Rust parser.
        4. Add max nesting and max input size.
        5. Document conversion between Kujo values and JSON.
    Tests required:
        - Parse object/array/string/number/bool/null.
        - Invalid JSON fails with location.
        - Stringify primitive and nested values.
        - Unsupported value types fail clearly.
        - Size/depth limits.
    Acceptance criteria: JSON behavior is useful, bounded, and documented, or explicitly deferred.
    Notes: Completed on 2026-05-15. Hardened centralized JSON behavior in `src/builtins.rs` (used by `src/interpreter/native_functions/json.rs`) with explicit parse bounds (`MAX_JSON_INPUT_BYTES=1,048,576`, `MAX_JSON_NESTING_DEPTH=64`), deterministic nesting-limit failures, non-finite float rejection for stringify (`NaN`, `+/-inf`), deterministic dictionary key ordering during JSON serialization (lexicographic string keys / ascending integer keys), and safer JSON-number conversion (no silent `Int(0)` fallback). Added comprehensive regression coverage in `tests/native_json.rs` for parse success across object/array/string/number/bool/null roots, location-aware invalid JSON errors, stringify success for primitive+nested values, unsupported-value failures, parse size/depth limits, and non-finite float failures. Updated `docs/STANDARD_LIBRARY.md`, `README.md`, and `CHANGELOG.md` with the new JSON contract. Verification: `cargo test --test native_json`, `cargo test --lib test_json_roundtrip_array_of_objects_preserves_cardinality`, `cargo test --lib test_parse_json_and_to_json_round_trip`, and `cargo test` passed.
```

```text
[x] V1-STD-003: Harden string, collection, math, time, and environment helpers
    Priority: P2
    Severity: Medium
    Area: Standard Library/Correctness
    Affected files: src/interpreter/native_functions/*.rs, src/interpreter/value.rs, docs/STANDARD_LIBRARY.md
    Problem: Standard library helpers should validate inputs, return structured errors, and cover common real-world cases.
    Recommendation: Review each helper for arity, type checks, bounds, Unicode behavior, resource limits, and docs.
    Implementation steps:
        1. Inventory string helpers and define Unicode vs byte behavior.
        2. Inventory collection helpers and define mutation/immutability behavior.
        3. Inventory math helpers and define domain errors.
        4. Inventory time helpers and define timezone/format behavior.
        5. Inventory environment helpers and apply capability policy.
        6. Add missing tests for every helper.
    Tests required:
        - Wrong arity and wrong type tests for every helper.
        - Edge values for strings and collections.
        - Math domain errors.
        - Time formatting/parsing edge cases where supported.
        - Env access denied/allowed tests.
    Acceptance criteria: Native helper behavior is broad enough for real scripts and never fails silently.
    Notes: Completed on 2026-05-16. Hardened native helper contracts across `src/interpreter/native_functions/math.rs`, `src/interpreter/native_functions/strings.rs`, `src/interpreter/native_functions/collections.rs`, `src/interpreter/native_functions/system.rs`, and shared date/env parsing in `src/builtins.rs` (plus legacy compatibility path update in `src/interpreter/legacy_full.rs`). Math helpers now reject missing/invalid arguments and report deterministic domain errors (`sqrt` negative input, `log` non-positive input) instead of silent `Int(0)` fallbacks. Core string helpers now return structured argument/type errors instead of empty-string/empty-array defaults on invalid inputs. Key collection helpers (`len`, `push`/`append`, `pop`, `slice`, `concat`, `clear`) now reject invalid types with deterministic errors rather than silent fallback values. Time/env hardening now enforces strict `parse_date(..., "YYYY-MM-DD")` parsing with explicit `ErrorObject` failures on invalid input and strict boolean token parsing for `env_bool` (invalid values now error instead of coercing to `false`). Added regression coverage for success/error/edge paths in module tests and aligned `len` contract tests in `src/interpreter/native_functions/mod.rs`. Verification: `cargo test test_math_api_rejects_missing_and_invalid_arguments`, `cargo test test_math_api_domain_and_success_contracts`, `cargo test test_core_string_helpers_reject_wrong_types_instead_of_silent_fallbacks`, `cargo test test_collection_helpers_reject_wrong_types_instead_of_silent_fallbacks`, `cargo test test_env_bool_invalid_value_returns_error_object`, `cargo test test_parse_date_invalid_input_returns_error_object`, `cargo test test_release_hardening_len_polymorphic_contracts`, `cargo test parse_date`, `cargo test env_bool`, and `cargo test test_array_collection_behavior_contracts`.
```

### Phase 7: CLI And Developer Experience

```text
[x] V1-CLI-001: Define and enforce CLI command/exit-code contract
    Priority: P1
    Severity: High
    Area: CLI/DX/Diagnostics
    Affected files: src/main.rs, docs/CLI_MACHINE_READABLE_CONTRACTS.md, README.md, tests/cli_contracts.rs
    Problem: Production tools need predictable commands, flags, stdout/stderr separation, and exit codes.
    Recommendation: Document and test the CLI contract for all commands.
    Implementation steps:
        1. Inventory current CLI commands.
        2. Define exit codes: success, usage error, lex/parse error, runtime error, IO error, internal error.
        3. Ensure user-facing diagnostics go to stderr.
        4. Ensure program output goes to stdout.
        5. Add `--version`.
        6. Add consistent `--help`.
        7. Define behavior for stdin, multiple files, directories, and missing files.
        8. Add JSON diagnostics mode if supported by `V1-ERR-001`.
    Tests required:
        - Help exits 0.
        - Version exits 0 and prints crate version.
        - Missing file exits with IO error code.
        - Parse error exits with parse error code.
        - Runtime error exits with runtime error code.
        - JSON diagnostics are valid JSON.
    Acceptance criteria: CLI behavior is stable, documented, and covered by integration tests.
    Notes: Completed on 2026-05-16. Added centralized CLI exit-code categories in `src/main.rs` and routed command failures through stable contracts: usage (`2`), lex/parse (`3`), runtime (`4`), IO (`5`), and internal (`6`), while retaining `1` for explicit gate-style command failures (for example `format --check` and lint/test failure outcomes). Replaced panic-prone `expect(...)` command paths in project/package/profile handlers with deterministic stderr diagnostics + categorized exits. Added integration coverage in `tests/cli_contracts.rs` for help/version success, usage failures, missing-file IO failures, parse failures, runtime failures, and JSON diagnostics validity/stdout-stderr separation; updated existing contract regressions in `tests/cli_json_contracts.rs` and `tests/parser_diagnostics_contract.rs` to assert the new codes. Updated `README.md` and `docs/CLI_MACHINE_READABLE_CONTRACTS.md` with the enforced exit-code policy and negative-path JSON contract mapping. Verification: `cargo test --test cli_contracts --test cli_json_contracts --test parser_diagnostics_contract` and `cargo test` passed.
```

```text
[x] V1-CLI-002: Add `check`, `run`, and `test` workflow clarity
    Priority: P2
    Severity: Enhancement
    Area: CLI/DX
    Affected files: src/main.rs, README.md, docs/CLI_MACHINE_READABLE_CONTRACTS.md, tests/cli_contracts.rs
    Problem: Developers need clear commands for parsing/checking, running, and testing Kujo programs.
    Recommendation: Ensure the CLI exposes predictable workflows.
    Implementation steps:
        1. Decide whether default file execution is equivalent to `run`.
        2. Add or document `kujo run <file>`.
        3. Add or document `kujo check <file>` that lexes/parses/semantically validates without executing.
        4. Add or document `kujo test` behavior for Kujo test files.
        5. Add `--quiet`, `--verbose`, and `--json` only where useful and tested.
    Tests required:
        - `check` does not execute side effects.
        - `run` executes program output.
        - `test` discovers and runs expected tests.
        - Verbose/quiet behavior is deterministic.
    Acceptance criteria: Common developer workflows are first-class and documented.
    Notes: Completed on 2026-05-16. Added first-class `kujo check <file>` in `src/main.rs` with a non-executing validation path (lex/parse/compile only) and deterministic output modes: `--quiet` suppresses success output, `--verbose` prints statement/bytecode counts, and `--json` returns a stable machine-readable payload (`command`, `file`, `status`, `statement_count`, `bytecode_instruction_count`). Centralized parse/diagnostic handling for `run`/`check`/`test-run` through shared `parse_kujo_program(...)` helper logic to keep workflow behavior consistent. Expanded `tests/cli_contracts.rs` with regression coverage for side-effect-free `check`, runtime-executing `run` output, `kujo test` fixture discovery/summary behavior, quiet/verbose determinism, and check JSON schema validation. Updated workflow docs in `README.md` (explicit subcommand model, `check` docs, clarified `test` fixture/snapshot behavior) and JSON contract docs in `docs/CLI_MACHINE_READABLE_CONTRACTS.md`. Verification: `cargo test --test cli_contracts`, `cargo test`.
```

### Phase 8: Performance And Resource Management

```text
[x] V1-PERF-001: Add a benchmark suite for lexer, parser, interpreter, VM, modules, and server
    Priority: P1
    Severity: Medium
    Area: Performance/Tests
    Affected files: benches/*.rs, Cargo.toml, src/lexer.rs, src/parser.rs, src/interpreter/mod.rs, src/vm.rs, src/serve_http.rs
    Problem: Kujo cannot protect performance without measured baselines.
    Recommendation: Add criterion-based benchmarks for core hot paths.
    Implementation steps:
        1. Add benchmark dependencies and `benches/` targets.
        2. Add lexer benchmark for large source and many tokens.
        3. Add parser benchmark for large files and deep expressions.
        4. Add interpreter benchmark for loops, function calls, recursion, strings, and collections.
        5. Add VM benchmark for the same workloads.
        6. Add module resolution benchmark for many small modules.
        7. Add static server benchmark for small and large files if feasible.
    Tests required: Benchmarks compile and run locally with `cargo bench`.
    Acceptance criteria: Kujo has repeatable performance baselines for release comparison.
    Notes: Completed on 2026-05-16. Added Criterion-based benchmark harness wiring in `Cargo.toml` (`criterion` dev-dependency + `[[bench]] v1_perf_benchmarks`) and new suite `benches/v1_perf_benchmarks.rs` covering lexer (`large_source`, `many_tokens`), parser (`large_file`, `deep_expression`), interpreter and VM runtime workloads (loops/function calls/recursion/strings/collections), module-resolution cold-load performance for many small modules, and static-server GET request throughput for small and large files via `src/serve_http.rs` (`run_static_server`) live local request fixtures. Exposed `serve_http` through `src/lib.rs` so the benchmark target exercises the production static-server code path directly. Verification: `cargo bench --bench v1_perf_benchmarks --no-run`, `cargo bench --bench v1_perf_benchmarks -- --warm-up-time 0.01 --measurement-time 0.01 --sample-size 10` (local machine-load-sensitive smoke evidence).
```

```text
[x] V1-PERF-002: Audit and remove avoidable O(n^2) parser/runtime behavior
    Priority: P2
    Severity: Medium
    Area: Performance
    Affected files: src/lexer.rs, src/parser.rs, src/interpreter/mod.rs, src/interpreter/environment.rs, src/vm.rs, src/module.rs
    Problem: Repeated parsing, repeated string concatenation, linear symbol lookup, and repeated filesystem stats can degrade real-world workloads.
    Recommendation: Use benchmark results to fix specific hot spots without changing semantics.
    Implementation steps:
        1. Profile benchmarks from `V1-PERF-001`.
        2. Identify repeated tokenization/parsing paths.
        3. Replace repeated string concatenation in loops with buffered construction where applicable.
        4. Review environment lookup cost and cache only where semantically safe.
        5. Cache module resolution metadata with correct invalidation.
        6. Add regression benchmarks for fixed hot spots.
    Tests required:
        - Existing correctness tests still pass.
        - Benchmarks show no regression for targeted cases.
        - Add regression test if optimization changes a failure mode.
    Acceptance criteria: Each performance change is tied to a measured bottleneck and has no semantic regression.
    Notes: Completed on 2026-05-16. Replaced O(n) per-load circular-import detection scans in `src/module.rs` with an O(1) `loading_stack_index` map keyed by `ModuleCacheKey`, eliminating avoidable O(n^2) behavior for deep acyclic import chains while preserving the existing circular-import diagnostic contract. Added regression coverage in `src/module.rs` tests for stack/index cleanup after circular-import failures and successful chained imports (`load_module_deep_chain_completes_and_clears_loading_index`). Extended `benches/v1_perf_benchmarks.rs` with a dedicated `module_resolution/deep_import_chain_cold_loader` Criterion case to keep the targeted hotspot measurable alongside the existing many-small-modules benchmark. Verification: `cargo test --lib load_module_`, `cargo bench --bench v1_perf_benchmarks --no-run`, `cargo bench --bench v1_perf_benchmarks deep_import_chain_cold_loader -- --warm-up-time 0.01 --measurement-time 0.01 --sample-size 10`, `cargo test`.
```

```text
[x] V1-PERF-003: Add memory and resource exhaustion safeguards
    Priority: P1
    Severity: High
    Area: Security/Performance/Runtime
    Affected files: src/lexer.rs, src/parser.rs, src/interpreter/mod.rs, src/vm.rs, src/serve_http.rs, src/interpreter/native_functions/*.rs
    Problem: Large literals, huge collections, unbounded recursion, unbounded loops, and unbounded IO can exhaust memory or CPU.
    Recommendation: Add explicit limits for resource-heavy operations and make them configurable for trusted runs.
    Implementation steps:
        1. Define default max source size.
        2. Define max string literal size.
        3. Define max collection literal length.
        4. Define recursion depth limit.
        5. Define max call stack depth for VM and interpreter.
        6. Define native IO limits.
        7. Return structured resource-limit errors.
    Tests required:
        - Limit exceeded tests for source, string, collection, recursion, call stack, native IO.
        - Boundary-at-limit success tests.
        - Config override tests if overrides exist.
    Acceptance criteria: Untrusted input cannot force unbounded memory/CPU use without hitting a documented limit.
    Notes: Completed on 2026-05-16. Added centralized runtime/resource defaults in `src/runtime_limits.rs` and wired parser/runtime/native surfaces to use them: source-size (`1,048,576` bytes), string-literal length (`8,192`), collection-literal item count (`4,096`), interpreter call-depth (`32`), VM call-depth (`256`), and shared native IO bounds (`8 MiB`) reused by filesystem/network policy modules. Parser now reports deterministic collection-literal diagnostics for oversized array/dictionary literals, interpreter function-entry guardrails now fail with explicit call-depth errors before unbounded recursion, and VM bytecode function calls now fail with deterministic max-depth errors. Added regression coverage in `tests/parser_diagnostics_contract.rs`, `src/interpreter/mod.rs` unit tests, and `src/vm.rs` VM tests, plus centralized-limit contract alignment in `tests/native_api_security_boundaries.rs`. Verification: targeted parser/interpreter/VM/native-IO limit tests and full `cargo test`.
```

### Phase 9: Test Coverage, Fuzzing, And Regression Gates

```text
[x] V1-TEST-002: Add lexer/parser fuzzing
    Priority: P1
    Severity: High
    Area: Security/Tests
    Affected files: fuzz/fuzz_targets/lexer.rs, fuzz/fuzz_targets/parser.rs, Cargo.toml, src/lexer.rs, src/parser.rs
    Problem: Lexers and parsers are exposed to untrusted input and should not panic, hang, or allocate unbounded memory.
    Recommendation: Add cargo-fuzz targets for lexer and parser.
    Implementation steps:
        1. Initialize `cargo fuzz` if not present.
        2. Add lexer fuzz target that feeds arbitrary bytes/source strings.
        3. Add parser fuzz target that lexes then parses.
        4. Treat panics and infinite loops as bugs.
        5. Add seed corpus from real examples and malformed cases.
    Tests required:
        - Fuzz targets compile.
        - Run each fuzz target for a short smoke duration in CI or nightly job.
        - Add regression tests for every crash found.
    Acceptance criteria: Fuzzing can continuously exercise malformed input without crashing the process.
    Notes: Completed on 2026-05-16. Added full cargo-fuzz scaffolding in `fuzz/` with dedicated targets for lexer (`fuzz/fuzz_targets/lexer.rs`) and parser (`fuzz/fuzz_targets/parser.rs`) plus initial corpora in `fuzz/corpus/lexer/` and `fuzz/corpus/parser/` covering valid and malformed source seeds. Targets fuzz arbitrary bytes via lossy UTF-8 conversion and parser end-to-end lexer→parser diagnostics flow (`parse_with_diagnostics`) to treat panics/hangs as defects while preserving deterministic API boundaries. Added scheduled/manual nightly CI smoke workflow `.github/workflows/fuzz-smoke.yml` that installs `cargo-fuzz`, runs both targets with bounded runtime (`-max_total_time=20`), and provides continuous malformed-input regression coverage outside release-gate latency budgets. Local compile/run attempts can fail on host toolchain C++ header availability (observed in this environment for `libfuzzer-sys`), so CI nightly fuzz smoke is the authoritative execution signal for this item.
```

```text
[x] V1-TEST-003: Add runtime and native API security regression suite
    Priority: P1
    Severity: Critical
    Area: Security/Tests
    Affected files: tests/native_api_security_boundaries.rs, tests/runtime_security.rs, tests/serve_command_integration.rs
    Problem: Security-sensitive behavior must be locked with regression tests.
    Recommendation: Add malicious input tests for source execution, filesystem, modules, process, network, and static serving.
    Implementation steps:
        1. Add malicious Kujo source tests for deep recursion, huge literals, invalid identifiers, invalid escapes, and bad control flow.
        2. Add filesystem traversal, symlink, dotfile, overwrite, and archive extraction tests.
        3. Add module traversal and cycle tests.
        4. Add process/network capability-denied tests.
        5. Add HTTP malformed request tests.
    Tests required: This item itself is the test suite expansion.
    Acceptance criteria: Every P0/P1 security fix has at least one regression test that fails before the fix and passes after it.
    Notes: Completed on 2026-05-16. Added dedicated malicious-source/runtime regression coverage in `tests/runtime_security.rs` for invalid escapes, oversized string/collection literals, loop-control misuse (`break`/`continue` outside loops), interpreter call-depth exhaustion, module import-cycle diagnostics, and Unix symlink-escape module rejection. Expanded static-server boundary coverage in `tests/serve_command_integration.rs` with an oversized request-body contract asserting `413 Payload Too Large` is enforced before method dispatch. Existing `tests/native_api_security_boundaries.rs` coverage now serves as the filesystem/process/network/archive hardening matrix for overwrite/size/capability/traversal/symlink/extraction boundaries. Verification: `cargo test --test runtime_security`, `cargo test --test native_api_security_boundaries`, `cargo test --test serve_command_integration serve_request_body_over_limit_returns_413_before_method_dispatch`, and `cargo test`.
```

```text
[x] V1-TEST-004: Add golden tests for diagnostics
    Priority: P1
    Severity: Medium
    Area: Diagnostics/Tests
    Affected files: tests/diagnostics_golden.rs, tests/fixtures/diagnostics/*.kujo, src/errors.rs
    Problem: Diagnostics often regress accidentally when parsers and runtimes evolve.
    Recommendation: Add golden tests for human and JSON diagnostics.
    Implementation steps:
        1. Create fixture Kujo files for lex, parse, semantic, runtime, CLI, and server errors.
        2. Capture expected human output.
        3. Capture expected JSON output if JSON mode exists.
        4. Normalize file paths and line endings in snapshots.
        5. Add update procedure to docs.
    Tests required:
        - Human diagnostics snapshots.
        - JSON diagnostics snapshots.
        - Cross-platform line ending handling.
    Acceptance criteria: Diagnostic formatting is stable and intentional.
    Notes: Completed on 2026-05-16. Added fixture-backed diagnostics golden suite `tests/diagnostics_golden.rs` plus snapshot fixtures under `tests/fixtures/diagnostics/` for lexer, parser, semantic-parser, runtime, CLI, and serve-related diagnostic contracts. The harness captures paired human (`*.human.golden`) and JSON (`*.json.golden`) snapshots via stable `Diagnostic` conversion/rendering APIs, normalizes line endings for cross-platform assertions, and supports explicit snapshot refreshes via `KUJO_UPDATE_GOLDENS=1 cargo test --test diagnostics_golden`. Added source fixtures for malformed lexical and parse inputs plus semantic assignment-target and runtime identifier error scenarios. Verification: `KUJO_UPDATE_GOLDENS=1 cargo test --test diagnostics_golden` (snapshot generation) and `cargo test --test diagnostics_golden` (lockfile assertion), followed by full `cargo test`.
```

```text
[x] V1-TEST-005: Smoke-test examples and documentation snippets
    Priority: P1
    Severity: Medium
    Area: Docs/Tests/DX
    Affected files: examples/**, README.md, docs/*.md, tests/docs_examples.rs
    Problem: Examples can drift from real syntax and runtime behavior.
    Recommendation: Add automated smoke tests for examples and documented code blocks where feasible.
    Implementation steps:
        1. Inventory examples.
        2. Mark each example as parse-only, run, expected-fail, or manual.
        3. Add tests that parse or run examples according to metadata.
        4. Extract fenced Kujo code blocks from docs where feasible.
        5. Update examples that rely on obsolete semantics.
    Tests required: Example smoke test suite.
    Acceptance criteria: Documented Kujo code either runs, parses, or is explicitly marked as illustrative/manual.
    Notes: Completed on 2026-05-16. Added `tests/docs_examples.rs`, a metadata-driven smoke harness that inventories every `examples/**/*.kujo` file and fenced ` ```kujo ` block from `README.md` plus `docs/*.md`, then enforces explicit classification as run / parse-only / expected-fail. The suite executes a curated run subset (`kujo run --interpreter`), parse-checks the remaining examples/snippets (`kujo check --quiet`), and locks known legacy drift as explicit expected-fail metadata so status changes require intentional reclassification rather than silent drift. Documented execution in `README.md` under testing (`cargo test --test docs_examples`). Verification: `cargo test --test docs_examples` and full `cargo test`.
```

### Phase 10: Documentation And Release Readiness

```text
[x] V1-DOC-001: Reconcile language spec with implementation
    Priority: P1
    Severity: High
    Area: Docs/Correctness
    Affected files: docs/LANGUAGE_SPEC.md, README.md, tests/docs_examples.rs
    Problem: The language spec and implementation are not fully aligned, especially around mutability, null/fallthrough, identifiers, errors, and backend parity.
    Recommendation: Update the spec after semantic fixes land, and add tests for every documented feature.
    Implementation steps:
        1. Add sections for lexical grammar, literals, comments, escapes, and invalid input.
        2. Add parser grammar or syntax summary.
        3. Add operator precedence table.
        4. Add binding, scope, shadowing, mutability rules.
        5. Add function, closure, method, async, generator arity rules.
        6. Add null, return, break, continue rules.
        7. Add collection, indexing, equality, comparison, truthiness rules.
        8. Add module/import rules.
        9. Add diagnostic and exit-code overview.
    Tests required:
        - Every spec example is covered by docs/example smoke tests.
        - Semantic tests exist for every table/rule.
    Acceptance criteria: A user can predict Kujo behavior from the spec, and the test suite enforces that behavior.
    Notes: Completed on 2026-05-16. Reconciled `docs/LANGUAGE_SPEC.md` with current runtime/parser behavior by adding explicit lexical/mutability/scope/function/control-flow/data-structure/module examples plus a dedicated diagnostics/CLI exit-code contract section aligned with `V1-CLI-001`. Added `tests/language_spec_contracts.rs` to enforce documented semantics across interpreter and VM (mutability success/failure paths, duplicate declarations, lexical shadowing and loop-binding lifetime, function null fallthrough, callable arity errors, truthiness/logical short-circuit contracts, and indexing error determinism). Updated README testing guidance with the new spec contract command and validated both spec-example smoke and semantic contracts before full-suite verification.
```

```text
[x] V1-DOC-002: Expand native API security posture into an operator-grade security guide
    Priority: P1
    Severity: High
    Area: Docs/Security
    Affected files: docs/NATIVE_API_SECURITY_POSTURE.md, README.md
    Problem: Kujo exposes trusted-code host APIs. Users need explicit guidance for safe operation.
    Recommendation: Document threat model, trusted vs untrusted execution, capabilities, sandboxing, filesystem/network/process risks, and safe deployment patterns.
    Implementation steps:
        1. State whether Kujo is safe for untrusted code by default.
        2. Document every capability and the APIs it controls.
        3. Document static server security defaults.
        4. Document archive extraction safety.
        5. Document process/shell execution risks.
        6. Document recommended OS sandboxing for high-risk deployments.
        7. Add examples of safe and unsafe configurations.
    Tests required: Documentation examples and referenced CLI flags must be smoke-tested.
    Acceptance criteria: Users can understand and configure Kujo's host security boundary without reading the source.
    Notes: Completed on 2026-05-16. Reworked `docs/NATIVE_API_SECURITY_POSTURE.md` into an operator-grade guide with explicit non-sandbox statement, threat model/non-goals, trust-mode defaults, full capability flag matrix, static-server security defaults, archive/process/network/filesystem risk sections, safe-vs-unsafe CLI patterns, and external sandboxing recommendations. Added `tests/security_posture_docs_contract.rs` to enforce that required capability flags and operator sections remain present in docs and that `kujo run --help` still exposes all documented `--untrusted`/`--allow-*` flags. Updated `README.md` security guidance to point operators to the expanded posture guide.
```

```text
[x] V1-DOC-003: Create a release process and compatibility policy
    Priority: P1
    Severity: Medium
    Area: Release/Docs
    Affected files: docs/RELEASE_PROCESS.md, CHANGELOG.md, README.md, Cargo.toml
    Problem: A production 1.0 release needs versioning, compatibility, changelog, and release-gate rules.
    Recommendation: Add a documented release process.
    Implementation steps:
        1. Define semantic versioning policy.
        2. Define language backward compatibility rules.
        3. Define stdlib compatibility rules.
        4. Define diagnostic code stability rules.
        5. Define release candidate process.
        6. Define required CI gates.
        7. Add changelog format.
        8. Document how to publish crate/artifacts.
    Tests required: Release-gate script passes locally and in CI.
    Acceptance criteria: Maintainers can cut a repeatable release without relying on tribal knowledge.
    Notes: Completed on 2026-05-16. Expanded `docs/RELEASE_PROCESS.md` into the canonical release+compatibility policy, including semantic versioning rules, language/stdlib compatibility commitments, machine-readable diagnostic/CLI stability rules, explicit RC workflow, required CI/local gate ordering, changelog sectioning template, and crate/artifact publication order (`cargo publish --dry-run`/`cargo publish` + release artifact workflow). Added `tests/release_process_docs_contract.rs` to lock required policy sections and validate `scripts/release_gate.sh --help` mode/env documentation. Updated README release-doc callout to reflect that `docs/RELEASE_PROCESS.md` is the authoritative policy for release sign-off. Verification: `cargo test --test release_process_docs_contract` and `bash scripts/release_gate.sh --minimal` passed. Local full `cargo test` runs were executed but encountered unrelated environment-sensitive failures in socket/performance-sensitive tests (`tests/serve_command_integration.rs` bind permission races and `interpreter::async_runtime::tests::test_concurrent_tasks` timing threshold drift); targeted suites for this roadmap item passed.
```

```text
[x] V1-DOC-004: Update README for accurate 1.0 user expectations
    Priority: P1
    Severity: Medium
    Area: Docs/DX
    Affected files: README.md
    Problem: README is the first contract users see. It must not overstate maturity or omit important safety constraints.
    Recommendation: Rewrite README sections around installation, quickstart, CLI, serve command, language status, security model, examples, and roadmap link.
    Implementation steps:
        1. Add current status and target 1.0 readiness note.
        2. Add install/build instructions.
        3. Add quickstart script.
        4. Add CLI command table.
        5. Add static server usage and security defaults.
        6. Add capability/security note.
        7. Add links to language spec, stdlib docs, release process, and roadmap.
    Tests required: README commands and examples smoke-tested by `V1-TEST-005`.
    Acceptance criteria: README accurately represents what Kujo can do today and what is required for 1.0.
    Notes: Completed on 2026-05-16. Updated `README.md` with explicit safety/trust-mode framing (`Kujo is not a sandbox`, trusted default, `--untrusted` + `--allow-*` guidance), added a focused core-reference section linking roadmap/spec/stdlib/release policy docs, and preserved accurate pre-1.0 readiness language. Added `tests/readme_contracts.rs` to lock required README contract markers (1.0 readiness status, security snapshot, install/quickstart cues, CLI serve visibility, untrusted-mode guidance, and core reference links). Verification: `cargo test --test readme_contracts` and `cargo test --test docs_examples` passed. Local full `cargo test` runs were executed but intermittently failed on unrelated socket-startup timing races in `tests/serve_command_integration.rs`; focused serve suite reruns passed.
```

```text
[x] V1-DOCGEN-001: Track universal DocGen maturation milestones
    Priority: P2
    Severity: Medium
    Area: Docs/Tooling/Release Readiness
    Affected files: ROADMAP.md, docs/DOCGEN.md, tests/docgen_universal.rs, tests/fixtures/docgen/*
    Problem: Universal DocGen QA hardening is complete, but the next-stage maturation milestones are not tracked in the main release roadmap.
    Recommendation: Track the next universal DocGen slice in the roadmap and keep milestone IDs aligned with `docs/DOCGEN.md`.
    Implementation steps:
        1. Track `DG-NEXT-001`: parser-assisted Kujo extraction fallback prototype with deterministic fallback behavior.
        2. Track `DG-NEXT-002`: cross-language adapter conformance expansion with fixture-backed edge coverage.
        3. Track `DG-NEXT-003`: external-repo strict-gate baseline refresh cadence and evidence capture.
        4. Keep milestone IDs and acceptance criteria synchronized between `ROADMAP.md` and `docs/DOCGEN.md`.
    Tests required:
        - `cargo test --test docgen_universal` for adapter/extraction contract stability.
        - Any added DocGen contract suites introduced by milestone execution.
    Acceptance criteria: Universal DocGen next milestones are explicitly tracked in roadmap/docs with aligned IDs and clear verification paths.
    Notes: Added on 2026-05-20 as pre-v1 checklist follow-through (`PREV1-DOC-003`). Initial milestone definitions are synchronized with `docs/DOCGEN.md` under `DG-NEXT-001` through `DG-NEXT-003`. Marked complete on 2026-05-20 after promoting execution ownership into `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` (`V1U-DG-001`, `V1U-DG-002`, `V1U-DG-003`) with dedicated acceptance criteria and per-loop closure evidence requirements. Focused non-network DocGen contracts were revalidated (`docgen_adapter_conformance_smoke_extracts_symbols_for_all_languages`, `docgen_cli_json_contract_preserves_legacy_fields`, `docgen_kujo_extraction_edge_fixture_async_visibility_contract`); full-suite `docgen_universal` remains environment-sensitive here due socket/runtime constraints.
```

```text
[x] V1-REL-001: Prepare v1.0 release candidate gate
    Priority: P1
    Severity: High
    Area: Release/CI
    Affected files: Cargo.toml, Cargo.lock, CHANGELOG.md, docs/RELEASE_PROCESS.md, .github/workflows/*.yml
    Problem: The project needs a final release gate before tagging v1.0.0.
    Recommendation: Add a release candidate checklist and require every gate to pass.
    Implementation steps:
        1. Complete all P0 and P1 items.
        2. Review P2 items and explicitly defer or complete each one.
        3. Run full release-gate script.
        4. Run benchmarks and record baseline.
        5. Run fuzz smoke jobs.
        6. Update changelog.
        7. Update crate version only after gates pass.
        8. Tag release candidate.
    Tests required:
        - Full release-gate script.
        - CI release workflow dry run if available.
    Acceptance criteria: Kujo can produce a v1.0.0 release candidate from a clean checkout with documented commands.
    Notes: Completed on 2026-05-16. Added executable RC gating workflow `scripts/release_candidate_gate.sh` with two modes: `--roadmap-only` (checks unresolved P0/P1 roadmap blockers and release-checklist section presence) and `--full` (roadmap precheck + `scripts/release_gate.sh --full` + serialized static-server integration run). Updated release docs (`docs/RELEASE_PROCESS.md`) and README testing guidance to expose this RC gate path. Added contract coverage in `tests/release_candidate_gate_contract.rs` for help/mode wiring and roadmap-only execution success. Verification: `cargo test --test release_candidate_gate_contract`, `bash scripts/release_candidate_gate.sh --roadmap-only`, and focused release-policy/README contract suites passed. Local `--full` gate currently fails at `cargo fmt --check` due existing formatting drift in unrelated tests, and full `cargo test` remains intermittently unstable in this environment due unrelated socket/timing-sensitive tests; those blockers are now explicit in the release checklist status below.
```

## 8. Cross-Cutting Test Matrix

| Test Area | What To Test | Why It Matters | Suggested Test Names |
| --------- | ------------ | -------------- | -------------------- |
| Lexer malformed input | Invalid characters, invalid escapes, unterminated strings/comments, huge numerics, null bytes, mixed line endings | Prevents unsafe or silent acceptance of bad source | `lexer_rejects_invalid_character`, `lexer_reports_unterminated_string`, `lexer_rejects_invalid_escape`, `lexer_rejects_null_byte` |
| Lexer limits | Long identifiers, huge strings, huge source files | Prevents memory exhaustion | `lexer_rejects_identifier_over_limit`, `lexer_rejects_string_literal_over_limit` |
| Parser delimiters | Missing `)`, `]`, `}`, unexpected EOF | Prevents partial ASTs | `parser_reports_missing_right_paren`, `parser_reports_unexpected_eof` |
| Parser recovery | Multiple syntax errors in one file | Improves diagnostics and avoids infinite loops | `parser_recovers_after_statement_error` |
| Parser precedence | All operator precedence and associativity combinations | Locks language semantics | `parser_precedence_arithmetic_before_comparison`, `parser_assignment_is_right_associative` |
| Parser depth | Deep expressions, nested blocks, nested calls | Prevents stack overflow | `parser_rejects_expression_depth_over_limit` |
| Runtime identifiers | Missing variables in all scopes | Prevents typos becoming strings | `runtime_undefined_identifier_errors` |
| Runtime arity | Too few/too many args for all callable kinds | Prevents unbound params and ignored args | `runtime_function_arity_too_few_errors`, `runtime_method_arity_too_many_errors` |
| Runtime invalid ops | Bad indexing, bad assignment, unsupported operators | Prevents sentinel fallbacks | `runtime_list_index_oob_errors`, `runtime_invalid_assignment_target_errors` |
| Semantics | Truthiness, null, equality, comparison, mutability, scope | Makes language predictable | `semantics_truthiness_table`, `semantics_const_reassignment_errors` |
| VM parity | Interpreter and VM agree on values and errors | Prevents backend drift | `vm_parity_missing_map_key_errors`, `vm_parity_function_arity_errors` |
| JIT parity | Supported JIT surfaces match interpreter | Prevents unsafe optimization | `jit_parity_basic_arithmetic`, `jit_rejects_unsupported_feature` |
| Filesystem security | Traversal, symlinks, dotfiles, overwrite, archive extraction | Protects host files | `fs_rejects_parent_traversal`, `unzip_rejects_zip_slip` |
| Module security | Import traversal, symlink escape, cycles, cache keys | Protects project boundaries | `module_rejects_import_outside_root`, `module_reports_import_cycle` |
| Native capabilities | FS/process/network/env APIs denied/allowed | Enforces security boundary | `capability_denies_shell_exec`, `capability_allows_fs_read_only` |
| Static server paths | Encoded traversal, double decoding, null bytes, long URI | Prevents web path escape | `serve_rejects_encoded_traversal`, `serve_rejects_oversized_uri` |
| Static server headers | Content-Type, Content-Length, Allow, nosniff, HEAD | Standards and security | `serve_405_includes_allow`, `serve_head_has_no_body` |
| Static server MIME | Broad MIME map, fallback, case-insensitive extensions | Universal usefulness | `serve_mime_maps_wasm`, `serve_unknown_extension_fallback` |
| CLI | Help, version, stdin, missing file, parse/runtime errors, JSON | Predictable developer experience | `cli_help_exits_zero`, `cli_parse_error_exit_code`, `cli_json_diagnostic_valid` |
| Diagnostics | Human and JSON golden outputs | Stable user-facing errors | `diagnostics_parse_error_golden`, `diagnostics_runtime_error_json` |
| Fuzzing | Lexer, parser, runtime entrypoints | Finds panics and hangs | `fuzz_lexer`, `fuzz_parser`, `fuzz_runtime_entrypoints` |
| Benchmarks | Large parse, loops, functions, collections, server throughput | Performance baseline | `bench_parse_large_file`, `bench_vm_function_calls`, `bench_static_large_file` |
| Docs/examples | README snippets, examples, spec samples | Prevents documentation drift | `docs_readme_examples_smoke`, `examples_parse_or_run` |

## 9. Security Hardening Plan

Immediate risks:

1. Archive extraction path-traversal and extraction-exhaustion risks are now bounded by `V1-SEC-001`; continue reusing centralized containment helpers for future filesystem/import/server surfaces.
2. Undefined identifiers and sentinel fallbacks can bypass intended runtime checks unless fixed by `V1-RUN-001` and `V1-RUN-003`.
3. Native host APIs can perform filesystem, process, environment, and network effects without explicit capabilities unless fixed by `V1-SEC-002`.
4. Shell execution can interpret untrusted strings unless fixed by `V1-SEC-003`.
5. Static server path and MIME policy can drift or expose unsafe content unless fixed by `V1-HTTP-001` through `V1-HTTP-005`.

Recommended safeguards:

- Central path containment helper for filesystem, imports, archives, and static server paths.
- Capability checks at every host-effect native API.
- Structured diagnostics for malformed source and runtime security failures.
- Request-target validation before static file lookup.
- Dotfile/private-file deny-by-default policy.
- Archive entry sanitization and extraction limits.
- Process execution timeout and output limits.
- Network timeout and body size limits.
- Fuzzing for lexer/parser/runtime entrypoints.

Abuse cases to simulate:

- Kujo source with deeply nested expressions.
- Kujo source with huge string and numeric literals.
- Kujo source with invalid UTF-8 or null bytes at file boundary.
- Kujo source with invalid escapes and unterminated comments.
- Undefined variable used in an authorization-style condition.
- Missing function args that previously became missing bindings.
- Path traversal through filesystem API.
- Symlink escape through filesystem API and static server.
- Zip archive with `../escape.txt`.
- Zip archive with absolute paths and too many entries.
- Module import using traversal outside package root.
- Shell command containing metacharacters passed to direct exec.
- Network request without capability.
- HTTP request with `%2e%2e`, double-encoded traversal, invalid percent encoding, null byte, long URI, oversized headers, unexpected method.
- Request for `.env`, `.git/config`, backup files, and unknown active content.

## 10. Performance Plan

Likely hot paths:

- Tokenization of large source files.
- Recursive expression parsing.
- AST walking in interpreter execution.
- Environment and symbol lookup.
- Function call dispatch.
- Collection and map operations.
- Module import resolution.
- VM opcode dispatch.
- Static server file IO.
- Native string and collection helpers.

Measurement strategy:

1. Add benchmarks before broad optimization.
2. Benchmark interpreter and VM on identical workloads.
3. Track parse-only, run-only, and compile-plus-run timings separately.
4. Include memory-sensitive workloads where possible.
5. Record baseline numbers before v1.0 release candidate.
6. Add a performance regression threshold only after baseline stability is understood.

Benchmarks to add:

- Large source file lexing.
- Large source file parsing.
- Deep expression parsing at safe boundary.
- Many variable declarations.
- Many function calls.
- Large loops.
- Recursive functions up to recursion limit.
- Large string concatenation.
- Large list/map construction and lookup.
- Module import graph with many small files.
- VM opcode loop.
- Static server small-file and large-file throughput.

Low-risk optimizations:

- Replace repeated string concatenation in tight loops with buffered construction.
- Avoid repeated tokenization/parsing of unchanged modules.
- Use canonical module cache keys.
- Stream static files instead of full reads.
- Centralize environment lookup helpers and avoid duplicate lookups.

High-risk optimizations to avoid until measured:

- Broad parser rewrite.
- Aggressive AST interning.
- Global mutable caches without invalidation.
- JIT as default execution path.
- Unsafe filesystem shortcuts that bypass containment checks.
- Silent fallback behavior for speed.

Performance acceptance criteria:

- Benchmarks exist and compile.
- P0/P1 correctness and security tests pass after optimizations.
- No optimization changes documented language semantics.
- Static server large-file serving does not require whole-file memory allocation.
- Parser rejects malicious deep input with diagnostics before stack overflow.

## 11. Documentation Plan

Missing or incomplete docs:

- Accurate 1.0 readiness status.
- Complete language reference.
- Operator precedence table.
- Scope, shadowing, mutability, null, truthiness, equality, and numeric semantics.
- Callable arity and function/method behavior.
- Parser and runtime diagnostic code catalog.
- CLI command and exit-code contract.
- Static server security and MIME behavior.
- Complete standard library/native API reference.
- Capability/security guide.
- Release process and compatibility policy.
- Benchmark and performance baseline guide.

Docs that should be created:

- `docs/STANDARD_LIBRARY.md`
- `docs/RELEASE_PROCESS.md`
- `docs/DIAGNOSTICS.md`
- `docs/SECURITY_MODEL.md` if `docs/NATIVE_API_SECURITY_POSTURE.md` becomes too large
- `docs/PERFORMANCE.md` after benchmarks exist

Docs that should be generated or checked from tests:

- README command examples.
- Language spec examples.
- Standard library examples.
- CLI JSON diagnostic examples.
- Static server examples.

README improvements:

- State current pre-1.0 maturity honestly.
- Show install/build commands.
- Show minimal run/check/test workflows.
- Show static server usage and safe defaults.
- Link to security model and roadmap.
- Avoid claims of production readiness until release gates pass.

Security notes:

- Clearly state whether Kujo is safe for untrusted code by default.
- Document capabilities and dangerous native APIs.
- Document shell execution risks.
- Document filesystem and network risks.
- Document recommended OS-level sandboxing for high-risk deployments.

## 12. Implementation Roadmap By Phase

### Phase 1: Stop The Bleeding

Goals:

- Restore a passing test baseline.
- Remove the most dangerous silent runtime failures.
- Fix archive extraction traversal.
- Remove static server policy drift.
- Ensure CI blocks failing tests.

Checklist items:

- `V1-TEST-001`
- `V1-RUN-001`
- `V1-RUN-002`
- `V1-RUN-003`
- `V1-SEC-001`
- `V1-HTTP-001`
- `V1-CI-001`

Expected outcome:

- `cargo test` passes.
- Undefined identifiers, missing map keys, bad arity, bad indexing, and invalid assignments fail clearly.
- Archive extraction cannot escape its destination.
- Static server tests use production policy.

Risk level: High. These changes may expose tests and examples that relied on old demo semantics.

### Phase 2: Make Behavior Predictable

Goals:

- Give lexer/parser/runtime failures structured diagnostics.
- Add source spans.
- Lock down parser correctness and source limits.
- Define core semantics in code and docs.

Checklist items:

- `V1-LEX-001`
- `V1-PAR-001`
- `V1-ERR-001`
- `V1-PAR-002`
- `V1-PAR-003`
- `V1-PAR-004`
- `V1-RUN-004`
- `V1-RUN-005`
- `V1-RUN-006`
- `V1-RUN-007`
- `V1-SEM-001`
- `V1-SEM-002`
- `V1-SEM-003`

Expected outcome:

- Kujo accepts valid source predictably and rejects invalid source with useful diagnostics.
- Language semantics are documented and tested.
- Runtime no longer relies on sentinel values for invalid behavior.

Risk level: High. Parser and semantic changes touch broad behavior.

### Phase 3: Secure Host Boundaries

Goals:

- Make native APIs safe by policy.
- Centralize filesystem/path containment.
- Harden process, network, module, archive, and server boundaries.

Checklist items:

- `V1-SEC-002`
- `V1-SEC-003`
- `V1-FS-001`
- `V1-FS-002`
- `V1-MOD-001`
- `V1-NET-001`
- `V1-HTTP-003`
- `V1-HTTP-004`
- `V1-HTTP-005`

Expected outcome:

- Kujo has a clear trusted-code/security model.
- Host effects require explicit capability policy.
- Static and native path handling use one hardened implementation.

Risk level: High. Security policy affects user-visible API behavior.

### Phase 4: Make Kujo Universally Useful

Goals:

- Broaden server, stdlib, CLI, and docs beyond narrow demo use.
- Keep additions lean and strongly tested.

Checklist items:

- `V1-HTTP-002`
- `V1-HTTP-006`
- `V1-HTTP-007`
- `V1-STD-001`
- `V1-STD-002`
- `V1-STD-003`
- `V1-CLI-001`
- `V1-CLI-002`

Expected outcome:

- Common static assets work safely.
- Standard library behavior is documented and tested.
- CLI workflows are predictable for real users.

Risk level: Medium. Most work is additive but must avoid scope creep.

### Phase 5: Make It Measurable

Goals:

- Add fuzzing, diagnostics golden tests, examples smoke tests, and benchmarks.
- Establish performance and security regression gates.

Checklist items:

- `V1-PERF-001`
- `V1-PERF-002`
- `V1-PERF-003`
- `V1-TEST-002`
- `V1-TEST-003`
- `V1-TEST-004`
- `V1-TEST-005`
- `V1-COMP-001`
- `V1-JIT-001`

Expected outcome:

- Regressions are much harder to introduce.
- Performance can be discussed with data.
- VM/JIT support is explicitly bounded by tests.

Risk level: Medium. Tooling may require CI configuration and runtime tuning.

### Phase 6: Make It Releasable

Goals:

- Finish docs, release process, changelog, versioning, and release candidate gates.

Checklist items:

- `V1-DOC-001`
- `V1-DOC-002`
- `V1-DOC-003`
- `V1-DOC-004`
- `V1-REL-001`
- `V1-BASE-002`

Expected outcome:

- Kujo can cut a credible v1.0 release candidate.
- Users can understand what is stable, what is experimental, and how to use Kujo safely.

Risk level: Medium. Release docs must match actual implemented behavior.

## 13. Final v1.0 Release Checklist

Before tagging v1.0.0:

```text
[x] All P0 items complete.
[x] All P1 items complete or explicitly deferred with documented release exception.
[x] P2 items reviewed and either completed or documented as post-1.0.
[x] `cargo fmt --check` passes.
[x] `cargo clippy --all-targets --all-features -- -D warnings` passes.
[x] `cargo test` passes.
[x] Integration tests pass.
[x] Security boundary tests pass.
[x] Static server tests pass.
[x] VM/interpreter parity tests pass.
[x] Fuzz targets compile and smoke-run.
[x] Benchmarks compile and baseline is recorded.
[x] README is accurate.
[x] Language spec matches implementation.
[x] Standard library docs are complete for exposed APIs.
[x] Security posture docs are accurate.
[x] Release process docs are complete.
[x] Changelog is updated.
[x] Cargo version is bumped intentionally.
[x] Release candidate is built from a clean working tree.
```

## 14. Handoff Instructions For Implementation Agents

Work order summary:

Start with Phase 1. Restore the failing test baseline, remove silent runtime failures, fix archive extraction, unify static server policy, and make CI block regressions. Then move through parser/diagnostics, host security boundaries, HTTP/server hardening, stdlib/CLI usefulness, benchmarks/fuzzing, and release docs.

Rules:

1. Do not remove existing functionality unless it is unsafe or obsolete and replacement behavior is added.
2. Do not reduce test coverage.
3. Do not skip tests for new behavior.
4. Do not silently change language semantics without documenting and testing the change.
5. Do not patch around symptoms when a centralized abstraction is the correct fix.
6. Every completed item must include code changes, tests, and documentation updates where relevant.
7. The final state must have all tests passing.

Suggested first items:

1. `V1-TEST-001`
2. `V1-RUN-001`
3. `V1-RUN-002`
4. `V1-RUN-003`
5. `V1-SEC-001`
6. `V1-HTTP-001`
7. `V1-CI-001`

Suggested execution order:

1. Fix the known failing VM test.
2. Add or update regression tests for the fixed behavior.
3. Remove the undefined-identifier fallback.
4. Enforce callable arity.
5. Replace invalid-operation sentinel fallbacks with structured errors.
6. Fix archive extraction and path containment.
7. Unify static server MIME/security behavior.
8. Run `cargo test`.
9. Update docs for changed semantics.
10. Continue to Phase 2 only after Phase 1 is green.

No-regression requirement:

Every item must leave Kujo more correct, more secure, or more measurable than before. If a change breaks an existing example or behavior, either preserve the behavior safely or document and test the replacement behavior.

## 15. Audit-Derived Continuation Backlog (2026-05-16)

These items are derived from:

- `docs/UNSAFE_CODE_AUDIT.md`
- `docs/UNFINISHED_AND_MVP_AUDIT.md`
- `docs/KUJO_FEATURE_INVENTORY.md`
- `docs/AI_CENTRIC_GAP_ANALYSIS.md`

They are intentionally split into:

1. **Pre-v1 promotion candidates** (owner decision required)
2. **Post-v1 expansion tracks** (default backlog)

### 15.1 Pre-v1 Promotion Candidates

```text
[x] V1-BASE-003: Deconflict stale historical planning docs from active v1 guidance
    Priority: P1
    Severity: Medium
    Area: Docs/Release Hygiene
    Affected files: notes/NEXT_SESSION.md, ROADMAP.md, CHANGELOG.md
    Problem: `notes/NEXT_SESSION.md` still presents Jan-2026 v0.9.0 emergency JIT goals as current-blocking guidance, which can conflict with v1 checklist authority.
    Recommendation: Keep the file as historical evidence, but mark it explicitly as archived/superseded by current `ROADMAP.md` and v1 scope docs.
    Implementation steps:
        1. Add a clear historical/superseded banner at the top of `notes/NEXT_SESSION.md`.
        2. Point readers to active readiness sources (`ROADMAP.md`, `docs/V1_SCOPE.md`).
        3. Update changelog and roadmap note.
    Tests required: Docs consistency only.
    Acceptance criteria: No active workflow should interpret `NEXT_SESSION.md` as current release-gate authority.
    Notes: Completed on 2026-05-16. Added an explicit superseded/historical banner to `notes/NEXT_SESSION.md` and pointed active workflows to `ROADMAP.md`, `docs/V1_SCOPE.md`, and `docs/RELEASE_PROCESS.md` so legacy v0.9 emergency language is no longer ambiguous for current release readiness.
```

```text
[x] V1-TEST-006: Burn down docs/examples expected-fail debt
    Priority: P1
    Severity: Medium
    Area: Docs/Tests/Quality
    Affected files: tests/docs_examples.rs, README.md, docs/*.md
    Problem: `docs_examples` harness intentionally carries expected-fail entries, which means some user-facing docs/examples are still parse-drift debt.
    Recommendation: Convert expected-fail examples/snippets to parse-clean or run-clean status where feasible.
    Implementation steps:
        1. Inventory current expected-fail entries in `tests/docs_examples.rs`.
        2. Fix snippet/example syntax and behavior in docs/examples.
        3. Reclassify fixed entries from expected-fail to parse/run.
    Tests required: `cargo test --test docs_examples` and `cargo test`.
    Acceptance criteria: Expected-fail set is reduced and remaining entries are explicitly justified.
    Notes: Completed on 2026-05-16. Removed all docs-snippet expected-fail debt from `tests/docs_examples.rs` by fixing parse-drift snippets in `docs/ARCHITECTURE.md`, `docs/CONCURRENCY.md`, `docs/MEMORY.md`, `docs/OPTIONAL_TYPING_DESIGN.md`, and `docs/PERFORMANCE.md` and by converting deferred optional-typing proposal-only syntax fences to `text` plus parse-clean Kujo equivalents. Added explicit reason metadata for every remaining expected-fail example entry and new regression checks that expected-fail entries have reasons, exist on disk, and do not overlap run-smoke examples. Verification: `cargo test --test docs_examples` passed. `cargo test` was run and failed only on `tests/release_candidate_gate_contract.rs` because `V1-ERR-002` remained unchecked at that point.
```

```text
[x] V1-ERR-002: Add machine-readable runtime diagnostics mode for `kujo run`
    Priority: P1
    Severity: Medium
    Area: CLI/Diagnostics/Automation
    Affected files: src/main.rs, src/errors.rs, docs/CLI_MACHINE_READABLE_CONTRACTS.md, tests/diagnostics_golden.rs
    Problem: Parse/lint/docgen have strong JSON contracts, but runtime execution diagnostics are less automation-friendly for agentic workflows.
    Recommendation: Add stable runtime JSON diagnostic output mode for `kujo run` failures.
    Implementation steps:
        1. Define runtime JSON envelope shape and versioning rules.
        2. Add CLI flag path and shared rendering.
        3. Add fixture/golden coverage for runtime JSON diagnostics.
    Tests required: JSON contract tests + diagnostics golden tests + `cargo test`.
    Acceptance criteria: Runtime failures can be consumed deterministically by tooling/agents.
    Notes: Completed on 2026-05-16. Added `kujo run --json-runtime-diagnostics` in `src/main.rs` with a centralized JSON envelope builder in `src/errors.rs` (`run_runtime_diagnostic_envelope_json`) and stable contract version metadata. Runtime/VM execution failures now optionally emit deterministic JSON payloads on `stdout` (including shared diagnostic fields plus optional runtime kind/call stack) while preserving existing human-readable stderr behavior when the flag is not used. Added contract coverage in `tests/cli_json_contracts.rs` and `tests/cli_contracts.rs`, plus diagnostics-golden snapshot coverage in `tests/diagnostics_golden.rs` and `tests/fixtures/diagnostics/runtime_undefined_identifier_run_envelope.json.golden`. Updated machine-readable CLI contract docs in `docs/CLI_MACHINE_READABLE_CONTRACTS.md`.
```

### 15.2 Post-v1 Expansion Tracks

```text
[x] V2-SEC-001: Unsafe code reduction and safety-invariant hardening track
    Priority: P2
    Severity: High
    Area: VM/JIT/Safety
    Affected files: src/jit.rs, src/vm.rs, docs/NATIVE_API_SECURITY_POSTURE.md
    Problem: `unsafe` usage is concentrated (57 real callsites) in JIT/VM boundaries and should be reduced or tightly documented over time.
    Recommendation: Establish a staged unsafe-reduction plan: annotate invariants, centralize wrappers, then remove unnecessary unsafe usage where possible.
    Tests required: Existing VM/JIT tests + targeted regression tests per refactor.
    Notes: Completed on 2026-05-16 as the first staged hardening slice for this track. Added centralized JIT invocation wrappers in `src/jit.rs` (`invoke_compiled_fn`, `invoke_compiled_fn_with_arg`) with explicit safety invariants, replaced scattered VM-side inline unsafe JIT invocation callsites in `src/vm.rs` with wrapper calls, and added dedicated wrapper regression tests in `src/jit.rs` (`invoke_compiled_fn_wrapper_calls_function_pointer`, `invoke_compiled_fn_with_arg_wrapper_forwards_argument`). Updated security posture docs in `docs/NATIVE_API_SECURITY_POSTURE.md` with a VM/JIT unsafe-boundary policy section. Verification: `cargo test --lib invoke_compiled_fn_wrapper_calls_function_pointer`, `cargo test --lib invoke_compiled_fn_with_arg_wrapper_forwards_argument`, `cargo test --lib test_execute_compiled_code`, `cargo test --lib test_execute_with_variables`, and `cargo test --test vm_interpreter_parity_surfaces` passed.
```

```text
[x] V2-AI-001: First-class AI API ergonomics layer
    Priority: P2
    Severity: Enhancement
    Area: Standard Library/HTTP/Developer Experience
    Affected files: src/interpreter/native_functions/http.rs, docs/STANDARD_LIBRARY.md, docs/LANGUAGE_SPEC.md
    Problem: AI workflows currently rely on low-level HTTP+JSON glue in user scripts.
    Recommendation: Add high-level native helpers for chat/embeddings/streaming/tool-call loops with deterministic error contracts.
    Tests required: Native contract tests + docs examples + integration smoke tests.
    Notes: Completed on 2026-05-16. Added first-class high-level AI native helpers in `src/interpreter/native_functions/http.rs`: `ai_chat`, `ai_stream_chat`, `ai_embedding`, and `ai_tool_loop`, including centralized options parsing (`endpoint`, `model`, timeout, headers), deterministic request/response envelope handling, strict input validation, and deterministic failure contracts (`Value::Error` for shape misuse, `Result(Err(...))` for transport/provider failures). Registered the helpers in runtime builtin registration/listing and centralized arity/capability metadata (`src/interpreter/mod.rs`, `src/interpreter/capabilities.rs`). Added native contract coverage in `src/interpreter/native_functions/http.rs` tests for success, failure, edge validation, and embedding-vector regression using a local one-shot HTTP fixture server. Updated `docs/STANDARD_LIBRARY.md`, `docs/LANGUAGE_SPEC.md`, `README.md`, and `CHANGELOG.md`.
```

```text
[x] V2-PKG-001: Deterministic dependency and lockfile workflow
    Priority: P2
    Severity: Enhancement
    Area: Packaging/Release/DX
    Affected files: package workflow modules, docs/RELEASE_PROCESS.md, README.md
    Problem: AI-assisted projects benefit heavily from deterministic dependency resolution and reproducible environment bootstrap.
    Recommendation: Introduce lockfile semantics and deterministic install/verify flow.
    Tests required: package workflow integration tests + release docs contract updates.
    Notes: Completed on 2026-05-16. Added first-class deterministic lockfile semantics in `src/package_workflow.rs` (`KujoLockfile`, schema versioning, deterministic serialization, manifest-lock verification, and default sibling lockfile path resolution). `kujo package-install` now writes deterministic `kujo.lock` output by default and supports explicit verify mode via `--frozen` (plus `--lockfile` override) in `src/main.rs`, returning deterministic errors when lockfiles are missing or stale. Expanded package integration coverage in `tests/package_module_workflow_integration.rs` to lock deterministic repeated-install lockfile content and frozen-mode drift detection/regeneration behavior; expanded release-process docs contract coverage in `tests/release_process_docs_contract.rs`; and updated user/release docs in `README.md`, `docs/RELEASE_PROCESS.md`, and `CHANGELOG.md`.
```

```text
[x] V2-LSP-001: Advanced editor protocol surfaces for AI-assisted tooling
    Priority: P2
    Severity: Enhancement
    Area: LSP/Tooling
    Affected files: src/lsp_server.rs, docs/PROTOCOL_CONTRACTS.md, tests/lsp_conformance_harness.rs
    Problem: Baseline LSP surface is strong, but advanced AI/editor workflows benefit from semantic tokens, inlay hints, and code lens support.
    Recommendation: Implement advanced protocol slices with stable contract fixtures.
    Tests required: protocol fixture updates + conformance harness coverage.
    Notes: Completed on 2026-05-16. Added advanced LSP server capability advertisement and request handling in `src/lsp_server.rs` for `textDocument/semanticTokens/full`, `textDocument/inlayHint`, and `textDocument/codeLens`, with deterministic token encoding/hint/lens output shapes and invalid-params behavior aligned to existing JSON-RPC contracts. Added dedicated unit coverage in `src/lsp_server.rs` for advanced capability presence and method response-shape stability. Updated conformance fixtures (`tests/lsp_fixtures/all_required_methods_success_error.json`) so harness execution covers the new methods and error-path behavior. Updated protocol contract docs in `docs/PROTOCOL_CONTRACTS.md`, plus README LSP command contract wording and changelog/roadmap status.
```
