# First task-owner physical-resource review

Eight isolated Linux processes completed on source
`06b4388f75aa0f6329853567a0dfad49d390a6cd`: pooled/unpooled execution of same/distinct
bytecode at 32/1,024 tasks. Exact results, compile/native counters, all sampled
checkpoints, log hashes, invocation/environment and process outcomes verify.
These observations are reviewed below; they do not accept the whole reuse candidate.

## Observations

Deltas below are KiB relative to each process's own warmed baseline. Sampled maximum
means the maximum of checkpoints every 32 tasks, not an operating-system peak.
RSS includes allocator/harness/runtime state; it is not live owner bytes.

| Mode | Code | Tasks | Max sampled RSS delta | RSS delta after drop | Max executable-map delta | Executable-map delta after drop |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Pooled | Same | 32 | 372 | 436 | 8 | 0 |
| Pooled | Same | 1,024 | 332 | 332 | 8 | 0 |
| Pooled | Distinct | 32 | 888 | 888 | 256 | 0 |
| Pooled | Distinct | 1,024 | 8,300 | 8,300 | 4,096 | 0 |
| Unpooled | Same | 32 | 600 | 664 | 0 | 0 |
| Unpooled | Same | 1,024 | 588 | 588 | 0 | 0 |
| Unpooled | Distinct | 32 | 536 | 536 | 0 | 0 |
| Unpooled | Distinct | 1,024 | 700 | 448 | 0 | 0 |

Same-code pooling compiles once and retains one owner; unpooled/distinct paths
compile once per task. Every case executes one native boundary per task. All
pooled checkpoint samples have one idle owner; unpooled samples have zero.

Executable mappings return exactly to the warmed baseline after destruction in
all eight processes. Pooled distinct-code mappings grow then drop during owner
turnover; the observed maximum is 4 MiB. This is not the emitted-code quota expressed
as a physical-memory guarantee: native page allocation and metadata have separate
costs. The test has no sampled two-idle-owner case.

The 1,024-distinct pooled process retains about 8.1 MiB RSS above its own baseline
after pool destruction, while its executable mappings return to baseline. Record
this as a resource cost. It does not identify a leak or prove allocator release
of every live allocation. There is one observation per cell, not repeated matched
sessions, and process-wide RSS is not attributable entirely to the pool.

## Evidence and limits

Run `41638388200`, job `125925298759`, artifact `13232834250` have independently
checked source/run/job/artifact provenance and successful terminal outcomes.
The same revision passes 80 targeted tests (twelve owner, sixty-two integration,
two minimal-feature, four accounting/reset), six membership-verifier controls and
forty async economics cells. These overlap earlier revisions and are not unique
new tests. The later numeric-guard/bytecode-edit regression is outside this revision.

Resource archive verification checks the source commit, successful Cargo build
record and exact test executable path. The executable SHA is CI collector evidence:
its bytes were not uploaded, so no independent off-host rehash is claimed. The
original CI receipt is preserved; archived review writes a separate receipt.
Nineteen synthetic verifier controls pass, including rejection of a failed build
or a different test binary in archive mode. Resource records remain explicitly
review-pending rather than being relabeled optimization acceptance.

`jit-owner-resource-review-2026-11-03.json.gz` retains source/build/provenance,
raw logs, resource/economics receipts, verifiers and controls. Remaining acceptance
includes final-source/platform/all150 checks, elapsed comparisons, and resource
coverage for repeated eviction/lineage and concurrent idle-owner retention. The
provisional pool limits are not changed by this review. No production speed,
clean-host memory reduction or Windows reclamation claim follows.
