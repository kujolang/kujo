# Contributing to Kujo Doctor

## Run locally

- `kujo doctor`
- `kujo doctor --json`
- `kujo doctor --deep`
- `kujo doctor --list-profiles`

## Test changes

At minimum:

- `cargo test --test cli_contracts cli_doctor`
- `cargo test --lib workflow_pack::registry::tests`
- `cargo test --lib workflow_pack::manifest::tests`

## Adding checks

Guidelines:

- keep checks generic in default profile
- use stable `id`, `category`, and `reason`
- provide `suggested_fix` for warn/fail when actionable
- avoid parsing English strings for logic

## Updating schema safely

- Keep `schema_version` and documented fields in sync.
- Add fields in a backward-compatible way when possible.
- Document behavior updates in `CHANGELOG.md` and `docs/REPORT_SCHEMA.md`.

## Docs sync

When changing behavior, update:

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/REPORT_SCHEMA.md`
- `docs/EXTENDING_DOCTOR.md`
- `docs/EXAMPLES.md`
