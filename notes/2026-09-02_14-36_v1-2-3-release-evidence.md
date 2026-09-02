# Kujo Field Notes — v1.2.3 Release Evidence

**Date:** 2026-09-02
**Session:** 14:36 local
**Branch/Commit:** `main` / `b1cec6b6c0f22e9015f5a8d3d1afc7e30b6964f7`
**Scope:** Signed v1.2.3 release publication and cross-platform artifact verification

---

## What I Changed

The signed v1.2.3 GitHub binary release is published. GitHub verified the SSH
signature on the annotated tag. The release contains checksum-addressed Linux,
macOS, and Windows binaries built from the exact tagged commit.

## Verification

| Evidence | Result |
| --- | --- |
| Pre-tag release gates for `b1cec6b` | PASS: release gate, formatting, clippy, VM/interpreter parity, LSP, artifact-install validation, field-note guard, release-state guard, and tool-artifact guard. |
| Filesystem/process conformance | PASS on Linux, macOS, and Windows, including descriptor-relative reads, VM/interpreter capability parity, and bounded process stdin. |
| Signed tag | PASS: GitHub reports `verified=true`, `reason=valid`, target `b1cec6b6c0f22e9015f5a8d3d1afc7e30b6964f7`. |
| Release workflow | PASS for the release gate, five native binary builds, npm-runtime packing, and GitHub release publication: <https://github.com/kujolang/kujo/actions/runs/33639105266>. |
| Published-artifact smoke | PASS on Linux x64/arm64, macOS x64/arm64, and Windows x64: <https://github.com/kujolang/kujo/actions/runs/33643141158>. |
| Cargo registry publication | EXCLUDED: the normalized package does not preserve the repository-local hardened `tiny_http` patch. GitHub binaries remain canonical. |

## Published Assets

- `kujo-v1.2.3-linux-x64.tar.gz` and `.sha256`
- `kujo-v1.2.3-linux-arm64.tar.gz` and `.sha256`
- `kujo-v1.2.3-macos-x64.tar.gz` and `.sha256`
- `kujo-v1.2.3-macos-arm64.tar.gz` and `.sha256`
- `kujo-v1.2.3-windows-x64.zip` and `.sha256`
- `checksums.txt`

## Security Boundary Delivered

- Descriptor-relative bounded text and binary reads reject traversal,
  symlinks/reparse points, special files, replacement races, and size drift.
- Structured process execution accepts bounded string or byte stdin without
  shell interpolation or a writable inherited input handle.
- The CLI lifecycle runs on a dedicated large-stack thread, preventing the
  Windows default main-thread stack overflow exposed by cross-platform
  capability parity tests.

The active release context included the required `UNBLOCK_V1_RELEASE`
directive.

## Gotchas (Read This Next Time)

- Do not publish the Cargo registry package while its normalized source omits
  the repository-local hardened `tiny_http` patch.
- Treat the GitHub binary artifacts and their checksums as the canonical
  v1.2.3 distribution.

## Things I Learned

- Windows hosted runners require the CLI lifecycle to use the dedicated
  large-stack thread for filesystem capability tests.
- Published-artifact smoke tests need to assert the requested tag explicitly,
  not merely install the newest available release.

## Debug Notes (Only if applicable)

The pre-release Windows stack overflow was reproduced in hosted CI and resolved
before tagging. The final five-platform published-artifact smoke run passed.

## Follow-ups / TODO (For Future Agents)

- Preserve the release-local `tiny_http` hardening in any future Cargo registry
  packaging design before enabling publication.

## Links / References

- Signed tag: <https://github.com/kujolang/kujo/releases/tag/v1.2.3>
- Release workflow: <https://github.com/kujolang/kujo/actions/runs/33639105266>
- Published-artifact smoke: <https://github.com/kujolang/kujo/actions/runs/33643141158>
