# Kujo — Known Gotchas & Sharp Edges

This document contains the most important non-obvious pitfalls in the Kujo codebase.

If you are new to the project, read this first.

---

## Parser & Syntax

### Expression precedence is NOT inferred
- **Problem:** Parser/evaluator behavior can look "wrong" when new syntax is added without explicit precedence placement.
- **Rule:** New expression forms and operators must be wired into the existing precedence chain intentionally; Kujo does not infer precedence from token shape.
- **Why:** The parser is hand-rolled with explicit parse stages and lookahead.

(Discovered during: 2026-01-25_14-30_destructuring-spread-implementation.md)

### Spread (`...`) is container-context only
- **Problem:** Treating spread as a standalone expression leads to invalid mental models and dead-end refactors.
- **Rule:** Spread is valid only inside `ArrayElement::Spread` and `DictElement::Spread`, not as a general `Expr`.
- **Why:** Semantics depend on container context (array flatten/concat vs dict merge).

(Discovered during: 2026-01-25_14-30_destructuring-spread-implementation.md)

### `Ok`/`Err`/`Some`/`None` are contextual identifiers, not lexer keywords
- **Problem:** Turning these into keywords breaks `match case` parsing and can hang parse flows.
- **Rule:** Keep them tokenized as identifiers and interpret contextually in parser/evaluator logic.
- **Why:** Pattern matching expects identifier tokens in case arms.

(Discovered during: 2026-01-25_17-30_result-option-types-COMPLETED.md)

### Parser hardening must preserve accepted compatibility syntax unless intentionally deprecated
- **Problem:** Replacing permissive `advance()` parsing with strict `expect_*` checks can silently regress accepted syntax (for example `let x = 1` or `else if ...` chains).
- **Rule:** When tightening parser diagnostics, explicitly preserve legacy-accepted forms or document and test intentional deprecations before changing behavior.
- **Why:** Compatibility regressions can surface as runtime-level failures (empty/partial AST execution) far from the parser change point.

(Discovered during: 2026-05-06_23-48_v1-par-001-parse-diagnostics.md)

### Precedence-tier refactors require helper-callsite audits
- **Problem:** Splitting parse stages (for example separating equality from comparison) can accidentally reduce expression support in literals/struct fields if helper parse functions still call old stage boundaries.
- **Rule:** After precedence changes, audit every `parse_*` helper callsite in array/dict/struct parsing and move to the stage that preserves the intended expression coverage contract.
- **Why:** These helper paths often avoid full `parse_expr()` recursion and depend on precise stage selection.

(Discovered during: 2026-05-07_08-12_v1-par-003-precedence-and-compound-assignment.md)

### Multi-character operators require explicit lexer lookahead
- **Problem:** Operators like `...` fragment into punctuation tokens when lookahead logic is incomplete.
- **Rule:** Any multi-character operator must have dedicated lookahead path in lexer tokenization.
- **Why:** Lexer is character-driven and does not auto-compose operator tokens.

(Discovered during: 2026-01-25_14-30_destructuring-spread-implementation.md)

### Identifier token column semantics are an editor-contract surface
- **Problem:** LSP definition/hover/references/rename lookups can break after lexer refactors even when token kinds remain correct.
- **Rule:** Preserve legacy identifier `Token.column` semantics (used by downstream `column - name_len` start-column math) unless all LSP symbol helpers are migrated together.
- **Why:** Existing LSP symbol tooling derives cursor containment from identifier token columns, so location-field drift causes false misses.

(Discovered during: 2026-05-06_23-22_v1-lex-001-structured-lexer-diagnostics.md)

---

## Runtime / Evaluator

### Variable scope in scripts is global-by-default
- **Problem:** Reusing variable names across top-level blocks causes state bleed in tests and scripts.
- **Rule:** Assume script-level assignments mutate shared global scope unless inside function boundaries.
- **Implication:** Use unique names or function wrappers for isolation in integration tests.

(Discovered during: 2026-01-27_07-46_phase2-vm-optimizations.md)

### Native dispatch and declaration can drift independently
- **Problem:** Builtins may appear declared but still fail at runtime if dispatcher wiring is missing.
- **Rule:** Treat declaration, registration, and dispatch handling as separate required surfaces.
- **Implication:** Any builtin change needs end-to-end contract tests (module + dispatcher + runtime behavior).

(Discovered during: 2026-02-15_09-18_release-hardening-alias-api-contract.md)

### VM builtin name registration does not bootstrap constants
- **Problem:** VM runs can report undefined `PI`, `E`, or `null` even when builtin globals seem initialized.
- **Rule:** Treat constant injection as a separate bootstrap contract from native builtin-name registration.
- **Implication:** When touching VM startup/bootstrap, assert both callable builtins and constant globals are present.

(Discovered during: 2026-05-05_10-29_vm-example-compatibility-followthrough.md)

### Method support can silently split between call paths
- **Problem:** A method can appear implemented but still fail in normal `obj.method(...)` usage.
- **Rule:** Keep method behavior wired to the active `Expr::MethodCall` path and do not leave logic only in legacy field-access call sugar branches.
- **Implication:** Verify both interpreter `call_method(...)` and VM `FieldGet`/native-call handling when adding methods for non-struct `Value` types.

(Discovered during: 2026-04-29_17-02_image-method-dispatch-parity.md)

### HTTP route matching must ignore query strings
- **Problem:** Exact/parameterized routes can fail when handlers are matched against full URLs containing `?query=...`.
- **Rule:** Split request URL into path and query first, then route-match using normalized path only.
- **Implication:** Keep request metadata (`path`, `raw_path`, `query`, `query_string`) explicit and parity-aligned across interpreter and VM runtimes.

(Discovered during: 2026-05-05_11-17_http-query-route-hardening.md)

### Deep import-chain stress tests can hit stack limits before recursion guards
- **Problem:** Module-import stress fixtures with high chain depth can fail with host stack overflow even when module semantics are otherwise correct.
- **Rule:** Keep import-chain depth bounded in performance/correctness tests until explicit recursion/call-depth safeguards are implemented.
- **Implication:** Do not treat deep-chain overflow as module-loader semantic drift; scope it to resource-limit work (`V1-PERF-003`).

(Discovered during: 2026-05-16_13-48_v1-perf-002-module-loader-cycle-check-scaling.md)

### Control-flow truthiness must explicitly handle integer zero
- **Problem:** Missing-key/fallback branches can silently execute incorrectly when condition values are numeric sentinels.
- **Rule:** `Int(0)` must be falsey and non-zero ints truthy in both `Stmt::If` and `Stmt::While` evaluation paths.
- **Implication:** Keep truthiness logic parity between `src/interpreter/mod.rs` and `src/interpreter/legacy_full.rs`, and guard with dedicated regression tests.

(Discovered during: 2026-05-05_22-20_ai-sdk-runtime-truthiness-and-test-run-gotchas.md)

### Strict-arity hardening must preserve legacy fallback contracts unless intentionally changed
- **Problem:** Rejecting extra args can accidentally alter established missing/invalid-arg behavior.
- **Rule:** Add explicit extra-arg rejection while preserving existing fallback semantics unless contract change is deliberate.
- **Implication:** Tests should assert both strict-arity and compatibility fallback behavior.

(Discovered during: 2026-02-16_20-23_release-hardening-strict-arity-follow-through-slices.md)

### JSON serialization must reject non-finite floats and sort map keys
- **Problem:** JSON helpers can silently coerce invalid numeric states (`NaN`/`+/-inf`) or emit non-deterministic object-key ordering when converting hash-map-backed values.
- **Rule:** Reject non-finite float serialization explicitly and define deterministic key ordering for dictionary-like JSON output.
- **Implication:** When touching JSON conversion paths, add tests for non-finite failure contracts, map key ordering determinism, and parse/stringify bounds.

(Discovered during: 2026-05-15_17-35_v1-std-002-json-hardening.md)

### Native API signature changes have three independent contract surfaces
- **Problem:** A native API can be "fixed" in runtime handling but still fail tests and user validation paths.
- **Rule:** When changing native argument contracts, update all three layers: runtime handler (`src/interpreter/native_functions/*`), type signature metadata (`src/type_checker.rs`), and native dispatch contract tests (`src/interpreter/native_functions/mod.rs`).
- **Implication:** Use a three-surface checklist for every native signature/arity change to avoid partial migrations.

(Discovered during: 2026-05-12_22-40_v1-sec-003-process-shell-hardening.md)

### Native `Value::Error` can be data, while undefined identifiers are fatal
- **Problem:** Treating every `Value::Error` as an immediate statement-level halt breaks native helpers that intentionally return error values for callers to inspect.
- **Rule:** Undefined identifier lookup must produce `Undefined variable: <name>` and stop execution, but `let x := native_call(...)` must still be allowed to bind a native `Value::Error` unless that API is intentionally changed.
- **Implication:** Runtime-error hardening should distinguish language semantic errors from native error-as-value contracts before changing `let`/assignment behavior.

(Discovered during: 2026-05-06_17-07_undefined-identifier-runtime-errors.md)

### Promise receivers are single-consumer
- **Problem:** Re-awaiting or re-aggregating promise handles can fail if cached result checks are skipped.
- **Rule:** Aggregation helpers must short-circuit on cached completion before touching receiver paths.
- **Implication:** `Promise.all`/`await_all` optimizations must preserve single-consumer safety.

(Discovered during: 2026-02-14_10-10_promise-cache-reuse-and-parallel-map-overhead.md)

### Shared state is process-global
- **Problem:** Tests interfere when shared keys are reused.
- **Rule:** Use unique shared keys per test and clean up deterministically.
- **Implication:** Parallel tests require namespacing discipline for `shared_*` APIs.

(Discovered during: 2026-02-14_13-09_shared-thread-safe-value-ops.md)

### SSG helper contracts are compatibility surfaces
- **Problem:** Throughput refactors can silently break checksum, stage-metric key, or output-path invariants.
- **Rule:** Preserve benchmark-facing contracts (`checksum`, file counts, stage keys) while optimizing internals.
- **Implication:** SSG performance work must include contract regressions, not only speed checks.

(Discovered during: 2026-03-12_10-43_ssg-read-render-write-fusion-follow-through.md)

---

## Compiler / VM / JIT

### `StoreVar`/`StoreGlobal` are PEEK-semantics, not POP
- **Problem:** Missing explicit `Pop` after assignment/pattern-binding pollutes stack and breaks loop/JIT invariants.
- **Rule:** Statement-level assignment paths must enforce stack hygiene explicitly.
- **Why:** Store opcodes keep top-of-stack value by design.

(Discovered during: 2026-01-28_18-50_compiler-stack-pop-fix.md)

### VM opcode semantics may have duplicate execution arms
- **Problem:** Fixing only the primary VM loop can leave nested bytecode/JIT-call execution with old behavior.
- **Rule:** Search every `OpCode::<Name>` arm and route shared semantics through one helper when changing opcode behavior.
- **Implication:** Indexing changes must cover `IndexGet`, `IndexGetInPlace`, optimized local paths, and nested bytecode-call paths.

(Discovered during: 2026-05-06_12-25_vm-map-missing-key-runtime-errors.md)

### Capability policy enforcement must include method-call and child-runtime paths
- **Problem:** Host-effect operations can bypass central native dispatcher checks when routed through method-call helpers or child runtime instances.
- **Rule:** Capability hardening must cover native dispatch, VM synthetic method markers (for example `__image_method_save`, `__http_server_method_listen`), and child runtime constructors (`spawn` and async function interpreters).
- **Implication:** Security policy changes must audit both `src/interpreter/mod.rs` and `src/vm.rs` helper paths, not only `src/interpreter/native_functions/mod.rs`.

(Discovered during: 2026-05-08_18-21_v1-sec-002-native-capability-policy.md)

### `Stmt::Import` parity depends on explicit compiler lowering
- **Problem:** Import/export scripts can pass in interpreter mode but fail in VM mode with undefined symbols.
- **Rule:** Treat `Stmt::Import` as a first-class compiler/VM parity surface; do not leave import statements as compiler no-ops.
- **Implication:** Keep VM import handlers (`__vm_import_all` / `__vm_import_symbol`) and parity tests in sync with interpreter module-loader behavior.

(Discovered during: 2026-05-08_17-05_v1-comp-001-parity-gate-import-and-unsupported-surface.md)

### Unsupported surfaces must fail with one deterministic contract across runtimes
- **Problem:** The same unsupported syntax can produce misleadingly different errors between interpreter and VM/compiler paths.
- **Rule:** Centralize unsupported-surface error messages and reject early in both runtime paths.
- **Implication:** Add negative parity tests that assert the same error contract (for example struct generator methods).

(Discovered during: 2026-05-08_17-05_v1-comp-001-parity-gate-import-and-unsupported-surface.md)

### Mutability checks are semantic effects, not removable store side effects
- **Problem:** Compiler dead-store/elision passes can accidentally bypass runtime mutability enforcement for `let`/`const`.
- **Rule:** Once assignment/store opcodes enforce language semantics (for example immutable reassignment/mutation errors), do not optimize them away without proving equivalent checks still run.
- **Implication:** Mutability work must update both compiler lowering and every VM execution arm (`StoreVar`/`StoreLocal`/`StoreGlobal` and in-place mutation prechecks).

(Discovered during: 2026-05-07_14-13_v1-run-005-mutability-enforcement.md)

### Logical short-circuit lowering is stack-shape sensitive
- **Problem:** New `&&`/`||` compiler lowering can cause VM stack underflow or invalid-op failures if branch stack cleanup and bool normalization are not explicit.
- **Rule:** For short-circuit lowering, keep stack depth balanced on both branch paths and normalize RHS truthiness via bool constants + `And`/`Or` (not `!!`, because `Not` is bool-only in hardened runtime semantics).
- **Implication:** When touching compiler logical lowering, run VM/interpreter parity tests and confirm optimizer compatibility (or gate optimizer for unsupported lowering shapes).

(Discovered during: 2026-05-07_13-03_v1-run-004-truthiness-centralization.md)

### Literal-only negative-op tests can be optimized away before runtime checks
- **Problem:** Runtime-error regression tests for invalid comparisons can pass unexpectedly if the optimizer folds constant expressions first.
- **Rule:** For tests that must validate runtime comparison/error behavior, avoid pure literal forms and force runtime evaluation through variables or function parameters.
- **Implication:** Comparison/invalid-op tests should use non-literal execution paths when optimizer passes are enabled.

(Discovered during: 2026-05-08_16-40_v1-sem-003-equality-comparison-contract.md)

### Script JIT can bypass runtime error semantics
- **Problem:** Top-level script JIT may return a default value for opcodes whose checked VM path would report a runtime error.
- **Rule:** Do not admit opcodes into script JIT unless the JIT implementation preserves the same error semantics as the bytecode VM.
- **Implication:** Undefined-name work gates `LoadVar`/`LoadGlobal` out of script JIT until JIT variable lookup has explicit parity tests, and strict-unary semantics now gate `Negate`/`Not` admission until JIT parity coverage exists.

(Discovered during: 2026-05-06_17-07_undefined-identifier-runtime-errors.md, 2026-05-06_21-06_v1-run-003-invalid-operation-errors.md)

### Named nested closure mutation requires capture + free-variable parity
- **Problem:** VM can raise `Undefined variable` for outer-scope variables mutated inside nested named functions when interpreter capture and compiler free-variable analysis drift.
- **Rule:** Keep interpreter nested `Stmt::FuncDef` closure capture semantics aligned with compiler free-variable detection, and treat identifier assignment targets as usage for upvalue capture.
- **Implication:** Any closure-capture change must run full VM/interpreter parity tests; passing expression-closure tests alone is insufficient.

(Discovered during: 2026-05-12_23-23_NO-ROADMAP_named-nested-closure-capture-parity.md)

### Dead-code elimination must remap all index-based metadata
- **Problem:** Jump targets/handlers/source-map entries become invalid after instruction removal.
- **Rule:** Build old→new index mapping and rewrite every metadata surface carrying instruction indices.
- **Implication:** Optimizer edits require holistic metadata audits, not opcode-only updates.

(Discovered during: 2026-01-27_07-46_phase2-vm-optimizations.md)

### New modules must be declared in both crate roots
- **Problem:** `unresolved import` appears when module is wired in `main.rs` but not `lib.rs` (or vice versa).
- **Rule:** Kujo uses binary+library crate roots; add module declarations to both when introducing new modules.
- **Implication:** Module-split refactors should include dual-root compile checks.

(Discovered during: 2026-01-27_07-46_phase2-vm-optimizations.md)

### Module cache keys must include package-root context
- **Problem:** Reusing a cache keyed only by module name or canonical module path can return exports from the wrong package when two package roots contain same-name modules.
- **Rule:** Scope module-cache identity by both canonical module path and import package-root context, and keep importer-package-root-first search order explicit.
- **Implication:** Module-system refactors must include dual-package same-name regressions plus cache refresh tests after source updates.

(Discovered during: 2026-05-13_01-13_v1-mod-001-module-import-hardening.md)

### Cranelift block sealing and terminators are correctness-critical
- **Problem:** Misordered sealing or missing terminators causes verifier failures and invalid SSA graphs.
- **Rule:** Ensure each basic block terminates and seal in control-flow-safe order.
- **Implication:** JIT edits require explicit CFG reasoning, not local opcode translation only.

(Discovered during: 2026-01-28_04-08_jit-execution-success.md)

---

## CLI & Tooling

### `kujo docgen --json` is an additive contract surface
- **Problem:** Refactoring docgen internals or expanding payload fields can silently break automation if legacy JSON keys are removed/renamed.
- **Rule:** Keep existing docgen JSON fields stable (`command`, `file`, `output_dir`, `module_doc_path`, `builtin_doc_path`, `item_count`) and only add new fields.
- **Implication:** After any docgen JSON change, run `cargo test docgen_json_contract_is_stable --test cli_json_contracts` before marking work complete.

(Discovered during: 2026-05-17_21-57_universal-docgen-architecture-and-gates.md)

### `cargo test` only accepts one positional test filter
- **Problem:** Supplying multiple positional filters leads to command misuse and misleading failures.
- **Rule:** Use one positional filter; pass additional selection logic via module path or `-- --nocapture`/other flags.
- **Implication:** Keep repro commands minimal and exact in notes/tests.

(Discovered during: 2026-02-17_18-48_io-strict-arity-hardening.md)

### `zip = 0.5` unix permission helpers do not encode symlink file type bits
- **Problem:** Tests that create zip entries with `FileOptions::unix_permissions(0o120777)` may still produce normal files, causing symlink-policy regressions to be untested.
- **Rule:** For symlink-metadata tests, verify `ZipFile::unix_mode()` in the fixture or patch central-directory metadata (`version_made_by` host + external attributes) explicitly.
- **Implication:** Do not assume unix permission setters alone can generate true symlink zip entries under `zip = 0.5`.

(Discovered during: 2026-05-06_22-19_v1-sec-001-unzip-hardening.md)

### Repository shell guards should remain Bash 3 compatible for local macOS runs
- **Problem:** CI/helper scripts can work in GitHub Actions but fail locally with `mapfile: command not found`.
- **Rule:** Prefer portable `while read` loops over Bash-4-only helpers (for example `mapfile`) in repo-level scripts.
- **Implication:** Validate new `.github/scripts/*.sh` locally on macOS before relying on CI-only execution confidence.

(Discovered during: 2026-05-06_11-30_field-notes-ci-guard.md)

### Socket-bound test gates need explicit environment policy
- **Problem:** Full test/gate runs can fail in restricted sandboxes with `PermissionDenied` when binding ephemeral TCP/UDP ports.
- **Rule:** Keep socket-heavy integration tests explicitly gated by environment intent (for example `KUJO_ENABLE_SOCKET_TESTS=1` in CI) and make unit tests skip gracefully when bind permissions are denied.
- **Implication:** Separate correctness failures from environment policy failures so release gates stay meaningful and reproducible.

(Discovered during: 2026-05-06_23-02_v1-ci-001-release-gate-enforcement.md)

### Deep interpreter recursion tests can overflow Rust test worker stacks before semantic limits trigger
- **Problem:** In-process integration recursion tests can abort with host stack overflow even when Kujo recursion/call-depth limits are implemented.
- **Rule:** Validate interpreter depth-limit contracts with direct state-based unit tests (or subprocess CLI tests), not only deep recursive integration scripts.
- **Implication:** For recursion-limit hardening, prefer deterministic guard checks in `src/interpreter/mod.rs` tests; treat deep recursive integration runs as environment-sensitive smoke evidence.

(Discovered during: 2026-05-16_16-12_v1-perf-003-resource-exhaustion-safeguards.md)

### Subprocess serve-readiness checks can false-positive under parallel test port races
- **Problem:** A test can incorrectly treat a different process as its spawned server readiness signal and later fail with `Connection refused`.
- **Rule:** In readiness loops, check child liveness both before and after successful port-connect probes.
- **Implication:** Keep `tests/serve_command_integration.rs` readiness checks process-aware, not port-only.

(Discovered during: 2026-05-15_13-58_v1-http-006-streaming-range-policy.md)

### Serve request limits are enforced before method checks
- **Problem:** Oversized `POST`/non-GET requests can return `413` rather than `405`, which looks wrong if tests assume method validation always runs first.
- **Rule:** Model `kujo serve` request evaluation as limits/target validation first, method dispatch second.
- **Implication:** Size-limit regressions in `tests/serve_command_integration.rs` should assert `413` for over-limit bodies even on unsupported methods.

(Discovered during: 2026-05-16_16-59_v1-test-003-runtime-native-security-regressions.md)

### Blocking reqwest clients can panic when used on Tokio runtime threads
- **Problem:** Network calls can panic with `Cannot drop a runtime in a context where blocking is not allowed` when `reqwest::blocking` is created/dropped in runtime-managed execution paths.
- **Rule:** Route blocking HTTP execution through a dedicated OS thread (or use a fully async client path) before applying request/response policy logic.
- **Implication:** Network hardening work must validate runtime context safety in addition to timeout/body-size enforcement.

(Discovered during: 2026-05-13_12-56_v1-net-001-network-timeout-size-controls.md)

### Native-function tests should avoid fixed temp directories
- **Problem:** Reusing a fixed temp path can make cleanup assertions fail due to stale nested state from earlier runs.
- **Rule:** Build per-run temp directory names (for example PID + timestamp) and pre-clean if the path already exists.
- **Implication:** Filesystem/native API tests remain deterministic across repeated local and CI runs.

(Discovered during: 2026-05-07_07-36_v1-base-002-release-gate-script-followthrough.md)

### Benchmark harness paths are sensitive to working directory
- **Problem:** Running bench commands outside repo-root can resolve fixtures/scripts incorrectly.
- **Rule:** Normalize CWD assumptions and prefer explicit tmp/artifact roots for harnesses.
- **Implication:** Bench tooling changes should be validated with non-root invocation paths.

(Discovered during: 2026-02-13_18-52_bench-cross-cwd-gotcha.md)

### Local cargo-fuzz builds can fail due host C++/SDK header setup
- **Problem:** `cargo +nightly check` / `cargo +nightly fuzz run` may fail on some hosts with `libfuzzer-sys` errors like missing `cassert`/`cstdint`.
- **Rule:** Treat dedicated Linux CI fuzz-smoke workflow as the authoritative compile/run signal unless local toolchain prerequisites are known good.
- **Implication:** When adding fuzz targets, wire nightly CI smoke coverage and do not assume every local machine can build libFuzzer C++ dependencies out of the box.

(Discovered during: 2026-05-16_16-35_v1-test-002-lexer-parser-fuzzing.md)

### Diagnostics snapshot updates must stay env-gated
- **Problem:** Golden diagnostics can silently drift if test harnesses auto-overwrite expected files.
- **Rule:** Require an explicit env flag (for example `KUJO_UPDATE_GOLDENS=1`) before writing snapshot files.
- **Implication:** Normal `cargo test` runs should only verify snapshots; intentional contract changes must opt in to regeneration.

(Discovered during: 2026-05-16_17-03_v1-test-004-diagnostics-golden-snapshots.md)

### Docs snippet expected-fail metadata is ordinal-index sensitive
- **Problem:** Expected-fail entries for fenced docs snippets can break after unrelated docs edits because block IDs shift.
- **Rule:** Recompute `<file>#<ordinal>` IDs whenever fenced Kujo blocks are added/removed before existing expected-fail snippets.
- **Implication:** After docs edits, rerun `cargo test --test docs_examples` and refresh expected-fail block IDs intentionally.

(Discovered during: 2026-05-16_17-10_v1-test-005-docs-examples-smoke-suite.md)

### VM microbench harnesses must explicitly seed builtin globals
- **Problem:** VM benchmark programs can fail with undefined builtin/runtime helper names (for example `len`, `range`) even when equivalent interpreter benchmarks work.
- **Rule:** In custom VM bench harnesses, register builtin names into VM globals before executing compiled chunks.
- **Implication:** Reuse the same builtin-seeding contract as production benchmark runners to avoid measuring setup bugs instead of runtime performance.

(Discovered during: 2026-05-16_13-31_v1-perf-001-criterion-baselines.md)

### Full-suite timing assertions are brittle under load
- **Problem:** Throughput/timing tests that assert monotonic runtime behavior flake in CI and loaded local environments.
- **Rule:** Assert deterministic correctness signals (checksums/counts/non-negative timing) instead of strict timing trends.
- **Implication:** Performance tests should detect regressions by contract, not raw wall-clock ordering.

(Discovered during: 2026-03-19_07-47_ssg-rayon-pool-cache-and-timing-test-stability.md)

### Top-level example harnesses must explicitly separate automation-safe and interactive flows
- **Problem:** One-shot example sweeps can fail or hang on interactive/server/network demos.
- **Rule:** Keep a clear automation contract (non-benchmark, non-interactive) and guard long-running examples.
- **Implication:** Use a deterministic smoke target for CI/local sweeps and validate interactive flows separately.

(Discovered during: 2026-05-05_10-29_vm-example-compatibility-followthrough.md)

### Shared diagnostic line/column fields are an LSP/code-action contract
- **Problem:** Changing shared diagnostics to optional coordinates can break compilation or behavior in LSP code actions and server range construction.
- **Rule:** Keep shared diagnostic `line`/`column` fields numeric for downstream LSP helpers; use sentinel `0` when location is unavailable instead of changing the field shape.
- **Implication:** Any diagnostics-structure refactor must audit `src/lsp_code_actions.rs`, `src/lsp_server.rs`, and CLI JSON diagnostic serialization together.

(Discovered during: 2026-05-07_07-22_v1-err-001-diagnostic-unification.md)

### Animated GIF to WebP conversion depends on external `gif2webp`
- **Problem:** Animated conversion may fail at runtime despite valid Kujo arguments.
- **Rule:** `gif_to_webp(...)` requires the `gif2webp` CLI tool to be installed and available in `PATH`.
- **Implication:** If animated conversion is a product requirement, verify `command -v gif2webp` in setup/CI and keep fallback/error messaging explicit.

(Discovered during: 2026-04-29_17-17_animated-gif-to-webp-conversion.md)

### `test-run` initialization is setup-boundary driven
- **Problem:** Tests can fail with undefined symbols even though the same file works under normal `run` execution.
- **Rule:** Put imports/bootstrap code needed by tests inside `test_setup`; do not rely on top-level import execution.
- **Implication:** When diagnosing `test-run` failures, verify setup/import timing before changing runtime internals.

(Discovered during: 2026-05-05_22-20_ai-sdk-runtime-truthiness-and-test-run-gotchas.md)

### Cross-project local preview should be a CLI primitive, not a one-off script
- **Problem:** Script-level server examples can depend on runtime-mode behavior and become project-specific workarounds.
- **Rule:** If a workflow should be available to all users, expose it as `kujo <subcommand>` (for example `kujo serve`) instead of per-project helper scripts.
- **Implication:** Prefer CLI-level shared capabilities for portability and lower maintenance; keep script examples as optional convenience only.

(Discovered during: 2026-05-06_10-09_cli-serve-command-holistic-preview.md)

### `tiny_http` header field comparisons are type-sensitive
- **Problem:** Header matching code can fail to compile when comparing `HeaderField` values directly with `&str` using methods that expect `AsciiStr`.
- **Rule:** In dynamic/header-name helper code, normalize the header field to string form before case-insensitive comparison against `&str`.
- **Implication:** For serve/header refactors in `src/serve_http.rs`, verify header lookup helpers compile before broader protocol changes.

(Discovered during: 2026-05-06_11-06_cli-serve-universal-hardening-followthrough.md)

### Static-serve MIME policy must live in production module code
- **Problem:** Duplicate `#[cfg(test)]` MIME/security helpers can report safer behavior than the actual `kujo serve` runtime path.
- **Rule:** Keep static-server MIME and active-content fallback policy centralized in `src/serve_http.rs` and exercise it via production helper tests or subprocess integration tests.
- **Implication:** Do not maintain separate serve MIME policy logic in `src/main.rs` tests; it causes security drift and false confidence.

(Discovered during: 2026-05-06_22-32_v1-http-001-mime-policy-unification.md)

### Precompressed extension handling must not override direct archive MIME contracts
- **Problem:** Treating every `.gz`/`.br` path as a precompressed representation can force archive files into incorrect response-shape semantics.
- **Rule:** Only apply precompressed `Content-Encoding` splitting for multi-extension assets (for example `index.html.gz`), and keep single-extension archive files on extension-based MIME resolution.
- **Implication:** MIME map changes should include regression coverage for both precompressed siblings and direct archive extensions.

(Discovered during: 2026-05-15_14-21_v1-http-007-mime-registry-coverage.md)

### Root-bound path policy must canonicalize candidate and root
- **Problem:** Lexical path checks alone can miss symlink escapes in module/file serving flows, and request-path checks after filesystem resolution can miss encoded traversal contracts.
- **Rule:** Use shared path helpers that combine lexical rejection (null byte, parent traversal, absolute path, drive prefixes) with canonical root containment (`canonical_candidate.starts_with(canonical_root)`) before allowing root-bound file/module access.
- **Implication:** For `kujo serve` and module loading, enforce encoded-traversal checks before resolution and canonical containment checks after resolution; do not rely on a single phase.

(Discovered during: 2026-05-12_23-06_v1-fs-001-path-security-centralization.md)

### Listener-level socket timeouts can destabilize `tiny_http` accept loops
- **Problem:** Enabling socket receive/send timeouts on the listening socket can cause intermittent `kujo serve` process exits and follow-on `Connection refused` test failures.
- **Rule:** Do not set `SO_RCVTIMEO`/`SO_SNDTIMEO` on the listener for `tiny_http` lifecycle control; prefer server-loop timeout APIs (for example `Server::recv_timeout(...)`) that do not alter accept-loop behavior.
- **Implication:** If timeout behavior is being hardened in `src/serve_http.rs`, validate full subprocess integration stability (`tests/serve_command_integration.rs`) and not only targeted timeout scenarios.

(Discovered during: 2026-05-15_13-17_v1-http-005-request-header-timeout-connection-limits.md)

---

## Mental Model Summary

- Kujo favors explicit contracts and defensive compatibility over implicit inference.
- Parser behavior is token- and precedence-driven; contextual identifiers are intentional.
- Runtime builtin reliability depends on synchronized declaration/registration/dispatch/testing.
- VM/JIT work is highly stack- and CFG-sensitive; local edits often have global invariants.
- Do **not** assume performance refactors are safe unless benchmark/output contracts are explicitly preserved.
