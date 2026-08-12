# CHANGELOG

All notable changes to Kujo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add stable `encode_uri_component(text)` RFC 3986 UTF-8 percent encoding for provider and web integrations.
- Accept standard JSON Schema Draft 2020-12 identification and annotation keywords in `json_schema_validate`, including `$schema`, `$id`, `$comment`, `format`, `deprecated`, `readOnly`, and `writeOnly`.

### Fixed

- Preserve outer-block indentation after formatting a nested closing brace.
- Ignore braces inside strings and comments when computing formatter indentation.
- Stop definition, reference, hover, and rename lookups from selecting an identifier when the cursor is immediately after it.
- Emit LSP reference ranges that span the complete identifier.
- Emit zero-width LSP edits for missing-delimiter insertion actions.
- Limit range-formatting edits to the client-requested range.
- Limit inlay hints to the client-requested range.
- Include trailing newline positions in full-document formatting edits.
- Consume cancelled request IDs so later requests may safely reuse them.
- Accept the case-insensitive `Content-Length` header names required by the LSP transport protocol.

## [1.0.1] - 2026-08-11

### Fixed

- Build Linux release binaries on Ubuntu 22.04 so the published artifact remains compatible with Ubuntu 22.04 hosts such as Cloudflare Pages.

## [1.0.0] - 2026-08-08

### Added

- Initial public release of Kujo.
- VM-first language runtime, CLI, LSP, package lockfile workflow, static server, capability controls, and deterministic AI runtime primitives.
- Prebuilt Linux x64, macOS x64/arm64, and Windows x64 binaries with SHA-256 checksums.
- User-local ecosystem installer with core, AI, quality, showcase, and operating profiles.

### Changed

- Finalized machine-readable CLI and runtime diagnostic contract version identifiers at `1.0.0`.

### Fixed

- Repaired all remaining syntax-drifted examples, removed the expected-fail example list, and added exhaustive per-file verification coverage.
