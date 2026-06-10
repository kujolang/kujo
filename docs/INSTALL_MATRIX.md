# Kujo Install & Distribution Matrix

Status: v1.0.0 baseline draft (active)
Last updated: 2026-05-25

This document defines supported installation paths and known platform caveats for Kujo operators.

## Install Matrix

| Use Case | Recommended Command | Output | When To Use | Notes |
| --- | --- | --- | --- | --- |
| Local development from source | `cargo run -- --help` | Debug binary via Cargo | Iterating on runtime/compiler code | Fastest edit/run loop for contributors. |
| Local production-like build | `cargo build --release` | `./target/release/kujo` | Performance verification, smoke checks | Preferred for realistic runtime/perf behavior. |
| Install on current machine via Cargo | `cargo install --path .` | `kujo` on `PATH` | Operator/dev host install without package manager | Re-run after local upgrades to refresh binary. |
| Pinned commit install | `cargo install --git https://github.com/kujolang/kujo --rev <sha>` | `kujo` on `PATH` | Reproducible deployment from known commit | Use immutable commit SHA, not floating branches. |
| CI reproducible build artifact | `cargo build --locked --release` | Deterministic release binary (lockfile pinned) | CI pipelines and artifact promotion | Fails fast if lockfile drift occurs. |

## Platform Caveats

### macOS

- Xcode Command Line Tools are required (`xcode-select --install`).
- Some test suites spawn local loopback servers; restrictive endpoint security tools may interfere with socket-bound tests.

### Linux

- Build requires standard Rust toolchain plus C build essentials (`clang`/`gcc`, linker, make).
- In hardened/containerized environments, ensure loopback networking is available for integration tests that validate HTTP/TCP behavior.

### Windows

- Use Rust MSVC toolchain for best compatibility.
- Path separator differences are covered by contract tests, but custom scripts should prefer Kujo-native path helpers (`path_join`, `path_absolute`) over hand-built separators.

## Distribution Guidance (Pre-v1)

- Kujo remains pre-1.0; do not claim API/runtime stability beyond documented contract surfaces.
- Prefer commit-pinned installs for production automation until v1 release gates are closed.
- Validate with:
  - `cargo test --test cli_contracts`
  - `cargo test --test cli_json_contracts`
  - `cargo test --test runtime_security`

## Verification Commands

```bash
cargo build --release
./target/release/kujo --version
cargo install --path .
kujo --version
```
