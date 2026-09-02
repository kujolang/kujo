# Kujo Field Notes — v1.2.3 Release Readiness

**Date:** 2026-09-02
**Session:** 08:03 local
**Branch/Commit:** main / 124cce9fd2a6e499ec66ef29868f5c70e1783269
**Scope:** Publish the filesystem capability and bounded process-stdin primitives required to close the RAG parser replacement race without shell interpolation.

---

## What I Changed

- Added descriptor-relative bounded file reads with no-follow component
  traversal, regular-file enforcement, replacement-race resistance, and
  interpreter/VM capability parity.
- Added bounded string/byte stdin to structured process execution. Input is
  staged privately, reopened read-only, and unlinked before process spawn.
- Added Unix and Windows conformance coverage and prepared aligned Cargo and
  npm package versions for v1.2.3.

## Gotchas (Read This Next Time)

- Passing a read/write temporary file directly as stdin gives the child a
  writable descriptor and permits temporary-disk growth beyond the input
  bound. Reopen the completed spool read-only before spawn.
- On Windows the private temporary file has an auto-deleting name until all
  handles close; do not describe the implementation as universally anonymous.
- A loaded local host can make highly parallel CLI contract tests fail with
  `EAGAIN`. Re-run the affected lane with `RUST_TEST_THREADS=1` and require the
  clean hosted matrix before tagging.

## Things I Learned

- A descriptor-relative read alone cannot feed a legacy file-path parser
  safely. The runtime also needs a bounded byte-input process contract so the
  exact authorized bytes can cross the parser boundary without reopening the
  original path.
- The runtime-level combination is reusable by RAG and other applications and
  avoids application-specific shell quoting or race-prone path validation.

## Debug Notes (Only if applicable)

| Command | Result | Notes |
| --- | --- | --- |
| Focused process-stdin unit tests | PASS | Covered bounds, binary data, descendant lifetime, read-only descriptor, and a cross-platform child contract. |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS | No first-party warnings. |
| Generated artifact freshness contract | PASS | Unsafe and TODO inventories were refreshed. |
| RAG aggregate test runner with the local runtime | PASS | 69/69 tests passed. |
| Independent security review | PASS | No remaining concrete finding after read-only reopen remediation. |
| Full local release gate | HOST-CONSTRAINED | The parallel CLI lane hit `EAGAIN`; the same 27-test lane passed with `RUST_TEST_THREADS=1`. Hosted clean-runner gates remain mandatory. |
| `npm test --prefix npm` | PASS | 12/12 package contract tests passed. |
| `npm run pack:dry-run --prefix npm` | PASS | Six aligned v1.2.3 packages were assembled. |

The repository-required `UNBLOCK_V1_RELEASE` directive was supplied in the
active release-execution context.

## Follow-ups / TODO (For Future Agents)

- Require clean hosted release-gate, filesystem-capability, LSP, and artifact
  validation matrices before creating the signed v1.2.3 tag.
- Verify Linux, macOS, and Windows release assets and checksums after tagging.
- Run published-artifact smoke, then promote public stable-version strings.
- Pin RAG to the signed v1.2.3 release commit and land its descriptor-relative
  parser migration.

## Links / References

- Release process: `docs/RELEASE_PROCESS.md`
- Filesystem conformance: `.github/workflows/filesystem-capability-conformance.yml`
- Standard library process contract: `docs/STANDARD_LIBRARY.md`
