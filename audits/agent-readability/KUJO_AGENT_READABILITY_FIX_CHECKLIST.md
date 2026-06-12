# Kujo Agent Readability Fix Checklist

## Instructions for Implementation Agent

You are implementing fixes based on the agent readability audit.

Rules:

- Read this checklist first.
- Read KUJO_AGENT_READABILITY_AUDIT.md for context.
- Make one logical change at a time.
- Keep commits small if committing is requested later.
- Do not redesign the language unless a checklist item explicitly says so.
- Preserve existing behavior unless the checklist item says behavior should change.
- Update tests and docs with every source change.
- Prefer clarity over cleverness.
- Prefer table-driven structure only when it improves readability.
- Do not hide simple logic behind unnecessary abstractions.

## Phase 1: Safe Documentation and Example Cleanup

### [x] AR-001: Make README quick start truly minimal

- Type: docs/examples
- Severity: High
- Files: `README.md`, `examples/hello.kujo`, `tests/readme_contracts.rs`
- Problem: README calls an advanced totals/report sample `hello.kujo`, which is not the smallest runnable Kujo program.
- Why this matters: Humans and agents should prove install/run with the smallest possible program before learning functions, arrays, dictionaries, and branches.
- Required change: Replace the first Quick Start code block with a minimal `print` or `greet()` example aligned with `examples/hello.kujo`; move the current totals/report program to a "Next program" subsection.
- Acceptance criteria: README first runnable program is under 10 lines; command and expected output are shown; current richer sample remains available but secondary.
- Validation commands: `cargo test --test readme_contracts`; `cargo test --test docs_examples`; `cargo run -- run examples/hello.kujo`.
- Risk: safe
- Notes: Completed. README now starts with the tracked `examples/hello.kujo` program, includes expected output, and keeps the richer report as a secondary "Next Program" sample.

### [x] AR-002: Reorder examples README around a progressive learning path

- Type: docs/examples
- Severity: High
- Files: `examples/README_examples.md`
- Problem: The examples index foregrounds large interactive apps before the learning path.
- Why this matters: Agents choose early examples as canonical style.
- Required change: Move "Learning Path" to the top after the intro; add install/run context; separate "canonical learning examples" from "showcases" and "legacy/advanced".
- Acceptance criteria: First visible list starts with hello, variables/output, control flow, data structures, functions, modules, file IO, complete apps.
- Validation commands: `cargo test --test docs_examples`.
- Risk: safe
- Notes: Completed. `examples/README_examples.md` now opens with install/run context, a progressive learning path, canonical learning examples, showcases, and a separate legacy/expected-fail section.

### [x] AR-003: Add a root agent onboarding guide

- Type: docs
- Severity: Medium
- Files: `AGENTS.md` or `docs/agent-onboarding.md`, optionally `README.md`
- Problem: There is no single repo entrypoint telling an implementation agent what to read first, which examples are canonical, or which paths to exclude from broad searches.
- Why this matters: Every agent session repeats discovery work and may learn from generated or stale files.
- Required change: Add a concise guide with repo map, first-read docs, canonical examples, validation commands, known direct fixture drift, generated-path search exclusions, and "do not refactor broadly" warnings.
- Acceptance criteria: Guide is under 200 lines; includes `rg` exclusion hints for `target`, `docs/generated`, and `examples/ssg/content`; points to `tests/docs_examples.rs`.
- Validation commands: docs-only; optionally `cargo test --test readme_contracts` if README links are changed.
- Risk: safe
- Notes: Completed. Added root `AGENTS.md` under 200 lines, linked it from README, and included repo map, first-read docs, canonical examples, validation commands, known direct fixture drift, and broad-refactor cautions.

### [x] AR-004: Label or quarantine expected-fail examples

- Type: examples/tests
- Severity: High
- Files: `tests/docs_examples.rs`, `examples/**`, `examples/README_examples.md`
- Problem: 29 known expected-fail examples are mixed into the examples tree without visible user-facing labels.
- Why this matters: Agents and humans can copy parser-incompatible or stale code.
- Required change: Add an examples README section listing expected-fail/legacy examples with reasons, or move them under a clearly named legacy directory and update tests.
- Acceptance criteria: Every path in `expected_fail_examples_with_reason()` is visible as non-canonical in docs; no canonical learning path points to expected-fail files.
- Validation commands: `cargo test --test docs_examples`.
- Risk: medium
- Notes: Completed as documentation quarantine. Every current `expected_fail_examples_with_reason()` path is listed in `examples/README_examples.md` with its non-canonical reason; no canonical learning path entry points to those files.

### [x] AR-005: Create progressive canonical example files

- Type: examples
- Severity: Medium
- Files: `examples/00-hello.kujo`, `examples/01-variables.kujo`, `examples/02-functions.kujo`, `examples/03-control-flow.kujo`, `examples/04-data.kujo`, `examples/05-modules.kujo`, `examples/06-agent-tool.kujo`, `tests/docs_examples.rs`
- Problem: Current examples are numerous but not ordered for install-to-use onboarding.
- Why this matters: Agents benefit from small, progressive, parseable examples.
- Required change: Add or rename a small canonical sequence and include them in the run or parse smoke policy.
- Acceptance criteria: Each file is minimal, runnable or parseable by policy, has expected output where relevant, and uses current syntax.
- Validation commands: `cargo test --test docs_examples`; `cargo run -- run examples/00-hello.kujo`.
- Risk: safe
- Notes: Completed. Added `examples/00-hello.kujo` through `examples/06-agent-tool.kujo`, documented them in `examples/README_examples.md`, and added `00-hello` to run coverage in `tests/docs_examples.rs`; the rest are parse-smoked by default.

## Phase 2: CLI and Output Consistency

### [x] AR-010: Add LSP CLI output helpers

- Type: cli/source
- Severity: Medium
- Files: `src/cli_output.rs`, `src/main.rs`, `tests/cli_contracts.rs`, `tests/cli_json_contracts.rs`
- Problem: LSP subcommands repeat JSON and plain row output branches inline.
- Why this matters: Repetition creates contract drift risk and token-heavy patches.
- Required change: Add small helpers for JSON arrays, optional JSON records, and tab-delimited row emission; migrate `lsp-complete` first.
- Acceptance criteria: `lsp-complete` output is byte-for-byte compatible in plain and JSON modes; helper tests cover serialization failure and row formatting.
- Validation commands: `cargo test --test cli_contracts`; `cargo test --test cli_json_contracts`.
- Risk: medium
- Notes: Completed. Added JSON array, optional JSON record, and tab-delimited row helpers in `src/cli_output.rs`; migrated `lsp-complete` with byte-compatible plain and JSON contract tests passing.

### [x] AR-011: Migrate remaining LSP helper commands one at a time

- Type: cli/source
- Severity: Medium
- Files: `src/main.rs`, `src/cli_output.rs`, `tests/cli_contracts.rs`, `tests/cli_json_contracts.rs`
- Problem: `lsp-definition`, `lsp-references`, `lsp-hover`, `lsp-diagnostics`, `lsp-rename`, and `lsp-code-actions` still duplicate output mechanics.
- Why this matters: Similar commands should share stable emission patterns.
- Required change: Apply the AR-010 helper pattern to each command in separate logical steps.
- Acceptance criteria: No JSON schema or plain-row contract changes unless tests/docs update in the same change.
- Validation commands: `cargo test --test cli_contracts`; `cargo test --test cli_json_contracts`; `cargo test`.
- Risk: medium
- Notes: Completed. Migrated `lsp-definition`, `lsp-references`, `lsp-hover`, `lsp-diagnostics`, `lsp-rename` success rows, and `lsp-code-actions` to shared emitters/row formatting while preserving `lsp-rename` failure JSON envelope.

### [x] AR-012: Add SSG benchmark report renderer snapshot

- Type: cli/tests
- Severity: Medium
- Files: `src/benchmarks/ssg.rs`, `src/main.rs`, `tests/cli_contracts.rs`
- Problem: SSG report output is long and manual, making behavior-preserving refactors risky.
- Why this matters: Output-heavy command handlers are expensive for agents to modify safely.
- Required change: Add a deterministic renderer or snapshot-style test for a representative SSG summary before moving code.
- Acceptance criteria: Test captures headings, metric lines, optional Python comparison, warnings, and gate summary.
- Validation commands: targeted SSG tests; `cargo test`.
- Risk: medium
- Notes: Completed. Added `render_ssg_benchmark_report` snapshot coverage for headings, metrics, Python comparison, profile breakdown, trend warnings, operator hints, and throughput gate summary.

### [x] AR-013: Move SSG report formatting out of command dispatch

- Type: cli/source
- Severity: Medium
- Files: `src/main.rs`, `src/benchmarks/ssg.rs`
- Problem: `Commands::BenchSsg` mixes validation, execution, aggregation, and human report rendering.
- Why this matters: Agents must read too much command code to change output or benchmark logic.
- Required change: Extract rendering into `src/benchmarks/ssg.rs` or a nearby renderer while preserving output.
- Acceptance criteria: `Commands::BenchSsg` primarily orchestrates; report lines are produced by named formatter/render functions; output tests pass.
- Validation commands: SSG tests; `cargo test`; optionally `cargo run -- bench-ssg --help`.
- Risk: medium
- Notes: Completed. `Commands::BenchSsg` now orchestrates validation, execution, aggregation, trend analysis, rendering, and gate exit handling while report lines are produced by `src/benchmarks/ssg.rs`.

### [x] AR-014: Document known direct fixture command status

- Type: docs/cli/tests
- Severity: High
- Files: `README.md`, possibly `docs/VM_INTERPRETER_PARITY_MATRIX.md`
- Problem: README lists `cargo run -- test --runtime vm` and `cargo run -- test --runtime dual`, but both currently fail 3 fixture snapshots.
- Why this matters: Validation docs should not surprise maintainers or agents.
- Required change: Either fix the fixture drift in Phase 3, or add a temporary note that names the failing fixtures and points to the tracking issue/doc.
- Acceptance criteria: README validation section is truthful at the time of change.
- Validation commands: `cargo run -- test --runtime vm`; `cargo run -- test --runtime dual`; `cargo test --test readme_contracts`.
- Risk: safe if docs-only, medium if fixing fixtures.
- Notes: Completed by fixing the Phase 3 fixture drift instead of adding a temporary README caveat. `cargo run -- test --runtime vm` and `cargo run -- test --runtime dual` now pass 144/144.

## Phase 3: Tests, Fixtures, and Harness Cleanup

### [x] AR-020: Resolve `tests/test_higher_order.kujo` snapshot drift

- Type: tests/source
- Severity: High
- Files: `tests/test_higher_order.kujo`, `tests/test_higher_order.out`, related runtime/builtin files if output is wrong
- Problem: Direct `kujo test --runtime vm|dual` expects `Kujo` but got `Ruff`.
- Why this matters: README validation commands fail.
- Required change: Determine whether fixture expected output or language behavior is correct; update exactly one side.
- Acceptance criteria: Fixture passes in VM and dual modes; reason is documented in commit notes if committing later.
- Validation commands: `cargo run -- test --runtime vm`; `cargo run -- test --runtime dual`; `cargo test`.
- Risk: medium
- Notes: Completed. The fixture source reduces `["R", "u", "f", "f"]`, so expected output and inline comment now correctly say `Ruff`; VM and dual fixture runs pass.

### [x] AR-021: Resolve `tests/bytecode_vm.kujo` snapshot drift

- Type: tests/source
- Severity: High
- Files: `tests/bytecode_vm.kujo`, `tests/bytecode_vm.out`, related string/indexing runtime files if behavior is wrong
- Problem: Direct fixture expected `R` then `u`, but VM/interpreter output was `K` then `u`.
- Why this matters: Direct fixture validation fails and string indexing expectations are semantically important.
- Required change: Determine intended string indexing source and update fixture or implementation.
- Acceptance criteria: Fixture passes in VM and dual modes with documented intended behavior.
- Validation commands: `cargo run -- test --runtime vm`; `cargo run -- test --runtime dual`; `cargo test --test vm_interpreter_parity_surfaces`.
- Risk: medium
- Notes: Completed. String indexing is zero-based for the actual string under test, so `text[0]` for `"Kujo"` now expects `K`; VM, dual, and parity-surface tests pass.

### [x] AR-022: Resolve `tests/stdlib_test.kujo` nondeterministic/current-output drift

- Type: tests/source
- Severity: High
- Files: `tests/stdlib_test.kujo`, `tests/stdlib_test.out`, compression/hash native functions if behavior is wrong
- Problem: Direct fixture expected different directory extraction count and hash values than current output.
- Why this matters: Standard library fixture drift is high-trust debt.
- Required change: Isolate environment-dependent files, make fixture deterministic, then update expected output only if current behavior is correct.
- Acceptance criteria: Fixture output is stable across repeated runs from a clean checkout; VM and dual modes pass.
- Validation commands: `cargo run -- test --runtime vm`; `cargo run -- test --runtime dual`; `cargo test --test stdlib_reference_contract`.
- Risk: medium
- Notes: Completed. Added guarded cleanup helpers around archive extraction paths, updated deterministic extraction count to 2, and refreshed SHA-256/MD5 outputs for the fixture input string after verifying current hashes.

### [x] AR-023: Burn down top-tier expected-fail examples

- Type: examples/tests
- Severity: High
- Files: `tests/docs_examples.rs`, high-visibility expected-fail files such as `examples/math_module.kujo`, `examples/pattern_matching.kujo`, `examples/project_markdown_converter.kujo`, `examples/projects/log_parser.kujo`
- Problem: Expected-fail examples include files a new user or agent might reasonably open.
- Why this matters: They teach stale syntax.
- Required change: Repair five high-visibility examples and remove them from `expected_fail_examples_with_reason()`.
- Acceptance criteria: Repaired files parse with `kujo check --quiet`; at least one is promoted to run coverage if safe.
- Validation commands: `cargo test --test docs_examples`; `cargo run -- check examples/<file>.kujo --quiet`.
- Risk: medium
- Notes: Completed. Repaired `examples/math_module.kujo`, `examples/pattern_matching.kujo`, `examples/project_markdown_converter.kujo`, `examples/projects/log_parser.kujo`, and `examples/stdlib_crypto.kujo`; removed them from expected-fail policy and promoted `math_module.kujo` to run coverage.

### [x] AR-024: Add exact output snapshots before output refactors

- Type: tests
- Severity: Medium
- Files: `tests/cli_contracts.rs`, `src/benchmarks/reporter.rs`, `src/benchmarks/profiler.rs`, `src/workflow_pack/renderer.rs`, `src/interpreter/test_runner.rs`
- Problem: Some render tests are contains-based, which is weak for behavior-preserving output refactors.
- Why this matters: Agents need stronger safety rails when reducing output repetition.
- Required change: Add exact or normalized snapshots for surfaces touched by AR-010 through AR-013.
- Acceptance criteria: Refactor target has deterministic baseline coverage before source movement.
- Validation commands: targeted tests plus `cargo test`.
- Risk: safe
- Notes: Completed for touched Phase 2 surfaces. Added exact SSG report renderer coverage and tightened `lsp-complete` plain rows to an exact tab-delimited output assertion.

## Phase 4: Source-Level DRY and Modularity Improvements

### [x] AR-030: Add common runtime diagnostic builders for top errors

- Type: diagnostics/source
- Severity: Medium
- Files: `src/errors.rs`, runtime call sites, `tests/fixtures/diagnostics/*`, `tests/diagnostics_golden.rs`, `tests/cli_json_contracts.rs`
- Problem: Common runtime diagnostics often lack actionable `help`.
- Why this matters: Agents can recover faster from structured errors with consistent help text.
- Required change: Add helpers/builders for undefined identifier, missing module, non-callable call, capability denied, invalid operation.
- Acceptance criteria: Human and JSON diagnostics include stable help where appropriate; goldens update deliberately.
- Validation commands: `cargo test --test diagnostics_golden`; `cargo test --test cli_json_contracts`; `cargo test --test vm_interpreter_parity_surfaces`.
- Risk: medium
- Notes: Completed. Added a targeted runtime help classifier for undefined identifiers, missing modules, non-callable calls, capability denial, invalid binary operations, and invalid unary operations; updated diagnostics goldens and JSON contracts without changing primary error messages.

### [x] AR-031: Refactor canonical examples to local output helper style

- Type: examples/docs
- Severity: Medium
- Files: `examples/file_operations_demo.kujo`, `examples/type_introspection_demo.kujo`, `examples/project_markdown_converter.kujo` after it parses, `docs/FIRST_TOOL_COOKBOOK.md`
- Problem: Cookbook recommends helper-oriented output, but canonical examples still use repeated print sections.
- Why this matters: Agents copy examples more than style notes.
- Required change: Add local `section`, `kv`, and/or `status` helpers to multi-section examples while keeping tiny examples direct.
- Acceptance criteria: Examples remain parseable/runnable by policy; output remains comparable or intentionally updated.
- Validation commands: `cargo test --test docs_examples`; `cargo run -- check examples/<file>.kujo --quiet`.
- Risk: safe to medium
- Notes: Completed. Added local `section`, `kv`, and/or `ok` helpers to `examples/file_operations_demo.kujo`, `examples/type_introspection_demo.kujo`, and the repaired `examples/project_markdown_converter.kujo`; kept tiny examples direct.

### [x] AR-032: Document generated-path search hygiene

- Type: docs
- Severity: Low
- Files: `AGENTS.md` or `docs/agent-onboarding.md`
- Problem: Broad searches include generated inventories, SSG content, target artifacts, and historical notes.
- Why this matters: Agents waste tokens and may mistake generated rows for canonical source.
- Required change: Add recommended `rg` patterns and exclusions.
- Acceptance criteria: Guide includes example searches for source/docs/examples and a note to include generated files only when relevant.
- Validation commands: docs-only.
- Risk: safe
- Notes: Completed with AR-003 in `AGENTS.md`, including `rg` examples and exclusions for `target`, `docs/generated`, and `examples/ssg/content`.

## Phase 5: Larger Design Review Items

### [x] AR-040: Decide whether command metadata should become a single source of truth

- Type: architecture
- Severity: Low
- Files: `src/main.rs`, `docs/CLI_MACHINE_READABLE_CONTRACTS.md`, `README.md`
- Problem: Command names/descriptions are repeated in Clap, README, and contract docs.
- Why this matters: Command docs can drift as CLI grows.
- Required change: Do a design note only. Decide whether Clap metadata remains source of truth or whether docs generation is worthwhile.
- Acceptance criteria: A short decision record exists; no source behavior changes.
- Validation commands: docs-only.
- Risk: safe
- Notes: Completed as design note only in `docs/AGENT_READABILITY_DESIGN_NOTES.md`; Clap remains the operational source of truth for now.

### [x] AR-041: Review large runtime files for future split points

- Type: architecture
- Severity: Low
- Files: `src/vm.rs`, `src/jit.rs`, `src/interpreter/mod.rs`, `src/interpreter/native_functions/mod.rs`, `src/type_checker.rs`
- Problem: Several core files are very large and costly for agents to inspect.
- Why this matters: Long files increase patch risk and context cost.
- Required change: Produce a split-point design note with candidate modules and required tests; do not move code yet.
- Acceptance criteria: Each proposed split names ownership boundary, risk, and validation command.
- Validation commands: docs-only.
- Risk: safe for design; high for implementation.
- Notes: Completed as design note only in `docs/AGENT_READABILITY_DESIGN_NOTES.md`; no VM/JIT/interpreter internals were moved.
