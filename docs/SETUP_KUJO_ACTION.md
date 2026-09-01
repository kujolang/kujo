# Setup Kujo in GitHub Actions

The repository publishes a composite action at `.github/actions/setup-kujo`. Pin the action itself to an immutable Kujo commit and pass an exact published release tag:

```yaml
- name: Setup Kujo
  uses: kujolang/kujo/.github/actions/setup-kujo@8ea137b0235f7272fb776eaf9203e34fc10591b1
  with:
    version: v1.2.0
- run: kujo --version
- run: kujo check main.kujo
- run: kujo test
```

The action supports Linux x64, macOS x64, macOS arm64, and Windows x64 GitHub-hosted runners. It downloads the release archive and its release-provided SHA-256, verifies the archive before extraction, caches by version/OS/architecture, adds the binary to `PATH`, verifies the reported version, and exposes the executable path, version, checksum, asset name, and release URL as outputs.

POSIX runners need `bash`, `curl`, `tar`, and either `sha256sum` or `shasum`; these are present on supported GitHub-hosted images. Windows uses PowerShell, `Invoke-WebRequest`, `Get-FileHash`, and `Expand-Archive`.

The setup action installs prebuilt binaries and does not compile OpenSSL. Workflows that intentionally build Kujo from source on Windows must install `openssl:x64-windows-static-md` with vcpkg and set `VCPKG_DEFAULT_TRIPLET=x64-windows-static-md` plus `OPENSSL_DIR` to that installed tree, matching the release workflow. Source builds must use `cargo build --locked` and should cache Cargo separately.

An unsupported runner, malformed/non-exact tag, missing release asset, malformed checksum, checksum mismatch, or binary-version mismatch fails with a `setup-kujo:` diagnostic. The action never falls back to an ambient or source-built runtime.
