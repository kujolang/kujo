# Output Helper Migration Note

Status: active  
Last updated: 2026-05-31

This note records output-heavy call sites intentionally left on low-level rendering mechanics after the helper consolidation pass.

## Completed In This Pass

- Centralized JSON emission for LSP helper commands in `src/main.rs`.
- Added reusable line-format helpers in `src/cli_output.rs`.
- Refactored workflow pack rendering to deterministic text helpers/snapshots.
- Added deterministic text renderers and snapshot-like tests for:
  - `src/benchmarks/reporter.rs`
  - `src/benchmarks/profiler.rs`
  - `src/repl.rs`
- Added `NO_COLOR` fallback rendering paths for profile and REPL output.

## Intentionally Left Low-Level

1. `src/errors.rs` human diagnostic assembly
- Why left: ordering, spacing, and source-span behavior are tightly coupled to existing diagnostics contract expectations.
- Risk tradeoff: low change frequency, high regression impact.
- Follow-up: extract only non-order-sensitive helpers after dedicated golden coverage expansion.

2. `src/interpreter/test_runner.rs` fixture pass/fail rendering
- Why left: legacy fixture runner output is currently used by multiple parity and migration workflows.
- Risk tradeoff: medium churn but high downstream coupling to fixture tooling.
- Follow-up: helper extraction should be bundled with explicit fixture-output snapshot contracts.

3. Colored benchmark profile presentation details in `print_profile_report`
- Why left: color/styling choices are intentional operator affordances; deterministic no-color renderer now exists in parallel.
- Risk tradeoff: keeping both text and color paths avoids breaking current visual UX.
- Follow-up: unify colorized and plain render pipelines when color policy is standardized across commands.

4. Value-specific REPL pretty-printing (`print_value` and `format_value_inline`)
- Why left: behavior is type-driven and currently optimized for interactive debugging ergonomics.
- Risk tradeoff: helper extraction here can obscure type-specific display semantics.
- Follow-up: only factor shared primitives after broader REPL UX contract is defined.

## Guardrails For Future Migrations

- Preserve exit-code behavior first; output styling changes are secondary.
- Keep JSON schemas and human text contracts separated in tests.
- Prefer additive deterministic text renderers before replacing interactive/colorized paths.
