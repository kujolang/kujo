# 2026-05-01 02:40 - Deprecation Policy And Native API Security Posture

## Summary

Completed P2 hardening backlog items for v1 planning:

- formalized deprecation policy for CLI/LSP/runtime surfaces
- documented security trust model and operational caveats for high-risk native APIs
- added targeted misuse/failure boundary integration tests across process/network/filesystem/crypto/database categories

## What Changed

- Added `docs/DEPRECATION_POLICY.md`.
  - defines canonical warning shape
  - defines semver-tied deprecation/removal windows
  - defines required artifact/test/changelog updates for deprecations
- Added `docs/NATIVE_API_SECURITY_POSTURE.md`.
  - documents trusted-script default model and no-sandbox baseline
  - documents operational caveats for process/network/filesystem/crypto/database builtins
  - records external isolation requirements for untrusted workloads
- Added `tests/native_api_security_boundaries.rs`.
  - asserts runtime misuse exits with code `1`
  - asserts deterministic runtime error text for each high-risk native category
- Updated roadmap/changelog/readme/notes index to reflect completed P2 items and policy doc references.

## Validation

Targeted test execution:

- `cargo test --test native_api_security_boundaries`

Validation objective:

- lock failure/misuse boundary behavior for high-risk native APIs
- provide explicit policy references for deprecation and operational security expectations
