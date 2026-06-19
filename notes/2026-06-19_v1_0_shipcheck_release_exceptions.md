# V1.0 ShipCheck Release Exceptions

Date: 2026-06-19
Checklist item: V1RR-P0-007
Status: complete

## Summary

ShipCheck scan and checklist were re-run against the Kujo runtime repository. ShipCheck passes with warnings. The warnings are intentional for this repository and are documented in `docs/SHIPCHECK_RELEASE_EXCEPTIONS.md`.

## Decisions

- Format command warning: exception documented. Canonical command is `cargo fmt --check`.
- Lint command warning: exception documented. Canonical command is `cargo clippy --all-targets --all-features -- -D warnings`.
- Missing `kennel.toml`: exception documented. Kujo runtime is a Cargo/Rust language runtime repo, not a Kennel package repo.
- Entry point warning: exception documented. Canonical entry point is `src/main.rs`, binary `kujo`, with metadata in `Cargo.toml`.

## Validation

Command/status manifest:

- `notes/release_evidence/2026-06-19_p0-007/status.tsv`

Commands:

- `./target/debug/kujo run ../shipcheck/shipcheck.kujo -- scan --dir .`
- `./target/debug/kujo run ../shipcheck/shipcheck.kujo -- checklist --dir .`
- `cargo test --test release_process_docs_contract`
- `cargo test --test docs_examples`

