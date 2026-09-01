# Kujo Field Notes — v1.2.0 Release Evidence

**Date:** 2026-09-01
**Session:** 15:18 local
**Branch/Commit:** main / 3eaf214b6d56392c1b2d5aa6b850bcc5ba8241a6
**Tag/Target:** v1.2.0 / ed51720892d8e475980909dffe54c8fba8731e11
**Execution context:** Local macOS x86_64 release host plus GitHub-hosted Linux x64, macOS x64, macOS arm64, and Windows x64 runners.
**Scope:** Signed tag, GitHub release binaries, checksums, published-artifact smoke validation, stable-release promotion, and registry-publication exception for Kujo v1.2.0.

---

## What I Changed

- Created and pushed the signed annotated `v1.2.0` tag at the release commit.
- Published four platform archives, four matching SHA-256 files, and the consolidated `checksums.txt` through the canonical GitHub binary workflow.
- Verified the published binaries on all four hosted platforms and repeated the checksum, version, execution, and LSP smoke locally with the macOS x64 archive.
- Promoted v1.2.0 to the current stable release in the installer, reusable setup action, release policy, and public documentation after artifact verification succeeded.

## Gotchas (Read This Next Time)

- Two unpublished tag attempts failed before artifact publication: the first exposed a stale hard-coded v1.1.0 source-version contract, and the second exposed formatting drift in its repair. Both were fixed before the final signed tag was created.
- `cargo publish --dry-run --locked` remains blocked because Cargo normalization removes the repository-local patched `tiny_http`, while upstream 0.12.0 lacks the read-timeout API required by the hardened HTTP server. This release therefore publishes signed GitHub binaries only.
- The published-artifact smoke is dispatched from `main`, while the immutable release binaries are built from the tag target. Record both SHAs when later `main` commits exist.

## Things I Learned

- The checksum-verified setup action can install the final v1.2.0 macOS x64 release directly and reports the exact asset, digest, installed path, and provenance URL.
- Platform artifact builds can substantially outlast the release gate; stable-release claims must remain unchanged until the publish job and the separate artifact-only smoke both finish successfully.

## Debug Notes (Only if applicable)

| Command or artifact | Result | Notes |
| --- | --- | --- |
| `KUJO_AGENT_FIXTURE_ECOSYSTEM_ROOT=/Users/robertdevore/2026/Kujolang/kujo-repos KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full` | PASS | Clean candidate verification covered formatting, Clippy, all Rust tests, 31 socket-bound serve tests, 146/146 runnable Kujo fixtures, and `cargo audit --deny warnings`. |
| `cargo publish --dry-run --locked` | FAIL / EXCLUDED | The normalized package cannot compile against unpatched upstream `tiny_http`; crates.io publication is explicitly excluded until package integrity is restored. |
| Signed tag `v1.2.0` | PASS | Local SSH verification passed. GitHub reports `verified: true`, reason `valid`; tag object `c9fbb82ce1c1e57530a6e27a15844aeb43a1d38e`, target `ed51720892d8e475980909dffe54c8fba8731e11`. |
| Release binaries run `33544702735` | PASS | Release gate, Linux x64, macOS x64, macOS arm64, Windows x64, packaging, checksums, and GitHub release publication passed. |
| Published-artifact smoke run `33548326185` | PASS | All four hosted platforms downloaded the published archives, verified checksums, extracted them, and passed version, execution, and LSP smokes. |
| Local macOS x64 artifact smoke | PASS | `shasum -a 256 -c` passed; the binary reported `kujo 1.2.0`, printed `artifact-smoke`, and passed `lsp --help`. |
| Local `.github/actions/setup-kujo/install.sh v1.2.0` smoke | PASS | The installer selected macOS x64, verified digest `7bb567822d0912e1c59836b84e894d23d93a4919ad5dc4a6dc6c313b3dbed6cc`, and exposed the release URL as provenance. |
| `bash .github/scripts/check-release-state.sh` | PASS | Cargo version, README source version, latest stable tag, and roadmap state agree on v1.2.0. |
| Focused stable-release documentation and installer contracts | PASS | Rust documentation contracts, setup-action contract, release-manifest installer contract, and `git diff --check` passed. |

Published SHA-256 values:

- Linux x64: `3f9d69779fef64c8f329c69301ac666cf74499ff083f1a5f25f729368b318fcb`
- macOS arm64: `31a26523d7e6edde38e42e54087b56c1333b0ff9fdc3e61e78c303624f82b038`
- macOS x64: `7bb567822d0912e1c59836b84e894d23d93a4919ad5dc4a6dc6c313b3dbed6cc`
- Windows x64: `f2ac9ef3de233a0b23be726d7e56c0f95dae4bcc02e7f42efd1df3fba325568c`

Warnings and exceptions:

- `cargo-deny` was unavailable on the local candidate host; it is an optional gate. The required RustSec audit passed with warnings denied.
- Registry publication is not part of v1.2.0. GitHub release binaries are the canonical distribution until the normalized Cargo package can rebuild successfully.

Release readiness sign-off:

Kujo v1.2.0 is signed, published, checksum-verified, and artifact-smoke-verified on every supported release platform. The GitHub binary release is complete and is the current stable distribution. Crates.io publication remains explicitly blocked and excluded.

## Follow-ups / TODO (For Future Agents)

- Replace or upstream the patched `tiny_http` read-timeout boundary, then restore a passing normalized-package dry run before enabling registry publication.

## Links / References

- GitHub release: <https://github.com/kujolang/kujo/releases/tag/v1.2.0>
- Release binaries: <https://github.com/kujolang/kujo/actions/runs/33544702735>
- Published-artifact smoke: <https://github.com/kujolang/kujo/actions/runs/33548326185>
- Release process: `docs/RELEASE_PROCESS.md`
- Candidate evidence: `notes/2026-09-01_14-17_v1-2-0-release-readiness.md`
