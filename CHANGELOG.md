# CHANGELOG

All notable changes to Kujo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
