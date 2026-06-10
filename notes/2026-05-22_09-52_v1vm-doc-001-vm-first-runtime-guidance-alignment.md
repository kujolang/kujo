# V1VM-DOC-001 — VM-First Runtime Guidance Alignment

Date: 2026-05-22
Owner: codex agent

## Scope

Aligned canonical runtime guidance docs so VM-first execution is clear, practical, and contract-locked without suggesting `--interpreter` is required for ordinary modular projects.

## Updates

- `README.md`
  - Added `## Runtime Mode Recommendations` with explicit command-level guidance.
  - Added explicit statement: developers should not need `--interpreter` for ordinary modular layouts.
- `docs/VM_INTERPRETER_PARITY_MATRIX.md`
  - Added `## VM-First Practical Recommendations` aligned with README recommendations.
- `scripts/generate_interpreter_flag_dependency_map.sh` + regenerated output
  - Added `VM-first practical recommendations` section in generated dependency map.

## Contract coverage

- `tests/readme_contracts.rs`
- `tests/runtime_path_matrix_contract.rs`
- `tests/interpreter_flag_dependency_map_contract.rs`

## Validation commands

```bash
cargo test --test readme_contracts
cargo test --test runtime_path_matrix_contract
cargo test --test interpreter_flag_dependency_map_contract
```
