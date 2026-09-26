# Runtime hardening wave 1 integration review

Evidence dates: 2026-09-25–26. This is a source integration review, not a release,
live-provider certification or whole-repository security certification.

## Integration branch

- Branch: `integration/runtime-hardening-wave-1`.
- Starting main: `719d6f7340bf179bcbd8b63b7eb551b51769d671` (local and freshly
  fetched `origin/main` agreed). Initial working tree was clean.
- Verified integration baseline: `dace9c1837ca7c7b27c406ea9b8944cc6aa7f0f3`.
- Reconciled handoff/documentation and contract test: `a6dba26245dd85854d21e0612693b99049ee43b1`.
- The final evidence commit changes only the review and ledger; obtain its exact tip
  with `git rev-parse integration/runtime-hardening-wave-1`. The final response
  records that tip. Main was not merged or modified.

## Agent branches reviewed

All three Kujo branches have merge base `719d6f7340bf179bcbd8b63b7eb551b51769d671`.

| Agent | Branch | Reviewed tip | Disposition |
|---|---|---|---|
| 1 | runtime/full-upvalue-closures | f7bda5b40035c9d30578f2265d1493fbfa62394d | Retained history, with capture and native teardown fixes before admission. |
| 2 | runtime/generator-async-spawn-audit | 6798c10eb49a37b0a9da5b236bcf6d7cbc23c491 | Plan and 13 characterization tests only; no runtime implementation. |
| 3 | workflow/failure-gate-evidence-control | 676cbf4ea143f5d661f0087b00118150adfd2270 | Contracts, tests, type-checker depth fix and documentation. |

Agent 1 commits: `23154ff`, `7511d6d`, `02c52ff`, `447f5ad`, `a27d75b`,
`f7bda5b`. Agent 2: `44c30b1`, `6798c10`. Agent 3: `bf91233`, `e9b2203`,
`ab923bc`, `14c27de`, `de80cc5`, `6a9b8a9`, `676cbf4`.

Companion repositories remain on their independent
`workflow/failure-gate-evidence-control` branches: Dispatch
`2bbc846221a739c09d6d2fce4ff0f8fa5c8412fa`; Workcell
`37772674dfc44b78eeb4531efa4a01ee059c8c3a`. They are not silently represented
as changes merged by the Kujo branch. Their review bases are respectively
`a01a36524b9e0dbaaf2bc30ef818ab4884e6c797` and
`e57ea19246dad08e57a5ecbc2a47145127af2f2a`; their commit series are
`16f351c`, `ae425eb`, `f8c2edd`, `2bbc846` and `6e573fa`, `5d28f44`, `3777267`. Eval, RunLedger and CaseFile participate
as existing fixture producers; they were not modified by this integration.

Agent 1 changed paths:

```text
CHANGELOG.md
ROADMAP.md
docs/ARCHITECTURE.md
docs/CLOSURE_UPVALUE_AUDIT.md
docs/CLOSURE_UPVALUE_IMPLEMENTATION.md
docs/LANGUAGE_SPEC.md
docs/V1_SCOPE.md
docs/VM_INTERPRETER_PARITY_MATRIX.md
scripts/closure_snapshot_bench.rs
src/bytecode.rs
src/compiler.rs
src/vm.rs
tests/closure_capture_audit.rs
tests/closure_capture_contracts.rs
```

Agent 2 changed paths:

```text
docs/GENERATOR_ASYNC_SPAWN_COMPLETION_PLAN.md
tests/runtime_semantics_characterization.rs
```

Agent 3 changed paths:

```text
CHANGELOG.md
ROADMAP.md
docs/ECOSYSTEM_GOLDEN_PATH.md
docs/FAILURE_GATE_EVIDENCE_REVIEW_PLAN.md
docs/FAILURE_GATE_IMPLEMENTATION.md
docs/generated/V1_CODE_TODO_TRIAGE.csv
docs/generated/V1_CODE_TODO_TRIAGE.md
schemas/workflow-control/README.md
schemas/workflow-control/control-event-v1.schema.json
schemas/workflow-control/evaluation-result-v1.schema.json
schemas/workflow-control/execution-result-v1.schema.json
schemas/workflow-control/intervention-request-v2.schema.json
schemas/workflow-control/reexecution-descriptor-v1.schema.json
src/type_checker.rs
tests/workflow_control_contracts.rs
```

## Merge order used

1. Agent 1 plus integration fixes: `1c4f208065aa1f39fd505ecda7d20aedaeda6802`.
2. Agent 3: `39dfda310abe047b1ec193355e2172477806649b`.
3. Agent 2 Phase A: `dace9c1837ca7c7b27c406ea9b8944cc6aa7f0f3`.
4. Canonical documentation and Phase-B handoff reconciliation (final child).

Every merge preserves its reviewed branch ancestry. Agent-1 and Agent-3 stage
gates passed before the next merge. Main remained unchanged after a fresh fetch.

## Agent 1 verdict

The substantive implementation replaces flat free-name discovery with lexical
capture descriptors and indexed access. It is **snapshot-compatible closure
completion**, not shared parent/sibling upvalues. The prior user decision is
recorded in the branch audit and Strata handoff. Repeated calls/aliases share a
closure's owned cells; new closures snapshot again. Local slots are not universally
heap-wrapped. No defining frame is retained, and no new unsafe Rust was added.
Nested self-binding is frame-owned to avoid an automatic capture cycle.

Review found one regression not covered by the original suites: local imports
were missing from definition-site capture metadata. A factory importing `value`
and returning `func(){return value()}` prints `7` on the unchanged compiler and
fails with `Undefined variable: value` at Agent 1's tip. Commit `4d96c11` restores
selected import bindings and lazy import-all/namespace capture forwarding, without
whole-environment capture. The same focused commit completes scoped constant
registration, a pre-existing omission exposed by the new descriptor review.
Tests verify lifetime, transitive imports, restricted callback execution and
constant mutability. Namespace import access retains an existing interpreter
`Undefined variable: bridge` limitation, asserted explicitly rather than hidden.

Checked descriptor/slot lengths return errors on new capture paths; capture locks
are released before callbacks. Existing capability policy and bridge recursion
limits remain in force. Ordinary locals remain values; compiler metadata adds
compile-time maps, while captures retain existing Arc/Mutex costs and a lazy
indexed view. JIT guards add eligibility inspection and intentionally fall back
for closure-bearing functions. No production-throughput guarantee follows.

Remaining limitations: interpreter returned-named-recursion probe remains ignored;
arbitrary cyclic values lack tracing collection; imported namespace binding
parity and general generator/async/spawn completion remain outside this change.
Historical Agent-1 failure reports are preserved; fresh gate results appear below.

The original full suite and release gate reproduced the reported HTTP shutdown
failure. A native tiny_http-only probe (no Kujo VM) and a sampled stack traced
it to Drop connecting to its wildcard `0.0.0.0` address on macOS. Commit
`ba7f962` uses family-matching loopback for wildcard listeners and a 100 ms
connect timeout. The existing process-shutdown deadline remains unchanged;
a direct teardown regression was added. The first corrected full run then caught
a stale generated unsafe-inventory line number caused by inserting that test.
`761c75a` regenerates both artifacts with `generate_unsafe_inventory.sh --strict`;
the unsafe boundaries themselves are unchanged. This is a pre-existing native
portability fix, not a closure regression or scheduler redesign.

## Agent 3 verdict

Contracts remain versioned and producer-neutral. Execution outcomes and evaluator
judgments are separate; evaluator errors are indeterminate rather than failed
quality checks. Schemas bound explicit collections and identify integrity evidence;
consumers still own byte/depth bounds, authentication and transition validation.
Embedded evidence now enforces the standalone integrity requirements. Additive
extensions remain allowed, but they grant no executable authority.

Dispatch owns policy and typed, revision-bound intervention. Unknown/partial effects
reject replay; a submitted key is insufficient without sink-enforcement evidence.
Same-workspace, clean-workspace and evaluator-only retries are explicit. Workspace
retention does not imply process suspension. Workcell preserves declared evidence
before cleanup; actual outcomes state unsupported provider modes. Immutable records
precede a hash-linked journal, and mismatched/torn history blocks admission.

The type-checker guard fix balances depth on every inference return and is narrowly
justified by Dispatch's command gate; it changes no closure/frame/task model.
Remaining boundaries: trusted producer attestations, caller/transport authentication,
adapter materialization, operator retention/recovery, an 8 MiB active journal cap,
and separate live-provider/pinned-release certification.

## Agent 2 Phase-A verdict

Both original commits were accepted; there was no premature implementation to
reject. The dated source audit remains historical evidence. Current dependency
classifications, G11/G12/A06/X03 test oracles, the starting branch and the frame
lifetime recommendations were reconciled with snapshots and derived capture caches.
Open-cell promotion and shared-sibling assumptions were rejected. Phase B has not
started; D1–D6 policy/compatibility choices remain proposals.

See [the authoritative handoff](GENERATOR_ASYNC_SPAWN_PHASE_B_HANDOFF.md) and
[the corrected plan](GENERATOR_ASYNC_SPAWN_COMPLETION_PLAN.md).

## Conflicts resolved

The only Git conflict was `CHANGELOG.md` while merging Agent 3. The resolution
retains the HTTP/capture fixes and the inference-depth fix, and consolidates the
workflow schema additions into one user-facing entry. `ROADMAP.md` merged
textually without conflict, then received the current wave status and Phase-B
next step. Agent 2 had no textual conflict. No runtime file used an ours/theirs
resolution.

A semantic mismatch mattered more than text conflicts: the branch name “full
upvalues” did not authorize shared sibling identity. Integration preserves the
explicit v1 snapshot decision and rewrites the Phase-B assumptions accordingly.
The local-import regression and independently reproduced native shutdown blocker
were repaired before admitting Agent 1.

## Files materially reconciled

`src/compiler.rs`, `tests/imported_vm_callback.rs`, `tests/closure_capture_audit.rs`,
`tests/architecture_docs_contract.rs`,
`vendor/tiny_http-0.12.0/src/lib.rs`, `tests/http_route_concurrency.rs`,
`CHANGELOG.md`, `ROADMAP.md`, `docs/ARCHITECTURE.md`,
`docs/VM_INTERPRETER_PARITY_MATRIX.md`,
`docs/GENERATOR_ASYNC_SPAWN_COMPLETION_PLAN.md`, and the new review/handoff.
`src/vm.rs` and `src/bytecode.rs` retain Agent 1's runtime architecture;
`src/type_checker.rs` and `schemas/workflow-control/` retain Agent 3's changes.
README and AGENTS required no factual changes. Historical release ledgers and
legacy concurrency docs are not silently rewritten as current completion claims.

Pre-merge inventory:

| Agent | Branch | Files | Runtime overlap | Docs overlap | Test overlap | Risk |
|---|---|---:|---|---|---|---|
| 1 | runtime/full-upvalue-closures | 14 | compiler, VM, bytecode; no Agent-3 overlap | changelog/roadmap with Agent 3 | distinct closure suites | High: lexical compatibility, callbacks, lifetime |
| 2 | runtime/generator-async-spawn-audit | 2 | none | stale assumptions about Agent 1 | separate characterization suite | Medium: incorrect next implementation model |
| 3 | workflow/failure-gate-evidence-control | 15 | type checker only | changelog/roadmap with Agent 1 | separate schema suite | Medium: retry authority, evidence, ecosystem boundaries |

`V1_SCOPE`, `ARCHITECTURE` and parity changes were Agent 1's; schemas were Agent
3's. No duplicate test files/schemas were introduced. Old VM-wide Upvalue opcodes
remain a deliberate legacy path; removing them or unrelated TODOs was out of scope.

## Combined validation

Exact commands, exit statuses, elapsed times, failure classifications, skips and
companion fixture identities are recorded in the
[validation ledger](RUNTIME_HARDENING_WAVE_1_VALIDATION.md).

The final combined run includes formatting, check, strict all-target/all-feature
clippy, the complete Rust suite, each requested documentation/CLI/diagnostic/parity
suite, VM and dual command suites, closure/imported-callback/schema/Phase-A tests,
and the full release wrapper. Every command in the final sequence exited 0.
Closure suites passed 21 + 9 cases (one known interpreter probe ignored), imported
callbacks passed 9, parity passed 115, and Phase A passed 13. VM and dual each
passed 154/154 runnable fixtures; dual used zero interpreter fallbacks.

Independent ecosystem validation is not inferred from Rust tests:

| Repo / reviewed commit | Canonical checks | Result |
|---|---|---|
| Dispatch / `2bbc846` | Offline release gate; integrated control 14, safety 19, execution 5; lifecycle/decision/golden fixtures | PASS |
| Workcell / `3777267` | Version/config/full tests/quality/release report/links; official adapter tests and integrity | PASS with source-runtime override; live provider matrix excluded |
| Kujo workflow artifacts / integrated runtime | Schema suite including both normally ignored actual-artifact checks | 13/13 PASS |

The original native shutdown failure and stale generated inventory were fixed
before Agent 1 admission. One unchanged LSP latency rerun passed after a loaded-host
failure. Docgen redirect checks also failed during the first combined run;
diagnostic edits were discarded because they did not establish a causal fix.
The ledger preserves these failures separately from subsequent results.

## Release gate

**PASS** — `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_gate.sh --full`
exited 0 (727 seconds) on the reconciled integration tree. Formatting, strict
clippy, the complete Rust suite, native security, package workflow, parity,
socket-serving tests and the dual command suite passed. Dependency audit passed
with the repository's existing build-only `RUSTSEC-2025-0141` exception.

The wrapper's optional benchmark smoke was not enabled; separate closure and
journal measurements are reported below. Optional `cargo deny check` was skipped
because `cargo-deny` is absent. Neither optional check is reported as a pass.
Default Rust ignores, six CLI fixture skips and companion live-provider exclusions
are recorded in the ledger. No failure was waived or assertion weakened.

## Security integration review

Manual review covered compiler/capture descriptors, mutability, frame ownership,
callback policy, JIT fallback, effect/replay admission, evidence command rejection,
path reads, redaction and decision concurrency. No new unsafe block or native
capability is introduced. No workflow descriptor executes arbitrary evidence
commands or grants Kujo capabilities. Restricted imported-callback tests and
native security tests protect the shared runtime boundary; Dispatch's negative
fixtures protect retry admission and stale/conflicting decisions.

Producer labels/extensions can still contain sensitive data if the producer
violates its contract. Actor fields are attribution, not authentication. Hash
chains are not signatures. Existing non-capture mutex unwraps and unsupported
cross-thread handle semantics were not redesigned. This scoped review establishes
no formal certification or general sandbox/exactly-once guarantee.

## Performance integration review

Source inspection found no heap cell or mutex added to every ordinary local.
Only selected captures own cells; ordinary local slots remain `Value`s. Closure
creation resolves descriptors once, captured accesses use a lazy indexed cache,
and callback guards preserve capability/recursion context. Closure-ineligible JIT
fallback remains an intentional cost, not a performance promise.

Fresh matched-profile smoke: `scripts/closure_snapshot_bench.rs`, one warmup and
seven samples per workload; three adjacent main→integrated runs, reporting the
median and range of those three per-run medians. Both drivers use the same default
features, unoptimized debug=0 library and optimized standalone driver. Main is
`719d6f7`; the integrated driver uses the final production source (documentation
and test changes do not alter it). Parsing, compilation and VM construction are
excluded. Every expected-value assertion passed. This shared macOS host had other
active workloads; measurements are not a release performance threshold.

| Workload | Main median ms (range) | Integrated median ms (range) |
|---|---:|---:|
| ordinary_locals | 91.194 (88.006–95.451) | 92.167 (91.277–97.536) |
| ordinary_calls | 117.621 (111.710–139.280) | 121.392 (118.542–122.604) |
| closure_creation | 257.071 (251.315–259.315) | 274.719 (258.408–276.076) |
| captured_reads | 118.717 (116.190–121.332) | 137.098 (118.614–140.321) |
| captured_writes | 132.438 (130.816–142.850) | 141.648 (128.344–163.461) |
| nested_creation | 218.444 (206.132–225.744) | 226.434 (220.472–230.741) |

Ordinary local/call medians were roughly 1%/3% higher; captured-read median was
roughly 15% higher, with overlapping run ranges. Other capture medians were also
higher in this sample. This establishes working workloads and bounded observed
costs, not production equivalence or a statistically isolated regression. Earlier
mixed-profile exploratory numbers were excluded from comparison. Agent 1's own
historical paired measurements were closer; no claim is made that these are a
controlled benchmark campaign.

Separate current-tree startup smoke (one warmup/seven samples, normal dev debug
profile) measured median `--version` 49.066 ms and hello execution 73.552 ms;
there is no paired startup baseline. Actual workflow schema validation passed
13/13 in 0.40 seconds of test time; this is validation evidence, not a serialization
throughput benchmark. Dispatch's 100-event journal smoke produced 50,419 bytes,
7,080 ms append time and 290 ms reconciliation time. Source inspection confirms
append does not rewrite history, reconciliation scans once under lock, and the
8 MiB active-journal limit bounds recovery. No long-run throughput claim follows.

## Remaining risks

- VM snapshot completion does not fix interpreter returned-named-recursion or
  namespace-import parity. The recursive interpreter probe remains ignored.
- Generator continuation, async ordering/overlapping mutation, spawn execution
  and handle transfer remain Phase B; D1–D6 remain proposed compatibility choices.
  The historical legacy generator fixture parse failure is not a passing test.
- Arbitrary cyclic values lack tracing collection. No cross-thread host-handle
  portability or read-modify-write atomicity is promised by Arc/Mutex captures.
- Workflow safety depends on truthful producer effect reports, authenticated
  callers, trusted materialization adapters and operator recovery/retention.
  Retained files are not live suspension; journals cap at 8 MiB.
- Live providers/OCI and Workcell's pinned-release matrix were not certified by
  these source-runtime fixture runs. This macOS review is not a platform matrix.
- Docgen redirect checks showed repeatable failures followed by passing diagnostic
  runs without a proven source fix; both final uninstrumented full suites passed.
  Preserve the timing/environment uncertainty rather than claiming a causal fix.
- One loaded-host LSP hover guardrail run measured 132.35 ms against 120 ms; an
  unchanged isolated rerun passed. LSP source paths were unchanged. Preserve
  the threshold and distinguish timing-sensitive runs from semantic failures.

## Agent-2 Phase-B base

Use `dace9c1837ca7c7b27c406ea9b8944cc6aa7f0f3` as the immutable integrated implementation checkpoint; the
reconciled handoff is committed in `a6dba26245dd85854d21e0612693b99049ee43b1`.
The final evidence-only child is the recommended checkout. Create `runtime/generator-async-spawn-completion` there.
The exact final tip is recorded in the final response and remote branch.
Handoff: [GENERATOR_ASYNC_SPAWN_PHASE_B_HANDOFF.md](GENERATOR_ASYNC_SPAWN_PHASE_B_HANDOFF.md).

## Main merge recommendation

**READY TO MERGE** for this bounded source-integration wave, with the documented
interpreter/concurrency/provider limitations retained. All intended reviewed Kujo
tips are ancestors of the integration branch. Final combined commands and the full
release wrapper exited 0. The branch is committed and pushed after this evidence
record; main remains at the starting commit and requires separate authorization.

Agent 2 should begin a new Phase-B branch from the final published integration tip,
using the reconciled handoff. Do not interpret this readiness as a v1 shared-sibling
capture change, completed Phase B, or live-provider/security certification.
