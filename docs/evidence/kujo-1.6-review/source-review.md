# Kujo 1.6 source readiness review

Reviewed source: Kujo `c1cc06ff67e6b8dc1419e5b91adc259891090aad`, clean at review start. Crate remains 1.5.0. Last runtime source change is `bdf634fcac43924f573805f30b99ee2646d4b58e`. This is source readiness, not verification of a published 1.6 artifact. The independent adopter rehearsal is owned separately; this review does not substitute source inspection for that result.

Supporting checkout baselines: Dispatch `1110ff77accaa5034b6cf0349c6771410d6079d5`; Workcell `1940da0639b70b702c1b1077dda51ca39b065216`; Ability `d6c970785f8d8bea04de0dce37920c2d0ca1c067`; Agents SDK `af0aa28f5960232cbafa7cf528a32db8cb36c7a9`; Watchdog `e18e437afb221a597d48295b432b0ff356931158`; RunLedger `97cb607a062efeecc7fcdc4e4a4e0308f90d45fe`.

## Recommendation

**READY_AFTER_BOUNDED_FIXES**, conditional on the separate source-blind agent adopter and real host integration passing. A known valid-language loop/return VM defect must be fixed and regression-tested before declaring runtime 1.6 ready. Existing assurance paths have a tested workaround; this is not evidence that Wave C/D contracts are broken. Do not reopen completed architecture slices or require post-1.6 features to close this bounded runtime blocker. Normal release preparation and actual artifact verification remain necessary even after it is fixed.

## Readiness matrix

| Area | Current source / bounded claim | Classification and required action |
|---|---|---|
| Closure/upvalue hardening | Definition-site indexed captures; per-closure snapshots, alias sharing within one closure, owned nested/returned lifetime; no sibling sharing or atomic read-modify-write. `src/compiler.rs`, `src/vm.rs`, `tests/closure_capture_{audit,contracts}.rs`, `docs/CLOSURE_UPVALUE_IMPLEMENTATION.md`, `docs/RUNTIME_HARDENING_WAVE_1_REVIEW.md`. | Include completed foundations in 1.6. MUST_FIX_FOR_1_6: valid loop-return VM defect below, without redesigning captures. |
| Generator/async/spawn and compatibility | Owned continuation, bounded eager tasks, reusable promises, real detached spawn; documented cancellation/snapshot limits. `docs/RUNTIME_CONCURRENCY_COMPLETION.md`, `docs/RUNTIME_COMPATIBILITY_FOLLOWUP.md`. | Include shipped-source changes. POST_1_6: async generators, yield-from, intentional-cycle collection, broader optional inference. |
| Failure control | Producer-neutral schemas in Kujo; Dispatch owns authorization/retry and Workcell preservation. Unknown effects fail closed; evaluation errors remain distinct. `schemas/workflow-control`, `docs/FAILURE_GATE_IMPLEMENTATION.md`. | Include foundation and bounded ecosystem integration, contingent on current integration gates. OUT_OF_SCOPE: provider-specific policy engine in core, automatic rollback, exactly-once effects. |
| Durable checkpoint/restart | Quiescent review checkpoint, immutable state/evidence bindings, locked v2 continuation and surviving authoritative store/journal. `docs/NEXT_PHASE_ARCHITECTURE.md` Wave B; Dispatch `docs/review-checkpoints.md`. | Include bounded review-boundary restart evidence. POST_1_6: lost-store restore, arbitrary mid-action recovery, archival/migration. No serialized-VM claim. |
| Wave A | Opt-in bounded numeric VM report; unchanged ordinary output; real Watchdog HTTP/persistence/restart and RunLedger integrity/privacy/null-provenance integration. `tests/runtime_measurements.rs`, `docs/WAVE_A_ECOSYSTEM_COMPLETION.md`. | Include completed foundation. POST_1_6: allocator/retained-graph attribution, long-run/contended/platform/JIT characterization, additional consumers. |
| Wave A measurements | Optimized comparable campaign records +16.95% short-call / +8.95% short-async paired enabled overhead, much smaller sustained samples, synced export fixed cost. `benchmarks/results/wave-a-release-2026-09-26/README.md`. | SHOULD_FIX_FOR_1_6: retain concrete limitations in release messaging. POST_1_6: buffered export experiment; no universal overhead guarantee or unmeasured performance claim. |
| Wave C three real profiles | SQLite, Workcell Git, Ability application-owned gateway; same existing execution-result/v1 and conservative replay policy. | Include only local single-effect experimental scope. OUT_OF_SCOPE: universal effect contracts, remote trust, global enablement, multi-effect assurance. |
| Wave C compatibility/persistence | Exact version/profile/config revision, immutable journal authority and locked live verification; alpha retained. Dispatch `docs/effect-assurance-compatibility.md`, `docs/persisted-assurance-negotiation.md`. | Include completed bounded protection; no hostile total-store rollback guarantee. |
| Wave C beta / independent consumer | Portable Kujo/Node/Python commitments, three owner specifications, alpha/beta migration, separate Python consumer with 58 vectors and 126 negative decisions. Dispatch `docs/audits/portable-beta-rehearsal.md`, `docs/audits/independent-beta-adoption.md`. | Completed prior beta blockers are superseded; beta remains opt-in required/deny/unreleased. POST_1_6: renewal, optional-beta policy, wider migration/stable promotion. |
| Wave D participants/core | Six forms across two effect families; independent native TypeScript/Python handoffs, generic correlation reader; historical bytes retained. Dispatch Wave D audit documents. | Include experimental interoperability evidence. OUT_OF_SCOPE: remote participant authentication, A2A or universal execution lifecycle. |
| Wave D SDK/packaging/distribution | Independent pure codecs/correlation and recording helpers, closed installed registration; no effect execution or replay authority. Experimental npm alpha tarball and Python alpha wheel/sdist; private localhost distribution has exact reviewed pins and offline dependency closure. Dispatch `docs/contracts/participant-sdk/design.md`, `docs/audits/participant-sdk-{packaging,distribution}.md`. | Include experimental bounded claim conditional on new adopter result. MUST_FIX_FOR_1_6 only if new adopter uncovers a defect material to the claim. POST_1_6: human usability pilot/API freeze. Public npm/PyPI publication is a separately authorized release action, not a technical readiness prerequisite. |
| Human usability | No human-only onboarding test performed. Agent source-blind adoption cannot prove human usability. | POST_1_6 for a bounded technical experimental claim. MUST_FIX only if release text promises verified human onboarding/ease of use; remove unsupported claim or obtain that evidence. |
| Release mechanics | v1.5 tag source `cc2d7db` lacks directory sync; source and public binary can both report 1.5.0. | MUST_FIX_FOR_1_6 before actual release: version/changelog alignment; clean reviewed candidate; full/platform gates; build and verify archives/checksums; downstream pins/clean installs against actual published runtime. These do not require further architecture implementation. |
| Documentation | Changelog has leading architecture text and duplicate Unreleased sections; chronological architecture notes retain superseded next steps. | SHOULD_FIX_FOR_1_6 during release prep: one coherent release section, bounded features, current authoritative status links, historical labels. |

## Exact bounded runtime blocker

Source reproducer is copied unchanged from Dispatch `docs/audits/evidence/persisted-negotiation/runtime-return-probe.kujo.txt` into `runtime-return-probe.kujo` beside this report. It compares a matching dictionary's six fields inside a for loop and returns `{ok:true}` after the loop; a conditional early return exists for mismatch. Default VM exits 4 with `Stack underflow in return`; interpreter exits 0 with `{"ok":true}`. This is ordinary supported syntax, independent of Dispatch effects. Existing SignalBox provenance is `cap_c71a1513-fc5a-493f-aec6-4d8585aa0e99`; do not create a duplicate.

Source lead, not a completed causal proof: `src/optimizer.rs::dead_code_elimination_pass` and `mark_reachable` handle other branch opcodes but omit `ForNext` in remapping/branch reachability. The reproducer reports three dead instructions removed. A fix needs positive and mismatch branches, loop completion, nested loop/control-flow and VM/interpreter regression coverage; then canonical gates. No source was modified during this review.

## Validation

Fresh local gates and current-source binary hashes are recorded separately in this evidence directory. Toolchain: Rust/cargo 1.96.0, Darwin x86_64; default features and ordinary dev/test profile. This review does not claim cross-platform, public artifact, hosted CI or human usability verification.

### Fresh gate receipt

| Command | Exit / result | Log |
|---|---|---|
| `cargo fmt --check` | 0 | `fmt.log` |
| `cargo check` | 0 | `check.log` |
| `cargo test` | 0; complete test/doc-test traversal | `test.log` |
| `cargo run -- test --runtime vm` | 0; 149 passed, 11 explicit skips | `vm.log` |
| `cargo run -- test --runtime dual` | 0; 149 VM-primary passed, 11 skips, 0 interpreter fallback | `dual.log` |
| `cargo run -- test --runtime interpreter` | 0; 149 passed, 11 skips | `interpreter.log` |
| named docs/README/CLI/JSON/diagnostics contracts | 0; respectively 6/1/32/19/2 passed (60 total) | `targeted-contracts.log` |
| Minimal return probe, VM | 4; Stack underflow in return | `return-current-vm.log` |
| Minimal return probe, interpreter | 0; `{"ok":true}` | `return-current-interpreter.log` |

All gates used current checkout runtime source; no runtime or Cargo changes occurred. The root agent concurrently modified ROADMAP.md during review. Existing vendored tiny_http warnings were retained. Binary reports `kujo 1.5.0`, SHA-256 `29ea1fd65a2aadb160b7ae215c4baf6403db66d8972808cbe885005b3b6c82cf`. Existing gates passing does not close the separately reproduced valid-program VM defect.

The full release wrapper (including Clippy/audit), platform matrix, published artifacts and hosted CI were not run in this bounded source review. They remain explicit release-preparation requirements, not alleged passes. Root owns independent adopter and ecosystem integration results. No subagents were used by this reviewer and no repository files were changed.
