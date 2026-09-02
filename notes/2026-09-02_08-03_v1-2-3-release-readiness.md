# Kujo v1.2.3 release readiness

## Context

- Host: macOS x86_64 local checkout
- Branch: `main`
- Purpose: publish the filesystem capability and bounded process-stdin runtime
  primitives required to close the RAG parser replacement race without shell
  interpolation

## Scope

This patch release adds descriptor-relative bounded file reads with no-follow
component traversal, plus bounded string/byte stdin for structured process
execution. Process input is staged privately, reopened read-only, and unlinked
before spawn. The RAG parser can therefore read an authorized PDF beneath its
ingestion root and pass those exact bytes to `pdftotext` through argv and stdin.

## Local evidence

| Command | Result |
| --- | --- |
| focused process-stdin unit tests | PASS — bounds, binary data, descendant lifetime, read-only descriptor, and cross-platform child contract |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS |
| generated artifact freshness contract | PASS |
| RAG aggregate test runner with the local runtime | PASS — 69/69 tests |
| independent security review | PASS — no remaining concrete finding |
| full release gate | Host-constrained parallel CLI lane hit `EAGAIN`; the same 27-test lane passed with `RUST_TEST_THREADS=1` |

## Pending tag-time evidence

- clean hosted release-gate and filesystem-capability matrices
- Windows, Linux, and macOS binary matrix
- GitHub release assets and checksums
- published-artifact smoke
- npm registry publication and provenance

## Authorization

The current release-execution context contains the exact
`UNBLOCK_V1_RELEASE` directive required by repository policy.
