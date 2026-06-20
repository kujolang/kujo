# v1.0.0 Release Artifact Checklist

## Artifact Requirements

- [x] Tagged-release workflow publishes prebuilt binaries for Linux, macOS, and Windows (`.github/workflows/release-binaries.yml`)
- [x] Tagged-release workflow publishes per-asset SHA-256 checksum files and consolidated `checksums.txt`
- [x] Install guide includes copy/paste standalone binary install + checksum verification steps (`INSTALLATION.md`)
- [x] Published-artifact smoke workflow validates artifact-only install/exec path (`.github/workflows/release-published-artifact-smoke.yml`)
- [x] Artifact validation matrix remains active for clean-root local install/checksum verification (`.github/workflows/release-artifact-validation-matrix.yml`)

## Validation Commands

- `bash .github/scripts/validate-release-artifact.sh`
- `cargo test --test vm_interpreter_parity_surfaces`
- `cargo test --test cli_json_contracts`
- `cargo test --test package_module_workflow_integration`
- `cargo test --test stdlib_reference_contract`
- `cargo test --test native_api_security_boundaries`

## Release Asset Naming Contract

For tag `<TAG>` (example: `v1.0.0`), release workflow publishes one archive/checksum pair per supported runner platform and architecture:

- `kujo-<TAG>-linux-x64.tar.gz`
- `kujo-<TAG>-linux-x64.tar.gz.sha256`
- `kujo-<TAG>-macos-x64.tar.gz`
- `kujo-<TAG>-macos-x64.tar.gz.sha256`
- `kujo-<TAG>-macos-arm64.tar.gz`
- `kujo-<TAG>-macos-arm64.tar.gz.sha256`
- `kujo-<TAG>-windows-x64.zip`
- `kujo-<TAG>-windows-x64.zip.sha256`
- `checksums.txt`

## Tag-Time Sign-Off

These rows must remain unchecked until a real release publication event occurs
under the explicit `UNBLOCK_V1_RELEASE` directive defined in
`docs/RELEASE_PROCESS.md`.

- [ ] Publish the actual `v1.0.0` GitHub release.
- [ ] Confirm Linux, macOS, and Windows artifact assets are attached to the release.
- [ ] Confirm per-asset `.sha256` files and `checksums.txt` are attached to the release.
- [ ] Confirm `.github/workflows/release-published-artifact-smoke.yml` passes for the published release.
- [ ] Record artifact URLs, checksum values, and command logs in a dated `notes/` evidence file.

## Notes

- Artifact publication and post-publish smoke validation are automated for release tags.
- Final release evidence (artifact URLs + checksums + command logs) must be recorded under `notes/` for the actual v1.0.0 tag event.
