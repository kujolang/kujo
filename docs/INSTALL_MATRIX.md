# Kujo Install & Distribution Matrix

Status: stable v1.2.3 install matrix
Last updated: 2026-08-30

This document defines supported installation paths and known platform caveats for Kujo operators.

## Install Matrix

| Use Case | Recommended Command | Output | When To Use | Notes |
| --- | --- | --- | --- | --- |
| Local development from source | `cargo run -- --help` | Debug binary via Cargo | Iterating on runtime/compiler code | Fastest edit/run loop for contributors. |
| Local production-like build | `cargo build --release` | `./target/release/kujo` | Performance verification, smoke checks | Preferred for realistic runtime/perf behavior. |
| Install on current machine via Cargo | `cargo install --path .` | `kujo` on `PATH` | Operator/dev host install without package manager | Re-run after local upgrades to refresh binary. |
| Install from crates.io after registry publication | `cargo install kujolang` | `kujo` on `PATH` | Rust/Cargo users who prefer registry distribution | The registry package is `kujolang`; the command remains `kujo`. |
| Install from npm after registry publication | `npm install --global @kujolang/kujo-runtime` | `kujo` on `PATH` | Node.js 18+ environments on a supported binary target | Uses an exact-version platform package and no npm lifecycle scripts. Optional dependencies must be enabled. |
| Pinned commit install | `cargo install --git https://github.com/kujolang/kujo --rev <sha>` | `kujo` on `PATH` | Reproducible deployment from known commit | Use immutable commit SHA, not floating branches. |
| Prebuilt release binary | Download `kujo-<TAG>-<PLATFORM>.<EXT>` from GitHub Releases | Standalone `kujo`/`kujo.exe` | End users and onboarding without Rust/Cargo | See `docs/RELEASE_BINARIES.md` for asset names and checksum verification. |
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

## Prebuilt Release Targets

The tagged release workflow publishes:

- `kujo-<TAG>-linux-x64.tar.gz`
- `kujo-<TAG>-linux-arm64.tar.gz`
- `kujo-<TAG>-macos-x64.tar.gz`
- `kujo-<TAG>-macos-arm64.tar.gz`
- `kujo-<TAG>-windows-x64.zip`

Each archive has a matching `.sha256` file and is included in the consolidated
`checksums.txt`.

The same build matrix also packs lifecycle-script-free npm platform tarballs
for release rehearsal. The npm package target set is Linux x64/arm64 glibc,
macOS x64/arm64, and Windows x64. npm registry publication remains a separate gate;
the presence of a workflow tarball does not prove registry publication.

## Distribution Guidance (v1.0.0)

- Prefer the signed-by-checksum GitHub release archives for supported binary platforms.
- Use commit-pinned source installs when an environment requires rebuilding or auditing the exact source revision.
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
