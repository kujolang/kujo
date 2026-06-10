# v0.13.0 Release Artifact Checklist

## Artifact Requirements

- [x] Release binary includes `kujo lsp` command surface
- [x] CLI JSON contract docs published
- [x] LSP conformance fixture harness present
- [x] Tree-sitter grammar baseline assets present
- [x] Editor adapter baseline docs present

## Validation Commands

- `cargo build --release`
- `./target/release/kujo lsp --help`
- `cargo test --test cli_json_contracts`
- `cargo test --test lsp_conformance_harness`
- `cargo test --test editor_adapter_contracts`

## Notes

Final release tagging should occur only after roadmap completion checklist sign-off.

External client smoke evidence:

- `notes/2026-04-30_16-05_external-lsp-client-smoke.md`
