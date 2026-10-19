# Kujo 1.8.2 maintenance release

Status: release preparation. Publication and five-platform install verification
must complete before stable installer and website defaults advance from 1.8.1.

This release packages the fixes and compatible additions since 1.8.1. It adds no
language syntax. Wave C beta, Wave D alpha and unpublished participant SDKs
retain their experimental status.

## Dependency security and maintenance

- ZIP 9.0.5 resolves RUSTSEC-2026-0367, RUSTSEC-2026-0368 and
  RUSTSEC-2026-0369. Kujo archive calls retain their filename decoding, supported
  codecs, extraction confinement, CRC checking and atomic publication behavior.
- Rust embedders accessing `Value::ZipArchive.writer` must update their direct
  ZIP dependency to version 9. This is an explicitly approved Rust source API
  migration; follow [the migration guide](ZIP_9_RUST_MIGRATION.md). Kujo programs
  require no archive API migration.
- Maintained YAML and native credential-store dependencies replace retired
  dependency chains, preserving YAML behavior and existing stored credentials.
- PDF hyphenation uses pinned bincode-next 2.1.0 with the legacy wire format.
  All 82 bundled dictionaries retain their contents and old/new decoder
  interoperability. Rust 1.89 remains the supported minimum.
- Dependency audits pass with no vulnerabilities, maintenance warnings or
  advisory exceptions. This describes the audited lockfile, not a guarantee
  against future advisories.

## Runtime correctness and boundaries

- Preserve all dictionary representations across native collection,
  serialization, schema and AI helpers, including sparse integer keys, explicit
  nulls, mixed merges and distinct numeric-looking string keys.
- Preserve caller capabilities through module initialization, test hooks and
  parallel bytecode mapping. Refresh cached exports when source state or
  capability policy changes; retain import-root search precedence.
- Apply AI endpoint allowlists to redirects, reject private static-file aliases,
  confine archive ancestor creation and bound schema diagnostics. Restricted
  programs must grant the effects they use; trusted execution remains local code
  execution with the user's privileges, not a sandbox.
- Encode native-layout JSON-LD safely and prevent later template passes from
  treating data as template instructions.

## HTTP streaming and efficiency

- Add one-shot `http_stream_collect` and HTTP request collection mode, with
  incremental UTF-8 callbacks, bounded reads, cancellation and capability checks.
  Existing buffered HTTP calls and stream forwarding remain supported.
- Stream canonical AI hashing, share immutable tool-call strings, optimize ASCII
  token estimation and redact credentials before shortening error previews.
  Request hashes, cassette keys and deterministic token estimates remain stable.
- Keep CLI exit classes, existing JSON shapes, configuration keys and environment
  variables compatible. Release-gate logging is opt-in. `yield from` remains
  deferred; existing generator syntax is unchanged.

## Verification and distribution

The [changelog](../CHANGELOG.md) records the complete change set. Reproduction
and compatibility evidence is retained in the repository hardening, dependency
maintenance, ZIP 9 and bincode maintenance reports under `docs/audits/`.
The pre-release baseline passed 3,112 tests across 121 test targets and all
12 CI workflows (29 jobs), including platform compatibility checks. Release
publication must additionally verify the exact 1.8.2 artifacts.

Distribution targets remain Linux x64/arm64, macOS x64/arm64 and Windows x64,
with native archives, deterministic source/checksums and six npm packages.
Crates.io publication is optional and outside the native/npm release scope.
