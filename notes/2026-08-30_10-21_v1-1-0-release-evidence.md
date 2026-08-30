# Kujo Field Notes — v1.1.0 Release Evidence

**Date:** 2026-08-30
**Session:** 10:21 local
**Branch/Commit:** main / b3446127869752d38398a83c9fd2d7c400379cd0
**Scope:** Signed tag, GitHub release binaries, checksums, published-artifact smoke validation, and release distribution status for Kujo v1.1.0.

---

## What I Changed

- Finalized the `1.1.0` changelog date and current-stable documentation.
- Synchronized the repository artifact-ignore contract.
- Added the pinned Agent ecosystem fixture checkouts to the tag-time release gate.
- Created and pushed the signed annotated `v1.1.0` tag.
- Published the GitHub release with four platform archives, four matching SHA-256 files, and `checksums.txt`.

## Gotchas (Read This Next Time)

- The first tag workflow was cancelled before artifact publication because the tag-time release gate lacked the pinned Agent fixture checkouts. The unpublished tag was removed, the workflow was fixed in `b344612`, and the signed tag was recreated at that commit.
- `cargo publish --locked` was not completed: no registry token is configured, and crates.io already assigns the `kujo` package name to an unrelated project at version `0.1.0`. GitHub binaries remain the canonical published Kujo distribution.

## Things I Learned

- Every workflow that executes the full release gate must provision the same pinned Agent ecosystem fixture set as `ci-release-gate.yml`.
- A successful Cargo publication dry run does not prove that the target crates.io package name is owned or publishable by this project.

## Debug Notes (Only if applicable)

| Command or artifact | Result | Notes |
| --- | --- | --- |
| `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full` | PASS | Clean-tree candidate gate, all Rust and Kujo tests, socket tests, Clippy, formatting, and `cargo audit --deny warnings` passed. |
| `cargo publish --dry-run --locked` | PASS | Packaged and rebuilt `kujo 1.1.0`; upload was intentionally suppressed by dry-run mode. |
| Signed tag `v1.1.0` | PASS | GitHub verification reports `verified: true`, reason `valid`; tag object `21abb1a15d49c23be030c208124a7155aa6a18b0`. |
| Release binaries run `33314952745` | PASS | Release gate, Linux x64, macOS x64, macOS arm64, Windows x64, checksums, and GitHub release publication passed. |
| Published-artifact smoke run `33316582020` | PASS | All four platform artifact-only download, checksum, extraction, version, execution, and LSP smokes passed. |
| Local macOS x64 published artifact | PASS | SHA-256 matched `checksums.txt`; binary reported `kujo 1.1.0` and passed `lsp --help`. |

Published SHA-256 values:

- Linux x64: `6cff72d35ed0b43daaf43bb50e2925e86ed72ed0e0e2433db46d2d069f44af91`
- macOS arm64: `3ba1b2bd89221f2e0024ea9cff7351771bf69cd17edac96fef8f9cd01d3af7a6`
- macOS x64: `9b344ca2cbb838f03033565f9dff5885191e49b9443f5cd2610f8d766218f416`
- Windows x64: `ada1398b40e3fffb21169981d91d01060e575b3d2849d3c4aadd491cc0dce783`

## Follow-ups / TODO (For Future Agents)

- Decide whether Kujo should pursue transfer of the existing crates.io `kujo` name or publish under a distinct package name, then configure scoped registry credentials before enabling Cargo publication.

## Links / References

- GitHub release: <https://github.com/kujolang/kujo/releases/tag/v1.1.0>
- Release binaries: <https://github.com/kujolang/kujo/actions/runs/33314952745>
- Published-artifact smoke: <https://github.com/kujolang/kujo/actions/runs/33316582020>
- Release process: `docs/RELEASE_PROCESS.md`
