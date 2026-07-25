# Kujo Bug Sweep Audit Scope

Date: 2026-07-25
Purpose: head-start map for a focused bug sweep, vulnerability review, or bounty-style cleanup audit.

This is a scoping guide, not a safety claim. Kujo is a local-first language runtime with ambient host-effect APIs in trusted mode. Reviewers should treat any path that executes user-provided Kujo source, opens files, spawns processes, accepts or initiates network traffic, talks to AI providers, or loads local extension manifests as security-sensitive.

## Triage Priority

Start with these areas before general cleanup:

1. Runtime host-effect boundary: `src/interpreter/capabilities.rs`, `src/main.rs`, `src/interpreter/mod.rs`, `src/vm.rs`, and `src/interpreter/native_functions/*`.
2. AI, HTTP, and network egress: `src/interpreter/native_functions/http.rs`, `src/network_policy.rs`, `src/http_request_utils.rs`, `src/interpreter/native_functions/network.rs`, and async HTTP paths in `src/interpreter/native_functions/async_ops.rs`.
3. Filesystem/archive/static-serving path safety: `src/interpreter/native_functions/filesystem.rs`, `src/interpreter/native_functions/io.rs`, `src/path_security.rs`, `src/serve_http.rs`, and SSG helpers in `src/interpreter/native_functions/async_ops.rs`.
4. Process, shell, environment, and secret handling: `src/interpreter/native_functions/system.rs`, `src/interpreter/value.rs`, and capability wiring in `src/interpreter/capabilities.rs`.
5. Database integrations: `src/interpreter/native_functions/database.rs` and database value/storage types in `src/interpreter/value.rs`.
6. Parser/compiler/VM execution surfaces: `src/lexer.rs`, `src/parser.rs`, `src/compiler.rs`, `src/vm.rs`, `src/interpreter/mod.rs`, `src/module.rs`, and `src/runtime_limits.rs`.
7. JIT unsafe boundary: `src/jit.rs`, `src/jit_disabled.rs`, `scripts/check_jit_safety_contracts.sh`, and `tests/jit_*`.
8. Extension and developer tooling surfaces: `src/workflow_pack/*`, `src/docgen/*`, `src/lsp_*`, `src/lsp_server.rs`, `tools/vscode-kujo-extension/extension.js`, and package workflow files.

## High-Touch Runtime Surfaces

### CLI Entrypoint And Policy Wiring

- `src/main.rs` is the central command router and capability policy constructor.
- High-frequency commands: `run`, `check`, `test`, `test-run`, `format`, `lint`, `repl`, LSP helper commands, `docgen`, `serve`, package commands, benchmark/profile commands, `pack run`, and `doctor`.
- Review focus:
  - Capability flag parsing: `--untrusted`, `--allow-all`, `--allow-fs-*`, `--allow-process-exec`, `--allow-shell-exec`, `--allow-env-*`, `--allow-net-*`, `--allow-ai`, `--allow-database`, `--allow-clock`, and `--allow-random`.
  - Interaction between explicit allow flags and trusted defaults.
  - `--deny-private-net` environment mutation and restoration.
  - Exit code and stdout/stderr contracts, especially JSON modes.
  - External workflow-pack alias routing and reserved namespace rejection.

### VM, Interpreter, And Execution Semantics

- `src/vm.rs` is the default runtime and one of the largest, highest-frequency files.
- `src/interpreter/mod.rs` is the tree-walking fallback/debug runtime and contains native registration plus interpreter-only server/callback paths.
- `src/compiler.rs`, `src/bytecode.rs`, and `src/optimizer.rs` bridge parsed source to VM execution.
- Review focus:
  - Runtime parity gaps between VM and interpreter.
  - Stack depth, recursion, loop, closure, module import, and async behavior.
  - Native-call dispatch and error propagation.
  - Panics reachable from user scripts.
  - Resource exhaustion through large collections, strings, recursion, bytecode loops, async task fanout, or oversized serialized values.

### Language Frontend

- `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/errors.rs`, `src/type_checker.rs`, `src/formatter.rs`, and `src/linter.rs`.
- Review focus:
  - Input-size limits and parser diagnostic paths.
  - Invalid UTF-8 or malformed escape handling.
  - Deep nesting, ambiguous syntax, and parse recovery behavior.
  - Formatter/linter behavior on malformed or attacker-controlled source.
  - Consistency between parsed optional typing and deferred enforcement claims.

## Host-Effect Vulnerability Surfaces

### AI Runtime

- Primary files: `src/interpreter/native_functions/http.rs`, `src/interpreter/native_functions/token.rs`, `src/interpreter/native_functions/schema.rs`, `src/interpreter/native_functions/vector.rs`, `src/network_policy.rs`, and `src/interpreter/value.rs`.
- User-facing functions include `ai_chat`, `ai_stream_chat`, `ai_embedding`, `ai_tool_loop`, `ai_request_hash`, `ai_text`, `ai_image_url`, `ai_message`, `ai_count_tokens`, `ai_fit_context`, `json_schema_validate`, `secret`, `reveal`, and `is_secret`.
- Environment variables: `KUJO_AI_ALLOWED_ENDPOINTS`, `KUJO_AI_REPLAY`, `KUJO_AI_REPLAY_MODE`, and `KUJO_AI_RECORD`.
- Review focus:
  - `--allow-ai` enforcement separate from `--allow-net-client`.
  - Endpoint allowlist matching by scheme, host, port, and path prefix.
  - Replay/record cassette path handling, overwrite behavior, and accidental live-network fallthrough.
  - Secret redaction in printing, JSON serialization, debug output, structured errors, replay cassettes, and streamed chunks.
  - `ai_tool_loop` tool-call/result matching and max-step enforcement.
  - Response parsing, non-JSON responses, body excerpts, provider error leakage, and retry metadata handling.

### HTTP Client, TCP, UDP, And Egress Policy

- Primary files: `src/interpreter/native_functions/http.rs`, `src/interpreter/native_functions/network.rs`, `src/interpreter/native_functions/async_ops.rs`, `src/network_policy.rs`, and `src/http_request_utils.rs`.
- User-facing functions include `http_get`, `http_post`, `http_put`, `http_delete`, `http_request`, `http_get_binary`, `http_get_stream`, `parallel_http`, `async_http_get`, `async_http_post`, `tcp_connect`, `tcp_listen`, `tcp_accept`, `tcp_send`, `tcp_receive`, `udp_bind`, `udp_send_to`, and `udp_receive_from`.
- Review focus:
  - SSRF/private-network protections under `KUJO_NET_DESTINATION_POLICY=deny_private` and `--deny-private-net`.
  - DNS resolution race behavior and IPv4/IPv6 private range classification.
  - URL scheme enforcement and malformed URL handling.
  - Request/response size limits, streaming limits, timeout application, and cancellation behavior.
  - Header handling and whether user-provided headers can override safety-sensitive defaults.
  - Parallel request fanout limits and worker-thread panic handling.

### Servers And Inbound Request Handling

- Static server: `src/serve_http.rs`.
- Script-level HTTP server: `src/interpreter/mod.rs`, `src/vm.rs`, and `src/interpreter/native_functions/http.rs`.
- User-facing functions include `http_server`, route registration methods, `http_response`, `json_response`, `html_response`, `redirect_response`, `set_header`, and `set_headers`.
- Review focus:
  - Bind host/port defaults and `--allow-net-server` enforcement.
  - Path traversal, percent-encoding, double-encoding, null-byte, hidden/private file, and symlink cases.
  - TLS certificate/key loading and error paths.
  - Request-line, header, body, timeout, and connection limits.
  - ETag/range handling, MIME detection, active-content sniffing, and cache headers.
  - Raw `html_response` XSS risk when scripts interpolate untrusted input.
  - Route matching, callback isolation, captured closures, and VM/interpreter differences.

### Filesystem, Paths, Archives, Images, And SSG

- Primary files: `src/interpreter/native_functions/filesystem.rs`, `src/interpreter/native_functions/io.rs`, `src/path_security.rs`, `src/interpreter/native_functions/async_ops.rs`, and `src/runtime_limits.rs`.
- User-facing functions include `read_file`, `read_file_lossy`, `read_binary_file`, `read_lines`, `write_file`, `append_file`, `write_binary_file`, `create_dir`, `rename_file`, `copy_file`, `delete_file`, `list_dir`, `path_*`, `io_*`, `zip_create`, `zip_add_file`, `zip_add_dir`, `zip_close`, `unzip`, `load_image`, `gif_to_webp`, and SSG helpers.
- Review focus:
  - Capability separation among read, write, and delete.
  - Whole-file size caps and async batch behavior.
  - Atomic write and overwrite semantics.
  - Archive Slip controls: absolute paths, `..`, drive prefixes, null bytes, symlink entries, entry count, per-entry size, and total uncompressed size.
  - ZIP writer lock poisoning and graceful error paths.
  - External `gif2webp` process invocation.
  - Image decoder attack surface from enabled formats.
  - SSG Markdown/link escaping and generated-output path containment.

### Process, Shell, Environment, And Secrets

- Primary files: `src/interpreter/native_functions/system.rs`, `src/interpreter/value.rs`, and `src/interpreter/capabilities.rs`.
- User-facing functions include `execute`, `execute_status`, `spawn_process`, `pipe_commands`, `env`, `env_or`, `env_required`, `env_list`, `env_set`, `secret`, `reveal`, and `is_secret`.
- Review focus:
  - `--allow-shell-exec` versus `--allow-process-exec` separation.
  - Shell injection risks in examples and docs.
  - Process timeout, output truncation, environment allow/deny behavior, inherited environment defaults, and process-group termination.
  - Redaction consistency for secrets crossing logs, errors, JSON output, and spawned process args/env.
  - Unix-specific unsafe/process-group behavior in `system.rs`.

### Databases

- Primary files: `src/interpreter/native_functions/database.rs` and `src/interpreter/value.rs`.
- Backends: SQLite, PostgreSQL, and MySQL when `runtime-db` is enabled.
- User-facing functions include `db_connect`, `db_execute`, `db_query`, `db_close`, `db_pool`, `db_pool_acquire`, `db_pool_release`, `db_pool_stats`, `db_pool_close`, `db_begin`, `db_commit`, `db_rollback`, and `db_last_insert_id`.
- Review focus:
  - `--allow-database` enforcement across connection, query, pool, and transaction helpers.
  - Parameter binding correctness across SQLite/Postgres/MySQL.
  - Connection string secret leakage in error messages.
  - Pool lifecycle, poisoned locks, transaction rollback on close/drop, and cross-thread safety.
  - SQLite path authority and whether database access should also imply filesystem read/write risk in operator guidance.

### Crypto, JWT, OAuth, And Password Helpers

- Primary files: `src/interpreter/native_functions/crypto.rs`, `src/interpreter/native_functions/http.rs`, and `src/interpreter/value.rs`.
- User-facing functions include `sha256`, `sha256_file`, `md5`, `md5_file`, `hash_password`, `verify_password`, AES/RSA helpers, `jwt_encode`, `jwt_decode`, `jwt_verify`, `oauth2_auth_url`, and `oauth2_get_token`.
- Review focus:
  - Key/nonce handling, algorithm defaults, padding/signature verification behavior, and error messages.
  - Password hash cost and denial-of-service implications.
  - JWT algorithm confusion, claim validation boundaries, and secret redaction.
  - OAuth token exchange egress policy and client secret handling.
  - MD5 availability as a non-security hash versus possible misuse in examples/docs.

## Extension, Tooling, And Supply-Chain Scope

### Packages And Module Resolution

- Primary files: `src/package_workflow.rs`, `src/module.rs`, `src/reserved_names.rs`, `config/reserved_names.toml`, and `modules/cli.kujo`.
- Review focus:
  - `kujo.toml` and `kujo.lock` parsing, deterministic serialization, and frozen install behavior.
  - Reserved package/namespace enforcement.
  - Module search roots, dotted imports, package-root awareness, symlink/path traversal behavior, and circular import diagnostics.
  - Clear boundary that v1 package scope is local manifest/lockfile only, with no public registry transport.

### Workflow Packs And External Commands

- Primary files: `src/workflow_pack/*`.
- Review focus:
  - Pack discovery from project-local `.kujo/packs`, user-local packs, and `KUJO_PACK_PATH`.
  - Manifest parsing and command entry path containment.
  - Distinction between `.kujo` command execution and native executable command execution.
  - JSON output parsing, renderer behavior, and reserved alias blocking.
  - Trust model clarity: local packs are code execution surfaces.

### Docgen And Link Validation

- Primary files: `src/docgen/*`.
- Review focus:
  - Recursive file discovery limits: max file size, max depth, max files, invalid encoding.
  - Parser-assisted extraction on untrusted source.
  - Markdown/HTML rendering and escaping.
  - Link validation behavior, external network checks, private-network link policy, cache directory handling, and generated output paths.

### LSP And Editor Integration

- Primary files: `src/lsp_server.rs`, `src/lsp_*`, `tools/vscode-kujo-extension/extension.js`, and docs under `docs/editor-adapters/`.
- Review focus:
  - JSON-RPC framing, content-length parsing, request timeout behavior, cancellation, and malformed payload handling.
  - Workspace symbol, rename, references, hover, diagnostics, completion, and formatting on malformed source.
  - File read behavior in CLI helper commands and editor extension command execution.

### Benchmarks, Profiling, Release Scripts, And Generated Artifacts

- Primary files: `src/benchmarks/*`, `benches/*`, `benchmarks/cross-language/*`, `scripts/*`, `docs/generated/*`, and `tests/generated_artifact_freshness_contract.rs`.
- Review focus:
  - Temporary directory and artifact cleanup.
  - Python subprocess benchmark bridges and shell scripts.
  - Publication claims and benchmark reproducibility.
  - Generated inventories staying fresh: unsafe inventory, TODO triage, VM mismatch inventory, and release gates.

## Existing Security Evidence To Reuse

Auditors should read these before starting targeted scans:

- `docs/NATIVE_API_SECURITY_POSTURE.md`
- `docs/SECURE_AI_SCRIPTING.md`
- `docs/SECURITY_RESPONSE.md`
- `docs/AI_RUNTIME.md`
- `docs/STANDARD_LIBRARY.md`
- `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
- `docs/WORKFLOW_PACKS.md`
- `docs/generated/UNSAFE_INVENTORY.md`
- `docs/generated/V1_CODE_TODO_TRIAGE.md`
- `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
- `docs/V1_0_HARDENING_AND_LEANNESS_CHECKLIST.md`

High-signal tests and gates:

- `tests/runtime_security.rs`
- `tests/native_api_security_boundaries.rs`
- `tests/security_posture_docs_contract.rs`
- `tests/ai_replay_hermeticity_contract.rs`
- `tests/serve_command_integration.rs`
- `tests/runtime_path_matrix_contract.rs`
- `tests/package_module_workflow_integration.rs`
- `tests/docgen_universal.rs`
- `tests/lsp_conformance_harness.rs`
- `tests/lsp_latency_guardrails.rs`
- `tests/jit_safety_contract_checker.rs`
- `tests/unsafe_inventory_contract.rs`
- `tests/release_dependency_advisory_contract.rs`
- `scripts/unsafe_safety_gate.sh`
- `scripts/release_candidate_gate.sh`
- `scripts/enterprise_verify.sh`
- `scripts/repo_hygiene_audit.sh`

## Suggested Audit Workstreams

1. Capability bypass review: verify every host-effect native function has correct primary and additional capability metadata, and that VM/interpreter/native dispatch cannot bypass it.
2. Egress and SSRF review: fuzz HTTP/TCP/UDP/AI destinations, private ranges, DNS edge cases, redirects, URL encodings, and endpoint allowlists.
3. Filesystem/path/archive review: fuzz relative paths, symlinks, hidden files, null bytes, Windows drive prefixes, ZIP entries, and output directory joins.
4. Runtime panic/resource review: run fuzzers and targeted stress tests for parser, VM, interpreter, async, collections, JSON/schema, and native helpers.
5. Secret-leak review: trace `Value::Secret` through display, debug, JSON, errors, AI cassettes, HTTP headers, process execution, database errors, and logs.
6. Server review: fuzz `kujo serve` and script HTTP server request parsing, routing, headers, body limits, ranges, MIME behavior, TLS startup, and shutdown behavior.
7. Supply-chain/local-extension review: inspect package workflows, module loading, workflow-pack discovery/execution, scripts, generated artifacts, and editor tooling.
8. Unsafe/JIT review: validate each unsafe boundary in `src/jit.rs` and Unix process-control unsafe blocks in `src/interpreter/native_functions/system.rs`; run the unsafe safety gate.

## Minimum Commands For A First Pass

```bash
cargo fmt --check
cargo check
cargo test --test runtime_security
cargo test --test native_api_security_boundaries
cargo test --test ai_replay_hermeticity_contract
cargo test --test serve_command_integration
cargo test --test cli_contracts
cargo test --test cli_json_contracts
cargo test --test docgen_universal
cargo test --test lsp_conformance_harness
cargo test --test package_module_workflow_integration
cargo test --test unsafe_inventory_contract
bash scripts/unsafe_safety_gate.sh
bash scripts/release_candidate_gate.sh
```

For broader runtime confidence, add:

```bash
cargo test
cargo run -- test --runtime vm
cargo run -- test --runtime dual
bash scripts/enterprise_verify.sh --minimal
```
