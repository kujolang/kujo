# Kujo Agent Readability Audit

## Executive Summary

- Overall readiness score: 3.4 / 5.
- Biggest strength: Kujo already has serious agent-facing foundations: structured diagnostics, JSON CLI contracts, LSP helpers, docgen contracts, docs/example smoke tests, and a passing full Rust test suite.
- Biggest weakness: the first learning path and examples corpus are not yet trustworthy enough for an agent to consume without caveats. The README quick start is not the smallest program, the examples index starts with large interactive apps, and `tests/docs_examples.rs` still tracks 29 expected-fail examples.
- Highest-impact improvement: make a canonical agent onboarding path with runnable examples in order, then burn down or quarantine stale examples.
- AI-native positioning verdict: credible foundation, not yet fully earned in day-to-day repo ergonomics. The implementation has strong contracts, but agents still spend too many tokens separating canonical surfaces from legacy or drifted surfaces.

## Repo Context

- Repo path: `/Users/robertdevore/2026/Kujolang/kujo-repos/kujo`
- Requested primary path: `/Users/robertdevore/Documents/Kujolang/kujo-repos/kujo` was unavailable, so the current repository root was used.
- Branch: `main`
- Commit: `108b87f`
- Working tree status: dirty before audit. Existing modified files: `docs/generated/UNSAFE_INVENTORY.md`, `docs/generated/V1_CODE_TODO_TRIAGE.md`, `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`, `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`, `scripts/generate_vm_runtime_mismatch_inventory.sh`, `tests/lsp_external_clients_smoke.rs`.
- Audit date: 2026-06-11 EDT

## Scorecard

| Category | Score | Reason | Highest-impact improvement |
|---|---:|---|---|
| Human readability | 4 | README, architecture docs, language spec, and contracts are direct and useful. Large files and mixed example freshness make navigation heavier than needed. | Add a short canonical reading order and split the most overloaded source/doc surfaces by intent. |
| Agent readability | 3 | JSON contracts and tests help agents, but agents must infer canonical examples from many stale or expected-fail ones. | Add `AGENTS.md` or `docs/agent-onboarding.md` with exact entrypoints, commands, and canonical examples. |
| Token efficiency | 3 | There are repeated print/output blocks, repeated LSP JSON/plain branches, and large examples that obscure intent. | Extend existing output helpers and make examples teach helper-oriented output for multi-section reports. |
| DRY/modularity | 3 | Some helper consolidation exists, but `src/main.rs` still owns several long command bodies and output branches. | Move LSP CLI emission and SSG report rendering behind small typed helpers. |
| CLI clarity | 4 | `kujo --help` is clear and command names are consistent. README command list aligns broadly. | Ensure README validation commands are green or label known fixture drift explicitly. |
| Error/diagnostic clarity | 4 | Structured `Diagnostic` JSON and human renderers are good. Runtime `KujoError` and `Diagnostic` still have two render paths. | Add runtime diagnostic help coverage for common errors and keep JSON/human shapes unified where practical. |
| Docs clarity | 3 | Strong docs exist, but there are many readiness/checklist/generated docs and no single agent-first path. | Make one canonical docs index for install, run, syntax tour, CLI, diagnostics, and tests. |
| Example quality | 2 | There are 243 Kujo examples, but 29 are expected-fail and the examples README foregrounds large interactive apps. | Create `examples/00-hello.kujo` through `examples/06-agent-tool.kujo`, then quarantine stale examples. |
| Test readability | 4 | Contract tests are extensive and `cargo test` passes. Some output tests remain contains-based or allow stale examples. | Add exact snapshots for high-churn human output and reduce expected-fail example debt. |
| Language onboarding flow | 3 | Install/run basics are present, but the first code block jumps to functions, arrays, dictionaries, and conditionals. | Reorder onboarding: install, run hello, print, variables, functions, control flow, data, modules, CLI, testing. |
| Source organization | 3 | Major subsystems are identifiable, but `src/main.rs`, `src/vm.rs`, `src/interpreter/mod.rs`, native dispatch, and JIT are very large. | Extract command handlers/output emitters only where contracts are already covered. |
| Implementation consistency | 4 | VM-first status, security posture, JSON contracts, and generated inventories are carefully tested. | Align direct `kujo test` fixture snapshots with README validation expectations. |

## Repo Map

| Area | Purpose | Important Files | Notes |
|---|---|---|---|
| Root docs | Product positioning, install, quick start, validation | `README.md`, `ROADMAP.md`, `CONTRIBUTING.md`, `INSTALLATION.md` | README is helpful but quick start is too advanced for the first program. |
| Language spec | Syntax and semantic baseline | `docs/LANGUAGE_SPEC.md` | Clear normative structure; good source for agents after quickstart. |
| CLI/runtime | Command definitions and dispatch | `src/main.rs`, `src/cli_output.rs`, `src/errors.rs` | `src/main.rs` is 3073 lines and mixes command definitions, validation, output, and dispatch. |
| Frontend | Lexing, parsing, AST, formatting, linting | `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/formatter.rs`, `src/linter.rs` | Parser is large but contract-tested; avoid broad refactors without specific tests. |
| VM/compiler | Default execution path | `src/compiler.rs`, `src/bytecode.rs`, `src/vm.rs` | VM is central and large; parity tests are strong. |
| Interpreter/native APIs | Fallback runtime and host capabilities | `src/interpreter/*`, `src/interpreter/native_functions/*` | Large native dispatch surface; capability tests are strong. |
| LSP | Editor-oriented analysis | `src/lsp_*.rs`, `tests/lsp_*` | Good modularity, but CLI helper output mapping repeats in `src/main.rs`. |
| Docgen | Universal docs generation | `src/docgen/*`, `tests/docgen_universal.rs` | Strong contracts and JSON shape tests. |
| Examples | Learning and showcase scripts | `examples/`, `examples/projects/`, `examples/benchmarks/` | Too many examples are stale or high-complexity for onboarding. |
| Tests | Contract, integration, fixtures, parity | `tests/`, `tests/fixtures/` | Full `cargo test` passes; direct `kujo test` commands currently fail snapshots. |
| Scripts/generated docs | Release gates and inventories | `scripts/`, `docs/generated/` | Generated inventories are useful but noisy for first-time agents. |
| Tooling adapters | Editor and tree-sitter support | `tools/`, `tree-sitter-kujo/`, `docs/editor-adapters/` | Good for ecosystem readiness, not first-read material. |

## Highest-Impact Findings

### Finding 1: Canonical onboarding starts too late

- Severity: High
- Area: docs/onboarding
- Files: `README.md`, `examples/hello.kujo`, `examples/README_examples.md`
- Evidence: README labels an advanced function/array/dict/control-flow sample as `hello.kujo` at `README.md:113-140`, while the actual smallest runnable program is `examples/hello.kujo` with a `greet()` wrapper that prints `Kujo Kujo!`. The examples README lists complete interactive apps first at `examples/README_examples.md:5-45`; the learning path appears later at `examples/README_examples.md:128-135`.
- Why it matters for humans: beginners hit multiple concepts before they have confirmed the language runs.
- Why it matters for AI agents: agents must spend tokens reconciling README quick start, actual example files, and the examples index.
- Token-efficiency impact: high for onboarding and generated code because the first copied pattern is larger than necessary.
- Recommended fix: make the first README code block a true minimal print/run example; move the current report example to "Next program"; reorder the examples README around progressive learning.
- Implementation risk: safe
- Suggested validation: `cargo test --test readme_contracts`; `cargo test --test docs_examples`; `cargo run -- run examples/hello.kujo`.

### Finding 2: Example trust debt is explicit and large

- Severity: High
- Area: examples/tests
- Files: `tests/docs_examples.rs`, `examples/**`
- Evidence: `tests/docs_examples.rs:117-183` tracks 29 expected-fail examples, including `examples/project_markdown_converter.kujo`, `examples/projects/log_parser.kujo`, `examples/math_module.kujo`, `examples/pattern_matching.kujo`, and `examples/stdlib_crypto.kujo`. Only five examples are run end-to-end in `run_examples()` at `tests/docs_examples.rs:107-115`.
- Why it matters for humans: users can land on examples that are known stale or parser-incompatible.
- Why it matters for AI agents: stale examples become false training context inside the repo.
- Token-efficiency impact: high because agents must inspect tests to learn which examples are canonical.
- Recommended fix: rank expected-fail examples by visibility, repair the top tier, and move or label legacy examples so agents do not treat them as current syntax.
- Implementation risk: medium
- Suggested validation: `cargo test --test docs_examples`; `cargo run -- check <repaired-example> --quiet`; update `expected_fail_examples_with_reason()` after each repair.

### Finding 3: README validation commands are not currently all green

- Severity: High
- Area: validation/fixtures
- Files: `README.md`, `tests/*.kujo`, `tests/*.out`
- Evidence: README lists `cargo run -- test --runtime vm` and `cargo run -- test --runtime dual` at `README.md:203-210`. Both commands exited `3` in this audit, passing 141/144 fixtures. Failing fixtures were `tests/test_higher_order.kujo`, `tests/bytecode_vm.kujo`, and `tests/stdlib_test.kujo`.
- Why it matters for humans: a documented validation command that fails reduces confidence.
- Why it matters for AI agents: agents may waste time trying to fix unrelated fixture drift during ordinary audits.
- Token-efficiency impact: medium to high because command output is long and failure classification is not summarized in docs.
- Recommended fix: either update snapshots/fixtures intentionally or document the known failing fixture set until fixed.
- Implementation risk: medium
- Suggested validation: `cargo run -- test --runtime vm`; `cargo run -- test --runtime dual`; `cargo test --test cli_contracts`.

### Finding 4: LSP CLI output mapping repeats the same shape across commands

- Severity: Medium
- Area: CLI output
- Files: `src/main.rs`, `src/cli_output.rs`, `tests/cli_contracts.rs`, `tests/cli_json_contracts.rs`
- Evidence: `src/main.rs:2674-2895` repeats command-specific JSON mapping plus plain row printing for `lsp-complete`, `lsp-definition`, `lsp-references`, `lsp-hover`, `lsp-diagnostics`, `lsp-rename`, and `lsp-code-actions`. `src/cli_output.rs:1-33` already contains small JSON and line helpers.
- Why it matters for humans: intent is hidden by repetitive serialization mechanics.
- Why it matters for AI agents: a future edit can update JSON but miss plain rows, or vice versa.
- Token-efficiency impact: medium; repeated branches cost reading and diff tokens.
- Recommended fix: add tiny typed helper functions for JSON rows, optional records, and TSV/plain rows, then migrate one LSP command at a time.
- Implementation risk: medium
- Suggested validation: `cargo test --test cli_contracts`; `cargo test --test cli_json_contracts`.

### Finding 5: SSG benchmark output is long, manual, and only partially helper-based

- Severity: Medium
- Area: CLI output/reports
- Files: `src/main.rs`, `src/benchmarks/ssg.rs`, `tests/cli_contracts.rs`
- Evidence: `Commands::BenchSsg` spans `src/main.rs:2223-2585` and emits many manual `println!` lines for summaries, profiles, trends, and warnings. Some `cli_output::format_kv` and `format_list_item` helpers are used, but the report remains mostly inline.
- Why it matters for humans: the command handler is hard to scan.
- Why it matters for AI agents: agents must keep output contract details in working context while editing benchmark logic.
- Token-efficiency impact: medium.
- Recommended fix: move report formatting into `src/benchmarks/ssg.rs` or a dedicated renderer, preserving byte-for-byte output first.
- Implementation risk: medium
- Suggested validation: existing SSG tests plus a new deterministic text snapshot for a representative summary.

### Finding 6: Docs recommend helper-oriented output, but examples still teach print-heavy style

- Severity: Medium
- Area: examples/docs
- Files: `docs/FIRST_TOOL_COOKBOOK.md`, `examples/*.kujo`, `examples/projects/*.kujo`
- Evidence: `docs/FIRST_TOOL_COOKBOOK.md:66-83` recommends local helpers such as `section` and `kv`. Many high-visibility examples still use long repeated `print(...)` blocks and section banners, such as `examples/student_grade_tracker.kujo`, `examples/type_introspection_demo.kujo`, and `tests/stdlib_crypto_test.kujo`.
- Why it matters for humans: style guidance and copied examples conflict.
- Why it matters for AI agents: agents copy the verbose pattern more often than the guidance.
- Token-efficiency impact: high for generated Kujo scripts.
- Recommended fix: refactor a small canonical set of examples to helper style and mark direct-print examples as tiny/tutorial-only.
- Implementation risk: safe to medium, depending on snapshot coverage.
- Suggested validation: `cargo test --test docs_examples`; `cargo run -- check examples/<file>.kujo --quiet`.

### Finding 7: Source organization is navigable but has large high-context files

- Severity: Medium
- Area: source organization
- Files: `src/main.rs`, `src/vm.rs`, `src/interpreter/mod.rs`, `src/interpreter/native_functions/mod.rs`, `src/jit.rs`, `src/type_checker.rs`
- Evidence: line counts show `src/vm.rs` at 9710 lines, `src/jit.rs` at 9311, `src/interpreter/mod.rs` at 6281, `src/interpreter/native_functions/mod.rs` at 6192, and `src/main.rs` at 3073.
- Why it matters for humans: localized changes require more scrolling and context management.
- Why it matters for AI agents: large files are expensive to read and easy to patch too broadly.
- Token-efficiency impact: medium to high.
- Recommended fix: do not split for its own sake; extract only stable command/output/rendering seams that have contract tests.
- Implementation risk: high if applied broadly; medium for CLI output helpers.
- Suggested validation: focused tests per extracted area plus full `cargo test`.

### Finding 8: Runtime diagnostics are structured, but help/actionability is uneven

- Severity: Medium
- Area: diagnostics
- Files: `src/errors.rs`, `tests/fixtures/diagnostics/*`, `tests/diagnostics_golden.rs`
- Evidence: `Diagnostic::render_human()` and JSON shape are clean in `src/errors.rs:220-278`, and golden tests cover parser/lexer/runtime/CLI/server cases. Some runtime JSON goldens still have `"help": null` for common failures such as undefined identifiers and capability denial, even though `tests/cli_json_contracts.rs` has started adding runtime hints for selected cases.
- Why it matters for humans: common runtime failures should tell users the next action.
- Why it matters for AI agents: action-oriented `help` reduces retry/search loops.
- Token-efficiency impact: medium.
- Recommended fix: add targeted runtime diagnostic help for common errors: undefined identifier, non-callable call, missing module, capability denial, invalid unary/binary operation.
- Implementation risk: medium
- Suggested validation: `cargo test --test diagnostics_golden`; `cargo test --test cli_json_contracts`; update goldens deliberately.

### Finding 9: There is no single agent-first repo entrypoint

- Severity: Medium
- Area: docs/navigation
- Files: `README.md`, `.github/AGENT_INSTRUCTIONS.md`, `docs/*`
- Evidence: README has core links at `README.md:74-86`, architecture docs list related docs at `docs/ARCHITECTURE.md`, and `.github/AGENT_INSTRUCTIONS.md` exists, but there is no root `AGENTS.md` or `docs/agent-onboarding.md` that tells an implementation agent what to read first and which examples are canonical.
- Why it matters for humans: contributors must infer the maintenance path.
- Why it matters for AI agents: every task begins with repeated repo discovery.
- Token-efficiency impact: high across repeated agent sessions.
- Recommended fix: add a concise root `AGENTS.md` that points to README, language spec, CLI contracts, architecture, docs/examples tests, and safe validation commands.
- Implementation risk: safe
- Suggested validation: docs-only review plus `cargo test --test readme_contracts` if README links change.

### Finding 10: Generated reports and scripts are useful but noisy for first-read context

- Severity: Low
- Area: generated reports/scripts
- Files: `docs/generated/*`, `scripts/generate_*`
- Evidence: generated inventories and scripts are thorough, but broad file searches surface many generated rows and repeated `echo`/table-building mechanics before core language files.
- Why it matters for humans: first-time navigation includes release-process noise.
- Why it matters for AI agents: broad searches spend tokens on generated artifacts.
- Token-efficiency impact: medium for repository inspection.
- Recommended fix: document search hygiene in `AGENTS.md`, such as excluding `docs/generated`, `target`, `examples/ssg/content`, and generated benchmark outputs unless the task targets them.
- Implementation risk: safe
- Suggested validation: docs-only.

## DRY and Repetition Opportunities

- CLI output: LSP helper branches in `src/main.rs:2674-2895`; SSG benchmark report output in `src/main.rs:2223-2585`; profile/flamegraph output around `src/main.rs:2640-2667`.
- diagnostics/errors: `Diagnostic` and `KujoError` are both useful, but runtime help/actionability should be normalized through shared diagnostic builders where possible.
- parser/runtime: no blanket parser or VM refactor recommended; the files are large, but tests are extensive and behavior-sensitive.
- docs: README, examples README, first-tool cookbook, CLI contracts, and language spec overlap on first-run and command guidance. Make one onboarding path canonical.
- examples: repeated `print` banners and section output in interactive/project examples. Refactor only canonical examples first.
- tests: `tests/docs_examples.rs` centralizes smoke policy well, but expected-fail examples are a backlog. Some output tests use contains checks; add exact snapshots for surfaces targeted by refactors.
- fixtures: language fixture snapshots are useful, but direct `kujo test --runtime vm|dual` currently fails three fixtures.
- generated reports: shell scripts build markdown with repeated `echo` blocks. Keep unless actively maintaining generated report style; document generated paths as search exclusions.

## Agent-Readable Formatting Opportunities

- docs structure: add a canonical agent onboarding document with "read these first" and "do not read these unless needed" sections.
- headings: examples README should lead with "Start Here", not "Featured Examples".
- command reference: keep `docs/CLI_MACHINE_READABLE_CONTRACTS.md` canonical for JSON; add a shorter `docs/cli-reference.md` or README table for humans if drift becomes a problem.
- examples: add expected output blocks to the first progressive examples.
- JSON/markdown output: existing JSON contracts are a strength; keep machine JSON on stdout and human diagnostics on stderr.
- diagnostics: add `help` to common runtime failures.
- onboarding path: install -> run hello -> print/output -> variables -> functions -> control flow -> data structures -> modules/imports -> CLI commands -> tests/checks -> agent-native workflows.

## Language Onboarding Review

- What should a new developer read first? README install/status, then a true minimal hello example, then `docs/LANGUAGE_SPEC.md` sections 2-5.
- What should an AI agent read first? A new `AGENTS.md`, README, `docs/LANGUAGE_SPEC.md`, `docs/CLI_MACHINE_READABLE_CONTRACTS.md`, `tests/docs_examples.rs`, and `docs/ARCHITECTURE.md`.
- What is missing? Root agent instructions that identify canonical examples, validation commands, known failing fixture commands, and generated-path search exclusions.
- What is duplicated? CLI command lists in README/help/contracts; output style guidance in cookbook versus example files; runtime mode guidance across README and VM migration docs.
- What is confusing? README "Create `hello.kujo`" sample is not minimal; examples README frontloads complete apps; expected-fail examples are not visible from examples docs.
- What should be reorganized? Move progressive examples to the top; quarantine stale examples; make docs/CLI contracts clearly the canonical source for machine JSON.

## Source Modularity Review

- `src/main.rs`: split LSP command emission and SSG report formatting only after tests lock output. Avoid moving Clap definitions unless command metadata generation is explicitly planned.
- `src/cli_output.rs`: extend current helpers with row/record emitters rather than creating a new abstraction tree.
- `src/errors.rs`: keep existing ordering-sensitive renderers, but add builders/helpers for common runtime diagnostics.
- `src/interpreter/native_functions/mod.rs`: large dispatch surface. Prefer smaller function-family modules when touching a family; do not mass-move.
- `src/vm.rs` and `src/jit.rs`: very large, but high-risk. Only split around stable internal components with parity tests.
- `tests/docs_examples.rs`: keep as policy center, but use it to drive expected-fail burn-down.

## Risks and Non-Goals

- Do not shorten names if clarity suffers.
- Do not DRY code that is intentionally explicit for parser tests.
- Do not change syntax without a language design review.
- Do not change CLI output contracts without updating tests/docs.
- Do not remove examples that serve different onboarding levels; repair, move, or label them.
- Do not split VM/interpreter/JIT files mechanically just to reduce line counts.
- Do not hide simple tutorial logic behind helpers in the very first tiny examples.
- Do not update fixture snapshots blindly; verify whether expected or actual output is the intended behavior.

## Recommended Implementation Order

1. Safe docs/examples cleanup: README minimal hello, examples README reorder, agent onboarding doc, search hygiene.
2. Safe CLI/output consistency cleanup: extend `src/cli_output.rs`; migrate one LSP helper command at a time.
3. Test harness and fixture cleanup: resolve three direct `kujo test` fixture drifts; add snapshots for LSP plain rows and SSG summary.
4. Source-level DRY improvements: move SSG rendering into a renderer; add runtime diagnostic help builders.
5. Larger language or architecture review items: expected-fail example burn-down, optional broader source splits, any syntax/support decisions.
