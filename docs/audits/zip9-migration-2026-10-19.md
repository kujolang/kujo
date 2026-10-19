# ZIP 9 migration — 2026-10-19

Repository: `kujolang/kujo`, branch `main`. Starting revision:
`3fb30796760ac40917b0ede63f8584184f0051b3`.
Implementation revision: `52d38e5d959aa728f78d72a6328fa34380c25b78`.
Ending revision is the documentation commit containing this receipt.

This follows the [dependency maintenance audit](dependency-maintenance-2026-10-19.md).
The owner explicitly authorized ZIP 9 and the associated Rust API migration,
while requiring Kujo-language compatibility. This does not alter already
published 1.8.1 artifacts.

## Findings and changes

| ID | Priority | Finding | Action / status |
| --- | --- | --- | --- |
| ZIP-1 | P1 | Locked ZIP 7.2.0 matched RUSTSEC-2026-0367, 0368 and 0369 | Upgrade to 9.0.5; resolved in current audit |
| ZIP-2 | P1 | Public `Value::ZipArchive.writer` exposes the dependency's concrete Rust type | Owner-authorized source migration documented in [Rust migration guide](../ZIP_9_RUST_MIGRATION.md); external construction test added |
| ZIP-3 | P1 | ZIP 9 guesses UTF-8 for unflagged names, unlike ZIP 7's CP437 interpretation | Shared private flag-aware decoder preserves language filenames |
| ZIP-4 | P1 | Earlier upstream oversize rejection changed an existing Kujo diagnostic | Narrow mapping preserves the decoded-size diagnostic; CRC failures remain distinct |

The updater handles fallible names and optional payload offsets while preserving
checked bounds and archive-envelope validation. All previous default codecs stay
enabled. Existing filesystem confinement, capability checks, limits, staging,
permission handling and cleanup remain in place. Kujo does not use upstream
filesystem extraction; dependency remediation removes the advisory matches
without substituting upstream extraction for Kujo's guarded implementation.

Six fixed archives generated using the pre-migration ZIP 7.2.0 library verify
backward decoding of Stored, Deflate, Bzip2, Zstd, XZ and PPMd data. The generator
and provenance are committed. ZIP 7 cannot write Deflate64/LZMA fixtures; those
decoder features remain enabled but are not covered by these six inputs.

## Baseline and compatibility

The starting revision passed 3,107 tests, strict Clippy and VM/dual fixture
suites; see the preceding audit. Before migration, the new external Rust writer
and initial three filename-encoding cases passed against ZIP 7.2.0. ZIP 9 adds
an invalid-UTF-8 case, legacy codec fixtures, CRC integrity and permission tests.

Kujo script signatures, successful results, archive format, CLI codes, schemas,
configuration and environment variables are unchanged. Malformed archives may
fail earlier; upstream diagnostic details are not a byte-for-byte contract.
Rust embedders directly manipulating the public writer must update their ZIP
dependency. No other repository was modified; no unknown consumer compatibility
is claimed. This is dependency security maintenance, with no performance claim.

## Verification receipt

Cargo commands use `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0`, `--locked` and `-j1` to bound local disk use.
The full test suite additionally sets `KUJO_ENABLE_SOCKET_TESTS=1`.

| Command | Result |
| --- | --- |
| `cargo test --locked --no-fail-fast -j1` | 3,112 passed across 121 targets; zero failed, 20 existing ignores |
| `cargo test --locked -j1 --test zip_migration_compatibility --test zip_payload_integrity --test native_api_security_boundaries --test archive_ancestor_confinement` | Focused checks passed; final codec additions also passed in full suite |
| `cargo clippy --locked --all-targets --all-features -j1 -- -D warnings` | Passed |
| `cargo check --locked --no-default-features -j1` | Passed; 17 existing feature-disabled warnings |
| `target/debug/kujo test --runtime vm`, `target/debug/kujo test --runtime dual` | Each 150/150; 11 existing policy skips; zero dual fallbacks |
| `cargo fmt --check`, `rustfmt --check scripts/zip_compat_fixture.rs`, `git diff --check` | Passed |
| `bash scripts/generate_unsafe_inventory.sh --strict` | Passed; only shifted source-line anchors |
| `bash scripts/repo_hygiene_audit.sh` | Passed |
| `cargo audit --deny warnings --ignore RUSTSEC-2025-0141 --json` | Passed: zero vulnerabilities and warnings under existing exception |
| `cargo audit --json` | Zero vulnerabilities; only existing bincode maintenance warning |

All five newly added integration tests passed. They cover public Rust writer
construction, four filename encoding cases, six ZIP 7 codecs, CRC rollback and
privileged/existing modes across VM and interpreter. Existing decoded-size,
traversal, ancestor-confinement and native-updater tests also passed.

A fresh read-only candidate reviewer found no concrete bypass or regression.
Review covered direct ZIP callers, encoding handling, decoder errors, updater
bounds and feature configuration. Host verification is macOS x64; live Windows
and Linux execution of this migration is not claimed. Existing cross-platform
CI remains in place.

The unfiltered audit has zero vulnerabilities and one existing maintenance
warning: bincode `RUSTSEC-2025-0141`. The repository's prior release-gate exception
is unchanged. No ZIP advisory suppression was added. No new unresolved finding
was admitted to SignalBox; the old ZIP decision is resolved by this migration.
Detailed local logs are retained in `.audit-evidence/zip9-migration-2026-10-19/`.

## Later maintenance follow-up

The remaining bincode warning and release-gate exception were subsequently
removed by the [compatible dictionary codec migration](bincode-maintenance-2026-10-19.md).
The exception noted above describes the ZIP migration revision's historical state.
