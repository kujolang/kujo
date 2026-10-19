# Dependency maintenance compatibility — 2026-10-19

Starting revision: `6d661240dbde9508205b600773d331bd34b51f34` (`main`).
Implementation commit: `15212bf` (the following documentation commit retains
this receipt). Outcome: maintenance fixes verified; ZIP compatibility decision
still pending, so the combined advisory-remediation request is not fully closed.

## Retained changes

- Replace deprecated `serde_yaml`/`unsafe-libyaml` with the YAML organization's
  maintained `yaml_serde` 0.10.7 and `libyaml-rs` 0.3.0. The dependency alias keeps
  internal call sites stable. No YAML dependency types appear in Kujo's public
  function signatures. Baseline compatibility tests cover scalar types/spelling,
  tags, aliases, non-string keys, duplicate keys, malformed input and multiple
  documents; workflow-pack and representation tests provide integration coverage.
- Replace the private `keyring` v1 wrapper with its existing `keyring-core` and
  native platform stores. macOS Keychain and Windows Credential Manager versions
  are unchanged. Non-Apple/Android Unix still uses encrypted Secret Service,
  through the maintained libdbus backend instead of zbus. The service name,
  username, collection/search attributes, lazy initialization and `NoEntry`
  handling remain unchanged. Existing environment/project credential precedence,
  redaction, test-store behavior and error prefixes at the Kujo boundary are
  preserved. Backend-specific diagnostic details can reflect the new transport.
- Add a Linux interoperability fixture and CI job using isolated D-Bus and GNOME
  Keyring. It compiles the production selector and tests old/new access in both
  directions, updates, deletion, identity attributes and missing entries. Its
  separate lockfile deliberately retains the legacy backend solely as a test
  oracle; it is not in the runtime dependency graph.

The runtime lockfile removes all four prior maintenance warnings. No new
advisory ignores, fabricated package versions or runtime feature removals are
used. Native platform stores remain capability-gated by `runtime-network`.
The existing release-gate exception for `RUSTSEC-2025-0141` (bincode 1.3.3 via
PDF hyphenation) is unchanged. Without that existing exception, the raw audit
still reports bincode as unmaintained; this report does not claim a clean
unfiltered audit.

## ZIP compatibility decision

ZIP remains unchanged pending the owner's decision. Public
`Value::ZipArchive.writer` exposes `zip` 7's concrete `ZipWriter<File>` type.
Removing that registry dependency requires changing the exposed Rust type; there
is no reader/extractor feature gate that makes retaining it advisory-free.

Published ZIP 7 releases through 7.4.0 remain in the affected ranges of
RUSTSEC-2026-0367, 0368 and 0369. The official 7.x security backport
`75f967dafa6ef174208629d15350d7091e29ada8` addresses other issues and still contains
the affected extraction permission/symlink operations. It is not a remedy for
these three advisories. A custom backport also changes crate source identity and
requires downstream root Cargo patches; patches do not propagate from libraries.

ZIP 9.0.5 is the maintained remediation, but it needs an explicitly accepted Rust
API migration (including `ZipFile::data_start()` handling internally). Kujo's own
bounded extraction avoids the upstream filesystem extraction APIs, rejects
archive symlinks, confines paths and does not trust privileged archive modes.
That limits demonstrated runtime exposure but does not clear the dependency
advisories. The strict audit remains red while this decision is pending.

## Verification

The three new YAML tests passed against the original dependency before migration
and pass against the replacement. An initial fixture incorrectly treated an
unquoted `null` mapping key as a string; it was corrected before recording the
baseline. No production behavior was changed to accommodate that fixture.

| Check | Result |
| --- | --- |
| `cargo test --locked --no-fail-fast -j 1` | Passed: 3,107 results across 119 targets, zero failures, 20 existing ignores; includes workflow-pack, CLI/JSON, YAML representation, archive/security and generated contracts |
| `cargo clippy --locked --all-targets --all-features -j 1 -- -D warnings` | Passed |
| `cargo check --locked --no-default-features -j 1` | Passed; 17 existing feature-disabled unused/dead-code warnings |
| `cargo test -j 1 --test yaml_compatibility --test agent_project_contracts` | Passed |
| `target/debug/kujo test --runtime vm` and `--runtime dual` | Each passed 150/150; 11 existing policy skips; zero dual fallbacks |
| `cargo fmt --check`, fixture `rustfmt --check`, `git diff --check` | Passed |
| Ruby `YAML.load_file` on the new workflow | Passed |
| Hygiene, contract-version sync and release-state scripts | Passed |
| `cargo audit --deny warnings --ignore RUSTSEC-2025-0141 --json` | Three ZIP matches; zero maintenance warnings; 621 locked packages |
| `cargo audit --deny warnings --json` | Three ZIP matches plus the existing bincode maintenance warning |
| Isolated Linux Secret Service interoperability fixture | Passed on Debian Bookworm, Rust 1.90, libdbus and GNOME Keyring |

Cargo checks use `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0` and one job to bound local disk use; full tests enable
`KUJO_ENABLE_SOCKET_TESTS=1`. The Linux fixture ran in a disposable
`rust:1.90-bookworm` Docker container with `dbus` and `gnome-keyring` installed,
`CARGO_TARGET_DIR=/tmp/target`, and the README's isolated-session commands.
An initial container invocation used a login shell that hid Cargo from PATH;
the corrected non-login invocation completed. No host credentials were used.

The independent candidate review found only the initially ignored fixture
lockfile; a narrow `.gitignore` exception now ensures `--locked` works from a
clean checkout. macOS compiled the unchanged native backend. No new live
Windows credential-store run is claimed; its dependency and constructor are
unchanged.

Detailed local command logs and audit JSON are retained under
`.audit-evidence/dependency-maintenance-2026-10-19/`. The prior SignalBox entries
already cover the remaining ZIP decision; resolved maintenance work does not
warrant new captures.

## Sources

- [YAML organization maintained serde fork](https://github.com/yaml/yaml-serde)
- [Maintained Secret Service backend](https://github.com/open-source-cooperative/dbus-secret-service-keyring-store)
- [ZIP 0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html),
  [0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html),
  [0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html)
- [Official 7.x backport](https://github.com/zip-rs/zip2/commit/75f967dafa6ef174208629d15350d7091e29ada8)
