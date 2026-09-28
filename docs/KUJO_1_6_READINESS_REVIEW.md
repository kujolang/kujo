# Kujo 1.6 technical readiness review

**READY_AFTER_BOUNDED_FIXES.** The source-blind agent adopter rehearsal passed.
A known valid-program VM loop/return defect remains a bounded runtime blocker.
The completed runtime/control and ecosystem work forms a coherent 1.6 scope;
remote trust, API stability and public participant-package publication are not
prerequisites for its explicitly experimental claims.

This is a source readiness review, not a release authorization or verification of
published 1.6 artifacts. Crate/package versions remain unchanged. No tag, npm
package, PyPI package or release was published.

## Agent evidence and its limits

The frozen Dispatch input commit is
`1110ff77accaa5034b6cf0349c6771410d6079d5`. The original 39-file onboarding bundle
was verified against manifest SHA-256
`37ed7e530b573bf0939243cc2c7337d527c1f82238b1ee73ab987b977cfca405`, without regeneration.
A fresh no-history agent used only public frozen documents and exact reviewed
archives in a separate temporary workspace. Isolation was procedural on a shared
host, not a security sandbox. The coordinator independently built the trusted
host; its source knowledge was not used to coach the adopter.

The adopter chose the Node SDK, designed `pilot.lantern` with a closed
`pilot.lantern-correlation/v1alpha1` extension, and required no private implementation
knowledge. It passed 45/45 SDK conformance and 11/11 transport probes. Its recorded
pre-code explanation correctly distinguished participant knowledge, correlation,
host effect execution, live verification and Dispatch replay authority. No public
API or onboarding correction was necessary.

The real Workcell integration passed pre/post-CAS SIGKILL, preserved unknown
participant knowledge with live not_started/committed effect states, persisted
required assurance across fresh controllers and a weaker default, and produced one
logical effect. Four contenders produced one admission and three denials. The
reviewed installed package operated with the private feed stopped.

Decision: **AGENT_REHEARSAL_PASS**. Human adopter validation is **unperformed** and
classified **POST_RELEASE_VALIDATION**. No 1.6 technical claim here requires a
human usability guarantee. Do not describe this result as a human pilot.

Detailed frozen hashes, independent comprehension/read audit/journal, exact package
pins, adopted worker, misuse probes, controller host and evidence are in Dispatch
`docs/audits/source-blind-adopter.md` and
`docs/evidence/source-blind-adopter/`.

## Technical scope

| Area | Coherent 1.6 claim | Boundary |
|---|---|---|
| Runtime/control foundations | Lexical snapshot-compatible closure/upvalue hardening; owned continuations/concurrency; producer-neutral failure control; durable review checkpoints and restart/resume | Fix VM defect below; no general serialized-VM or automatic rollback claim |
| Wave A | Opt-in runtime measurements, Watchdog observation and RunLedger evidence correlation | Numeric measurement limits and recorded overhead remain explicit; observation is not authority |
| Wave C | Three real SQLite/Git/Ability profiles; compatibility/migration; immutable persisted negotiation/config revisions; local live beta verification; independent beta consumer | Beta opt-in required/deny, alpha retained; local trusted-host and single-effect domain |
| Wave D | Six participant forms across two effect families; generic correlation core; independent TypeScript/Python; common SDK conformance; real alpha packages/private distribution; source-blind adopter | SDK only encodes, correlates and records; Dispatch alone admits/replays; no stable/public/remote claim |

These are core foundations plus separately versioned ecosystem companion work.
A Kujo runtime version does not implicitly stabilize or bundle every ecosystem
contract/package. Existing `execution-result/v1` is unchanged; Wave C beta and
Wave D alpha retain their own compatibility/status labels.

## Remaining-item classification

| Classification | Exact item | Required disposition |
|---|---|---|
| MUST_FIX_FOR_1_6 | Valid loop/early-return program causes VM `Stack underflow in return`; interpreter succeeds | Fix with causal analysis and VM/interpreter regression coverage, then canonical runtime gates |
| MUST_FIX_FOR_1_6 | Any failed mandatory release-candidate gate | Resolve concrete failure; do not treat green existing tests as covering the reproducer |
| MUST_FIX_FOR_1_6 | Release-source/version/artifact reconciliation before actual release | Reviewed clean candidate, version/changelog alignment, full/platform candidate gates, built archives/checksums and downstream pins/clean installs against that exact runtime |
| SHOULD_FIX_FOR_1_6 | Duplicate Unreleased sections / leading architecture prose in changelog; superseded chronological next-step language | Consolidate during release prep without rewriting historical evidence |
| SHOULD_FIX_FOR_1_6 | Bounded feature/performance claims | Preserve opt-in status, actual measured overhead, local authority assumptions and human-validation limitation in release notes |
| POST_1_6 | Human adopter usability and additional selected adopters | Collect independent human evidence; never relabel this agent result |
| POST_1_6 | Host framing boilerplate and concrete malformed-wire corpus convenience | Optional ergonomic improvements; no missing normative behavior was required |
| POST_1_6 | Allocator/retained-graph attribution, broader platform/JIT/long-run characterization, optional inference | Separate measured runtime work, no unmeasured guarantee |
| POST_1_6 | Assurance renewal, optional-beta policy, broader adoption, stable participant API | Separate design/review; current alpha/beta remains experimental |
| POST_1_6 | Remote authenticated participant threat model, A2A, broader protocols | Separately scoped next architecture, not a local adoption prerequisite |
| OUT_OF_SCOPE | Public npm/PyPI publication or package-name reservation | No authorization in this rehearsal |
| OUT_OF_SCOPE | Multi-effect assurance, exactly-once, universal rollback, hostile total-store rollback, machine-loss recovery, generic remote attestation | Not claimed by 1.6; no release gate invented for these capabilities |

## Bounded runtime blocker

The independent source reviewer reproduced the pre-existing dictionary comparison
loop with conditional early return on current source. VM exits 4 with
`Stack underflow in return`; interpreter exits 0. Existing integration loops avoid
this path with boolean accumulation; the tested Wave C/D safety predicates do not
require interpreting a runtime crash as a correlation denial.

The suspected source lead is optimizer dead-code elimination/remapping omitting
`ForNext` while handling other branch opcodes. This is a lead, **not a verified
root cause or completed fix**. Do not broaden scope or change language semantics
on that assumption. Add positive matching, early mismatch, loop completion and
nested/control-flow regressions before fixing. Preserve closure/runtime contracts.
Evidence and canonical gate summaries are retained in `docs/evidence/kujo-1.6-review/`.

## Next bounded prompt

Fix the independently reproduced VM loop/early-return defect from
`docs/evidence/kujo-1.6-review/runtime-return-probe.kujo.txt`. Establish the causal
optimizer/compiler/runtime issue, add minimal positive and negative VM/interpreter
regressions, preserve supported semantics, and run full canonical runtime gates.
Recheck the Dispatch release gate against the exact fixed candidate. Do not add
Wave D features, publish packages or stabilize contracts.

After that bounded fix is green, perform 1.6 release preparation: reconcile source
and runtime pins, consolidate roadmap/changelog and documentation truth, bump the
release candidate from 1.5 to 1.6, run complete candidate/platform/artifact gates,
record artifact provenance and clean downstream installs, and prepare reviewable
release/tag/package pins. Keep Wave C beta and Wave D SDK alpha opt-in and
experimental. Publication remains a separate explicit action.

## Local validation record

At baseline `c1cc06ff67e6b8dc1419e5b91adc259891090aad`, `cargo fmt --check`,
`cargo check`, `cargo test` and the named docs/README/CLI/JSON/diagnostics contracts
passed. VM, dual and interpreter fixture sweeps each passed 149/149 with 11 skips;
dual used zero interpreter fallback. Targeted contracts passed 60/60. Exact
commands, exclusions, toolchain, source hashes and log hashes are in
[`results.json`](evidence/kujo-1.6-review/results.json). The separate failing VM
probe is intentionally not represented as a passing canonical gate.

The current-source debug runtime digest is
`29ea1fd65a2aadb160b7ae215c4baf6403db66d8972808cbe885005b3b6c82cf`.
Ecosystem integrations use the previously pinned optimized runtime source
`5d72aab4b99e7f8c01e4c208d6c97061934c7447`, binary digest
`4ef726d0020b6df0be78da4b7e96a79d099d414efa83874676038da501a72a93`.
This distinction matters: green ecosystem fixtures are not a release-artifact
claim for the current debug build. Exact fixed-candidate downstream reconciliation
remains in the next bounded task.
