# Release Artifact Validation

This document defines reproducible artifact validation steps for Kujo release candidates.

## Minimum Supported Toolchain And Platform Assumptions

- Rust toolchain: stable Rust `1.86+`
- Supported release validation OS targets:
  - Linux x64: `ubuntu-latest` (GitHub Actions baseline)
  - macOS Intel: `macos-15-intel` (GitHub Actions baseline)
  - macOS Apple Silicon: `macos-15` (GitHub Actions baseline)
  - Windows x64: `windows-latest` (GitHub Actions baseline)

If additional OS or architecture targets are released, this document and the validation matrix must be updated in the same change.

## Clean-Environment Install Validation Flow

The canonical validation command is:

```bash
bash .github/scripts/validate-release-artifact.sh
```

The script performs:

1. `cargo build --release`
2. clean-root install with `cargo install --path . --root <temp> --force --locked`
3. installed binary checks:
   - `kujo --version`
   - `kujo run examples/hello.kujo`
4. SHA-256 checksum generation for `target/release/kujo`
5. SHA-256 verification from generated checksum file
6. local release tarball package/checksum/extract run to confirm repository-independent install path

Cross-platform checksum tools:

- Linux: `sha256sum`
- macOS: `shasum -a 256`
- Windows: `Get-FileHash -Algorithm SHA256`

## Reproducible Binary Verification Steps (Manual)

From repository root:

```bash
cargo build --release
sha256sum target/release/kujo > target/release/kujo.sha256
sha256sum -c target/release/kujo.sha256
```

On macOS, replace `sha256sum` with `shasum -a 256`:

```bash
cargo build --release
shasum -a 256 target/release/kujo > target/release/kujo.sha256
shasum -a 256 -c target/release/kujo.sha256
```

Repository-independent tarball validation example:

```bash
mkdir -p target/release-artifacts
cp target/release/kujo target/release-artifacts/kujo
tar -czf target/release-artifacts/kujo-local.tar.gz -C target/release-artifacts kujo
shasum -a 256 target/release-artifacts/kujo-local.tar.gz > target/release-artifacts/kujo-local.tar.gz.sha256
shasum -a 256 -c target/release-artifacts/kujo-local.tar.gz.sha256

tmp_dir="$(mktemp -d)"
tar -xzf target/release-artifacts/kujo-local.tar.gz -C "$tmp_dir"
"$tmp_dir/kujo" --version
```

## CI Matrix Coverage

The clean-environment install + checksum flow is enforced in:

- `.github/workflows/release-artifact-validation-matrix.yml`

This workflow runs on Linux and macOS for push and pull request events. Tagged
release archives are built and post-publish smoked for Linux x64, macOS x64,
macOS arm64, and Windows x64 by the release workflows documented in
`docs/RELEASE_BINARIES.md`.

## Evidence Requirements

For each release candidate, capture a dated note under `notes/` containing:

- command list executed
- pass/fail result per command
- OS/environment context
- checksum output and verification result
- any exception rationale if full release-gate quality evidence is unavailable
