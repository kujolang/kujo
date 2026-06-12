# Agent Readability Design Notes

These notes capture design-review outcomes from the agent readability audit. They are not implementation approval for broad source movement.

## AR-040: Command Metadata Source of Truth

Decision: keep Clap command definitions in `src/main.rs` as the operational source of truth for now.

Rationale:

- Clap already owns parsing, help text, defaults, and validation.
- README and `docs/CLI_MACHINE_READABLE_CONTRACTS.md` serve different audiences and should stay curated rather than generated wholesale.
- Generating all command docs from metadata would be useful only after command descriptions, JSON contracts, and examples share a stable schema.

Future split point:

- Add a small metadata snapshot test before introducing generation.
- Start with command name, aliases, exit-code category, and JSON support flags.
- Keep machine-readable JSON contracts in `docs/CLI_MACHINE_READABLE_CONTRACTS.md` until a generator can preserve the current review quality.

Validation for future work:

```bash
cargo test --test cli_contracts
cargo test --test cli_json_contracts
cargo test --test readme_contracts
```

## AR-041: Large Runtime File Split Points

Decision: do not mechanically split VM, JIT, interpreter, native dispatch, or type checker files during readability cleanup.

Candidate split points:

- `src/main.rs`: continue extracting output renderers and command helpers only after exact contract tests exist.
- `src/vm.rs`: consider module-loading helpers, callable dispatch helpers, or diagnostic builders only when a targeted parity test covers the behavior.
- `src/interpreter/mod.rs`: consider moving feature-family evaluation helpers when native/runtime parity tests already isolate the surface.
- `src/interpreter/native_functions/mod.rs`: prefer function-family modules when touching a family for product work; avoid mass moves.
- `src/jit.rs`: defer splits until JIT safety and execution contracts cover the candidate boundary.
- `src/type_checker.rs`: split only around stable diagnostic or annotation subsystems with parser/type-checker contract coverage.

Risks:

- VM/interpreter drift can hide behind mechanical moves.
- JIT changes need safety and fallback validation beyond compile success.
- Native dispatch movement can change capability or alias behavior if registration order changes.

Required validation for any future split:

```bash
cargo fmt --check
cargo check
cargo test
cargo test --test vm_interpreter_parity_surfaces
cargo test --test native_api_security_boundaries
cargo run -- test --runtime vm
cargo run -- test --runtime dual
```
