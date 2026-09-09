# Kujo v1.4.0 release readiness

Date: 2026-09-09. Scope: compatible Kujo runtime release; Kennel publication remains separate.

## Release scope

Additive minor release from merged main 1778152f3ec38555511b3e09e841f83a1d984541.
Includes native package installation/process primitives and isolated imports, bounded
web-data operations, confined file publication, callback/scope corrections and
HTTP/crypto hardening. CHANGELOG.md records the complete user-facing changes.
Language, CLI and protocol contract schema versions remain 1.0.0.
The user explicitly authorized tagging, GitHub publication and website updates
once checks pass. Existing npm trusted publication remains part of the release.

## Verification

Merged-main release gate 34352182371, native-upgrade 34352182217 (all five
platforms), artifact validation 34352182272, filesystem conformance 34352182268,
LSP contracts 34352182090, release-state 34352182156, tool-artifacts 34352182108
and field-notes 34352182110 all passed. Candidate version metadata is 1.4.0;
published-stable references remain 1.3.1 until publication succeeds.
Local format, tag/version alignment, release-state and contract-version guards,
npm tests and npm package dry run passed. Candidate hosted verification and
published artifact evidence will be recorded separately.

## Boundaries

POSIX locks, ownership, atomic symlinks and process replacement enable the
upcoming native Kennel installer on Linux/macOS. Do not claim native Kennel
installation on Windows or a released Kennel client from this runtime release.
Keep existing website build pins and hosting. No Cargo registry publication:
crate normalization does not preserve the repository-local tiny_http patch.
Original kujo checkout contains unrelated user fuzz changes; they are preserved
and excluded from this isolated release checkout.
