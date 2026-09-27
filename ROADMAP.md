# Kujo Roadmap

Updated: 2026-09-26
Stable release: [v1.5.0](https://github.com/kujolang/kujo/releases/tag/v1.5.0)

> Current crate version: `1.5.0` in [Cargo.toml](Cargo.toml)

Kujo 1.0 shipped on August 8, 2026. The current 1.5 line is stable and adds
bounded, descriptor-confined artifact I/O on top of the package-installer
runtime primitives introduced in 1.4.
This page now tracks what comes next instead of repeating the closed 1.0 plan.

## Where Kujo stands

- `kujo run` uses the bytecode VM by default. The interpreter remains a fallback
  and debugging path.
- The language, CLI, LSP, capability model, AI runtime primitives, and supported
  machine-readable contracts are stable within the v1 compatibility policy.
- Linux x64/arm64, macOS x64/arm64, and Windows x64 release binaries ship with
  SHA-256 checksums. The public npm 1.5.0 runtime package covers the same targets
  with trusted-publisher provenance.
- The official Kennel registry is live at
  [kennel.kujolang.ai](https://kennel.kujolang.ai/). Kennel 1.1.0 manages project
  packages and global tools on macOS and Linux.
- Kujo's built-in `kujo.toml` / `kujo.lock` commands remain separate from
  Kennel's `kennel.toml` / `kennel.lock` workflow.

## Recently completed

- Published `@kujolang/kujo-runtime` and all five native platform packages at
  1.5.0 through npm trusted publishing, with signed provenance and clean-install
  verification across the supported runtime matrix.

- Merged the runtime-hardening baseline into Kujo `main` at
  `9d3c6edeba2b20cb22216816b1a95ee5f24a61b1`
  ([PR #12](https://github.com/kujolang/kujo/pull/12)), with companion
  [Dispatch #1](https://github.com/kujolang/dispatch/pull/1) and
  [Workcell #2](https://github.com/kujolang/workcell/pull/2) merged into their
  respective `main` branches. Final PR and post-merge CI checks passed in all
  three repositories. These are merged source changes, not a new published
  runtime release; the latest stable release remains 1.5.0.

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
| Next release preparation | Package the merged runtime and compatibility changes into a reviewed release; update affected ecosystem runtime pins and distribution channels through their normal release process. | Full release/platform gates, published archives and checksums, and ecosystem clean-install checks against the actual published runtime. Until then, Dispatch's exact source pin remains necessary: tagged native and npm 1.5.0 binaries are built from `cc2d7db`, before `sync_directory_beneath`; current main and the Dispatch source pin also report crate version 1.5.0. See the [artifact/source distinction](docs/NEXT_PHASE_ARCHITECTURE.md#release-reconciliation). |
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
| B — Durable Agent Execution | Review-boundary slice implemented on ecosystem main; unreleased; broader recovery design required | Dispatch binds immutable review checkpoints to surviving state/journal and locked v2 continuation. Real offline Workcell/Eval → controller exit → fresh CLI review/resume → RunLedger proof passes without action replay. Lost-store restore, migration and arbitrary mid-action recovery remain unscheduled. See [slice evidence](docs/NEXT_PHASE_ARCHITECTURE.md#wave-b-implemented-slice-and-evidence). |
| C — First-Class Effect Contracts | Design validated with three local adapter families; opt-in assurance prototype implemented, unreleased; compatibility specification next, broad migration unscheduled | [Validated direction](docs/EFFECT_CONTRACT_DIRECTION.md): Dispatch resolves an additive single-effect assurance document against real SQLite, Workcell Git, and application-owned Ability gateway sinks. Exact subject/result/input/scope/expiry checks preserve existing v1 policy, including the unknown external-idempotent exception. Ability validation separates business/receipt commits and checks authenticated principal, live revocation, expiry and concurrent retries. Default admission is unchanged; remote provenance and compatibility migration remain design work. Runtime unchanged. |
| D — Universal Tool and Agent Interoperability | SDK/MCP/CLI pieces implemented; universal contract proposed, unscheduled | Ecosystem-owned adapters for MCP, tool calling, agent SDKs, A2A, HTTP/OpenAPI, CLI, Python and TypeScript; provider-neutral core. |
| E — Structured Context, Memory and Provenance | RAG/Scent/SDK pieces implemented; semantic interop design required, unscheduled | Ecosystem-owned typed information and artifact/evidence/decision provenance; no hidden model reasoning. |
| F — Graph-Native Multi-Agent Execution | Dispatch DAG foundation implemented; universal typed nodes proposed, unscheduled | Ecosystem-owned graphs of compatible agents, programs, tools, evaluators, humans and services, with explicit inputs/outputs, capabilities, budgets, effects and failure policy. |

No wave introduces exactly-once external effects, universal rollback, or automatic
replay of unknown effects. Long-lived review must survive controller termination.
Observability is a prerequisite wave, not a replacement for durable control.

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
