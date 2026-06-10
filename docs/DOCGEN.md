# DOCGEN

`kujo docgen` is Kujo's universal documentation generator.

It is model-driven and adapter-based:

- Universal core pipeline
- Language adapters
- Shared symbol model
- Gap analyzer
- Renderers (HTML, Markdown, JSON)
- CI quality gates
- Optional AI task emission

## Supported Languages

- Kujo (`.kujo`)
- PHP (`.php`)
- Python (`.py`)
- TypeScript (`.ts`, `.tsx`)
- JavaScript (`.js`, `.jsx`, `.mjs`, `.cjs`)
- Ruby (`.rb`)
- Go (`.go`)
- Haskell (`.hs`, `.lhs`)
- Zig (`.zig`)

## Core Design

The core lives under `src/docgen/` and is language-agnostic.

- `core.rs`: orchestration + output + gates
- `discovery.rs`: safe deterministic file discovery
- `model.rs`: shared project/module/symbol/gap model
- `gaps.rs`: missing-doc and link-gap analysis
- `render/*`: HTML/Markdown/JSON rendering
- `adapters/*`: language-specific symbol/doc extraction

## Kujo Visibility Policy

For Kujo symbols, DocGen visibility is explicit and gate-oriented:

1. Top-level symbols are `Public` only when declared with `pub`:
   - `pub func ...`
   - `pub struct ...`
   - `pub enum ...`
   - `pub const ...` / `pub let ...`
2. Non-`pub` top-level symbols are `Private`.
3. Struct method visibility requires both:
   - method declaration is `pub`
   - containing struct is `Public`
   Methods under private structs remain `Private` even if the method itself is declared `pub`.
4. Enum variants inherit visibility from the containing enum:
   - variants under `pub enum` are `Public`
   - variants under non-`pub enum` are `Private`
5. `--public-only` with `--include-private` disabled filters to `Public` symbols only and is the intended strict CI gate surface.

Visibility classification is implemented through shared adapter helpers that centralize explicit modifier mapping, naming-convention mapping, and container/member effective visibility rules. Kujo and TypeScript adapter semantics are regression-locked and unchanged by this refactor.

## Kujo Doc Comment Styles

Kujo DocGen currently attaches inline documentation from these Kujo comment forms:

1. `///` line doc comments
2. `//!` line doc comments
3. `/** ... */` block doc comments

Non-doc block comments (`/* ... */`) are not treated as API documentation.
Attachment matching is decorator-aware: DocGen will skip Kujo decorator/attribute lines (for example `@...` and `#[...]`) between a doc block and its symbol target.
Proximity behavior is stable and test-locked:
1. Blank lines between a doc block and symbol are allowed.
2. Regular non-doc comment lines break attachment.
3. The nearest eligible doc block is attached when multiple blocks appear before a symbol.

## Kujo Extraction Decision (Workstream B-3)

Decision: keep a hybrid Kujo extraction strategy, with regex-based symbol discovery as the default production path and an opt-in parser-assisted prototype (`--kujo-parser-assisted`) that gracefully falls back to regex on lexer/parser diagnostics.

Rationale:

1. `kujo docgen` is expected to be best-effort and resilient even when repositories contain partially invalid Kujo sources; a hard parser-only path would reject symbol extraction for files that fail full parse.
2. Current parser surfaces are optimized for program execution semantics, so parser-assisted extraction is bounded and guarded by deterministic fallback to avoid CI gate instability.
3. Fixture-backed regressions (`tests/fixtures/docgen/*`) cover both parser-success and parser-fallback paths to keep ordering/strict-gate behavior stable.

Follow-through policy:

1. Continue expanding fixture-backed Kujo extraction edge cases as they are discovered.
2. Keep parser-assisted extraction opt-in while broadening fixture coverage before any default-path promotion.
3. Preserve deterministic output and strict-gate stability as non-negotiable acceptance criteria for any parser-assisted rollout.

## Security Model

DocGen is scan-only.

- No source code execution
- No imports/build steps
- No external AI calls by default
- Symlink traversal is skipped during discovery
- File size, depth, and file count limits are enforced
- Deterministic ordering for CI stability
- HTML output escapes documentation content by default

## Discovery Skip Diagnostics

When discovery limits skip input, DocGen emits warning diagnostics in `docgen.json`:

1. `DOCGEN_DISCOVERY_MAX_FILE_SIZE`
2. `DOCGEN_DISCOVERY_MAX_DEPTH`
3. `DOCGEN_DISCOVERY_MAX_FILES`
4. `DOCGEN_DISCOVERY_INVALID_ENCODING`

`kujo docgen --json` also reports per-reason skip counters under `discovery_skip_counts`:
1. `max_file_size`
2. `max_depth`
3. `max_files`
4. `invalid_encoding`
5. Link-validation budget truncation counters under `link_validation_skip_counts`:
   - `max_link_checks`
   - `max_external_checks`
   - `max_total_time`
6. Symbol volume counters:
   - `item_count` (total symbols in output scope)
   - `project_symbol_count` (non-builtin symbols)
   - `builtin_symbol_count` (builtin symbols)
7. Per-kind symbol counters:
   - `symbol_kind_counts` with deterministic keys such as `function`, `method`, `struct`, `enum`, `enum_variant`, and `builtin`
8. Stable dashboard summary block:
   - `summary.schema_version` (`docgen-summary/v1`)
   - `summary` mirrors key totals/gate counters for machine consumers while preserving existing top-level contract fields
9. Effective discovery limits block:
   - `discovery_limits.max_file_size_bytes`
   - `discovery_limits.max_depth`
   - `discovery_limits.max_files`
   - mirrored under `summary.discovery_limits`

Discovery limits can be overridden per run through:
1. CLI flags (`--max-discovery-file-size-bytes`, `--max-discovery-files`, `--max-discovery-depth`)
2. Environment (`KUJO_DOCGEN_MAX_FILE_SIZE_BYTES`, `KUJO_DOCGEN_MAX_FILES`, `KUJO_DOCGEN_MAX_DEPTH`)
3. Built-in defaults when neither CLI nor env are set (CLI values take precedence over env values).

## Adapter Health Diagnostics

`kujo docgen --json` emits per-language extraction counters under `adapter_health` (and mirrored under `summary.adapter_health`):
1. `files_scanned`
2. `symbols_extracted`
3. `doc_blocks_attached`
4. `placeholders_emitted`

DocGen emits `DOCGEN_ADAPTER_LOW_YIELD` warnings when extraction yield is suspiciously low for scanned language inputs:
1. Three or more files scanned with zero extracted symbols.
2. Ten or more files scanned with fewer than one extracted symbol per five files.

## Incremental Cache Mode

DocGen supports optional incremental extraction reuse for CI through `--cache-dir`:
1. Per-file extraction artifacts are cached using a key that includes source content hash, language, and adapter cache version.
2. Cache hits reuse extracted symbol payloads and still preserve deterministic module/symbol output ordering.
3. Cache misses recompute extraction and refresh cache entries.

Machine-readable JSON includes deterministic cache counters in both top-level and summary blocks:
1. `cache_stats.hits`
2. `cache_stats.misses`

Discovery and project diagnostics are emitted in deterministic sorted order for CI-stable JSON comparisons.

## CLI

### Basic Kujo docs

```bash
kujo docgen src/ --language kujo --out-dir docs/generated
```

### Auto-detect languages

```bash
kujo docgen . --out-dir docs/generated
```

### Explicit multi-language run

```bash
kujo docgen . --languages kujo,php,python,typescript,javascript,ruby,go,haskell,zig --out-dir docs/generated
```

### Strict CI gates

```bash
kujo docgen . --public-only --fail-on-undocumented --fail-on-broken-links
```

### AI-ready gap files

```bash
kujo docgen . --emit-ai-tasks --out-dir docs/generated
```

## Output Files

The output directory includes:

- `index.html`
- `docgen.md` (when format includes markdown)
- `docgen.json`
- `docgen-gaps.json`
- `docgen-capabilities.json`
- `docgen-ai-tasks.md` (with `--emit-ai-tasks`)
- `builtins.html` (unless `--no-builtins`)
- `search-index.json` + `symbol-index.json` (with `--search-index`)

## Gaps and Placeholders

Public symbols are documented even when no inline docs exist.

Missing docs are rendered as:

- `Documentation needed.`
- `This symbol was discovered from the source code, but no human-authored documentation was found.`

`docgen-gaps.json` and `docgen-ai-tasks.md` include bounded source context and constrained prompts:

- Use only provided context
- Do not invent behavior
- Mark uncertainty
- Keep docs concise
- Add examples only when source supports them

## Link Validation Default Mode

Default DocGen link validation is local-file existence only:
1. Local links are checked by filesystem existence.
2. Local link fragments (`#anchor`) and query segments (`?query`) are ignored in default mode.
3. External links (`http://`, `https://`, `mailto:`) are not validated in default mode.

Optional local-anchor validation mode is available with `--validate-local-anchors`:
1. Local links that include a fragment (`#...`) require the target anchor to exist in the referenced local file.
2. Markdown heading slugs and basic HTML `id="..."`/`name="..."` anchors are supported.

Optional external-link validation mode is available with `--validate-external-links`:
1. Validation only runs for hosts in `--external-link-allowlist`.
2. Private/loopback/link-local/multicast targets are blocked by default (including DNS-resolved hostnames) to reduce SSRF risk.
3. Use `--allow-private-network-links` to opt in when private-network link validation is intentionally required.
4. Allowlist confinement is enforced on every redirect hop; if a redirect leaves the allowlist, the link is reported as broken with mode `external-redirect-allowlist`.
5. Validation requests use `--external-link-timeout-ms`.
6. Links that fail allowlisted external validation are reported as broken links.
7. If external validation is enabled with an empty allowlist, DocGen emits `DOCGEN_LINK_EXTERNAL_ALLOWLIST_EMPTY`.
8. If an allowlist is provided without `--validate-external-links`, DocGen emits `DOCGEN_LINK_EXTERNAL_ALLOWLIST_IGNORED`.
9. Broken-link diagnostics and gate failures include mode-specific categories (`local_file`, `local_anchor`, `external`, `external_redirect_allowlist`, `external_private_address`) for clearer CI triage.
10. Link validation resource budgets are available for bounded CI/runtime behavior:
   - `--max-link-checks`
   - `--max-external-link-checks`
   - `--max-total-validation-time-ms`
11. When a budget truncates checks, DocGen emits deterministic warnings:
   - `DOCGEN_LINK_VALIDATION_BUDGET_MAX_LINK_CHECKS`
   - `DOCGEN_LINK_VALIDATION_BUDGET_MAX_EXTERNAL_CHECKS`
   - `DOCGEN_LINK_VALIDATION_BUDGET_TOTAL_TIME`
   and reports skip counts in `link_validation_skip_counts`.

## Source-Link Providers

DocGen source-link rendering supports pluggable template providers:
1. Default behavior (no template configured) keeps source rendering unchanged (plain source location text).
2. `--source-link-template` enables URL template expansion when `--source-links` is enabled.
3. Supported template placeholders:
   - `{path}` (normalized, percent-encoded relative source path)
   - `{line}` (1-based source line)
4. Path normalization safety:
   - absolute paths are rejected
   - parent-traversal paths (`..`) are rejected
   - rejected paths do not emit template links and fall back to plain source-location rendering

## QA Hardening Roadmap (Post-Feature Completion)

The following roadmap is a focused QA/pass-two backlog for tightening DocGen implementation quality after the initial feature-completion tracks.

### P0 Security And Reliability

1. [x] `DG-QA-001` External link redirect confinement and allowlist re-validation. (Completed 2026-05-18)
   Acceptance criteria:
   - Re-validate host allowlist on every redirect hop, not only on the initial URL.
   - Emit deterministic diagnostics when redirects leave the allowlist.
   - Add regression tests for same-host redirect, cross-host allowed redirect, and blocked redirect.
2. [x] `DG-QA-002` SSRF guardrails for external link mode. (Completed 2026-05-18)
   Acceptance criteria:
   - Resolve and block private/loopback/link-local/multicast IP targets by default in external-link mode.
   - Add explicit opt-in for private network validation where needed.
   - Add tests for DNS names resolving to blocked ranges and direct-IP URLs.
3. [x] `DG-QA-003` Link validation resource budgets. (Completed 2026-05-19)
   Acceptance criteria:
   - Add max link checks, max external checks, and total validation time budget controls.
   - Surface budget truncation in diagnostics and JSON summary counts.
   - Keep deterministic behavior under budget exhaustion.
4. [x] `DG-QA-004` Encoding-safe file ingestion. (Completed 2026-05-19)
   Acceptance criteria:
   - Replace hard failure on non-UTF-8 source reads with deterministic skip diagnostics.
   - Preserve strict-gate stability while reporting skipped file count by encoding reason.
   - Add fixtures for invalid UTF-8 and mixed-encoding repositories.

### P1 Performance

1. [x] `DG-QA-005` Static adapter registry/lookups. (Completed 2026-05-19)
   Acceptance criteria:
   - Replace per-call boxed adapter registry construction with static/lazy lookup maps.
   - Preserve adapter ordering determinism and capability-index output stability.
   - Benchmark and document adapter lookup overhead reduction.
2. [x] `DG-QA-006` Regex compilation caching across adapters. (Completed 2026-05-19)
   Acceptance criteria:
   - Move regex compilation from per-file extraction paths to static/lazy compiled regexes.
   - Ensure no behavior drift in existing adapter extraction fixtures.
   - Add micro-benchmark evidence for extraction throughput improvement.
3. [x] `DG-QA-007` Link/anchor validation caching. (Completed 2026-05-19)
   Acceptance criteria:
   - Reuse one HTTP client per run and cache parsed local anchors per file path.
   - Avoid repeated file reads for multiple anchors targeting the same file.
   - Add regression tests covering repeated-anchor checks and repeated external hosts.
4. [x] `DG-QA-008` Gap call-site indexing optimization. (Completed 2026-05-19)
   Acceptance criteria:
   - Replace per-symbol full-source scans with a one-pass call-site index.
   - Preserve deterministic known-call-site ordering and limit semantics.
   - Add large-repo performance regression coverage.

### P1 DRY And Maintainability

1. [x] `DG-QA-009` Shared extraction helpers for C-style languages. (Completed 2026-05-19)
   Acceptance criteria:
   - Extract shared symbol/doc-block parsing utilities for TypeScript/JavaScript (and optionally Go/Zig where applicable).
   - Reduce duplicated regex/loop logic without changing symbol contracts.
   - Add adapter conformance tests to prove no language-specific regression.
2. [x] `DG-QA-010` Shared visibility-policy helper layer. (Completed 2026-05-19)
   Acceptance criteria:
   - Centralize effective visibility calculation patterns used by adapters (top-level, container/member inheritance, explicit modifiers).
   - Keep Kujo/TypeScript visibility semantics unchanged unless explicitly versioned.
   - Add matrix tests for adapter-specific visibility edge cases.
3. [x] `DG-QA-011` Single-source docgen JSON contract serialization. (Completed 2026-05-19)
   Acceptance criteria:
   - Move CLI JSON contract assembly from ad hoc `main.rs` maps into a typed summary payload builder.
   - Ensure backward compatibility for existing top-level keys.
   - Lock output contract with dedicated snapshot tests.
4. [x] `DG-QA-012` Renderer deduplication cleanup. (Completed 2026-05-19)
   Acceptance criteria:
   - Remove no-op duplicated branches (for example source-link conditionals that currently emit identical output).
   - Centralize shared symbol card rendering helpers across HTML/Markdown renderers where safe.
   - Preserve deterministic render output ordering.

### P2 Universal Usefulness

1. [x] `DG-QA-013` Configurable discovery limits from CLI. (Completed 2026-05-19)
   Acceptance criteria:
   - Add CLI/env overrides for max file size, max depth, and max files.
   - Emit effective limits in JSON summary for reproducible CI runs.
   - Add contract tests for default and overridden values.
2. [x] `DG-QA-014` Adapter health and extraction-confidence diagnostics. (Completed 2026-05-19)
   Acceptance criteria:
   - Emit per-language extraction counters (files scanned, symbols extracted, doc blocks attached, placeholders emitted).
   - Add warnings when extraction yield is suspiciously low for a language.
   - Expose these counters in the machine-readable summary block.
3. [x] `DG-QA-015` Incremental/cached docgen mode for CI. (Completed 2026-05-19)
   Acceptance criteria:
   - Add optional cache keyed by file content hash and adapter version.
   - Recompute only changed modules while preserving deterministic aggregate output.
   - Provide cache-hit/miss counters in JSON summary.
4. [x] `DG-QA-016` Source-link provider abstraction. (Completed 2026-05-19)
   Acceptance criteria:
   - Add pluggable source-link templates (local path, GitHub/GitLab URL patterns).
   - Keep default behavior unchanged when no provider is configured.
   - Add tests for URL rendering and path normalization safety.

## Universal Maturation Milestones (Roadmap-Aligned)

The next universal DocGen maturation slice is tracked in `ROADMAP.md` under `V1-DOCGEN-001`.

1. [x] `DG-NEXT-001` Parser-assisted Kujo extraction fallback prototype. (Completed 2026-05-20)
   Acceptance criteria:
   - Add an opt-in parser-assisted extraction path for Kujo symbols with graceful fallback to regex extraction when parser diagnostics occur.
   - Preserve deterministic output ordering and strict-gate stability.
   - Add fixture-backed coverage for both parser-success and parser-fallback paths.
2. [x] `DG-NEXT-002` Cross-language adapter conformance expansion. (Completed 2026-05-20)
   Acceptance criteria:
   - Expand fixture coverage for multi-language edge patterns (nested containers, visibility inheritance, and async/doc-attachment variants).
   - Add contract checks that keep adapter output shape stable across all supported languages.
   - Document any intentional extraction gaps per language.
3. [x] `DG-NEXT-003` External-repo strict-gate baseline refresh cadence. (Completed 2026-05-20)
   Acceptance criteria:
   - Define a repeatable external-repo validation cadence and evidence format in `notes/`.
   - Track strict/public-only undocumented-count deltas across representative repositories.
   - Document mitigation playbooks for regressions detected during baseline refresh runs.

### Intentional Adapter Extraction Gaps (Current)

These gaps are intentional in the current adapter contracts and must stay documented when conformance tests are expanded:

- Kujo:
  - Regex mode is declaration-focused (`func`/`struct`/`enum`/`const`/enum variants); parser-assisted mode remains opt-in and falls back to regex on diagnostics.
  - Visibility inheritance is container-aware for Kujo structs/enums only; it is not a cross-language global rule.
- TypeScript:
  - Extraction currently targets classes, interfaces, type aliases, functions, and class methods; enum/namespace/re-export graphs are not fully modeled.
  - Method visibility follows explicit modifiers (`public/private/protected`) without class-level visibility inheritance.
  - `export async function ...` declarations and `async` class methods are currently outside the TypeScript adapter's declaration-pattern coverage.
- JavaScript:
  - Extraction targets `export class`, `function`, and class method declarations; dynamic patterns (for example object-literal methods and assigned arrow functions) are intentionally out of scope.
  - `export async function ...` declarations are currently outside the JavaScript adapter's declaration-pattern coverage.
- Python:
  - Extraction focuses on `class`/`def` declarations and naming-convention visibility (`_private`); decorator semantics and runtime metaprogramming are not resolved.
- PHP:
  - Extraction targets `class`, `function`, and `const` declarations with method modifiers; traits/interfaces/namespaced alias graphs are not fully modeled.
- Ruby:
  - Extraction targets `module`, `class`, and `def`; dynamic method-definition patterns and metaprogrammed visibility rules are intentionally not inferred.
- Go:
  - Extraction targets `type ... struct|interface`, top-level functions, and receiver methods; embedded-interface composition and build-tag-aware symbol filtering are not modeled.
- Haskell:
  - Extraction targets module/data/newtype/typeclass/function signatures using declaration patterns; export lists and advanced type-level constructs are not fully resolved.
- Zig:
  - Extraction targets `fn`, `const`, `struct`, and `enum` declarations; container member traversal/inference is intentionally out of scope (`supports_methods = false`).

### External-Repo Strict Baseline Refresh Cadence

Cadence:
- Run baseline refreshes weekly during active DocGen feature work.
- Run baseline refreshes before RC/final release gate passes.
- Run an additional refresh after any adapter extraction/visibility/link-validation contract change.

Validation set (current):
- `/Users/robertdevore/Documents/Kujolang/kujo-repos/ai-sdk`
- `/Users/robertdevore/Documents/Kujolang/kujo-repos/mcp`
- `/Users/robertdevore/Documents/Kujolang/kujo-repos/scout`

Required command modes per repo:
- Strict include-private mode (`--include-private` + strict gate flags).
- Strict public-only mode (`--public-only` + strict gate flags).

Evidence format (required):
- Capture one dated `notes/YYYY-MM-DD_HH-mm_*.md` entry with:
  - exact command forms used
  - strict/include-private and strict/public-only counts (`undocumented_count`, `broken_link_count`, `warning_count`, `gate_failures`)
  - per-repo deltas versus the prior baseline note
  - explicit pass/fail interpretation and follow-up owner if counts regress

Mitigation playbook for regressions:
1. Re-run the affected repo with `--format json --json` and inspect `gate_failures` + `diagnostics`.
2. Isolate whether drift is extraction, visibility, docs attachment, or link-validation policy.
3. Add/adjust fixture-backed coverage in `tests/docgen_universal.rs` before changing adapter logic.
4. If regression is expected/intentional, document the boundary in this file and update the latest evidence note with rationale.
5. Re-run `cargo test --test docgen_universal` before closing the refresh loop.
