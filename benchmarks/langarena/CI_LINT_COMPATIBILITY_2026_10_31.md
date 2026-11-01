# CI lint compatibility follow-up

The loop candidate's CI run [41261332562](https://github.com/kujolang/kujo/actions/runs/41261332562)
failed Clippy on two baseline locations unchanged from accepted runtime `fd7b764`:

- `src/bytecode.rs`: newer Rust deprecates `AtomicU64::fetch_update` in favor of
  `try_update`. The [Rust API documentation](https://doc.rust-lang.org/std/sync/atomic/struct.Atomic.html)
  identifies the replacement as available from Rust 1.95, above Kujo's 1.89 MSRV.
  Keep the equivalent old spelling with a deprecation allowance scoped to that
  expression. Do not raise the MSRV or replace the atomic algorithm merely to
  accommodate a renamed API.
- `DictionaryManyPublication::contains_key` was only used by one unit assertion.
  Remove the unused production helper and check the missing key through `get`,
  the actual journal-aware read path. The existing ownership/snapshot test remains.

This branch is separate from the frozen loop candidate and accepted benchmark
binary. It makes no performance claim and does not change release gate settings.
Formatting and diff checks are local; MSRV, Clippy, and regression results must be
reviewed from this branch's CI before integration. No validation pass is implied
by committing the candidate.

Follow-up: run `41261682361` passed formatting and Rust 1.89 checking, then
Clippy reached an additional test-only warning in `capture_budget_contracts`.
Replace `(width + 2) / 3` with `width.div_ceil(3)` for the count of mutable
fixture bindings. This preserves the existing fixture counts while avoiding
the intermediate addition. The next CI run must verify this revision; the
previous run is not a Clippy pass.

Run `41262046030` then reached binary-target dead-code diagnostics for three
standalone public JIT compilation entrypoints. The binary compiles its own private
`jit` module and now uses specialized VM entrypoints; the library still exports
the standalone methods, including integration-test consumers. Preserve that API
with method-local dead-code annotations explaining the target distinction.
No method body, caller, or global lint setting changes.

Clippy now uses Cargo's `--keep-going` to collect independent target failures
in the same run. `--all-targets`, `--all-features`, and `-D warnings` remain
required, and any failure still fails the job. This avoids serial CI cycles
that each reveal one additional target's diagnostics.

All five prerequisite jobs passed on `5391aee`. Its full release gate then failed
the three generated-artifact freshness tests: the committed inventories were
11–13 days old against a seven-day limit. Regenerated the unsafe and TODO
inventories with their strict generators; repeat generation matched both Markdown
and CSV byte for byte (299 unsafe matches, zero unknown classifications; 25 TODO
markers, zero unclassified). Refreshed the mismatch inventory using the frozen
accepted `fd7b764` executable, SHA-256
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`,
without compiling locally during production benchmarking. All 150 repository
fixtures matched snapshots in both VM and interpreter; this fixture inventory
is distinct from the 50 LangArena workloads. Freshness limits and classification
rules remain unchanged. The refreshed revision still requires its full CI gate.

The full gate on `e67a936` (run `41265321279`, job `124728414564`)
passed the SSG startup regression and reached `jit_execution_contract`, where
an old fixture still expected array literals/indexing to trigger a JIT warning.
Those operations are supported by the accepted runtime. The warning fixture now
uses an explicitly mutable top-level binding (`DefineGlobal`) and verifies that
exact unsupported opcode while retaining successful VM fallback. Array coverage
moves to the supported-surface test, which now explicitly rejects the warning.
All four CLI execution contracts pass against the frozen accepted executable
above, using a standalone `rustc --test` build of this std-only integration suite.
No runtime behavior or warning policy changed; the full CI gate remains required.

The next full gate, `d0f9e9a` run `41266978042` / job `124733309399`,
passed the corrected CLI suite and failed a stale measurement expectation:
owned-array identity calls were expected never to enter native code. The accepted
runtime's owned-value ABI now executes them natively. Reproduction on the frozen
accepted binary verifies 101 native function/boundary entries and 201 owned
argument dispatches (including compilation retry) for 200 calls. A separate
identity function containing an exception handler genuinely falls back: 200 owned
dispatches and zero native entries. Float calls retain 101 entries/201 dispatches
and zero owned dispatches. All four counters are zero without JIT for all three
fixtures. Plain and measured runs have identical exit status, stdout and stderr.
The regression now covers all three cases; no measurement implementation changes.
The complete ten-test measurement integration suite passes against that same
accepted executable, including schema validation and measured/plain parity,
compiled with `runtime-jit` enabled and the existing release dependency artifacts.

The full gate now uses `cargo test --no-fail-fast` so independent failing test
targets are collected in the same run. No tests are excluded and no failure is
suppressed. Shell syntax validation passes. A controlled command-stub check in
both default and logged output modes confirms Cargo exit 101 remains the gate's
exit status and subsequent gate commands do not execute. This avoids repeated
full CI rounds revealing only the next stale integration expectation.
