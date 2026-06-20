# Kujo

Kujo is the programming language for AI-native software, built in Rust.

It is designed for local-first automation, agentic workflows, and application scripting where deterministic behavior, strong native capabilities, and practical ergonomics matter.

Kujo is VM-first (`kujo run`), with a tree-walking interpreter available as an explicit fallback/debug path.

## Current Status

- Kujo is usable from source today.
- VM runtime parity for modular workflows has been significantly hardened.
- Dotted module import workflows are supported on the default VM path.
- Package workflows are deterministic: `kujo init`, `kujo package-add`, `kujo package-install`, and `kujo package-install --frozen` work with nested source layouts and reproducible `kujo.lock` snapshots.
- Kujo v1.0 package scope is local manifest and lockfile determinism only; it does not include a public Kennel registry or package publish transport.
- Native helper coverage has expanded for everyday scripting work: hashing (`sha256`, `sha256_file`, `md5`), file inspection (`read_file_lossy`, `path_is_symlink`), formatting (`pad_start`, `pad_end`), introspection (`type_of`, `is_truthy`), and stderr output (`eprint`) are all available without shelling out.
- Runtime-generated sequence and string helpers now reject unsafe edge cases such as non-finite range bounds, reversed random bounds, negative string widths/counts, and oversized generated outputs instead of panicking or attempting unbounded allocation.
- Native capability controls are available for trusted and untrusted execution modes.
- Kujo is in pre-tag `1.0.0` release-candidate readiness: the crate metadata is staged at `1.0.0`, but final tag/publish evidence is still bounded by `ROADMAP.md`, the pre-v1 checklist, and the release artifact checklist.

## Why Kujo

- VM-first execution for predictable runtime behavior in local and production scripts.
- Practical native APIs (filesystem, process, network, async, crypto, database).
- The native standard library keeps growing with practical helper surfaces for checksums, padding, truthiness, file inspection, and stderr-friendly output, which makes common automation scripts more self-contained.
- Security policy controls for trusted and untrusted execution.
- Bounded native helper behavior for generated arrays, ranges, random IDs, and string expansion.
- Module workflows that support both flat and dotted imports.
- Package bootstrap and lockfile workflows that stay deterministic across repeated installs.
- Strong diagnostics, contract tests, and release-gate automation.
- Core surfaces for `doctor`, `docgen`, and machine-readable CLI contracts keep the language agent-readable.

## 1.0 Readiness Status

- The project is currently at `1.0.0` in `Cargo.toml` for release-candidate validation.
- Kujo has not yet published the final `v1.0.0` tag or release artifacts.
- [ROADMAP.md](ROADMAP.md) is the single source of truth for release readiness and blocker tracking.
- Kujo `1.0.0` must not be released until all P0/P1 roadmap items, the final release checklist, and tag-time artifact evidence are complete.
- Canonical readiness boundary: Kujo remains a pre-tag `1.0.0` release candidate until `ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`, and tag-time artifact evidence are closed.
- Deferred/non-goal boundaries are tracked in [docs/V1_SCOPE.md](docs/V1_SCOPE.md) and [docs/OPTIONAL_TYPING_DESIGN.md](docs/OPTIONAL_TYPING_DESIGN.md).

## Safety Model Snapshot

- Kujo is not a sandbox.
- `kujo run` and `kujo test-run` default to trusted mode.
- For untrusted code, start with `--untrusted` and add only required `--allow-*` flags.
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

When `--untrusted` and outbound network client access are enabled, Kujo now defaults the outbound destination policy to `deny_private` (unless `KUJO_NET_DESTINATION_POLICY` is already set). This helps reduce accidental private-network access in untrusted runs.

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
- [docs/DOCGEN.md](docs/DOCGEN.md)
- [docs/CLI_MACHINE_READABLE_CONTRACTS.md](docs/CLI_MACHINE_READABLE_CONTRACTS.md)
- [AGENTS.md](AGENTS.md)
- [docs/INSTALL_MATRIX.md](docs/INSTALL_MATRIX.md)
- [docs/RELEASE_BINARIES.md](docs/RELEASE_BINARIES.md)
- [docs/FIRST_TOOL_COOKBOOK.md](docs/FIRST_TOOL_COOKBOOK.md)
- [docs/RELEASE_PROCESS.md](docs/RELEASE_PROCESS.md)
- [docs/VM_INTERPRETER_PARITY_MATRIX.md](docs/VM_INTERPRETER_PARITY_MATRIX.md)
- [docs/ENTERPRISE_READINESS_NEXT_SESSION_2026-06-20.md](docs/ENTERPRISE_READINESS_NEXT_SESSION_2026-06-20.md)

For script ergonomics, see the output/report style guidance in [docs/FIRST_TOOL_COOKBOOK.md](docs/FIRST_TOOL_COOKBOOK.md) and [docs/STANDARD_LIBRARY_REFERENCE.md](docs/STANDARD_LIBRARY_REFERENCE.md).

## Install

This repository builds the Kujo language/runtime. If another `kujo` command is already installed on your system, prefer the full path to this repo's binary while testing so you do not confuse it with unrelated tools.

```bash
git clone https://github.com/kujolang/kujo.git
cd kujo
cargo build --release
./target/release/kujo --version
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

## Quick Start

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
- `kujo serve [dir]`: static file server for local preview/testing.
- `kujo lsp`: run Kujo’s LSP server.

Machine-readable contracts and diagnostics behavior are documented in [docs/CLI_MACHINE_READABLE_CONTRACTS.md](docs/CLI_MACHINE_READABLE_CONTRACTS.md).

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
```
