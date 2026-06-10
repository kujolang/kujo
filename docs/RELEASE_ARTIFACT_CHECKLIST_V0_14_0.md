# v0.14.0 Release Artifact Checklist

## Artifact Requirements

- [x] Release binary includes `kujo lsp` command surface
- [x] CLI/LSP contract fixtures and reliability suites are green
- [x] Tree-sitter corpus/query regression assets are present
- [x] First-party extension smoke check is wired and passing
- [x] Release-state guard and artifact validation workflows are present

## Validation Commands

- `bash .github/scripts/check-release-state.sh`
- `cargo test --test cli_json_contracts`
- `cargo test --test lsp_conformance_harness`
- `cargo test --test lsp_reliability_track`
- `bash .github/scripts/validate-release-artifact.sh`

## Notes

- v0.14.0 release prep intentionally leaves unrelated untracked docs untouched.
- Artifact validation logs and per-track evidence are recorded in dated notes under `notes/`.
