# Snapshot-compatible VM closure completion

Evidence date: 2026-09-25 (local session date).

## Branch

- Branch: `runtime/full-upvalue-closures`.
- Base: `719d6f7340bf179bcbd8b63b7eb551b51769d671`.
- Implementation commit: `a27d75b`; final documentation commit
  is the commit containing this report (`git log -1 -- docs/CLOSURE_UPVALUE_IMPLEMENTATION.md`).
- Work is isolated from `main`; no merge performed.

## What changed

Compiler-produced closures now carry explicit definition-site capture descriptors
and use indexed captured loads/stores. The user selected the non-breaking option:
retain v1 per-closure snapshots. Separate sibling closures do not become aliases
of a parent's binding. Aliases and repeated calls of one closure share its cells.

## Previous limitation

The compiler flattened free names before full lexical compilation. The VM then
searched names across a completed parent slot list, allowing an inactive later
shadow to supply the wrong slot. Nested named functions also escaped into globals.
Existing closures already owned heap snapshots and shared them through callback
bridges; the legacy VM-wide `Upvalue` structure was not that mechanism. The
[baseline audit](CLOSURE_UPVALUE_AUDIT.md) preserves the pipeline, semantics matrix,
compatibility decision and pre-change test evidence.

## New closure model

- Capture discovery follows lexical resolution. Descriptors identify a parent
  local slot, parent capture index, scoped script binding or runtime-created named
  binding. Globals remain globals. Runtime-created bare bindings retain existing
  rules for assignment to a previously defined global.
- Captured values are owned `Arc<Mutex<Value>>` snapshots. Frames lazily index the
  same cells already shared by callback bridges. Ordinary locals stay plain slots.
- There is no open stack-reference phase to close. Scope exit, return and errors
  release frame owners; escaped closures retain their heap snapshots.
- Intermediate functions forward only bindings referenced by descendants. Creating
  another closure snapshots the current intermediate value, as v1 requires.
- Stores preserve binding kinds and reject captured `let`/`const` mutation.
- Nested named functions use lexical slots. Recursion installs a frame-local self
  value rather than inserting the function into its own capture environment.
- Legacy manually built chunks without descriptors retain their compatibility
  path. Bytecode is not a documented persistent public format.

## Files changed

- `src/compiler.rs`: lexical capture resolution, transitive descriptors, scoped
  named functions, script/loop/runtime-created binding handling.
- `src/bytecode.rs`: descriptor metadata and indexed capture instructions.
- `src/vm.rs`: indexed cells, checked operands, creation/lifetime handling,
  generator dispatch and explicit JIT exclusion/fallback.
- `tests/closure_capture_audit.rs`: deterministic differential behavior coverage.
- `tests/closure_capture_contracts.rs`: compiler/storage/lifetime/JIT contracts.
- `scripts/closure_snapshot_bench.rs`: standalone before/after measurement harness.
- `docs/CLOSURE_UPVALUE_AUDIT.md`: preserved historical audit and decision.
- `docs/LANGUAGE_SPEC.md`, `docs/ARCHITECTURE.md`,
  `docs/VM_INTERPRETER_PARITY_MATRIX.md`, `docs/V1_SCOPE.md`, `ROADMAP.md`,
  `CHANGELOG.md`: current contract, completed mechanism and explicit limits.

## Tests added

- Snapshot identity: sibling independence, alias sharing, factory independence,
  parent writes, transitive snapshots and state after a caught throw.
- Lexical lifetime: returned closures, inactive shadowing, scopes/early returns,
  unwind escape, deterministic nesting depths 1–12 and unrelated-local exclusion.
- Values/mutability: arrays, maps, structs, enums, callables, scalar/collection
  `let`/`const` rejection and persistent captured mutation.
- Definitions: nested function locality, replacement preserving old values,
  runtime-created bindings, script blocks, loop shadowing and iterable resolution.
- VM internals: exact descriptor slots, indexed operations, last-owner release,
  malformed operand errors and recursive closure self-binding.
- Integration: same-name JIT cache rejection, JIT-to-VM closure factories and VM
  generator captures surviving factory exit/yields. Existing imported callback
  capability, arity, error, async and recursion-guard tests remain passing.

## Validation

Final source patch SHA-256:
`17758d122e125ae8d90fce9d7237aefcc3456de968c3c76887b39cec348a47da`.
Default feature builds used `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` to limit disk/memory use;
features and debug assertions were not disabled. Check/Clippy used normal profiles.

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| `cargo check` | Passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | Passed on final source |
| `cargo test --no-fail-fast` | Completed every target; two failures described below |
| Closure audit + capture contracts | 20 + 9 passed; one known interpreter probe ignored |
| VM/interpreter parity suite | 115 passed |
| Imported VM callback suite | 8 passed |
| `cargo run -- test --runtime vm` | 154/154 runnable passed; 6 skipped |
| `cargo run -- test --runtime dual` | 154/154 runnable passed; 6 skipped; no interpreter fallback |
| Repository hygiene | Passed |
| Affected scope, maturity, docs-policy and README contracts | 4 passed after documentation updates |
| `bash scripts/release_gate.sh --full` | Not repeated: its mandatory full-test step already fails; no release-gate pass claimed |

The pre-existing HTTP shutdown deadline assertion at
`tests/http_route_concurrency.rs:195` failed again (70.92 seconds for its suite).
It also failed before runtime changes and on an isolated baseline rerun; its root
cause is not established here. The binary unit-test copy of
`test_release_hardening_filesystem_core_contracts` failed its atomic-beneath write
assertion once; its isolated rerun passed. The library copy passed in the same full
run. This direct native-API test does not execute closure bytecode. Its transient
failure is recorded rather than concealed or attributed to an unproven cause.
The complete binary unit suite also passed on rerun: 945 passed, 7 ignored.

All other full-run targets passed, including diagnostics/CLI JSON, documentation,
package workflow and native API security boundaries. Detailed local logs are in
`/tmp/kujo-upvalue-audit/`; these are session evidence, not required build inputs.

## Performance

The committed Rust harness parses/compiles and constructs a VM outside each timed
region, checks results, warms once and reports seven-sample medians. It uses Kujo
programs, no network or new dependencies. Three adjacent before/after pairs were
run after Cargo completed, with JIT off. The table is the median of those three
medians in milliseconds (parentheses show their minimum–maximum).

| Workload | Before | After |
| --- | ---: | ---: |
| Ordinary locals (20,000 iterations) | 67.477 (66.390–68.857) | 67.285 (66.969–80.081) |
| Ordinary calls (2,000) | 84.824 (80.132–86.360) | 82.637 (81.089–96.269) |
| Closure creation (2,000) | 186.656 (181.090–187.870) | 187.044 (185.326–232.589) |
| Captured reads (2,000) | 82.744 (79.319–83.127) | 84.412 (84.161–105.033) |
| Captured writes (2,000) | 98.414 (92.712–101.510) | 93.961 (91.320–115.499) |
| Nested creation (1,000) | 154.557 (152.575–165.651) | 159.373 (156.767–196.883) |

These are debug-runtime smoke measurements, not production throughput claims.
The baseline binary was linked against the unchanged `719d6f7` runtime library
before rebuilding it; the after binary uses the final implementation. Both use
the same default features, debug=0, incremental=0 library profile; the standalone
harness was built with `rustc --edition=2021 -O`, linked through `--extern kujo`
and `-L dependency=target/debug/deps`. Raw CSVs are `before-{1,2,3}.csv` and
`after-{1,2,3}.csv` in the evidence directory. The third after run slowed across
all workloads, so small deltas are inconclusive. Median comparisons show no
material ordinary-local/call regression; captured-read and nested-creation costs
remain within a few percent, with overlapping/noisy timing ranges. Release-build
performance claims would require controlled optimized measurements.

## Security review

Focused manual review checked descriptor lengths/indices, binding kinds, frame
teardown, capture lock lifetime, callback authority and JIT cache dispatch. There
is no new unsafe code, capability, native effect or dependency. Captures hold owned
values, not stack references. New indexed operations return checked errors rather
than panicking for malformed operands and poisoned capture locks. User callbacks
do not execute while these locks are held. JIT closure factories reuse the existing
callback bridge's policy, output and recursion guard. Capture propagation resolves
one lexical level per compiler boundary within existing parser/AST traversal;
there is no new unbounded parent-pointer walk. The weak-owner test verifies ordinary
cell release. This is a scoped review, not a whole-repository security certification.

## Remaining limitations

- A pre-existing interpreter missing-self-binding error for a returned recursive
  named closure remains an explicit ignored differential probe; the VM case passes.
- Arbitrary user-created cyclic value graphs still have no tracing collector.
  The implementation does not introduce a default self-capture cycle.
- Snapshot semantics deliberately do not share parent/sibling variable identity.
- General generator restoration and spawn/async parity remain separate work.
- Full release validation is not green because of the recorded test failures.

## Interactions with other roadmap work

Generator restoration must preserve existing captured cell identities and rebuild
frame index views. This branch adds indexed-op dispatch and a bounded VM yield
regression, not a generator scheduler redesign. Async/callback bridges continue
using the existing captured cell maps. Spawn lowering is unchanged and remains
explicitly deferred. Future JIT work must honor capture descriptors and self
bindings before removing the closure/cache guards. No other feature branch was
merged into this worktree.

## Merge recommendation

Ready for code review under the user-selected v1 snapshot contract. Do not treat
this as a green release gate: investigate/resolve or explicitly accept the recorded
unrelated validation failures before release. No merge into `main` was performed.
