# Kujo Roadmap

Updated: 2026-09-29
Stable release: [v1.7.0](https://github.com/kujolang/kujo/releases/tag/v1.7.0)

> Current crate version: `1.7.0` in [Cargo.toml](Cargo.toml)

Kujo **1.7.0 is released** for Linux x64/arm64, macOS x64/arm64 and Windows x64,
with matching lifecycle-script-free runtime npm packages. The exact source/tag
target is `44af277848173664f72ca85f2a1b3b98d634ecdd`.
See [publication evidence](docs/KUJO_1_6_RELEASE.md) for hashes and hosted native/npm
install checks. The unchanged [RC review](docs/KUJO_1_6_RC_REVIEW.md) retains the
original local/downstream validation. Later release-engineering commits are not
the runtime artifact source. Wave C beta and Wave D alpha remain experimental;
participant SDK packages remain unpublished. Human adopter usability remains
post-release validation.

## Where Kujo stands

- `kujo run` uses the bytecode VM by default. The interpreter remains a fallback
  and debugging path.
- The language, CLI, LSP, capability model, AI runtime primitives, and supported
  machine-readable contracts are stable within the v1 compatibility policy.
- Linux x64/arm64, macOS x64/arm64, and Windows x64 release binaries ship with
  SHA-256 checksums. The public npm 1.7.0 runtime package covers the same targets
  with trusted-publisher provenance.
- The official Kennel registry is live at
  [kennel.kujolang.ai](https://kennel.kujolang.ai/). Kennel 1.1.0 manages project
  packages and global tools on macOS and Linux.
- Kujo's built-in `kujo.toml` / `kujo.lock` commands remain separate from
  Kennel's `kennel.toml` / `kennel.lock` workflow.

## Recently completed

- Published `@kujolang/kujo-runtime` and all five native platform packages at
  1.6.0 through npm trusted publishing, with signed provenance and clean-install
  verification across the supported runtime matrix.

- Merged the runtime-hardening baseline into Kujo `main` at
  `9d3c6edeba2b20cb22216816b1a95ee5f24a61b1`
  ([PR #12](https://github.com/kujolang/kujo/pull/12)), with companion
  [Dispatch #1](https://github.com/kujolang/dispatch/pull/1) and
  [Workcell #2](https://github.com/kujolang/workcell/pull/2) merged into their
  respective `main` branches. Final PR and post-merge CI checks passed in all
  three repositories. These runtime changes are included in the published 1.6.0 release.

- Followed up the Phase-B interpreter fixture drift: corrected static-checker
  binding/signature handling, ArgParser dispatch, VM relational overloads and
  interpreter error-frame preservation, including unknown collection-value
  inference exposed by Dispatch's CLI smoke checks. VM, dual and interpreter
  sweeps each pass all 149 runnable fixtures; eleven dedicated-harness skips
  remain explicit. Exact engine-specific diagnostic snapshots retain real
  warnings. See [the follow-up record](docs/RUNTIME_COMPATIBILITY_FOLLOWUP.md).

- Completed explicit VM lexical captures under the existing per-closure snapshot
  contract, including nested/returned lifetime, captured mutation, mutability,
  imports and callback regressions. This does not introduce shared sibling cells
  or atomic concurrent capture mutation.
- Completed generator/async/spawn Phase A: source audit, 13 characterization
  tests and an updated dependency plan. Completed and merged Phase B: owned generator
  continuation, eager bounded tasks, reusable promise completion, real detached
  spawn and interpreter recursion/namespace fixes. Validation and remaining boundaries
  are recorded in [the completion record](docs/RUNTIME_CONCURRENCY_COMPLETION.md). See the [historical Phase-B handoff](docs/GENERATOR_ASYNC_SPAWN_PHASE_B_HANDOFF.md)
  and [wave-1 review](docs/RUNTIME_HARDENING_WAVE_1_REVIEW.md).

- Completed the producer-neutral failure-control path: versioned result/evidence
  contracts, Dispatch failure policies and safe intervention, pre-evaluation
  Workcell preservation, and an offline failure/review golden path. See
  [implementation record](docs/FAILURE_GATE_IMPLEMENTATION.md) for evidence and
  explicit provider, authentication, replay and recovery boundaries.

- Corrected ecosystem runtime compatibility: Workcell preservation deadlines
  work with its pinned runtime, and Dispatch now checks and pins the source
  runtime that supplies its directory-durability primitive. Added disposable
  database validation and clean-host networking stress coverage; each final
  Linux and macOS stress run passed 50 consecutive host/Docgen rounds.

The September 13 reliability sweep closed the current short-term checklist:

- Rechecked VM/interpreter behavior for closures, lexical scopes, async
  captures, generators, spawn, imports, collections, control flow, and errors.
- Re-ran the host-effect security boundaries for filesystem, archive, process,
  network, HTTP, TLS, database, AI, environment, clock, and random access.
- Removed a race from the source-bound TCP security regression.
- Expanded scheduled fuzzing to every maintained language and bounded-decoder
  target, and added a weekly RustSec dependency audit.
- Rechecked the release installer, native upgrade contracts, npm packages, and
  Kennel's public installer boundary.
- Aligned current docs on Kujo 1.5.0, the live Kennel registry, and the split
  between Kujo's built-in lockfile commands and Kennel packages.

These are continuing release gates, not one-time tasks. Their automated checks
must stay green as the runtime changes.

## Next work and remaining boundaries

The integration is complete; there is no known blocker to those merges. Start
new work from current `main`, not the old Phase-A audit or Phase-B implementation
branches. Completed closure/upvalue and generator/async/spawn work is no longer
an implementation backlog item.

| Priority / status | Work | Completion boundary |
| --- | --- | --- |
| Completed — 1.6 release reconciliation | Publish one reviewed source identity through native archives and runtime npm packages. | Completed for all five supported targets. Exact hashes and public-install gates are in [the release receipt](docs/KUJO_1_6_RELEASE.md). Historical 1.5 artifacts remain unchanged; source pins and runtime versions are distinct provenance fields. |
| Next language maintenance | Improve optional inference for destructuring, module existence checks, struct fields, promises and callable fallback. | Add positive and negative regressions without turning the VM into a static type gate or suppressing genuine annotation errors. The collection-inference bug found during integration is already fixed. |
| Measurement before optimization | Profile generator continuation allocation, retained captures, bounded task admission and scheduling costs. | Comparable measurements and unchanged lifecycle, capability, snapshot and cancellation contracts; no unmeasured performance promise. |
| Unscheduled language candidates | Evaluate async generators, yield-from and struct generator methods; separately assess intentional ownership cycles and explicit atomic shared-state operations. | Design and compatibility review before implementation, followed by cross-runtime, lifetime, capability and concurrency tests. Implicit sibling capture sharing, arbitrary cycle collection, effect rollback and atomic captured read-modify-write are not current guarantees. |
| Owner-deferred ecosystem validation | Live Workcell provider validation. | The operator supplies the provider/profile, account, region, image and spend limit, then runs the provider's real lifecycle and preservation checks. Offline success is not remote certification. This is not a Kujo merge blocker. |
| Open host observation | Investigate the intermittent Intel Mac loopback stall if it recurs. | Capture a reproducible host-level failure and establish its cause. Standard Rust TCP also reproduced it; host load, free disk space and network filters remain hypotheses. Clean-host stress checks passed. Keep the existing observation open without claiming a Kujo defect or a proven repair. |

Workflow follow-ups belong to the ecosystem repositories: authenticated
intervention transports, trusted effect attestations, provider-backed
preservation/materialization and journal archival beyond the current bounded
active journal. Unknown effects and uncertain crash recovery must keep blocking
unsafe replay; there is no general exactly-once or automatic rollback promise.
See [the operational boundaries](docs/FAILURE_GATE_IMPLEMENTATION.md#remaining-operational-boundaries).
These are explicit boundaries and future candidates, not a request to add a
provider-specific policy engine to Kujo core.

## Agentic architecture lane

Kujo's direction is an execution system for agentic software: a language and
runtime for controlled, observable, reproducible execution, composed with the
existing ecosystem. This is an architectural direction, not a next-release promise.
The [source audit and staged plan](docs/NEXT_PHASE_ARCHITECTURE.md) records ownership,
contract reuse, implementation evidence and open boundaries.

| Wave | Current status | Ownership and completion boundary |
| --- | --- | --- |
| A — Observability and Measurement Foundation | Complete end-to-end foundation on ecosystem main; unreleased | Bounded runtime facts → existing Watchdog execution observations → verified RunLedger artifact notes/correlation. Offline HTTP/restart/privacy/integrity proof and all repository gates pass. Optimized baseline/disabled/enabled evidence records short-run synced-export cost; no universal overhead budget. Complete allocator attribution, long-run production tuning, platform/JIT-specific characterization and additional consumers remain non-blocking maintenance. See [completion evidence](docs/WAVE_A_ECOSYSTEM_COMPLETION.md) and [optimized measurements](benchmarks/results/wave-a-release-2026-09-26/README.md). |
| B — Durable Agent Execution | Review checkpoints and bounded retained-host reconciliation implemented; unreleased | Dispatch binds immutable review checkpoints to surviving state/journal and locked v2 continuation. Real offline Workcell/Eval → controller exit → fresh CLI review/resume → RunLedger proof passes without action replay. Lost-store restore, migration and arbitrary mid-action recovery remain unscheduled. See [slice evidence](docs/NEXT_PHASE_ARCHITECTURE.md#wave-b-implemented-slice-and-evidence). |
| C — First-Class Effect Contracts | Beta contract independently validated for bounded local required-policy adoption; alpha retained; opt-in and unreleased | [Validated direction](docs/EFFECT_CONTRACT_DIRECTION.md): Dispatch resolves an additive single-effect assurance document against real SQLite, Workcell Git, and application-owned Ability gateway sinks. Exact subject/result/input/scope/expiry checks preserve existing v1 policy, including the unknown external-idempotent exception. Ability validation separates business/receipt commits and checks authenticated principal, live revocation, expiry and concurrent retries. Default admission is unchanged; the canonical Dispatch compatibility specification defines exact profile matching and downgrade rules. Dispatch now persists immutable mode/profile/configuration revisions, resolves exact installed authority under lock, and rejects downgrade through restart, stale exports and incompatible older readers. Protected state uses a guarded storage codec; bundles without surviving journal authority cannot resume. Portable SHA-256 vectors now agree in Kujo, Node and Python; owner profile specifications and a real three-family alpha/beta migration rehearsal close the scoped beta-review blockers. Distinct beta envelope/profile/configuration/negotiation/controller features preserve alpha runs and reject cross-version protected admission. New beta runs use explicit required/deny policy. A separate Python consumer now reproduces the published commitments, validates all three real profile observations and rejects 126 cross-version/binding/configuration/freshness negatives without importing ecosystem assurance code; scoped adoption rehearsal passed. Beta is preferred for new experimental integrations in this domain; existing alpha runs remain alpha. Broader migration and remote trust remain incomplete. Runtime unchanged. |
| D — Universal Tool and Agent Interoperability | Independent TypeScript/Python adoption and experimental participant SDK prototypes; unreleased | `kujo.interop-handoff/v1alpha1` now has six participant forms across two effect families. Independent TypeScript and Python codecs emit the core directly; published vectors and cross-runtime canonical-wire/correlation cases agree. Both external runtimes use host-owned one-use admission, real Workcell Git pre/post-commit crashes, persisted required/deny review and fresh-controller replay. Dispatch still uses its existing generic reader and live verifier, without a family-specific reader. Historical participant bytes remain unchanged. See Dispatch `docs/audits/wave-d-python.md`. Experimental SDK prototypes now wrap both independent codecs with closed installed registrations, exact-byte correlation and recording-only helpers. Common conformance and real crash/replay remain required; admission/execution/storage stay host-owned. See Dispatch `docs/contracts/participant-sdk/design.md`. Experimental npm tarball and Python wheel/sdist packaging now supports clean external installs, installed-package conformance and real crash/replay. Packages remain alpha and unpublished. Private localhost distribution now verifies exact reviewed SDK/dependency pins before fresh offline installations. New consumer-owned registrations pass distributed-package conformance and real crash/replay. A source-blind agent adopter rehearsal now tests the frozen public onboarding and privately distributed SDK without source coaching; human usability remains unperformed. See `docs/KUJO_1_6_READINESS_REVIEW.md` for the final technical decision and gate evidence. No publication or API freeze is implied. See Dispatch `docs/audits/participant-sdk-distribution.md`. Remote trust, multi-effect and stable promotion remain deferred. Runtime unchanged. |
| E — Structured Context, Memory and Provenance | RAG/Scent/SDK pieces implemented; semantic interop design required, unscheduled | Ecosystem-owned typed information and artifact/evidence/decision provenance; no hidden model reasoning. |
| F — Graph-Native Multi-Agent Execution | Bounded static heterogeneous terminals, branches/subgraphs and explicit attempt accounting; experimental | Ecosystem-owned graphs of compatible agents, programs, tools, evaluators, humans and services, with explicit inputs/outputs, capabilities, budgets, effects and failure policy. |

No wave introduces exactly-once external effects, universal rollback, or automatic
replay of unknown effects. Long-lived review must survive controller termination.
Observability is a prerequisite wave, not a replacement for durable control.

## Coordinated Wave C/D bounded proof · 2026-09-29

Dispatch now has an additive, read-only eight-effect assessment and append-only
freshness/crash proof, plus an independent Go recording participant over CLI/MCP.
Original alpha/beta single-effect admission and historical wire are unchanged.
The next bounded [one-effect admission proof](docs/WAVE_CD_ONE_EFFECT_ADMISSION.md)
now binds one selected SQLite effect to durable authority and a one-use claim.
This is not generalized partial-action scheduling or remote trust. The earlier
[read-only proof](docs/WAVE_CD_EFFECT_SET_PROOF.md) retains its historical scope.

## Parent lifecycle foundation · 2026-09-29

[Bounded parent finalization](docs/PARENT_FINALIZATION.md) separates independent
complete effects from required outputs, Eval, preservation and an explicit terminal
operator decision. Existing Workcell `$ref` preservation binds without changing
historical bytes. Retained-host reconciliation can reconstruct a durable terminal
decision, but cannot invent one from completed prerequisites.
[Bounded Wave F composition](docs/WAVE_F_NODE_COMPOSITION.md) now uses independent
protected node runs and exact typed artifact provenance in the existing Dispatch
DAG. [Heterogeneous terminals](docs/WAVE_F_HETEROGENEOUS_TERMINALS.md) now add
Eval and authorized human decisions without fake execution results, plus a separate
locked graph outcome over immutable required/optional membership. General graph
execution remains incomplete.

## Maintenance lane

Continue optional inference for destructuring, module existence, struct recognition
and field lookup, Promise unwraps, and callable inference/fallback. Maintain
VM/interpreter compatibility, security, fuzzing, release reliability, compiler/VM
optimizations and measured concurrency optimization. Generics, macros, WASM,
FFI, async generators and yield-from remain explicitly deferred and unscheduled.

## Current direction

### Keep the v1 line dependable

Patch releases should favor correctness, security, compatibility, and clear
diagnostics over new syntax. Every change to a stable CLI or JSON contract needs
tests, documentation, and a changelog entry.

Work in this lane includes:

- VM/interpreter parity and closure, scope, async, and generator correctness.
- Bounded filesystem, archive, process, network, HTTP, TLS, and database APIs.
- Cross-platform release, installer, upgrade, and npm-package checks.
- Fuzzing, dependency audits, and regression tests for fixed bugs.

### Make Kujo packages easy to trust

Kujo 1.4 and 1.5 supply isolated imports, file locks, ownership checks, atomic
publication, safe symlink updates, and exact process replacement. Kennel uses
those mechanisms to install packages and commands without hiding the source or
lock state.

The next package work should keep these rules:

- Resolve releases to exact versions and checksums.
- Keep project dependencies local to the project.
- Protect existing global commands and preserve working installs after failure.
- Keep registry reads public and release provenance visible.
- Put package policy in Kennel, not in the Kujo runtime.

Third-party accounts, scoped publishing, and private packages are not current
release promises.

### Improve the language without breaking it

Near-term language work should close known runtime gaps before adding broad new
syntax. The explicit candidates from the v1 scope are:

- measured generator/task allocation and scheduling optimization while preserving
  the [concurrency contracts](docs/RUNTIME_CONCURRENCY_COMPLETION.md);
- better optional type inference for destructuring, imports, struct fields,
  promises, and callable values;
- compiler and VM optimizations that do not change program behavior.

Generics, FFI, a WASM target, and macros remain possible post-v1 projects. None is
scheduled or promised here.

### Keep AI features composable

Kujo core owns deterministic request hashes, record/replay, structured output,
streaming, token estimates, secret redaction, schema validation, vector math, and
explicit AI egress. Provider drivers, agents, retrieval, evaluation, workflows,
and observability belong in packages.

Future work should preserve offline fixtures, stable data shapes, redacted errors,
bounded input and output, and explicit capabilities. Core should not grow a
provider-specific control plane.

### Make the first hour shorter

The install, quickstart, package, and safety paths should stay visible near the
top of the docs. Examples must run on the default VM path unless they clearly say
otherwise. Website and repository docs should agree on the current release,
supported platforms, and the line between Kujo and Kennel.

## Release rules

Before a release:

1. Keep `Cargo.toml`, the README, this roadmap, and release notes on the same
   version.
2. Update tests and docs with every behavior or contract change. Regenerate
   checked-in source inventories when source locations change, even if the
   inventoried TODO or security marker itself is unchanged.
3. Run the full release gate and the affected platform checks.
4. Build from a clean tree and verify published archives and checksums.
5. Keep claims no broader than the shipped artifacts and recorded evidence.

The canonical command is:

```bash
bash scripts/release_gate.sh --full
```

## Final v1.0 Release Checklist

The v1.0 checklist is complete and kept only as launch evidence. See:

- [Official v1.0 release checklist](docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md)
- [v1.0 artifact checklist](docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md)
- [v1 scope and compatibility policy](docs/V1_SCOPE.md)
- [Release process](docs/RELEASE_PROCESS.md)

Git history preserves the former 2,000-line implementation ledger. It should not
be used as the current product roadmap.

### Completed bounded sequential continuation tranche (2026-09-29)

See [the sequential continuation record](docs/WAVE_CD_SEQUENTIAL_CONTINUATION.md)
for append-only rebinding/cancellation, independent next-effect admission, two real
sink families, validation status, and remaining architecture boundaries. This is
additive experimental Dispatch control; Kujo stable runtime scope is unchanged.

### Retained-host operator reconciliation

[The recovery tranche](docs/RETAINED_HOST_RECOVERY.md) connects surviving local
Dispatch authority to bounded inventory, explicit repair planning and locked
mechanical reconciliation. Workcell retention remains separate from replay
permission. Machine migration, remote trust and hostile-storage recovery remain
unsolved. The next architecture slice is parent lifecycle finalization with
explicit output/evaluation/publication requirements, before general graph execution.

[Static Wave F graph policy](docs/WAVE_F_STATIC_GRAPH_POLICY.md) adds declared branch
activation, conditional joins, explicit subgraph terminal decisions and a durable
program-dispatch ceiling. The subsequent [attempt-accounting slice](docs/WAVE_F_GRAPH_ATTEMPT_ACCOUNTING.md) separates reservations, consumption and release and adds explicitly authorized retry after a sealed pre-admission refusal. The subsequent [sourced-resource slice](docs/WAVE_F_RESOURCE_ACCOUNTING.md) adds distinct Eval reservations/consumption, exact runtime and provider-usage evidence, and bounded active-path reservation plans. Nested/conditional groups and general retries remain deferred.
