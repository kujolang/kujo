# Kujo Field Notes — v1.2.3 Release Evidence

**Date:** 2026-09-02  
**Signed tag:** `v1.2.3`  
**Release commit:** `b1cec6b6c0f22e9015f5a8d3d1afc7e30b6964f7`  
**Release URL:** <https://github.com/kujolang/kujo/releases/tag/v1.2.3>

## Outcome

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
