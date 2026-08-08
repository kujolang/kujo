# Kujo v1.0.0 Launch Evidence

Date: 2026-08-08

## Release

- Directive: `UNBLOCK_V1_RELEASE`
- Tag: [`v1.0.0`](https://github.com/kujolang/kujo/releases/tag/v1.0.0)
- Tagged commit: `2b3e07d398016e92008d8399e79c441e012dce38`
- Release artifact run: [31249625734](https://github.com/kujolang/kujo/actions/runs/31249625734)
  - Release gate: passed
  - Linux x64 build/package/upload: passed
  - macOS x64 build/package/upload: passed
  - macOS arm64 build/package/upload: passed
  - Windows x64 build/package/upload: passed
  - GitHub release publication and asset upload: passed
  - The run's final dispatch step failed because the publish job had no checkout-derived repository context. Commit `8a31c2a` fixed the dispatch by passing `--repo` explicitly.
- Published-artifact smoke: [31250599043](https://github.com/kujolang/kujo/actions/runs/31250599043), passed on Linux x64, macOS x64, macOS arm64, and Windows x64 after commit `146a721` corrected the Windows PowerShell extraction path.

## Published Assets

- [Linux x64 archive](https://github.com/kujolang/kujo/releases/download/v1.0.0/kujo-v1.0.0-linux-x64.tar.gz)
- [Linux x64 checksum](https://github.com/kujolang/kujo/releases/download/v1.0.0/kujo-v1.0.0-linux-x64.tar.gz.sha256)
- [macOS x64 archive](https://github.com/kujolang/kujo/releases/download/v1.0.0/kujo-v1.0.0-macos-x64.tar.gz)
- [macOS x64 checksum](https://github.com/kujolang/kujo/releases/download/v1.0.0/kujo-v1.0.0-macos-x64.tar.gz.sha256)
- [macOS arm64 archive](https://github.com/kujolang/kujo/releases/download/v1.0.0/kujo-v1.0.0-macos-arm64.tar.gz)
- [macOS arm64 checksum](https://github.com/kujolang/kujo/releases/download/v1.0.0/kujo-v1.0.0-macos-arm64.tar.gz.sha256)
- [Windows x64 archive](https://github.com/kujolang/kujo/releases/download/v1.0.0/kujo-v1.0.0-windows-x64.zip)
- [Windows x64 checksum](https://github.com/kujolang/kujo/releases/download/v1.0.0/kujo-v1.0.0-windows-x64.zip.sha256)
- [Consolidated checksums](https://github.com/kujolang/kujo/releases/download/v1.0.0/checksums.txt)

## SHA-256

```text
77398f60d0f9d29b7ae1351ea234ba7a56ccc1ce6802e2891d7ff0572ac2e52f  kujo-v1.0.0-linux-x64.tar.gz
e0a9bdb5c74b152f8ba12994e0fdbce856e68f544c769c8aa9535d0bcee71f06  kujo-v1.0.0-macos-arm64.tar.gz
a264f214ec8f5afbf4720c5ac8803fffd7e504c722970245327fd910f8c1474f  kujo-v1.0.0-macos-x64.tar.gz
1d30df557f75b7d3f622b0f00f33b1a7882fafb811a629df993ab5aa5b80d03d  kujo-v1.0.0-windows-x64.zip
```

All four downloaded archives passed their attached `.sha256` verification locally after publication.

## Verification

- `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full`: passed locally.
- `bash scripts/build_local_binary_artifact.sh --version 1.0.0`: passed locally.
- `bash .github/scripts/validate-release-artifact.sh`: passed locally.
- Main-branch CI for the tagged commit: [ci-release-gate 31249260383](https://github.com/kujolang/kujo/actions/runs/31249260383), passed.
- Release-state, LSP, artifact-validation, tool-artifact, and field-notes workflows for the tagged commit all passed before tagging.
- RustSec blocking vulnerability `RUSTSEC-2026-0204` was removed by updating `crossbeam-epoch` to `0.9.20`; `cargo audit --no-fetch --ignore RUSTSEC-2023-0071` then reported no vulnerabilities.
