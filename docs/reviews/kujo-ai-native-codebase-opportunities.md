# Kujo AI-Native Codebase Opportunities Review

## Executive summary

Kujo already has solid foundations for AI-friendly workflows: stable JSON contracts, deterministic text renderers in several subsystems, and a small shared output helper module in src/cli_output.rs.

The biggest remaining opportunities are targeted consistency and compression improvements:

1. Unify remaining output mechanics in high-churn CLI/report surfaces.
2. Reduce repeated line-building/string-assembly logic in renderers.
3. Improve Kujo script examples so they teach intent-first output patterns.
4. Expand human-output contract tests where rendering is currently weakly asserted.
5. Reduce example debt that makes agent context less trustworthy.

Recommended implementation order:

1. Lock down current behavior with stronger tests.
2. Add minimal shared helpers (Rust-side first, then Kujo-script conventions).
3. Refactor highest-value call sites with no behavior changes.
4. Update docs/examples to codify preferred style.
5. Run regression and contract checks before broader migration.

## Highest-priority opportunities

### 1) Consolidate dual-mode CLI output shaping

- Problem
  - LSP command handlers still repeat the same branch shape: collect result, map JSON fields, map plain rows, print.
- Evidence from the codebase
  - Shared JSON serialization helper exists at src/main.rs:955, but output shaping remains duplicated across src/main.rs:2647, src/main.rs:2670, src/main.rs:2704, src/main.rs:2737, src/main.rs:2773, src/main.rs:2798, src/main.rs:2852.
  - SSG benchmark output also has long manual output sequences at src/main.rs:2308-2567.
- Why it matters
  - Human readability and maintainability suffer.
  - Agents are likely to edit one branch and miss sibling branches.
  - Token-heavy repetitive code increases diff noise.
- Recommended direction
  - Add a tiny internal output-emitter layer for row/record emission and JSON mapping wrappers.
- Risks
  - Plain-output formatting regressions.
  - stdout/stderr routing regressions.
- Suggested implementation checklist
  - Add baseline plain/json output tests for every LSP command.
  - Introduce helper primitives: emit_json_rows, emit_plain_rows, emit_optional_record.
  - Migrate one LSP command at a time and run contracts after each step.

### 2) Reduce renderer duplication across workflow, benchmark, and docgen paths

- Problem
  - Multiple renderers manually append lines via push_str and format sequences.
- Evidence from the codebase
  - Workflow markdown renderer: src/workflow_pack/renderer.rs:143-212.
  - Benchmark text renderers: src/benchmarks/reporter.rs:68-158 and src/benchmarks/profiler.rs:357-420.
  - Docgen renderers: src/docgen/render/html.rs:11-79, src/docgen/render/markdown.rs:11-34, src/docgen/core.rs:876-917.
- Why it matters
  - Intent is hidden by mechanical string assembly.
  - Agents must touch many near-identical lines for small output changes.
- Recommended direction
  - Introduce small internal builders (TextBuilder, MarkdownBuilder) with explicit primitives.
- Risks
  - Over-abstraction can hide ordering semantics.
- Suggested implementation checklist
  - Start with workflow markdown renderer.
  - Preserve byte-for-byte output for first migration.
  - Add stronger output tests before expanding to docgen.

### 3) Normalize Kujo script output idioms in examples

- Problem
  - Many examples still teach manual print storms and mechanical report construction.
- Evidence from the codebase
  - examples/file_operations_demo.kujo:4-85.
  - examples/projects/log_parser.kujo:4-193.
  - examples/project_markdown_converter.kujo:177-219.
  - README quick start still models direct output branching at README.md:120-122.
- Why it matters
  - Humans copy verbose patterns.
  - Agents consume token-heavy examples and generate similarly noisy code.
- Recommended direction
  - Adopt canonical local helper idiom in examples (section, kv, item, status).
- Risks
  - Over-refactoring may make beginner examples less direct.
- Suggested implementation checklist
  - Define “tiny script” vs “report script” guideline.
  - Refactor three canonical examples first.
  - Add output snapshots for those examples.

### 4) Expand output contract coverage where tests are currently lenient

- Problem
  - Several rendering tests rely on contains assertions rather than exact output shape.
- Evidence from the codebase
  - workflow markdown test checks fragments: src/workflow_pack/renderer.rs:426-464.
  - benchmark reporter tests are contains-based: src/benchmarks/reporter.rs:182-219.
  - profiler text test is contains-based: src/benchmarks/profiler.rs:575-591.
  - REPL text tests are contains-based: src/repl.rs:643-656.
  - interpreter test runner has output logic but no dedicated output tests: src/interpreter/test_runner.rs:175-218.
  - docgen renderers currently have no output snapshot tests.
- Why it matters
  - Formatting drift can pass tests.
  - Agents lack strong safety rails during refactors.
- Recommended direction
  - Add deterministic full-output snapshots for operator-facing renderers.
- Risks
  - Snapshot churn if formatting intent is not clearly defined.
- Suggested implementation checklist
  - Add snapshot normalization helper.
  - Snapshot strict surfaces first (LSP plain rows, workflow markdown, test-runner summary).
  - Keep contains checks only where intentional flexibility is desired.

### 5) Reduce documentation/example trust debt in AI context

- Problem
  - The expected-fail example list is still large and includes high-visibility scripts.
- Evidence from the codebase
  - tests/docs_examples.rs:91 tracks 29 expected-fail examples.
  - Includes examples/benchmarks/run_benchmarks.kujo (tests/docs_examples.rs:99), examples/project_markdown_converter.kujo (tests/docs_examples.rs:133), and examples/projects/log_parser.kujo (tests/docs_examples.rs:148).
  - Docs already recommend helper-oriented style at docs/FIRST_TOOL_COOKBOOK.md:68 and docs/STANDARD_LIBRARY_REFERENCE.md:86-99, but examples are inconsistent.
- Why it matters
  - Mixed style signals for humans.
  - Agents learn from stale or known-broken patterns.
- Recommended direction
  - Prioritize cleanup for the most copied/high-visibility expected-fail examples.
- Risks
  - Removing examples without replacement reduces tutorial coverage.
- Suggested implementation checklist
  - Rank expected-fail examples by visibility.
  - Repair or quarantine highest-value examples first.
  - Add explicit legacy markers for intentionally non-canonical examples.

## Repeated output/rendering patterns

1. Repeated dual-mode output branching in LSP handlers.
- Evidence: src/main.rs:2647-2869.

2. Repeated status/field/warning output loops in SSG benchmark command.
- Evidence: src/main.rs:2308-2567.

3. Manual markdown report assembly via push_str chains.
- Evidence: src/workflow_pack/renderer.rs:143-212, src/docgen/render/markdown.rs:11-34, src/docgen/core.rs:891-917.

4. Manual HTML assembly via string fragment pushes.
- Evidence: src/docgen/render/html.rs:11-79, src/docgen/core.rs:880-885.

5. Repeated println sequences for headers and metrics.
- Evidence: src/benchmarks/reporter.rs:27-61, src/interpreter/test_runner.rs:178-218.

6. Repeated print section formatting in Kujo examples.
- Evidence: examples/file_operations_demo.kujo:4-85, examples/projects/log_parser.kujo:4-193.

7. Repeated line array push/join mechanics in converter example.
- Evidence: examples/project_markdown_converter.kujo:116-124, examples/project_markdown_converter.kujo:167-171, examples/project_markdown_converter.kujo:177-219.

## Token-efficiency opportunities

1. LSP branch duplication in src/main.rs.
- Why it costs extra tokens
  - Similar JSON/row logic repeated across commands.
- Affects humans/agents
  - Both.
- Best fix type
  - Internal Rust helper + call-site refactor.

2. push_str-heavy report rendering in Rust modules.
- Why it costs extra tokens
  - Mechanical append operations dominate semantic intent.
- Affects humans/agents
  - Both.
- Best fix type
  - Builder helper + snapshot-backed refactor.

3. print storms in high-visibility examples.
- Why it costs extra tokens
  - Repetitive separators and labels dominate logic.
- Affects humans/agents
  - Mostly agents, also humans.
- Best fix type
  - Documentation pattern + canonical example updates.

4. Mechanics-heavy workflow pack sample.
- Why it costs extra tokens
  - Manual summary loops and push mechanics for a common task.
- Affects humans/agents
  - Both.
- Best fix type
  - Docs sample redesign + helper pattern.

5. Expected-fail example corpus as context noise.
- Why it costs extra tokens
  - Agents must reason around known-broken examples.
- Affects humans/agents
  - Both.
- Best fix type
  - Test corpus migration policy.

## Semantic compression opportunities

1. LSP command output currently encodes mechanics, not intent.
- Current shape
  - Per-command branch with inline JSON map + inline plain row formatting.
- Better semantic shape
  - emit_lsp_rows and emit_lsp_optional helpers.

2. Report output currently encodes push mechanics.
- Current shape
  - md.push_str and out.push_str sequences.
- Better semantic shape
  - section, kv, list, table row primitives.

3. Kujo script examples encode formatting mechanics.
- Current shape
  - repeated print of headers, blanks, and aligned labels.
- Better semantic shape
  - section/kv/item/status helper idiom.

4. Converter rendering encodes line-buffer plumbing.
- Current shape
  - push lines and join in multiple functions.
- Better semantic shape
  - focused render helpers per structural region.

## Potential standard primitives or helpers

### 1) CLI output adapters (Rust internal)

- Name candidates
  - emit_json_rows
  - emit_plain_rows
  - emit_optional_record
- Purpose
  - Remove duplicated branch mechanics in dual-mode commands.
- Example API
  - emit_lsp_rows(json_mode, values, to_json, to_plain_row, context)
- Example before/after code

Before:
```rust
if json {
	let rows = values.iter().map(|v| serde_json::json!({ ... })).collect::<Vec<_>>();
	emit_json_or_internal_error(&rows, "context");
} else {
	for v in values {
		println!("...", ...);
	}
}
```

After:
```rust
emit_lsp_rows(
	json,
	&values,
	|v| serde_json::json!({ ... }),
	|v| format!("...", ...),
	"context",
);
```

- Where it would be used
  - src/main.rs:2647-2869.
- Test requirements
  - Row-shape tests for every LSP command.
  - JSON-shape tests and stderr/stdout routing tests.

### 2) TextBuilder (Rust internal)

- Name candidates
  - TextBuilder
  - ReportText
  - Lines
- Purpose
  - Express line, section, kv, bullet semantics directly.
- Example API
  - tb.section("Summary")
  - tb.kv("Files", files)
  - tb.item("hint: rerun with --json")
- Example before/after code

Before:
```rust
out.push_str("## Summary\n\n");
out.push_str(&format!("| PASS | {} |\n", pass));
out.push_str(&format!("| WARN | {} |\n", warn));
```

After:
```rust
tb.section("Summary");
tb.table_row(["PASS", pass.to_string()]);
tb.table_row(["WARN", warn.to_string()]);
```

- Where it would be used
  - src/workflow_pack/renderer.rs, src/benchmarks/reporter.rs, src/benchmarks/profiler.rs, src/docgen/core.rs.
- Test requirements
  - Byte-for-byte deterministic output tests.
  - No-color path parity tests where applicable.

### 3) MarkdownBuilder (Rust internal)

- Name candidates
  - MdBuilder
  - MarkdownOut
  - MdReport
- Purpose
  - Replace manual markdown push_str mechanics with semantic operations.
- Example API
  - md.h2("Recommended Next Actions")
  - md.bullets(actions)
- Example before/after code

Before:
```rust
md.push_str("## Recommended Next Actions\n\n");
for action in actions {
	md.push_str(&format!("- {}\n", action));
}
```

After:
```rust
md.h2("Recommended Next Actions");
md.blank();
md.bullets(actions);
```

- Where it would be used
  - src/workflow_pack/renderer.rs:139-212, src/docgen/render/markdown.rs:11-34, src/docgen/core.rs:891-917.
- Test requirements
  - Full markdown snapshot tests, not fragment-only assertions.

### 4) Kujo-script output helper convention (docs/examples first)

- Name candidates
  - section, kv, item, status_ok, status_warn
  - out.section, out.kv, out.item
- Purpose
  - Standardize intent-first output style for multi-step scripts.
- Example API
  - section("Summary")
  - kv("ERROR", to_string(error_count))
- Example before/after code

Before:
```kujo
print("=== Log Level Summary ===")
print("INFO: " + info_count)
print("ERROR: " + error_count)
print("WARNING: " + warning_count)
```

After:
```kujo
section("Log Level Summary")
kv("INFO", to_string(info_count))
kv("ERROR", to_string(error_count))
kv("WARNING", to_string(warning_count))
```

- Where it would be used
  - examples/file_operations_demo.kujo, examples/projects/log_parser.kujo, docs/WORKFLOW_PACKS.md sample.
- Test requirements
  - Canonical example output snapshots.
  - docs snippet parse/run verification.

### 5) Workflow report accumulator helper (Kujo-script side)

- Name candidates
  - report_add_check
  - report_summary
  - DoctorReportBuilder
- Purpose
  - Eliminate manual summary loops and status drift.
- Example API
  - report = report_add_check(report, check_warn(...))
  - summary = report_summary(report["checks"])
- Example before/after code

Before:
```kujo
summary := {"pass": 0, "warn": 0, "fail": 0, "skip": 0, "info": 0}
while (i < len(checks)) {
	status := checks[i]["status"]
	...
	i = i + 1
}
```

After:
```kujo
summary := report_summary(checks)
```

- Where it would be used
  - docs/WORKFLOW_PACKS.md:105-141 and third-party pack scripts.
- Test requirements
  - Summary correctness tests for mixed statuses.
  - JSON schema compatibility tests against current DoctorReport shape.

## Agent-readability risks

1. Near-identical LSP command blocks can be edited inconsistently.
- Likely failure mode
  - One command updated, sibling commands left stale.
- Reduction strategy
  - Shared emit helpers + per-command contract tests.

2. Mixed style guidance (helper recommendations vs manual examples).
- Likely failure mode
  - Agents copy verbose print mechanics from examples.
- Reduction strategy
  - Promote canonical examples and mark legacy patterns explicitly.

3. Literal-heavy renderer blocks are fragile in partial patches.
- Likely failure mode
  - Unbalanced output fragments and formatting regressions.
- Reduction strategy
  - Builder primitives + whole-output snapshots.

4. Contains-only render tests miss format drift.
- Likely failure mode
  - Meaningful output changes pass tests.
- Reduction strategy
  - Snapshot equality tests for deterministic surfaces.

5. Expected-fail corpus reduces context trust.
- Likely failure mode
  - Agents learn obsolete syntax/patterns.
- Reduction strategy
  - Prioritized expected-fail burn-down.

## Documentation updates needed

1. Update README quick start to include one intent-first output helper example (README.md:103-122).

2. Update docs/WORKFLOW_PACKS.md sample to avoid manual summary loops and direct push mechanics (docs/WORKFLOW_PACKS.md:105-141).

3. Keep docs/FIRST_TOOL_COOKBOOK.md and docs/STANDARD_LIBRARY_REFERENCE.md as style source of truth, then align examples to those recommendations (docs/FIRST_TOOL_COOKBOOK.md:68-81, docs/STANDARD_LIBRARY_REFERENCE.md:86-99).

4. Add a short “AI-readable output patterns” section that defines when direct print is acceptable vs helper style expected.

5. Add migration notes for examples moved from expected-fail to parse/run-clean status (tests/docs_examples.rs:91-165).

## Implementation checklist for the next agent

### Phase 1: Discovery and tests

1. Capture baseline outputs for lsp-complete, lsp-definition, lsp-references, lsp-hover, lsp-diagnostics, lsp-rename, and lsp-code-actions.

2. Expand tests in tests/cli_contracts.rs so every LSP command has:
- plain-output shape assertions,
- JSON shape assertions.

3. Add deterministic renderer snapshots for:
- workflow markdown output,
- benchmark reporter text,
- profiler text,
- REPL banner/help text,
- interpreter test-runner output,
- docgen markdown/html outputs.

4. Add snapshot update and normalization helpers (line endings, optional ANSI stripping).

### Phase 2: Minimal helper implementation

1. Add narrowly scoped LSP output helpers in Rust (internal only).

2. Add TextBuilder and MarkdownBuilder with core primitives:
- line, blank, section, kv, item, table_row.

3. Keep helper APIs small and additive; no output behavior changes in this phase.

4. Add unit tests for helper formatting edge cases.

### Phase 3: Refactor high-value call sites

1. Refactor LSP command handlers in src/main.rs to helper-based output while preserving payload fields and row formats.

2. Refactor workflow markdown renderer (src/workflow_pack/renderer.rs:139-212) to builders with output parity.

3. Refactor one docgen renderer first (src/docgen/render/markdown.rs), then html renderer if stable.

4. Refactor src/interpreter/test_runner.rs output to helper lines and add dedicated tests.

5. Refactor SSG benchmark summary/warning output in src/main.rs:2308-2567 after tests lock formatting intent.

### Phase 4: Documentation and examples

1. Update README quick start to demonstrate concise output style.

2. Update docs/WORKFLOW_PACKS.md sample to use cleaner report/output helpers.

3. Refactor three canonical examples to helper style:
- one simple CLI demo,
- examples/projects/log_parser.kujo,
- examples/project_markdown_converter.kujo (output/report sections first).

4. Add explicit guidance section covering:
- acceptable direct-print cases,
- preferred helper style for multi-step scripts,
- machine-readable output contract guidance.

5. Reclassify fixed examples in tests/docs_examples.rs from expected-fail to parse/run modes.

### Phase 5: Regression review

1. Run contract and golden suites:
- CLI contracts,
- diagnostics goldens,
- docs examples smoke tests,
- renderer snapshots.

2. Verify no JSON schema changes for documented machine-readable outputs.

3. Verify stderr/stdout routing and exit-code behavior remain unchanged.

4. Review diffs for token-noise reduction and semantic clarity improvements.

5. Record intentionally deferred call sites in docs/OUTPUT_HELPER_MIGRATION_NOTE.md with rationale.

## Files reviewed

- src/main.rs
- src/cli_output.rs
- src/workflow_pack/mod.rs
- src/workflow_pack/renderer.rs
- src/benchmarks/reporter.rs
- src/benchmarks/profiler.rs
- src/interpreter/test_runner.rs
- src/repl.rs
- src/errors.rs
- src/docgen/core.rs
- src/docgen/render/html.rs
- src/docgen/render/markdown.rs
- src/docgen/render/mod.rs
- tests/cli_contracts.rs
- tests/diagnostics_golden.rs
- tests/docs_examples.rs
- docs/FIRST_TOOL_COOKBOOK.md
- docs/STANDARD_LIBRARY_REFERENCE.md
- docs/WORKFLOW_PACKS.md
- docs/CLI_DUAL_MODE_OUTPUT_INVENTORY.md
- docs/OUTPUT_HELPER_MIGRATION_NOTE.md
- README.md
- examples/file_operations_demo.kujo
- examples/projects/log_parser.kujo
- examples/project_markdown_converter.kujo
- examples/benchmarks/run_benchmarks.kujo

## Open questions

1. Should the next implementation pass prioritize LSP/CLI consolidation in src/main.rs first, or example/script style cleanup first?

2. For human output, should changes remain byte-for-byte where practical, or are readability-only tweaks acceptable if tests are updated?

3. Should Kujo-script output helpers stay docs/convention-level initially, or should a stdlib/module surface be introduced in the same milestone?

4. Should docgen HTML/markdown builder migration happen now, or after LSP/workflow renderers are stabilized?

5. What expected-fail burn-down target should be set for the next milestone (current count: 29)?
