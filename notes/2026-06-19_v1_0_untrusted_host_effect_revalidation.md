# V1 Untrusted Host-Effect Revalidation - 2026-06-19

## Scope

`V1RR-P1-005` required a launch-quality pass over untrusted-mode wording,
host-effect examples, and negative-path security coverage.

Reviewed:

- `README.md`
- `docs/NATIVE_API_SECURITY_POSTURE.md`
- `docs/STANDARD_LIBRARY_REFERENCE.md`
- representative host-effect examples for filesystem, process, HTTP/network,
  archive, image, and database workflows

## Changes

- Added explicit trusted/untrusted capability notes to host-effect sections in
  `docs/STANDARD_LIBRARY_REFERENCE.md`.
- Added trusted-local workflow comments to:
  - `examples/file_logger.kujo`
  - `examples/stdlib_process.kujo`
  - `examples/http_client.kujo`
  - `examples/database_unified.kujo`
  - `examples/stdlib_compression.kujo`
  - `examples/image_processing.kujo`
- Added image capability-denial tests for:
  - `load_image(...)` requiring filesystem read permission in untrusted mode
  - `gif_to_webp(...)` requiring filesystem write permission in untrusted mode

Existing `tests/native_api_security_boundaries.rs` coverage already included
filesystem write/delete, process/shell, HTTP/network client/server and
destination policy, archive extraction, and database denial paths.
`tests/runtime_security.rs` covers module/path security boundaries.

## Validation

All commands passed:

- `cargo test --test native_api_security_boundaries`
- `cargo test --test runtime_security`
- `cargo test --test stdlib_reference_contract`
- `cargo test --test docs_examples`

Logs and status manifest:

- `notes/release_evidence/2026-06-19_p1-005/status.tsv`
