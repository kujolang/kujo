# V1VM-DOC-002 — Interpreter Migration Playbook

Date: 2026-05-22
Owner: codex agent

## Scope

Published a canonical migration playbook for downstream teams currently pinned to `--interpreter`, with deterministic runtime-mode and verification command recipes.

## Artifacts

- New guide: `docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md`
- README link added from `## Runtime Mode Recommendations`
- Regenerated dependency map: `docs/INTERPRETER_FLAG_DEPENDENCY_MAP.md`

## Contract coverage

- `tests/vm_interpreter_migration_playbook_contract.rs`
- `tests/readme_contracts.rs`
- `tests/interpreter_flag_dependency_map_contract.rs`

## Validation commands

```bash
cargo test --test vm_interpreter_migration_playbook_contract
cargo test --test readme_contracts
cargo test --test interpreter_flag_dependency_map_contract
```
