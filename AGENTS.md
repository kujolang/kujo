# Kujo Agent Guide

Use this file as the first stop for agents working in this repository or building projects in the Kujo programming language. It is intentionally compact, operational, and opinionated. Prefer the linked canonical docs over guessing.

## Mission And Philosophy

Kujo is the programming language for AI-native software, local-first automation, deterministic workflows, and practical application scripting. The runtime is built in Rust, but Kujo projects and Kujo ecosystem tools should be written in Kujo wherever Kujo can reasonably do the job.

Core principles:

- Local-first: workflows should run from source, committed fixtures, local files, and deterministic lockfiles before depending on hosted services.
- Deterministic by default: stable inputs should produce stable outputs, stable JSON shapes, stable exit codes, and reproducible `kujo.lock` snapshots.
- Decentralized and portable: avoid designs that require a central hosted control plane, public registry, provider-specific runtime, or live network path unless the feature explicitly requires it.
- Capability-aware: treat filesystem, process, network, AI, database, clock, and random access as explicit effects, especially for untrusted scripts.
- Kujo-native ecosystem: when building tools, agents, packages, connectors, bridges, scaffolds, examples, or automation for the Kujo language, implement them in Kujo first. Do not build project bridges, language adapters, workflow glue, or connectors in Python by default. Use Python only for repository-maintenance scripts when no Kujo or Rust path is practical, and document why.
- Mechanism before policy: core Kujo owns deterministic primitives. Provider routing, retries, RAG, agent frameworks, eval policy, registries, and observability belong in ecosystem packages unless the roadmap says otherwise.
- Agent-readable contracts: prefer structured APIs, JSON contracts, fixtures, and concise diagnostics over brittle human parsing.

## Canonical Reading Order

1. `README.md`: status, install, quick start, runtime recommendations, and validation commands.
2. `docs/LANGUAGE_SPEC.md`: syntax, semantics, parser limits, mutability, scope, functions, modules, and compatibility policy.
3. `docs/STANDARD_LIBRARY.md`: builtin inventory, arity, capability gates, and native API contracts.
4. `docs/AI_RUNTIME.md`: deterministic AI hashes, replay cassettes, egress controls, secrets, vectors, schemas, and token estimation.
5. `docs/CLI_MACHINE_READABLE_CONTRACTS.md`: JSON output, exit codes, diagnostics, and contract-change rules.
6. `docs/ARCHITECTURE.md`: lexer/parser/AST, VM, interpreter, native surfaces, CLI, LSP, docgen, and package workflow.
7. `docs/NATIVE_API_SECURITY_POSTURE.md`: trust model and effect-capability boundaries.
8. `examples/README_examples.md`: current examples, showcases, and legacy/expected-fail examples.
9. `tests/docs_examples.rs`: executable policy for which examples should run, parse, skip, or fail.

For release state, read `ROADMAP.md`, `docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md`, and `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`. Kujo `v1.2.2` is the current stable release, and `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` is retained as historical launch evidence.

## Repository Map

- `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/errors.rs`: frontend and diagnostics.
- `src/compiler.rs`, `src/bytecode.rs`, `src/vm.rs`: bytecode compiler and default VM runtime.
- `src/interpreter/*`: tree-walking interpreter, runtime values, environment, async runtime, test runner, native dispatch.
- `src/interpreter/native_functions/*`: standard library and host-effect builtin implementations.
- `src/main.rs`: CLI command parsing and dispatch.
- `src/cli_output.rs`: human output helpers and deterministic rendering surfaces.
- `src/lsp_*`: LSP helper commands and server features.
- `src/docgen/*`: universal documentation generation pipeline and language adapters.
- `src/package_workflow.rs`, `modules/cli.kujo`: package and module workflow support.
- `tests/`: Rust contract tests, diagnostics goldens, runtime fixtures, CLI JSON contracts, parity checks, and example smoke policy.
- `examples/`: canonical learning examples, showcase tools, expected-fail fixtures, and benchmark examples.
- `docs/`: language, security, standard library, AI runtime, CLI contracts, release process, architecture, and generated reports.
- `scripts/`: release gates, inventory helpers, and local artifact scripts.
- `notes/`: historical implementation notes. Use for context, not as the current contract.

Avoid searching `target/**`, `docs/generated/**`, and `examples/ssg/content/**` unless the task targets generated reports, build artifacts, or SSG fixtures.

## Runtime Model

Kujo source files use `.kujo`. Source text is UTF-8. CLI parse entrypoints reject source files larger than `1,048,576` bytes with parser diagnostics.

Execution pipeline:

```text
.kujo source
  -> lexer
  -> parser
  -> AST
  -> compiler + bytecode
  -> VM runtime by default
```

Use `kujo run` for default VM execution. Use `kujo run --interpreter` only for explicit fallback/debug cases or documented interpreter-only coverage. `kujo test` supports `--runtime dual|vm|interpreter`; default `dual` is VM-primary with bounded interpreter fallback for fixture drift. `kujo test-run` uses the interpreter-hosted test framework path.

Runtime parity and known divergences live in `docs/VM_INTERPRETER_PARITY_MATRIX.md`. Do not assume interpreter-only behavior is valid VM behavior.

## Language Style For Kujo Code

Prefer current syntax from `docs/LANGUAGE_SPEC.md` and canonical examples. Important rules:

- Bind with `let`, `mut`, or `const`: `let name := value`, `mut count := 0`, `const version := "v1"`.
- Use `mut` for reassigned variables and mutable containers.
- Assignment operators are statement-level only: `:=`, `=`, `+=`, `-=`, `*=`, `/=`, `%=`. Chained assignment is invalid.
- Functions use `func name(args) { ... }`; imported functions must be declared with `export func`.
- `if`, `while`, `loop`, `for item in items`, `match`, `try/except`, `async`, `await`, and `spawn` are supported according to the spec.
- Arrays and dictionaries support spread in literal element positions.
- Unknown identifiers are runtime errors. Quote strings explicitly.
- `let` and `const` bindings cannot be reassigned or mutated through that binding. Inner-scope shadowing is allowed.
- Predicate helpers such as `contains`, `starts_with`, and `has_key` may return `1`/`0`; compare explicitly when needed.
- Collection helpers such as `push` return a new value; reassign when keeping the update.
- Prefer small local output helpers such as `print_lines(lines)`, `section(title)`, `kv(label, value)`, `ok(message)`, and `fail(message)` over long blocks of repeated `print(...)`.

Start examples from:

- `examples/hello.kujo`
- `examples/test_print.kujo`
- `examples/string_interpolation.kujo`
- `examples/test_if_else.kujo`
- `examples/for_loops.kujo`
- `examples/arrays.kujo`
- `examples/dictionaries.kujo`
- `examples/test_simple_func.kujo`
- `examples/basic_import.kujo`
- `examples/file_logger.kujo`

Do not copy files listed under "Legacy or Expected-Fail Examples" in `examples/README_examples.md` until they are repaired and removed from `tests/docs_examples.rs`.

## Building Kujo Projects

For new Kujo-language projects, keep the implementation Kujo-native:

- Use `.kujo` entrypoints and modules.
- Use Kujo's standard library for filesystem, JSON, HTTP, process, crypto, database, vectors, schemas, and AI runtime primitives.
- Use `kujo init`, `kujo package-add`, `kujo package-install`, and `kujo package-install --frozen` for deterministic local manifest and lockfile workflows.
- Commit `kujo.toml` and `kujo.lock` when package determinism matters.
- Prefer explicit JSON config and schema validation over implicit environment behavior.
- Use replay cassettes for AI examples and tests instead of live provider calls.
- Keep adapters and connectors in Kujo unless they need to modify the Rust runtime itself.
- Avoid Python bridge scripts for Kujo project behavior. If a non-Kujo helper is unavoidable, prefer Rust for runtime/tooling internals and explain the exception in docs.

Current package scope is local manifest and lockfile determinism only. Do not imply a public Kennel registry or package publish transport exists unless the repository gains that feature.

## Determinism And Automation Contracts

Automation-facing CLI behavior is locked in `docs/CLI_MACHINE_READABLE_CONTRACTS.md` and tests.

Exit codes:

- `0`: success
- `1`: generic command failure or unmet gate
- `2`: CLI usage/argument parse error
- `3`: lexer/parser diagnostic failure
- `4`: runtime execution/semantic failure
- `5`: I/O failure
- `6`: internal/tooling failure

JSON policy:

- For successful `--json` commands, read machine JSON from `stdout`.
- For failures, treat non-zero exit as authoritative and capture `stderr`.
- Documented exceptions emit JSON failures on `stdout` for `kujo run --json-runtime-diagnostics` and `kujo lsp-rename --json`.
- Any payload-affecting JSON change must update docs, tests, and `CHANGELOG.md` in the same change.

When adding automation, prefer commands and outputs that are easy to assert in tests: no ambient color, stable field names, deterministic ordering, explicit limits, and no live network dependency.

## AI Runtime Guidance

Kujo core AI primitives are deterministic mechanisms:

- `ai_request_hash` computes credential-independent SHA-256 request hashes without network I/O.
- `ai_chat`, `ai_stream_chat`, `ai_embedding`, and `ai_tool_loop` support record/replay cassettes.
- `ai_text`, `ai_image_url`, and `ai_message` build portable multimodal messages.
- `ai_count_tokens` and `ai_fit_context` provide deterministic estimates, not provider-exact billing counts.
- `json_schema_validate` validates model output and ordinary JSON-like data locally.
- `vec_dot`, `vec_norm`, `vec_normalize`, `vec_cosine`, and `vec_top_k` provide local embedding-style math only; vector stores and retrieval policy belong outside core.
- `secret`, `reveal`, and `is_secret` keep secrets redacted unless explicitly revealed.

For tests and examples, prefer:

```bash
KUJO_AI_REPLAY=tests/fixtures/ai_cassettes \
KUJO_AI_REPLAY_MODE=strict \
kujo run examples/ai_enterprise_replay_showcase.kujo
```

Strict replay is hermetic and should not open sockets. Review committed cassettes because model outputs may still be sensitive even when credentials are redacted.

## Security And Capabilities

Kujo is not a sandbox. Running Kujo code is equivalent to running local code with the current process privileges unless capability restrictions and external isolation are applied.

Trusted/default:

- `kujo run` and `kujo test-run` default to trusted mode when no capability flags are provided.
- Trusted mode enables ambient host-effect APIs.

Restricted:

- Use `--untrusted` for deny-by-default host effects.
- Add only required `--allow-*` flags.
- Explicit `--allow-*` flags imply a restricted baseline with only requested capabilities enabled.
- Treat `--allow-all` as trusted mode.

Important capabilities:

- `--allow-fs-read`, `--allow-fs-write`, `--allow-fs-delete`
- `--allow-process-exec`, `--allow-shell-exec`
- `--allow-env-read`, `--allow-env-write`
- `--allow-net-client`, `--allow-net-server`, `--allow-net`
- `--allow-ai`
- `--allow-database`
- `--allow-clock`, `--allow-random`

Guidance:

- Prefer `spawn_process` argv arrays over shell strings.
- Do not pass untrusted input to `execute` or `execute_status`.
- Use `--deny-private-net` when outbound HTTP/TCP/UDP calls must reject private, loopback, link-local, multicast, and unspecified destinations.
- Prefer `--allow-ai` over broad `--allow-net-client` for AI-only egress.
- Set `KUJO_AI_ALLOWED_ENDPOINTS` in shared automation.
- Keep secrets out of source files; wrap runtime secret strings with `secret(...)`.
- Treat `html_response(...)` as raw HTML. Escape untrusted content or return JSON.

## Development Workflow

Before editing:

- Check `git status --short` and preserve user changes.
- Read the smallest canonical docs and code paths needed for the task.
- Use `rg` first for search:

```bash
rg "pattern" src tests docs examples \
  -g '!target/**' \
  -g '!docs/generated/**' \
  -g '!examples/ssg/content/**'
```

Useful searches:

```bash
rg "expected_fail_examples" tests examples
rg "Diagnostic" src tests/fixtures/diagnostics
rg "lsp-" src tests docs
rg "bench-ssg|BenchSsg" src tests docs
```

Change rules:

- Preserve behavior unless the issue, checklist, or contract explicitly requires behavior change.
- Update tests or docs with every source change.
- Do not blindly update snapshots; inspect expected versus actual output first.
- Do not redesign language syntax during cleanup tasks.
- Avoid broad refactors of `src/vm.rs`, `src/jit.rs`, or interpreter internals unless a specific task requires it.
- Prefer small output helpers and renderer tests before moving CLI formatting code.
- Keep generated files out of manual edits unless the task targets generation.

## Validation

Use targeted tests after each logical change. For broad language/runtime/readability work, run as much of this gate as practical:

```bash
cargo fmt --check
cargo check
cargo test
cargo test --test docs_examples
cargo test --test readme_contracts
cargo test --test cli_contracts
cargo test --test cli_json_contracts
cargo test --test diagnostics_golden
cargo run -- test --runtime vm
cargo run -- test --runtime dual
```

For docs-only changes, at minimum run `cargo fmt --check` and any affected docs/example contract tests when snippets or examples changed. If validation is skipped, state exactly why.

## Release And Claims

Keep release claims bounded by the shipped artifacts, documented v1 scope, verification evidence, and explicit compatibility policy. Do not broaden the stable-release claim into unsupported enterprise, performance, package-registry, or sandbox guarantees.

Keep release-impacting changes tied to:

- `ROADMAP.md`
- `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`
- `docs/V1_SCOPE.md`
- `docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md`
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`
- `CHANGELOG.md`

## Agent Output Discipline

When making changes, keep final responses short and evidence-based: what changed, what passed, and what remains unresolved. Do not bury the user in implementation narration. Save durable handoff details to the configured memory system when repository instructions require it.
