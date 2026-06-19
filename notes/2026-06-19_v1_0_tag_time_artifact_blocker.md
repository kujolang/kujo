# V1.0 Tag-Time Artifact Blocker

Date: 2026-06-19
Checklist item: V1RR-P0-002
Status: blocked until explicit release publication authorization

## Blocker

`V1RR-P0-002` cannot be completed before the real `v1.0.0` tag/publication event. The active release-readiness instruction explicitly says not to tag, publish, or mark tag-time artifact sign-off complete unless `UNBLOCK_V1_RELEASE` is provided.

## Evidence

Files still intentionally open:

- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`
- `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`

Open rows are tag-time/release-flight work:

- publish the actual `v1.0.0` GitHub release,
- confirm Linux/macOS/Windows assets,
- confirm per-asset `.sha256` files and `checksums.txt`,
- confirm published-release smoke workflow success,
- record artifact URLs, checksum values, and command logs.

## Required Unblock

Completion requires explicit `UNBLOCK_V1_RELEASE`, plus real artifact URLs/checksums/workflow evidence after publication.

