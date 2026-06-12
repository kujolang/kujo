# Kujo Agent Onboarding

Use this guide as the first repo entrypoint for implementation agents. It is intentionally short and points to canonical files instead of repeating all project docs.

## Read First

1. `README.md`: install, quick start, runtime recommendations, and validation commands.
2. `docs/LANGUAGE_SPEC.md`: current syntax and semantics.
3. `docs/CLI_MACHINE_READABLE_CONTRACTS.md`: JSON output and diagnostics contracts.
4. `docs/ARCHITECTURE.md`: execution paths and subsystem map.
5. `examples/README_examples.md`: canonical examples versus showcase and expected-fail examples.
6. `tests/docs_examples.rs`: source of truth for example smoke policy.

## Repo Map

- `src/`: parser, compiler, VM, interpreter, CLI, LSP, docgen, and runtime support.
- `tests/`: Rust contract tests, Kujo snapshot fixtures, diagnostics goldens, and parity tests.
- `examples/`: learning examples, showcases, benchmarks, and legacy/expected-fail files.
- `docs/`: language, CLI, security, roadmap, release, and architecture docs.
- `scripts/`: release gates and generated inventory helpers.
- `docs/generated/`: generated reports; search here only when the task targets generated inventories.

## Canonical Examples

Start with these before reading larger apps:

1. `examples/hello.kujo`
2. `examples/test_print.kujo`
3. `examples/string_interpolation.kujo`
4. `examples/test_if_else.kujo`
5. `examples/for_loops.kujo`
6. `examples/arrays.kujo`
7. `examples/dictionaries.kujo`
8. `examples/test_simple_func.kujo`
9. `examples/basic_import.kujo`
10. `examples/file_logger.kujo`

Avoid treating files listed in `examples/README_examples.md` under "Legacy or Expected-Fail Examples" as current syntax. That list is mirrored by `expected_fail_examples_with_reason()` in `tests/docs_examples.rs`.

## Validation Commands

Use targeted tests after each logical change. Before finishing broad readability work, run:

```bash
cargo fmt --check
cargo check
cargo test
cargo test --test docs_examples
cargo test --test readme_contracts
cargo test --test cli_contracts
cargo test --test cli_json_contracts
cargo test --test diagnostics_golden
cargo run -- test --runtime vm
cargo run -- test --runtime dual
```

The 2026-06-11 readability audit found drift in `tests/test_higher_order.kujo`, `tests/bytecode_vm.kujo`, and `tests/stdlib_test.kujo`; those fixtures have been realigned. Treat any future direct fixture failure as new drift and inspect expected versus actual output before updating snapshots.

## Search Hygiene

Prefer `rg` and exclude generated or bulky paths unless the task targets them:

```bash
rg "pattern" src tests docs examples \
  -g '!target/**' \
  -g '!docs/generated/**' \
  -g '!examples/ssg/content/**'
```

Useful focused searches:

```bash
rg "expected_fail_examples" tests examples
rg "Diagnostic" src tests/fixtures/diagnostics
rg "lsp-" src tests docs
rg "bench-ssg|BenchSsg" src tests docs
```

Include `docs/generated/**`, benchmark output directories, or `examples/ssg/content/**` only when maintaining generated inventories, benchmarks, or SSG fixtures.

## Change Rules

- Preserve behavior unless the checklist or issue explicitly says behavior should change.
- Update tests or docs with every source change.
- Do not blindly update snapshots; inspect expected versus actual behavior first.
- Do not redesign language syntax during cleanup tasks.
- Avoid broad refactors of `src/vm.rs`, `src/jit.rs`, or interpreter internals unless a specific checklist item requires it.
- Prefer small output helpers and renderer tests before moving CLI formatting code.
- In Kujo examples, avoid long runs of literal `print(...)` calls. Prefer small local helpers such as `print_lines(lines)`, `section(title)`, `kv(label, value)`, and `ok(message)` when they remove repetition without hiding the language feature being demonstrated.
