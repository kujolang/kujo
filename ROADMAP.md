# Kujo Roadmap

Updated: 2026-09-23
Stable release: [v1.5.0](https://github.com/kujolang/kujo/releases/tag/v1.5.0)

> Current crate version: `1.5.0` in [Cargo.toml](Cargo.toml)

Kujo 1.0 shipped on August 8, 2026. The current 1.5 line is stable and adds
bounded, descriptor-confined artifact I/O on top of the package-installer
runtime primitives introduced in 1.4.
This page now tracks what comes next instead of repeating the closed 1.0 plan.

## Where Kujo stands

- `kujo run` uses the bytecode VM by default. The interpreter remains a fallback
  and debugging path.
- The language, CLI, LSP, capability model, AI runtime primitives, and supported
  machine-readable contracts are stable within the v1 compatibility policy.
- Linux x64/arm64, macOS x64/arm64, and Windows x64 release binaries ship with
  SHA-256 checksums. The npm runtime package covers the same targets but its
  public channel remains at 1.4.0 pending publisher authorization for 1.5.0.
- The official Kennel registry is live at
  [kennel.kujolang.ai](https://kennel.kujolang.ai/). Kennel 1.1.0 manages project
  packages and global tools on macOS and Linux.
- Kujo's built-in `kujo.toml` / `kujo.lock` commands remain separate from
  Kennel's `kennel.toml` / `kennel.lock` workflow.

## Recently completed

The September 13 reliability sweep closed the current short-term checklist:

- Rechecked VM/interpreter behavior for closures, lexical scopes, async
  captures, generators, spawn, imports, collections, control flow, and errors.
- Re-ran the host-effect security boundaries for filesystem, archive, process,
  network, HTTP, TLS, database, AI, environment, clock, and random access.
- Removed a race from the source-bound TCP security regression.
- Expanded scheduled fuzzing to every maintained language and bounded-decoder
  target, and added a weekly RustSec dependency audit.
- Rechecked the release installer, native upgrade contracts, npm packages, and
  Kennel's public installer boundary.
- Aligned current docs on Kujo 1.5.0, the live Kennel registry, and the split
  between Kujo's built-in lockfile commands and Kennel packages.

These are continuing release gates, not one-time tasks. Their automated checks
must stay green as the runtime changes.

## Current direction

### Keep the v1 line dependable

Patch releases should favor correctness, security, compatibility, and clear
diagnostics over new syntax. Every change to a stable CLI or JSON contract needs
tests, documentation, and a changelog entry.

Work in this lane includes:

- VM/interpreter parity and closure, scope, async, and generator correctness.
- Bounded filesystem, archive, process, network, HTTP, TLS, and database APIs.
- Cross-platform release, installer, upgrade, and npm-package checks.
- Fuzzing, dependency audits, and regression tests for fixed bugs.

### Make Kujo packages easy to trust

Kujo 1.4 and 1.5 supply isolated imports, file locks, ownership checks, atomic
publication, safe symlink updates, and exact process replacement. Kennel uses
those mechanisms to install packages and commands without hiding the source or
lock state.

The next package work should keep these rules:

- Resolve releases to exact versions and checksums.
- Keep project dependencies local to the project.
- Protect existing global commands and preserve working installs after failure.
- Keep registry reads public and release provenance visible.
- Put package policy in Kennel, not in the Kujo runtime.

Third-party accounts, scoped publishing, and private packages are not current
release promises.

### Improve the language without breaking it

Near-term language work should close known runtime gaps before adding broad new
syntax. The explicit candidates from the v1 scope are:

- fuller VM closure and generator state handling;
- clearer spawn behavior and async parity;
- better optional type inference for destructuring, imports, struct fields,
  promises, and callable values;
- compiler and VM optimizations that do not change program behavior.

Generics, FFI, a WASM target, and macros remain possible post-v1 projects. None is
scheduled or promised here.

### Keep AI features composable

Kujo core owns deterministic request hashes, record/replay, structured output,
streaming, token estimates, secret redaction, schema validation, vector math, and
explicit AI egress. Provider drivers, agents, retrieval, evaluation, workflows,
and observability belong in packages.

Future work should preserve offline fixtures, stable data shapes, redacted errors,
bounded input and output, and explicit capabilities. Core should not grow a
provider-specific control plane.

### Make the first hour shorter

The install, quickstart, package, and safety paths should stay visible near the
top of the docs. Examples must run on the default VM path unless they clearly say
otherwise. Website and repository docs should agree on the current release,
supported platforms, and the line between Kujo and Kennel.

## Release rules

Before a release:

1. Keep `Cargo.toml`, the README, this roadmap, and release notes on the same
   version.
2. Update tests and docs with every behavior or contract change.
3. Run the full release gate and the affected platform checks.
4. Build from a clean tree and verify published archives and checksums.
5. Keep claims no broader than the shipped artifacts and recorded evidence.

The canonical command is:

```bash
bash scripts/release_gate.sh --full
```

## Final v1.0 Release Checklist

The v1.0 checklist is complete and kept only as launch evidence. See:

- [Official v1.0 release checklist](docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md)
- [v1.0 artifact checklist](docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md)
- [v1 scope and compatibility policy](docs/V1_SCOPE.md)
- [Release process](docs/RELEASE_PROCESS.md)

Git history preserves the former 2,000-line implementation ledger. It should not
be used as the current product roadmap.
