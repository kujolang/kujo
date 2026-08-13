# Kujo v1.0.1 Published Release Audit

Audit date: 2026-08-13
Release: `v1.0.1`
Release URL: <https://github.com/kujolang/kujo/releases/tag/v1.0.1>
Tagged commit: `86690e62ec323b9e836fa0e98592b83eecb4a494`

## Result

Kujo `v1.0.1` is the latest published stable release. The tag is annotated and points to source metadata version `1.0.1`. The GitHub release is public, non-draft, non-prerelease, and contains all nine assets required by the release artifact contract.

The repository `main` branch has since advanced to Cargo version `1.0.2`, but no `v1.0.2` tag or GitHub release exists. Public documentation that called `v1.0.2` stable was therefore ahead of publication state.

## Published Assets And Verified SHA-256 Values

- `kujo-v1.0.1-linux-x64.tar.gz`: `fb5e48158aa53a330158e7b0c5340c771b55c8d91ff900ba54bc199b1e451da4`
- `kujo-v1.0.1-macos-arm64.tar.gz`: `f97d5509a83bbe1d4bd75ffdd84d44a08266f632fcd010a6cf049dbcd07f438f`
- `kujo-v1.0.1-macos-x64.tar.gz`: `bf33c2a92b3dbf3df188c8b30dd4fbff1d6103868b43c1c3d520e7bf1059342c`
- `kujo-v1.0.1-windows-x64.zip`: `ca4a69061a444312f651e231732470e1b2f0b2d6613c099b5abd34f06de4432f`
- One matching `.sha256` file per archive
- Consolidated `checksums.txt`

Downloaded copies passed both consolidated and per-asset SHA-256 verification. Each archive contains the expected single platform binary (`kujo` or `kujo.exe`). A local macOS x64 artifact smoke passed `kujo --version`, script execution, and `kujo lsp --help`; it reported `kujo 1.0.1`.

## GitHub Actions Evidence

- Release build and publication run `31497224423`: success. The release gate, four platform builds, binary command smokes, packaging, checksums, and asset publication all passed: <https://github.com/kujolang/kujo/actions/runs/31497224423>
- Published-artifact smoke run `31499456642`: success. Linux x64, macOS x64, macOS arm64, and Windows x64 all downloaded the published assets, verified checksums, extracted them, and passed command smokes: <https://github.com/kujolang/kujo/actions/runs/31499456642>

An unrelated `kujo-tool-artifacts-guard` job failed on the tag because six generated-output ignore rules were absent. This did not affect source compilation, release gates, packaging, checksums, publication, or published-binary execution. Commit `d9ec7fbb9df9664b7bbfcd5952d08003340a9d13` added the missing ignore rules immediately after the tag.

## Sign-Off

`v1.0.1` satisfies the repository's official binary-release artifact contract for Linux x64, macOS x64, macOS arm64, and Windows x64. The release is complete and installable for the supported platforms. The release tag is unsigned, so provenance is provided by the successful GitHub Actions build/publication records and SHA-256 assets rather than a cryptographically signed Git tag.
