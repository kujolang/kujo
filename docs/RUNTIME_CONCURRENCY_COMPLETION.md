# Runtime concurrency completion record

Follow-up: the user subsequently requested fixing the retained interpreter,
Docgen and provider-validation items. See [the follow-up record](RUNTIME_COMPATIBILITY_FOLLOWUP.md)
for the new evidence. The validation table below records the earlier Phase-B delivery.

Status: Phase-B implementation verified; full release gate **PASS**. Live-provider
certification and complete legacy interpreter parity are not claimed. The
intermittent Docgen timeout below remains unresolved despite final passing runs.

Branch: `runtime/generator-async-spawn-completion`, based on merged wave-1 main
`87fae36dd331b185d29256f74d2a92f1aefc5ea5`. The owner requested the remaining
concurrency, interpreter, reliability and provider/trust work. Main is unchanged
by this Phase-B branch. The Phase-A plan and wave-1 handoff remain historical
inputs, superseded for current behavior by the language specification and this
record.

## Runtime contracts

- A generator owns its continuation. Aliases share progress; distinct instances
  remain independent. One active resume is allowed. Return completes without
  yielding. The first error is cached. For-loop consumption is lazy, including
  early break. Yield expressions retain evaluated operands across suspension.
- VM generators resume the ordinary dispatcher with their operand stack, frames,
  handlers, lexical scopes and capture cells. Their global owner is weakly held;
  moving a live generator into an unrelated runtime rejects. Caller state is
  restored on success, exhaustion and language errors. Nested resume is bounded.
- Interpreter generators retain explicit statement continuations. Expression
  lowering introduces private, unlexable operand bindings and preserves
  short-circuit branches. Creation/resume capability policies intersect.
- Async calls eagerly submit a body and return a promise. Arity and admission
  errors occur at the call; body errors belong to the promise. An ordinary
  synchronous function containing await does not become an async declaration.
- Tasks use the existing Tokio blocking lane, with 16 admitted language tasks per
  process. Admission remains occupied until the body exits. Worker VMs disable
  JIT and synchronously await within their owned context. Native I/O retains its
  existing executor and bounds; no new scheduler is introduced.
- Promise aliases and repeated/concurrent waiters share one terminal outcome.
  A timeout or dropped waiter does not consume the source completion.
- `spawn_task` executes a zero-argument ordinary or async callable. `await_task`
  returns its reusable completion promise. Cancellation wins through one sender,
  rejects waiters immediately and requests cooperative exit at language and
  channel-wait boundaries. It cannot undo effects or interrupt every native call.
- Detached spawn executes referenced transferable snapshots with binding kinds.
  Referenced channels, callables, promises, generators and host handles reject
  before scheduling; unrelated unsupported values do not block submission.
  Recursive transfer is bounded to depth 64 and 100,000 visited nodes, not a total
  byte-memory sandbox. Secrets retain their wrapped representation.
- Detached work has no join and process exit does not implicitly wait for it.
  Errors go to stderr, capped at 16 messages of 512 characters and one suppression
  notice per process. Explicit user printing remains ordinary user output.
- Task globals are submission snapshots. Existing per-closure snapshot semantics
  remain authoritative: different closures are independent, aliases share that
  closure's state. Same-binding concurrent read-modify-write is not atomic;
  `shared_add_int` is the existing shared integer operation. AST async writeback
  journals changed bindings so unrelated writes do not overwrite one another.
- Channel send releases the common channel mutex before blocking on capacity.
  Receive waits for a value in both runtimes and observes task cancellation.

Async generators, yield-from, struct generator methods, arbitrary ownership-cycle
collection, new syntax, implicit shared sibling cells and a replacement executor
are outside this completion. Unsupported async generators reject explicitly.

## Interpreter and reliability corrections

Named returned AST closures bind lexical self in their invocation scope; the
previously ignored recursion probe is enabled. Module namespace calls work in the
interpreter, including imported async callbacks retaining submitting VM globals.
Captured mutability failures enter an owning VM exception handler.

A drop probe found that an unrelated global generator handle was retained by a
closure created inside that generator. The correction snapshots lexical free
bindings and follows referenced top-level callable dependencies, preserving scope
and binding-kind layout. Shared containers are visited once during dependency
inspection. Interpreter-only bodies that the compiler cannot resolve retain the
legacy snapshot fallback. This is not a tracing collector for intentional cycles.

Hover reuses one tokenization for definition/reference lookup. The latency guard
and all result assertions remain unchanged. A loaded-host 20-call run measured
25.02 ms completion, 14.22 ms diagnostics and 14.25 ms hover; all guardrails passed.

The language fixture now requires all three repeated messages and the first seven
Fibonacci values; its old snapshot encoded the incomplete generator loop. A
separate runner defect executed five already-labelled `Inventory: skip` probes,
then generated missing-configuration error snapshots. The runner now honors those
headers in all runtime modes. A CLI contract proves skipped probes neither run
nor create snapshots. Four database probes still require explicit live targets;
the fifth PDF probe was exercised through its dedicated external validator.
That validator now checks physical A4 dimensions to 0.01 point rather than
requiring a rounded `842` string. Both runtime engines passed with bundled Poppler.

Docgen HTTP fixtures now own their server/thread until teardown, eliminating
premature idle expiration. This alone did **not** resolve all redirect variability:
two assertions intermittently failed with request timeouts before the local
fixture received a request. Diagnostic runs sampled macOS proxy discovery during
client initialization and idle client/server transport waits afterward; this
does not establish the timeout's root cause. Repeated diagnostic runs and the
fresh uninstrumented 55-test suite passed. All temporary transport logging was
removed. No retry, relaxed assertion or enlarged request timeout is presented as
a fix; the intermittent timeout remains an open reliability limitation.

## Provider and trust boundary evidence

No provider deployment, credentials, account, region or spend ceiling was supplied.
Only offline/local evidence is claimed. Authentication belongs to the caller or
intervention transport; actor metadata is not authentication. Effect attestations
require trusted adapters. Same-workspace/clean replay requires actual preservation
and materialization. Unknown external effects remain fail-closed. No general
exactly-once, automatic compensation, live suspension or rollback promise is made.

| Repository | Reviewed branch/commit | Current evidence |
| --- | --- | --- |
| Dispatch | `workflow/failure-gate-evidence-control`, `2bbc846221a739c09d6d2fce4ff0f8fa5c8412fa` | Full `scripts/run_release_gate.sh` passes: focused contracts, 24 core shards, command smoke, 3/3 bounded release workloads in 24 seconds |
| Workcell | `workflow/failure-gate-evidence-control`, `37772674dfc44b78eeb4531efa4a01ee059c8c3a` | Full `tests/run.sh` passes with the explicit source-runtime version override; three official offline adapters pass |

The deterministic Dispatch failure example passed with real Workcell fixture
lifecycle, manifest validation, Eval, typed intervention, evaluator-only rerun,
RunLedger and CaseFile. Its final handoff is retained at
`/var/folders/wb/0cck3lgd08n55g_8ly9qmf_00000gn/T/failure-golden-1790414130826/handoff.json`.
These companion repositories are unchanged by this Phase-B task. Their branch
identities and independent release pins remain intact.

Workcell's source-runtime command is:

```bash
KUJO=/absolute/path/to/kujo/target/debug/kujo \
WORKCELL_TEST_KUJO_VERSION=1.5.0 bash tests/run.sh
```

This does not certify its pinned Kujo 1.2.1 release. Optional real OCI/provider
execution is not counted as a pass when skipped by the fixture gate.

## Security and performance review

Managed diff review `845d76ef-9c72-4a51-aa91-4fd668680ae1` covers immutable
`87fae36..05727ec`: 19 changed production files, no reportable security findings.
It is a parent-only source/targeted-test review, not formal certification. Later
capture-lifetime changes were manually reviewed against the free-binding resolver,
binding kinds, transitive callable/container dependencies and owner-drop tests;
that follow-up is outside the immutable managed scan. Temporary transport probes
were removed, so no docgen production-code change remains.
No new unsafe Rust was introduced. Capability propagation, generator owner checks,
transfer rejection, shared promise completion and bounded admission are the
principal controls. Workflow control does not bypass ordinary native capabilities.

Ordinary VM locals remain stack slots, not per-local locks or allocations. Async
admission/worker allocation applies only at task submission; captured AST writes
allocate a journal. Snapshot globals and interpreter capture dependency analysis
have costs; this report makes no zero-cost or universal latency claim.

The existing VM harness (`scripts/closure_snapshot_bench.rs`) was run against the
final default-feature development library, with an optimized standalone driver,
one warmup and seven measured samples. No Kujo build ran concurrently:

| Workload | Phase-B median ms |
| --- | ---: |
| Ordinary locals | 57.00 |
| Ordinary calls | 76.15 |
| Closure creation | 173.47 |
| Captured reads | 80.17 |
| Captured writes | 82.63 |
| Nested creation | 147.29 |

All checksums passed. Startup medians with the same warmup/sample count were
23.60 ms for `--version` and 30.22 ms for hello execution. These are observations
on a shared macOS host, not release performance guarantees. Earlier exploratory
paired numbers are excluded from comparison: the archived wave-1 harness used
a debug=0 library while the current library uses the normal development profile,
and concurrent host load materially changed timings. There is no controlled
release-profile performance-equivalence claim.

## Validation ledger

Logs and command exit files are in `/tmp/kujo-phase-b`. Current results include:

- Baseline closure/callback/characterization suites passed before implementation.
- Shared promise completion: six contracts passed, including pending multi-waiter
  wakeups, producer drop/rejection, timeout recovery and duplicate aggregation.
- Closure capture audit: 22/22 passed, zero ignored; snapshot contracts: 9/9.
- Imported callbacks: 9/9 passed; async/task contracts: 8/8, including transitive
  top-level functions in arrays and iterator transformers; generator contracts:
  18/18 including the drop regression; runtime characterization: 14/14 in the
  broad run, including expression yields. Bounded concurrency lifecycle: 16/16.
- `cargo clippy --all-targets --all-features -- -D warnings` passed before the
  capture follow-up. Two vendored tiny_http warnings remain baseline dependency
  warnings, not new root-crate lint suppressions.
- The first `cargo test --no-fail-fast` failed seven old async unit cases in both
  lib/bin copies because their helper did not drive suspended execution, one stale
  architecture-doc assertion, one malformed new cancellation fixture and two
  docgen redirect cases. The value-returning helper now explicitly uses synchronous
  await; cooperative lifecycle tests remain separate. No result assertion was
  weakened. The intermittent docgen timeout is recorded above.
- An intermediate release wrapper stopped on compile errors introduced while
  adding test diagnostics. Those diagnostic bindings are corrected; this is a
  failed attempt, not a passing release gate.
- A later iterator-capture fixture initially used the builtin name `items` as a
  declaration. Renaming the fixture binding corrected that test; the fresh task
  suite passed 8/8 without changing its expected result.
- The first complete uninstrumented Rust suite passed, but its release wrapper
  failed at `cargo run -- test`: 153/154 fixtures, because of the old generator
  snapshot. After correcting that snapshot, dual passed 154/154 using five
  interpreter fallbacks for accidentally discovered provider probes. Strict VM
  failed those five generated error snapshots. Their existing inventory-skip
  policy is now honored, and the generated untracked snapshots were removed.
  Those interim outcomes are not the final gate result.
- PDF external validation initially lacked Poppler on PATH. Bundled Poppler
  exposed the old dimension-string assertion; after the numeric correction,
  `bash tests/pdf_external_validation.sh` passed in VM and interpreter modes.

## Final validation

Rust commands used `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2` and
`CARGO_INCREMENTAL=0` on this shared host. No assertion or timeout was relaxed.

| Command / gate | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS; the two pre-existing vendored tiny_http warnings remain |
| `cargo test` | PASS; 914 library and 947 CLI unit tests, seven existing ignores in each; integration targets below pass |
| `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_gate.sh --full` | PASS, exit 0; dedicated security 90/90, package 9/9, parity 115/115, sockets 31/31 |
| `cargo run -- test --runtime vm` | PASS, 149/149 runnable, 11 explicit skips |
| `cargo run -- test --runtime dual` | PASS, 149/149 runnable, zero interpreter fallbacks, 11 explicit skips |
| `cargo test --test workflow_control_contracts -- --include-ignored` with the actual handoff/Dispatch paths | PASS, 13/13, zero ignored, 0.21 seconds of test time |
| `bash tests/pdf_external_validation.sh` with bundled `PDFINFO`, `PDFTOTEXT` and Poppler library path | PASS in VM and interpreter |
| Dispatch canonical release gate / Workcell canonical `tests/run.sh` | PASS at the companion commits above; fresh failure-control example also PASS after the capture follow-up |
| `cargo audit --deny warnings --ignore RUSTSEC-2025-0141` | PASS, 652 dependencies scanned; the existing exception is the documented unmaintained build-only bincode dependency |
| Supplemental `cargo run -- test --runtime interpreter` | FAIL against VM snapshots: 117/149, 32 mismatches, 11 explicit skips; every mismatch is already listed in the baseline inventory |

The full Rust run includes `docs_examples` 6/6, `readme_contracts` 1/1,
`cli_contracts` 31/31, `cli_json_contracts` 19/19, `diagnostics_golden` 2/2,
`docgen_universal` 55/55 and `lsp_latency_guardrails` 1/1, plus the runtime suites
listed above. Workflow artifact tests are normally ignored in `cargo test` but
were separately executed with real fixture artifacts. The existing ignored
doctest remains ignored. Optional `cargo deny check` was unavailable; the wrapper's
optional broad benchmark smoke was not enabled. The explicit measurements above
are separate evidence, not substitutes labelled as those optional gates.

The eleven language-fixture skips comprise six test-run-only fixtures and five
dedicated-harness probes. Four probes require live database targets; the PDF
probe passed separately. No skipped probe is counted as a pass.

The supplemental interpreter result does not identify a new Phase-B regression:
all 32 names were already `interpreter_only_mismatch` in
`docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`. Many are advisory type-checker
warnings or intentionally engine-specific diagnostics; legacy ArgParser method
dispatch and operator/coercion behavior also differ. The closure recursion and
namespace/callback defects targeted here are fixed. This report does not relabel
the broader interpreter-only run as green or silently normalize its diagnostics.

Validation logs and receipts remain under `/tmp/kujo-phase-b`, particularly
`release-final-2.log`, `check-final.log`, `vm-verified.log`, `dual-verified.log`,
`interpreter-verified.log`, `workflow-verified.log`, `failure-golden-final.log`,
`pdf-final.log`, `closure-final.csv` and `startup-final.json`.

## Durable open finding

Only the unresolved Docgen timeout was admitted to SignalBox:

- Capture: `cap_529acae7-227c-4096-afe8-5068c1efee7d`.
- Signal: `sig_b0b21419-26ba-4e41-99eb-3c0cb8520ac1`.
- Exact-ID retrieval and the `loopback` concept search returned both records.
- No duplicate capture was found. Completed fixes, routine verification and
  already-inventoried interpreter differences were not captured as new findings.

The Strata session handoff records the final pushed branch tip, this evidence
path and remaining scope. Main and the companion branches are not merged by this
Phase-B delivery. Start continuation from the pushed Phase-B branch, not its old
audit branch.
