# Evidence-Driven Bug Audit — 2026-08-12

## Executive Summary

- Baseline: `3bc5b4f1634d9883a789a0c2a0e6a266f72b77b2` (`main`, clean worktree).
- Scope: runtime native functions, LSP protocol and analysis paths, input validation, resource bounds, security-sensitive streaming, CLI/release contracts, and representative persistence/integration boundaries.
- Confirmed and fixed: 11 defects. Severity: 4 High, 6 Medium, 1 Low.
- Each admitted defect had a focused regression that failed before its production fix and passed afterward. The original baseline also passed `cargo fmt --check`, `cargo check`, and `cargo test` in an isolated detached worktree.
- No confirmed defect remains unresolved. One externally visible behavior needs specification before implementation.

Implementation revisions:

- `da26160`: native validation, allocation bounds, exact schema numerics, streaming redaction, regressions, and generated inventory refresh.
- `cdd7a31`: LSP declaration/scope handling, rename preservation, UTF-16/source ranges, transport bounds, and regressions.

## Repository Orientation

Kujo is a Rust implementation of an AI-native programming language. Its default execution path is lexer → parser → AST → bytecode compiler → VM, with a tree-walking interpreter retained for fallback and parity checks. `src/main.rs` owns CLI dispatch; `src/interpreter/native_functions/` owns host and standard-library effects; `src/lsp_*` owns editor analysis and the JSON-RPC/LSP server. The stable v1 contract emphasizes deterministic local execution, explicit effect capabilities, bounded machine-readable outputs, replayable AI calls, and VM/interpreter parity.

The audit read the canonical contracts in `README.md`, `docs/LANGUAGE_SPEC.md`, `docs/STANDARD_LIBRARY.md`, `docs/AI_RUNTIME.md`, `docs/CLI_MACHINE_READABLE_CONTRACTS.md`, `docs/ARCHITECTURE.md`, `docs/NATIVE_API_SECURITY_POSTURE.md`, `examples/README_examples.md`, and the release checklists. It inspected recent history, CI/release scripts, implementation boundaries, and existing regression coverage before forming hypotheses.

## Baseline Health

The detached baseline at `3bc5b4f` passed:

```text
cargo fmt --check
cargo check
cargo test
```

No unrelated baseline failure was present. The audit tests below were then introduced one at a time and observed failing against baseline behavior before the corresponding production changes.

## Attack Surface Matrix

| Category | Applicable surface | Evidence inspected or exercised | Result |
|---|---|---|---|
| Startup/config/CLI | argument parsing, JSON contracts, capability defaults | CLI contract docs/tests and release gate | No defect admitted |
| Core runtime/control flow | lexer/parser/VM/interpreter limits and parity | parser limits, docs examples, VM and dual runtime gates | No defect admitted |
| State/mutation | bindings, filesystem writes, rename edits | mut syntax, atomic-write tests, CRLF rename backtest | 2 LSP defects fixed |
| Persistence/cache | modules, package lockfiles, SQLite/native paths | module reload/traversal tests, package workflow contracts | No defect admitted |
| External integrations | AI replay/egress and process execution | strict replay hermeticity, process stream handling | 1 security defect fixed |
| Concurrency/async | process streams, cancellation, LSP timeouts | stream chunk boundaries, cancellation and timeout tests | 1 security defect fixed |
| Security/privacy | secret redaction, path confinement, capability gates | redactor RED test; static-server no-follow/canonicalization tests | 1 High defect fixed |
| Reliability/recovery | malformed framing, oversized generation, invalid option combinations | focused panic and validation tests | 3 defects fixed |
| Performance/resource use | allocation bounds and large LSP workspaces | generated-sequence bound, message size, timeout guardrail | 2 High defects fixed |
| Validation/input handling | JSONL option combinations and numeric schemas | focused invalid-input and precision tests | 2 defects fixed |
| UX/API correctness | LSP definitions, ranges, symbols, semantic tokens | protocol-coordinate and scope tests | 6 LSP defects fixed |
| Observability/errors | deterministic errors and exit behavior | JSON/diagnostic contract tests | No defect admitted |
| Build/package/release | formatting, clippy, tests, release scripts | full release gate and targeted contract suites | Passed |
| Backward compatibility | protocol defaults, line endings, source spelling | UTF-16, CRLF, semantic source-span tests | 3 defects fixed |

## Confirmed Bugs

### 1. Streaming process redaction could disclose a configured secret

- Severity: **High** (security/privacy).
- Contract: configured `spawn_process` secrets must remain redacted across arbitrary stream chunking.
- Minimal repro: feed `AAAAAAAAAAAAAAAAsecretBBBBBBBB` to the incremental redactor. The old fixed-size flush split `secret` between independently redacted buffers and emitted it intact after concatenation.
- RED: `interpreter::native_functions::system::tests::test_streaming_redactor_does_not_flush_partial_secret_prefix` failed because the rendered output still contained `secret`.
- Root cause: the implementation retained a fixed suffix but redacted the flushed prefix independently, so a match crossing the artificial flush boundary was invisible to both scans.
- Fix: replace split-and-redact with an incremental longest-secret scanner that retains any incomplete secret prefix and incomplete UTF-8 suffix.
- GREEN/backtest: the focused regression and the existing UTF-8 boundary test pass on the fixed tree; the focused regression fails on baseline behavior.
- Residual risk: process behavior is platform-dependent; the pure scanner and Unix integration are covered, while Windows process-stream behavior is only covered by CI.

### 2. Oversized LSP `Content-Length` could panic before input was read

- Severity: **High** (availability).
- Contract: malformed or hostile LSP framing must return an I/O error rather than panic or attempt unbounded allocation.
- Minimal repro: frame a request with `Content-Length: usize::MAX` and no body.
- RED: `lsp_server::tests::transport_rejects_oversized_content_length_without_allocating` panicked with `capacity overflow`.
- Root cause: the parsed length flowed directly into `vec![0; size]` without a protocol resource limit.
- Fix: enforce an 8 MiB maximum before allocating the body buffer.
- GREEN/backtest: focused test passes on fixed code and fails on baseline behavior.
- Residual risk: the limit is deliberately conservative and documented; clients needing larger single messages must split work or negotiate a future contract change.

### 3. SSG output path generation could panic on an unbounded count

- Severity: **High** (availability/resource exhaustion).
- Contract: generated sequences are bounded by `MAX_GENERATED_SEQUENCE_ITEMS` and invalid native arguments return `Value::Error`.
- Minimal repro: call `ssg_build_output_paths` with `i64::MAX` as the file count.
- RED: `interpreter::native_functions::strings::tests::test_ssg_build_output_paths_validates_argument_contracts` panicked with `capacity overflow`.
- Root cause: the count was cast to `usize` and passed to `Vec::with_capacity` before any runtime-limit check.
- Fix: use checked conversion and reject counts above the generated-sequence limit before allocation.
- GREEN/backtest: focused contract test passes on fixed code and fails on baseline behavior.
- Residual risk: ordinary allowed requests can still consume memory proportional to the documented sequence limit.

### 4. LSP positions used Unicode scalar counts instead of UTF-16 code units

- Severity: **High** (editor protocol correctness/interoperability).
- Contract: absent a negotiated alternative, LSP positions and semantic-token lengths use UTF-16 code units.
- Minimal repro: request references for `x` in `print("😀", x)` at UTF-16 character 12.
- RED: `lsp_server::tests::reference_positions_use_lsp_utf16_code_units` returned no references instead of two; the server interpreted character 12 as a scalar-value column.
- Root cause: analysis uses one-based Unicode-scalar columns internally, but request decoding and response rendering performed only one/zero-based arithmetic.
- Fix: convert incoming UTF-16 positions to internal columns, reject positions inside surrogate pairs, and convert hover, definition, reference, rename, action, symbol, diagnostic, hint, lens, formatting, and semantic-token ranges back to UTF-16. Workspace-symbol conversion reuses a per-document line index to preserve the timeout guardrail.
- GREEN/backtest: the UTF-16 regression, existing LSP suites, and the 50,000-symbol timeout regression pass; the UTF-16 regression fails on baseline behavior.
- Residual risk: no explicit alternative `positionEncoding` negotiation is implemented; UTF-16 remains the protocol default and documented contract.

### 5. Semantic token ranges were derived from normalized values, not source spans

- Severity: **Medium** (editor highlighting correctness).
- Contract: semantic tokens identify the original source range.
- Minimal repro: tokenize `let message := "hi"`.
- RED: `lsp_server::tests::semantic_tokens_preserve_non_identifier_source_ranges` reported the string at UTF-16 column 11 rather than 15.
- Root cause: the lexer stores string token columns at token start while the old encoder assumed all token columns were end-exclusive; numeric lengths also used normalized `to_string()` spelling.
- Fix: derive starts and lengths from lexer byte offsets and raw source spelling, then encode those spans as UTF-16.
- GREEN/backtest: focused semantic-token test passes fixed and fails baseline.
- Residual risk: interpolation is covered by the same raw quoted-span scanner but does not yet expose expression sub-tokens separately.

### 6. Standalone mutable bindings were missing from LSP analysis

- Severity: **Medium** (language/editor contract mismatch).
- Contract: `mut name := value` is canonical binding syntax and must be treated as a declaration.
- Minimal repro: define `mut counter := 0`, use `counter`, and request definition, rename, symbols, or inlay hints.
- RED: `lsp_definition::tests::resolves_mutable_binding_definition` returned no definition; `lsp_server::tests::mutable_bindings_have_document_symbols_and_inlay_hints` omitted `counter`.
- Root cause: LSP declaration collectors recognized `let`, `let mut`, and `const`, but not standalone `mut`.
- Fix: add `mut` declaration classification consistently to definition, reference/rename, document-symbol, and inlay-hint paths.
- GREEN/backtest: focused definition, rename, symbol, and hint tests pass fixed and fail baseline behavior.
- Residual risk: declaration logic remains token-based rather than AST-based and must track future syntax additions.

### 7. Go-to-definition ignored lexical scope after an inner block closed

- Severity: **Medium** (editor navigation correctness).
- Contract: a use resolves to the visible declaration in its lexical scope.
- Minimal repro: shadow `value` inside a function, close the function, then request the definition of outer `print(value)`.
- RED: `lsp_definition::tests::ignores_inner_scope_definition_after_scope_closes` resolved line 3 instead of line 1.
- Root cause: definition lookup selected the nearest preceding same-name token without checking scope visibility.
- Fix: reuse the scope-aware reference resolver to identify the declaration, while preserving definition-kind metadata and parameter behavior.
- GREEN/backtest: scope regression plus definition, hover, reference, and parameter suites pass fixed; the scope regression fails baseline.
- Residual risk: brace-derived scope tracking is intentionally lightweight and should migrate with any future AST-backed LSP analysis.

### 8. LSP rename normalized CRLF documents to LF

- Severity: **Medium** (source data integrity).
- Contract: a symbol rename changes only selected identifier ranges.
- Minimal repro: rename `value` in `let value := 1\r\nprint(value)\r\n`.
- RED: `lsp_rename::tests::rename_preserves_crlf_line_endings` produced LF-only output.
- Root cause: edit application used `str::lines()` and reconstructed the document with `join("\n")`.
- Fix: retain each line's original terminator with `split_inclusive('\n')`, apply character-range edits to line content, and reconstruct unchanged endings.
- GREEN/backtest: focused rename test passes fixed and fails baseline.
- Residual risk: mixed LF/CRLF is preserved per line; lone carriage-return line endings are not a documented source format.

### 9. Lexer-valid Unicode identifier continuations were rejected by rename

- Severity: **Low** (internationalized editor correctness).
- Contract: rename validation should accept the same identifier spelling accepted by the lexer.
- Minimal repro: rename an ASCII-starting identifier to `café`.
- RED: `lsp_rename::tests::accepts_unicode_identifier_name_supported_by_lexer` returned the ASCII-only validation error.
- Root cause: pre-validation required every character to be ASCII alphanumeric or underscore even though the lexer accepts Unicode alphabetic/alphanumeric continuation characters.
- Fix: mirror the lexer's character classes and retain final lexer-based validation.
- GREEN/backtest: focused rename test passes fixed and fails baseline.
- Residual risk: leading non-ASCII identifiers remain rejected because the lexer does not currently accept them.

### 10. JSONL joins accepted incomplete three-field configuration

- Severity: **Medium** (input validation/reliability).
- Contract: `join_path`, `left_field`, and `right_field` are required together.
- Minimal repro: provide `join_path` plus only `right_field`.
- RED: `test_jsonl_query_rejects_partial_join_configuration` returned an empty array rather than `Value::Error`.
- Root cause: a compound Boolean check distinguished only “path present versus both fields absent,” allowing exactly one join field through.
- Fix: reject any join configuration where at least one but not all three fields is present.
- GREEN/backtest: focused interpreter test passes fixed and fails baseline.
- Residual risk: nested-loop join cost remains intentionally bounded by documented row limits rather than indexed.

### 11. JSON Schema integer limits lost precision above 2^53

- Severity: **Medium** (validation correctness).
- Contract: Kujo `int` values and integer-valued schema bounds compare exactly.
- Minimal repro: validate `9007199254740993` against `maximum: 9007199254740992`.
- RED: `validates_items_enum_const_numbers_strings_and_array_bounds` returned no `maximum` error.
- Root cause: both integers were cast to `f64`, where the adjacent values compare equal.
- Fix: retain tagged integer/float numeric values and use exact mixed-type ordering without lossy integer conversion.
- GREEN/backtest: focused schema regression passes fixed and fails baseline.
- Residual risk: floating-point schema values retain IEEE-754 semantics, as represented by Kujo `float`.

## Rejected Hypotheses

1. **Strict AI replay could fall through to live network.** Rejected: replay lookup precedes transport policy, and `ai_replay_hermeticity_contract` proves strict missing/matching cassette behavior without sockets.
2. **Static serving allowed traversal through encoded paths or symlinks.** Rejected: canonical-root checks, no-follow file opens, decoding validation, and existing static-server security tests cover traversal, double encoding, and symlink escapes.
3. **Module cache returned stale source or followed traversal/symlink aliases.** Rejected: module tests exercise source changes, root confinement, and symlink rejection; no counterexample reproduced.
4. **Atomic writes exposed partial destination state.** Rejected: unique create-new temporary files, sync/rename or hard-link publication, cleanup paths, and concurrency tests preserve the stated contract.
5. **Process timeout/cancellation left the spawned process group alive.** Rejected on applicable Unix paths: process-group creation/termination and timeout/cancellation tests passed. Cross-platform residual risk is listed below.
6. **LSP UTF-16 conversion necessarily violated the large-workspace timeout.** Initially observed during GREEN validation, then eliminated by reusing a per-document line index; the pre-existing timeout regression passes.

## Needs Specification

### Duplicate HTTP query parameters

- Ambiguous behavior: `parse_http_query_params` stores query pairs in a dictionary, so duplicate keys currently collapse to the last decoded value.
- Missing contract: the standard-library and HTTP docs do not state first-value, last-value, rejection, or multi-value semantics.
- External behavior affected: request routing, signature verification, webhook validation, and application code consuming duplicate query keys.
- Options:
  1. Preserve last value (current behavior) and document it.
  2. Preserve first value.
  3. Return arrays for duplicates.
  4. Reject duplicates.
- Recommendation: specify and expose multi-value semantics while retaining a documented compatibility accessor for the last value. No code was changed without that decision.

## What Was Not Audited

- Live calls to external AI providers, registries, or third-party services; the audit used committed replay and offline contracts.
- Extended fuzzing beyond repository-defined smoke/contract gates.
- Performance benchmark claims; correctness resource guardrails were exercised, but benchmark suites marked ignored were not promoted into release claims.
- Native behavior on Linux and Windows; this run was on macOS, with cross-platform confidence delegated to repository CI.
- Generated documentation and `examples/ssg/content/**`, except where referenced by contract tests.
- End-to-end behavior inside every supported GUI editor; the JSON-RPC server and adapter contracts were tested directly.

## Validation Summary

Targeted RED/GREEN regressions cover every confirmed defect. Final repository validation included formatting, compilation, full Rust tests, documentation/example/CLI/diagnostic contracts, VM and dual-runtime suites, and the full release gate with socket tests enabled. Exact final command outcomes are recorded in the audit commit and session handoff.

## Audit Stop Condition

The audit completed an orientation pass, a contract-versus-implementation pass across every applicable attack-surface category, focused RED/GREEN work for each reproducible defect, and a second pass through adjacent boundaries and regressions. The second pass produced no additional reproducible contract violation after the fixes above; remaining uncertainty is explicitly bounded in “Needs Specification” and “What Was Not Audited.”
