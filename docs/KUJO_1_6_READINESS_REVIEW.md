# Kujo 1.6 technical readiness review

**KUJO_1_6_RC_READY** for the locally built/tested macos-x64 candidate. See the
[release-candidate review](KUJO_1_6_RC_REVIEW.md) for exact source, artifacts,
downstream pins and authorization boundaries. The earlier technical readiness
decision was READY_FOR_1_6_RELEASE_PREP. The source-blind agent adopter rehearsal passed.
The valid-program VM loop/return blocker is now fixed and verified against the
exact optimized runtime and the full Dispatch release gate.
The completed runtime/control and ecosystem work forms a coherent 1.6 scope;
remote trust, API stability and public participant-package publication are not
prerequisites for its explicitly experimental claims.

This is a source readiness review, not a release authorization or verification of
published 1.6 artifacts. Source crate/npm metadata now identifies the unpublished
1.6.0 candidate; 1.5.0 remains the latest published stable release. See the
[1.6 candidate notes](RELEASE_NOTES_1_6.md) and
[version truth audit](RELEASE_1_6_TRUTH_AUDIT.md). No tag, npm package, PyPI package
or release was published.

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
| Runtime/control foundations | Lexical snapshot-compatible closure/upvalue hardening; owned continuations/concurrency; producer-neutral failure control; durable review checkpoints and restart/resume | VM defect fixed with causal regressions; no general serialized-VM or automatic rollback claim |
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
| COMPLETED_LOCAL_RC | Release-source/version/artifact reconciliation | 1.6.0 source, exact optimized macos-x64 archives/checksums, clean installs and full Kujo/Dispatch/Workcell gates; see RC receipt. Other platform builds remain explicit release work |
| COMPLETED_LOCAL_RC | Changelog and release-note truth | One Unreleased section; explicit unpublished 1.6 section; historical evidence and experimental status retained |
| COMPLETED_LOCAL_RC | Bounded feature/performance claims | Release notes retain opt-in status, measured limits, local authority and unperformed human validation |
| POST_1_6 | Human adopter usability and additional selected adopters | Collect independent human evidence; never relabel this agent result |
| POST_1_6 | Host framing boilerplate and concrete malformed-wire corpus convenience | Optional ergonomic improvements; no missing normative behavior was required |
| POST_1_6 | Allocator/retained-graph attribution, broader platform/JIT/long-run characterization, optional inference | Separate measured runtime work, no unmeasured guarantee |
| POST_1_6 | Assurance renewal, optional-beta policy, broader adoption, stable participant API | Separate design/review; current alpha/beta remains experimental |
| POST_1_6 | Remote authenticated participant threat model, A2A, broader protocols | Separately scoped next architecture, not a local adoption prerequisite |
| OUT_OF_SCOPE | Public npm/PyPI publication or package-name reservation | No authorization in this rehearsal |
| OUT_OF_SCOPE | Multi-effect assurance, exactly-once, universal rollback, hostile total-store rollback, machine-loss recovery, generic remote attestation | Not claimed by 1.6; no release gate invented for these capabilities |

## Completed runtime blocker

**VM_BLOCKER_FIXED.** The original probe and ten minimized language-level cases
now produce identical VM/interpreter output. The optimizer omitted `ForNext`
exhaustion from reachability and target relocation; instruction deletion left the
historical loop jumping directly to `Return` instead of constructing its result.
Constant-folding and peephole deletion shared the same address-preservation defect.
The fix is confined to optimizer control-flow preservation; no Return check,
interpreter behavior, iterator cleanup or closure/generator/task semantics changed.

Permanent coverage includes eleven seeds, six optimizer/runtime contracts, an
aggregate VM/interpreter/dual fixture and a parser fuzz seed. All canonical Kujo
gates passed: 2,856 Rust tests (17 existing ignored), 232 targeted contracts and
150/150 fixtures in each runtime (11 existing skips, zero dual fallback).
The full Dispatch release gate, separate Workcell assurance gate and automated
installed-package adopter integration passed against the exact optimized fix.

See [causal analysis, commit map and full evidence](VM_LOOP_RETURN_FIX.md) and
[gate/provenance manifest](evidence/vm-loop-return-fix/results.json). No known
technical MUST item remains from this blocker. That milestone left release
source/version/artifact reconciliation, now completed for the local RC. Other
platform verification and publication remain separately scoped release actions.

## Next bounded release action

Review the [RC provenance and checklist](KUJO_1_6_RC_REVIEW.md). Authorize the
intended release scope, complete remaining platform builds/gates, and separately
authorize tag/release/upload/publication at the recorded tested source. Do not tag
the subsequent evidence-receipt commit. Published install defaults should advance
only when actual 1.6 publication exists. Participant SDK publication is separate;
Wave C beta and Wave D alpha remain experimental. Human usability is unperformed
post-release validation, not an engineering blocker.

## Historical local validation record

At baseline `c1cc06ff67e6b8dc1419e5b91adc259891090aad`, `cargo fmt --check`,
`cargo check`, `cargo test` and the named docs/README/CLI/JSON/diagnostics contracts
passed. VM, dual and interpreter fixture sweeps each passed 149/149 with 11 skips;
dual used zero interpreter fallback. Targeted contracts passed 60/60. Exact
commands, exclusions, toolchain, source hashes and log hashes are in
[`results.json`](evidence/kujo-1.6-review/results.json). The separate failing VM
probe is intentionally not represented as a passing canonical gate.

The baseline debug runtime digest was
`29ea1fd65a2aadb160b7ae215c4baf6403db66d8972808cbe885005b3b6c82cf`.
The earlier ecosystem integrations used the pinned optimized runtime source
`5d72aab4b99e7f8c01e4c208d6c97061934c7447`, binary digest
`4ef726d0020b6df0be78da4b7e96a79d099d414efa83874676038da501a72a93`.
Those are historical baseline artifacts, not the fixed candidate. The new
optimized source is `db4608c1df59668ff9c89956ae4ef1ab18e86f14`, binary SHA-256
`74f42acdf3470ea73cd770f3ac2eae281c1bfe3609c3db5e2cfa8c197e523685`.
Exact fixed-candidate downstream reconciliation is complete in the linked evidence;
final release/platform artifacts still belong to release preparation.
