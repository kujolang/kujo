# Kujo

[![Version](https://img.shields.io/badge/version-1.5.0-black)](https://github.com/kujolang/kujo/releases/tag/v1.5.0)
[![License](https://img.shields.io/badge/license-MIT-lightgrey)](LICENSE)
[![built with Rust](https://img.shields.io/badge/built%20with-Rust-white.svg)](https://www.rust-lang.org/)

Kujo is a programming language for local-first automation, AI-native software,
and practical application scripting. The runtime is written in Rust. `kujo run`
uses the bytecode VM by default; a tree-walking interpreter remains available for
compatibility and debugging.

## Install

On macOS or Linux, the Kujo installer sets up the current runtime and the core
Kujo toolset in user-owned directories:

```bash
curl -fsSL https://kujolang.ai/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"
kujo --version
```

The default install includes Kujo, Kennel, and the core tools used for packages,
task specs, context, tests, and run records. See [Installation](INSTALLATION.md)
for a runtime-only npm install, Windows steps, source builds, profiles, updates,
and uninstall instructions.

## Run your first program

Create `hello.kujo`:

```kujo
func greet(name) {
    print("Hello, " + name + "!")
}

greet("Kujo")
```

Then check and run it:

```bash
kujo check hello.kujo
kujo run hello.kujo
```

## Add packages with Kennel

[Kennel](https://kennel.kujolang.ai/) is Kujo's package manager and official
package registry. Public package reads need no account. Install the current
Kennel client on macOS or Linux after installing Kujo 1.4.0 or newer:

```bash
curl -fsSLO https://kennel.kujolang.ai/install.sh
sh install.sh
. "$HOME/.kennel/env"
kennel --version
```

Add a project dependency:

```bash
kennel init --name my-project
kennel add ability
kennel install
```

Or install a command you can use from any directory:

```bash
kennel tool install shipcheck
shipcheck --help
```

Kennel writes `kennel.toml` and `kennel.lock`. Kujo also has a smaller built-in
`kujo.toml` / `kujo.lock` workflow. The formats serve different jobs and are not
interchangeable. Read [Packages with Kennel](https://docs.kujolang.ai/learn/packages/)
before choosing one.

## What Kujo includes

- A VM-first runtime with functions, modules, collections, structs, enums,
  pattern matching, exceptions, async work, and generator support with documented
  VM/interpreter limits.
- Native filesystem, process, network, HTTP, crypto, database, and data helpers.
- Deterministic AI request hashes, record/replay cassettes, streaming callbacks,
  multimodal messages, token estimates, secret redaction, JSON Schema validation,
  and vector math.
- A formatter, linter, test runner, LSP, documentation generator, static server,
  benchmarks, diagnostics, and stable machine-readable output contracts.
- Capability controls for scripts that should not receive ambient host access.

Core AI-native runtime mechanisms are implemented for deterministic request hashing, offline record/replay cassettes, structured response metadata, JSON Schema validation, vector math, token budgeting, runtime secret redaction, dedicated AI egress capability controls, streaming callbacks, and multimodal message builders.

Dotted module import workflows are supported on the default VM path. Package
workflows are deterministic: `kujo init`, `kujo package-add`, `kujo package-install`, and `kujo package-install --frozen` work with nested source layouts and reproducible `kujo.lock` snapshots.

## Everyday commands

```bash
kujo run app.kujo                 # run on the VM
kujo check app.kujo               # validate without running
kujo format --write app.kujo      # format in place
kujo lint app.kujo                # find common problems
kujo test                         # run snapshot fixtures
kujo test-run tests.kujo          # run test declarations
kujo doctor --json                # inspect the environment
kujo serve [dir]                  # preview a static directory
kujo lsp                          # start the language server
```

Use `--` before script arguments that could be mistaken for Kujo options:

```bash
kujo run tool.kujo -- --help
```

## Safety Model Snapshot

Kujo is not a sandbox. A normal `kujo run` is trusted and can use host-effect
APIs. For code you do not trust, start with `--untrusted` and grant only the
capabilities it needs:

```bash
kujo run --untrusted --allow-fs-read report.kujo
kujo run --untrusted --allow-ai agent.kujo
```

Use `--deny-private-net` when network clients must reject local, private,
link-local, multicast, and unspecified destinations. For AI-only access, prefer
`--allow-ai` to broad network access and set `KUJO_AI_ALLOWED_ENDPOINTS` to the
provider URLs the script may call.

Read [Secure AI scripting](docs/SECURE_AI_SCRIPTING.md) and the
[native API security posture](docs/NATIVE_API_SECURITY_POSTURE.md) before running
untrusted code on a shared or sensitive machine.

## AI-Native Runtime Snapshot

Kujo keeps deterministic mechanisms in the runtime and leaves provider routing,
RAG policy, agent frameworks, and observability to packages.

- `ai_chat`, `ai_stream_chat`, `ai_embedding`, and `ai_tool_loop` share replay,
  response, and error contracts.
- `ai_request_hash` creates credential-free cache and cassette keys.
- `ai_text`, `ai_image_url`, and `ai_message` build portable messages.
- `ai_count_tokens` and `ai_fit_context` provide deterministic estimates.
- `secret`, `reveal`, and `is_secret` keep values redacted until code reveals them.

Run the replay example without live provider credentials:

```bash
KUJO_AI_REPLAY=tests/fixtures/ai_cassettes \
KUJO_AI_REPLAY_MODE=strict \
cargo run -- run examples/ai_enterprise_replay_showcase.kujo
```

The short and full verification commands are:

```bash
bash scripts/enterprise_verify.sh --minimal
bash scripts/enterprise_verify.sh --full
```

See [AI runtime](docs/AI_RUNTIME.md) and the
[standard library](docs/STANDARD_LIBRARY.md) for the complete contracts.

## Runtime Mode Recommendations

- Use `kujo run <file>` for normal work. Developers should not need `--interpreter` for ordinary modular project layouts.
- Use `--interpreter` to isolate a compatibility or runtime-path problem.
- Treat `--jit` as experimental. Unsupported bytecode falls back to the VM.
- Use `kujo package-install --frozen` to check `kujo.toml` and `kujo.lock`
  without changing either file.
- `kujo package-publish`: preview package publish metadata only. It does not
  publish to Kennel. Kennel owns registry installation and distribution.

See the [VM/interpreter migration playbook](docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md).

## 1.0 Release Status

Release boundary: Kujo `v1.5.0` is the current stable release; explicit deferrals and compatibility guarantees are governed by `docs/V1_SCOPE.md`.

- The source tree is currently at `1.5.0` in `Cargo.toml`; the latest published stable release tag is `v1.5.0`.
- Prebuilt Linux x64/arm64, macOS x64/arm64, and Windows x64 binaries ship with
  SHA-256 checksums.
- Kujo v1.0 package scope was local manifest and lockfile determinism only; it
  did not include a public Kennel registry or package publish transport. That
  describes the historical 1.0 runtime boundary. The separate Kennel registry is
  now live at [kennel.kujolang.ai](https://kennel.kujolang.ai/).
- The v1.0 launch record remains in
  [the completed release checklist](docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md).

## Build and test from source

Kujo requires Rust 1.86 or newer:

```bash
git clone https://github.com/kujolang/kujo.git
cd kujo
cargo build --release
cargo test
cargo run -- test --runtime vm
cargo run -- test --runtime dual
```

This first-ten-minutes path gives you a normal script, a replay-only AI example,
and a least-privilege execution example. The longer contributor workflow lives in
[CONTRIBUTING.md](CONTRIBUTING.md).

## Core Reference Links

- [Language specification](docs/LANGUAGE_SPEC.md)
- [Standard library](docs/STANDARD_LIBRARY.md)
- [AI runtime](docs/AI_RUNTIME.md)
- [CLI machine-readable contracts](docs/CLI_MACHINE_READABLE_CONTRACTS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [v1 scope and deferrals](docs/V1_SCOPE.md)
- [Optional typing policy](docs/OPTIONAL_TYPING_DESIGN.md)
- [ROADMAP.md](ROADMAP.md)
- [AI-native release evidence](docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md)
- [v1.0 release checklist](docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md)
- [Secure AI scripting](docs/SECURE_AI_SCRIPTING.md)
- [Security response](docs/SECURITY_RESPONSE.md)
- [Release process](docs/RELEASE_PROCESS.md)
- [VM/interpreter migration playbook](docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md)

## Upgrade

Standalone releases from v1.3.0 onward can update themselves:

```bash
kujo upgrade --check
kujo upgrade
```

Use the original package manager for npm or Cargo installs. `kujo upgrade`
changes only the runtime; it does not update Kennel, tools, or project packages.
See [runtime upgrade and recovery](docs/RUNTIME_UPGRADE.md).

Kujo is available under the [MIT License](LICENSE).
