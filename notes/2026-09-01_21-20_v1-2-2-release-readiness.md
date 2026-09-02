# Kujo v1.2.2 release readiness

## Context

- Host: macOS x86_64 local checkout
- Branch: `main`
- Purpose: restore the Windows release build and publish the native npm runtime family required by `@kujolang/paperclip`

## Scope

The `v1.2.1` release workflow passed Linux x64/arm64 and macOS x64/arm64 builds but failed on Windows because `io_private_spool_finish` compiled Unix-only `PermissionsExt` and `MetadataExt` calls on every platform. Commit `8e641f3` gates the finish operation on Unix and returns the existing managed-storage guidance on other platforms.

## Local evidence

| Command | Result |
| --- | --- |
| `npm test --prefix npm` | PASS — 12 tests |
| `npm run pack:dry-run --prefix npm` | PASS — six aligned `1.2.2` packages |
| `bash .github/scripts/check-tag-version.sh` | PASS — Cargo and npm manifests agree on `1.2.2` |
| `bash .github/scripts/check-release-state.sh` | PASS after recording `v1.2.1` as the latest pushed stable tag |

## Pending tag-time evidence

- full release gate
- Windows, Linux, and macOS binary matrix
- GitHub release assets and checksums
- published-artifact smoke
- npm registry publication and provenance

## Sign-off

The source and npm manifests are ready for the `v1.2.2` release gate. Tagging and publication remain blocked until the repository's explicit `UNBLOCK_V1_RELEASE` directive is present and npm publication authority is configured.
