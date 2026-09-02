# Kujo

[![Version](https://img.shields.io/badge/version-1.2.0-black)](https://github.com/kujolang/kujo/releases/tag/v1.2.0)
[![License](https://img.shields.io/badge/license-MIT-lightgrey)](LICENSE)
[![built with Rust](https://img.shields.io/badge/built%20with-Rust-white.svg)](https://www.rust-lang.org/)

Kujo is the programming language for AI-native software, built in Rust.

It is designed for local-first automation, agentic workflows, and application scripting where deterministic behavior, strong native capabilities, and practical ergonomics matter.

Kujo is VM-first (`kujo run`), with a tree-walking interpreter available as an explicit fallback/debug path.

## Current Status

- Kujo is usable from source today.
- VM runtime parity for modular workflows has been significantly hardened.
- Dotted module import workflows are supported on the default VM path.
- Package workflows are deterministic: `kujo init`, `kujo package-add`, `kujo package-install`, and `kujo package-install --frozen` work with nested source layouts and reproducible `kujo.lock` snapshots.
- Kujo v1.0 package scope is local manifest and lockfile determinism only; it does not include a public Kennel registry or package publish transport.
- Core AI-native runtime mechanisms are implemented for deterministic request hashing, offline record/replay cassettes, structured response metadata, JSON Schema validation, vector math, token budgeting, runtime secret redaction, dedicated AI egress capability controls, streaming callbacks, and multimodal message builders.
- Native helper coverage has expanded for everyday scripting work: hashing (`sha256`, `sha256_file`, `md5`), file inspection (`read_file_lossy`, `path_is_symlink`), formatting (`pad_start`, `pad_end`), introspection (`type_of`, `is_truthy`), and stderr output (`eprint`) are all available without shelling out.
- Runtime-generated sequence and string helpers now reject unsafe edge cases such as non-finite range bounds, reversed random bounds, negative string widths/counts, and oversized generated outputs instead of panicking or attempting unbounded allocation.
- Native capability controls are available for trusted and untrusted execution modes.
- Kujo `v1.2.0` is the current stable release, with prebuilt binaries and checksums published through GitHub Releases.

## Why Kujo

- VM-first execution for predictable runtime behavior in local and production scripts.
- Practical native APIs (filesystem, process, network, async, crypto, database).
- The native standard library keeps growing with practical helper surfaces for checksums, padding, truthiness, file inspection, and stderr-friendly output, which makes common automation scripts more self-contained.
- Security policy controls for trusted and untrusted execution.
- Strict outbound network policy can be forced from the CLI with `--deny-private-net`.
- Bounded native helper behavior for generated arrays, ranges, random IDs, and string expansion.
- Native Markdown/SSG helpers escape generated HTML by default.
- Module workflows that support both flat and dotted imports.
- Package bootstrap and lockfile workflows that stay deterministic across repeated installs.
- Strong diagnostics, contract tests, and release-gate automation.
- Core surfaces for `doctor`, `docgen`, and machine-readable CLI contracts keep the language agent-readable.

## AI-Native Runtime Snapshot

Kujo's core AI features are mechanism-first primitives for scripts and libraries:

- `ai_chat`, `ai_stream_chat`, `ai_embedding`, and `ai_tool_loop` share deterministic request parsing, replay cassettes, response envelopes, and redacted error handling.
- `ai_request_hash` gives credential-independent cache and cassette keys without network I/O.
- `ai_stream_chat(..., on_chunk)` supports replay-backed chunk callbacks and cancellation by returning `false`.
- `ai_text`, `ai_image_url`, and `ai_message` build portable multimodal message dictionaries accepted by the AI helpers.
- `ai_count_tokens` and `ai_fit_context` provide deterministic prompt-budget estimates without provider tokenizers.
- `secret`, `reveal`, and `is_secret` keep API keys and other runtime secrets redacted unless code explicitly reveals them.
- `json_schema_validate` plus `vec_*` helpers give local validation and embedding math building blocks without adding provider routing, RAG, agents, MCP, eval, observability, or registry policy to core.

See [docs/AI_RUNTIME.md](docs/AI_RUNTIME.md) and [docs/STANDARD_LIBRARY.md](docs/STANDARD_LIBRARY.md) for contracts and examples.

## 1.0 Release Status

Release boundary: Kujo `v1.2.0` is the current stable release; explicit deferrals and compatibility guarantees are governed by `docs/V1_SCOPE.md`.

- The source tree is currently at `1.2.1` in `Cargo.toml`; the latest published stable release tag is `v1.2.0`.
- Prebuilt Linux x64, macOS x64/arm64, and Windows x64 binaries are distributed with per-asset SHA-256 files and a consolidated `checksums.txt`.
- [docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md](docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md) preserves the completed launch verification record.
- [ROADMAP.md](ROADMAP.md) remains the source of truth for historical v1 implementation tracking and post-1.0 planning.
- The supported v1 contract and explicit deferrals are documented in [docs/V1_SCOPE.md](docs/V1_SCOPE.md).
- Deferred/non-goal boundaries are tracked in [docs/V1_SCOPE.md](docs/V1_SCOPE.md) and [docs/OPTIONAL_TYPING_DESIGN.md](docs/OPTIONAL_TYPING_DESIGN.md).
- Release artifacts and verification evidence are tracked in [docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md](docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md).

## Safety Model Snapshot

- Kujo is not a sandbox.
- `kujo run` and `kujo test-run` default to trusted mode.
- For untrusted code, start with `--untrusted` and add only required `--allow-*` flags.
- Use `--deny-private-net` when outbound HTTP/TCP/UDP calls must reject local, private, link-local, multicast, and unspecified destinations even in trusted-mode automation.
- High-level AI helpers require the separate `--allow-ai` capability in untrusted mode; set `KUJO_AI_ALLOWED_ENDPOINTS` to restrict provider endpoints by scheme, host, optional port, and optional path prefix.
- When explicit `--allow-*` flags are present, execution is restricted to the listed capabilities.
- Review [docs/NATIVE_API_SECURITY_POSTURE.md](docs/NATIVE_API_SECURITY_POSTURE.md) before running untrusted scripts in shared or sensitive environments.

### Script Argument Separator

When passing script-level flags that may overlap with Kujo CLI flags (for example `--help`), use `--` to separate Kujo options from script arguments.

```bash
# Kujo options first, then "--", then script args
kujo run tool.kujo -- --help
kujo run tool.kujo -- summarize --format json
```

### Enterprise Hardening Quickstart

For untrusted scripts, use capability-minimal execution and explicit network intent:

```bash
kujo run --untrusted --allow-fs-read --allow-net-client script.kujo
```

When `--untrusted` and outbound network client or AI egress access are enabled, Kujo defaults the outbound destination policy to `deny_private` (unless `KUJO_NET_DESTINATION_POLICY` is already set). This helps reduce accidental private-network access in untrusted runs.

For AI-only egress, grant `--allow-ai` instead of general network client access. To pin calls to approved provider surfaces:

```bash
export KUJO_AI_ALLOWED_ENDPOINTS=https://api.example.test/v1,https://llm.example.internal/chat
kujo run --untrusted --allow-ai agent.kujo
```

To allow private/local destinations in trusted environments:

```bash
export KUJO_NET_DESTINATION_POLICY=allow_all
# or keep strict mode and permit local/private overrides per execution
export KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS=1
```

## Core Reference Links

- [ROADMAP.md](ROADMAP.md)
- [docs/LANGUAGE_SPEC.md](docs/LANGUAGE_SPEC.md)
- [docs/STANDARD_LIBRARY.md](docs/STANDARD_LIBRARY.md)
- [docs/AI_RUNTIME.md](docs/AI_RUNTIME.md)
- [docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md](docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md)
- [docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md](docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md)
- [docs/DOCGEN.md](docs/DOCGEN.md)
- [docs/CLI_MACHINE_READABLE_CONTRACTS.md](docs/CLI_MACHINE_READABLE_CONTRACTS.md)
- [AGENTS.md](AGENTS.md)
- [docs/SECURE_AI_SCRIPTING.md](docs/SECURE_AI_SCRIPTING.md)
- [docs/SECURITY_RESPONSE.md](docs/SECURITY_RESPONSE.md)
- [docs/INSTALL_MATRIX.md](docs/INSTALL_MATRIX.md)
- [docs/RELEASE_BINARIES.md](docs/RELEASE_BINARIES.md)
- [docs/FIRST_TOOL_COOKBOOK.md](docs/FIRST_TOOL_COOKBOOK.md)
- [docs/RELEASE_PROCESS.md](docs/RELEASE_PROCESS.md)
- [docs/VM_INTERPRETER_PARITY_MATRIX.md](docs/VM_INTERPRETER_PARITY_MATRIX.md)

For script ergonomics, see the output/report style guidance in [docs/FIRST_TOOL_COOKBOOK.md](docs/FIRST_TOOL_COOKBOOK.md) and [docs/STANDARD_LIBRARY_REFERENCE.md](docs/STANDARD_LIBRARY_REFERENCE.md).

## Install

This repository builds the Kujo language/runtime. If another `kujo` command is already installed on your system, prefer the full path to this repo's binary while testing so you do not confuse it with unrelated tools.

For the language plus the local-first tooling ecosystem, use the [ecosystem installer](docs/ECOSYSTEM_INSTALL.md). It defaults to the runtime, package, context, proof, and agent-operating tools; `--all` adds AI, quality, and showcase profiles.

```bash
git clone https://github.com/kujolang/kujo.git
cd kujo
cargo build --release
cargo install --path .
kujo --version
```

Development usage:

```bash
cargo run -- --help
cargo run -- run examples/hello.kujo
```

Install locally through Cargo:

```bash
cargo install --path .
kujo --version
```

After npm registry publication, Node.js 18+ users on a supported target can use
the lifecycle-script-free runtime package:

```bash
npm install --global @kujolang/kujo-runtime
kujo --version
```

See [docs/RELEASE_BINARIES.md](docs/RELEASE_BINARIES.md) for the native package
layout and the separate registry-publication gate.

Build a standalone local artifact for your current machine, with optional user-path install:

```bash
bash scripts/build_local_binary_artifact.sh --install
kujo --version
```

Windows PowerShell:

```powershell
pwsh -File scripts/build_local_binary_artifact.ps1 -Install
kujo --version
```

This local installer path currently supports:

- macOS Intel
- macOS Apple Silicon
- Linux x64
- Windows x64 via PowerShell

It builds a native binary on the current machine, then warns if the install directory is not on `PATH`.

## Quick Start

This first-ten-minutes path gives you a normal script, a replay-only AI example,
and the secure execution posture without requiring live provider credentials.

Create `hello.kujo`:

```kujo
func greet() {
    print("Kujo Kujo!")
}

greet()
```

Run it:

```bash
kujo run hello.kujo
```

Expected output:

```text
Kujo Kujo!
```

The same minimal program is tracked as `examples/hello.kujo`:

```bash
cargo run -- run examples/hello.kujo
```

Run the replay-only AI showcase. It uses committed cassettes, so it should not
open a live provider socket:

```bash
KUJO_AI_REPLAY=tests/fixtures/ai_cassettes \
KUJO_AI_REPLAY_MODE=strict \
cargo run -- run examples/ai_enterprise_replay_showcase.kujo
```

Check the same example without execution:

```bash
cargo run -- check examples/ai_enterprise_replay_showcase.kujo
```

For untrusted AI scripts, prefer AI-specific egress and an endpoint allowlist:

```bash
export KUJO_AI_ALLOWED_ENDPOINTS=https://api.example.test/v1
kujo run --untrusted --allow-ai script.kujo
```

Run the compact enterprise verification wrapper when reviewing product
readiness:

```bash
bash scripts/enterprise_verify.sh --minimal
```

### Next Program

Once the first run works, try a small report that uses functions, arrays, dictionaries, and branches:

```kujo
func total(values) {
    mut sum := 0
    for value in values {
        sum = sum + value
    }
    return sum
}

let scores := [8, 13, 21]
let report := {"name": "build", "total": total(scores)}

if report["total"] > 40 {
    print("ok: " + report["name"] + " = " + to_string(report["total"]))
} else {
    print("too low")
}
```

Run it:

```bash
kujo run report.kujo
```

Expected output:

```text
ok: build = 42
```

Need a project skeleton?

```bash
kujo run /path/to/kennel/kennel.kujo --interpreter -- new my-tool
```

## Runtime Mode Recommendations

- Use VM by default (`kujo run <file>`).
- Developers should not need `--interpreter` for ordinary modular project layouts.
- Use `--interpreter` only as an explicit compatibility/debug path when isolating runtime-path issues.
- Use `--jit` only as an experimental opt-in for JIT-compatible bytecode surfaces; unsupported surfaces fall back to VM execution with deterministic messaging.
- Use `kujo package-install --frozen` to verify manifests and lockfiles without rewriting them.
- Treat `kujo package-publish` as metadata preview only; `--publish` is reserved
  for future registry transport and is rejected in v1.0.
- Migration guidance and diagnostics workflow: [docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md](docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md)

## CLI Overview

Common commands:

- `kujo run <file>`: execute Kujo scripts on the VM path.
- `kujo run --jit <file>`: opt in to experimental JIT execution for compatible bytecode surfaces, with VM fallback for unsupported surfaces.
- `kujo run --interpreter <file>`: execute on the interpreter fallback path.
- `kujo check <file>`: validate source without execution.
- `kujo doctor`: run first-party diagnostics and environment checks.
- `kujo docgen <path>`: generate documentation from Kujo source code.
- `kujo test`: run snapshot fixture corpus (`--runtime vm|dual|interpreter`, `--update`).
- `kujo test-run <file>`: run Kujo `test "..." {}` declarations in a file.
- `kujo init`, `kujo package-add`, `kujo package-install`, `kujo package-install --frozen`: create and verify reproducible package manifests and lockfiles.
- `kujo package-publish`: preview package publish metadata only; no public registry publish occurs in v1.0.
- `kujo serve [dir]`: static file server for local preview/testing, including a root `404.html` fallback for missing routes.
- `kujo lsp`: run Kujo’s LSP server.

Machine-readable contracts and diagnostics behavior are documented in [docs/CLI_MACHINE_READABLE_CONTRACTS.md](docs/CLI_MACHINE_READABLE_CONTRACTS.md).
VS Code extension installation and Marketplace publishing are documented in [docs/VSCODE_EXTENSION.md](docs/VSCODE_EXTENSION.md).

## Repository Layout

- `src/`: core runtime/compiler/parser/VM/interpreter implementation.
- `tests/`: contract, integration, and parity coverage.
- `docs/`: language spec, security posture, roadmap, release process, and readiness checklists.
- `examples/`: runnable scripts and integration fixtures.
- `scripts/`: release gates and generation/verification utilities.

## Repository Hygiene

- Canonical tracked root files are intentionally minimal (`README`, manifests, policy docs).
- Most generated artifacts and local backups are ignored and should not be committed.
- Use the hygiene audit script before publishing release branches:

```bash
bash scripts/repo_hygiene_audit.sh
```

## Language Snapshot

Implemented and actively used surfaces include:

- variables/bindings (`let`, `mut`, `const`), functions (`func`, `async func`), conditionals, loops, structs, enums, `match`, `try/except`, and `throw`.
- arrays/dictionaries, interpolation, string/collection helpers, and a broad native standard library.
- module imports with both flat and dotted paths (for example `from src.util import value`).
- Kennel-installed dependencies are automatically importable from a project
  with a valid `kennel.lock`; `KUJO_MODULE_PATH` remains available for custom
  or advanced module roots.

Detailed semantics and contracts are in [docs/LANGUAGE_SPEC.md](docs/LANGUAGE_SPEC.md).

## Testing

Core validation commands:

```bash
cargo test
cargo run -- test --runtime vm
cargo run -- test --runtime dual
cargo test --test vm_interpreter_parity_surfaces
```

Security-focused suites:

```bash
cargo test --test runtime_security
cargo test --test native_api_security_boundaries
```

Release-gate scripts:

```bash
bash scripts/release_gate.sh
bash scripts/release_candidate_gate.sh --full
bash scripts/enterprise_verify.sh --minimal
```

For the current AI-native release evidence matrix:

```bash
bash scripts/enterprise_verify.sh --full
```
