# Kujo Field Notes — v1.2.2 Release Readiness

**Date:** 2026-09-01
**Session:** 23:07 local
**Branch/Commit:** codex/release-v1.2.1 / da7b8b1abb80134b75f2a3cef093ba5503a4a49c
**Scope:** Kujo v1.2.2 Windows portability correction and complete hosted release preflight for the routed-HTTP security patch.

---

## What I Changed

- Made private-spool metadata verification Unix-only and fail closed on other
  platforms, restoring Windows compilation without weakening the Unix
  ownership, link-count, and permission checks.
- Refreshed the generated unsafe inventory after the platform guard changed
  source line positions.
- Made Windows npm artifact assembly invoke npm's JavaScript entry point through
  Node instead of relying on a `.cmd` shim or a shell.
- Advanced source and npm package versions to 1.2.2.

## Gotchas (Read This Next Time)

- The signed `v1.2.1` tag is an unpublished failed release candidate. Its hosted
  gate and four native platforms passed, but Windows failed before publication
  because Unix metadata APIs were compiled unconditionally.
- The first v1.2.2 manual preflight, run `33579280380`, proved Windows compilation
  and binary validation but found that Node cannot execute npm's `.cmd` shim with
  `shell: false`. No release assets were published by that manual run.
- The normalized Cargo package still replaces the repository-local patched
  `tiny_http` with upstream 0.12.0, which lacks
  `Server::http_with_read_timeout`. Crates.io publication remains blocked.

## Things I Learned

- Cross-platform release preflight must exercise the complete native packaging
  step, not only compilation and command smoke tests.
- Directly invoking npm's JavaScript CLI preserves a shell-free packaging
  boundary on Windows and avoids command-shim behavior differences.

## Debug Notes (Only if applicable)

## Verification Evidence

| Check | Result | Evidence |
| --- | --- | --- |
| Local npm workspace tests | PASS | 12/12 tests at `da7b8b1`. |
| Generated artifact freshness | PASS | 3/3 freshness tests after unsafe-inventory regeneration. |
| Hosted release gate | PASS | Manual run `33582024274`. |
| Linux x64 and arm64 artifacts | PASS | Build, command validation, packaging, checksums, npm platform pack, and upload in run `33582024274`. |
| macOS x64 and arm64 artifacts | PASS | Build, command validation, packaging, checksums, npm platform pack, and upload in run `33582024274`. |
| Windows x64 artifact | PASS | Static OpenSSL setup, build, command validation, ZIP/checksum, shell-free npm pack, and upload in run `33582024274`. |
| npm runtime package assembly | PASS | `pack-npm-runtime` succeeded in run `33582024274`; publication was intentionally skipped for a branch dispatch. |
| `cargo publish --dry-run --locked` | BLOCKED | The normalized dependency graph does not preserve the hardened `tiny_http` API. |

Warnings and exceptions:

- GitHub release binaries are the canonical distribution for v1.2.2. Do not
  publish the crate until its normalized dependency graph preserves the hardened
  HTTP API.
- The repository-required `UNBLOCK_V1_RELEASE` directive was supplied in the
  active release-execution context.
- This note authorizes no synthetic threat-model approval. RAG's accountable
  human review and deployment attestations remain separate release evidence.

Release readiness sign-off:

The v1.2.2 source is ready for a signed GitHub release tag at the commit adding
this note. Publication is complete only after all platform assets, per-asset and
consolidated checksums, configured npm publication, and the published-artifact
smoke workflow have succeeded. Crates.io is explicitly excluded by the
documented dependency-normalization blocker.

## Follow-ups / TODO (For Future Agents)

- Create and verify the signed v1.2.2 tag after this note commit's guards pass.
- Verify every published platform artifact and checksum, then promote stable
  release strings and downstream immutable pins.
- Resolve the patched `tiny_http` packaging boundary before enabling crates.io.

## Links / References

- Successful all-platform preflight: `33582024274`
- Failed v1.2.1 tag workflow: `33575961966`
- Windows compile/npm diagnostic preflight: `33579280380`
- Release process: `docs/RELEASE_PROCESS.md`
- RAG regression fixture: `b850db11f353a9ffbaea627d1991e62d7c281fb0`
