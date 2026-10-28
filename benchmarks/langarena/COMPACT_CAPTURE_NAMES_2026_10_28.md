# Compact captured-name storage — verified allocation reduction

The candidate packs private captured-local binding names into one owned string slab,
entry array and randomized lookup table at the existing oversized admission boundary.
Ordinary environments retain their standard HashMap representation. Public owner
maps, value snapshots and pool limits are unchanged; insertion/removal paths can
materialize ordinary storage when needed. Layout and empty-global pool-boundary
regressions protect against increasing enclosing storage or retaining oversized maps.

Accepted baseline: runtime c83ee4b, evidence commit 7f44985. Candidate runtime commit: `b4dfdfdafbd8c4b298793d83d464c1c70da467c9`. Exact source, library, executable, probe and runner identities are retained
in the audit manifests and final assembled evidence.

Measured per added outer call with 8,192 captured names:

| Operation | Baseline requests | Candidate requests | Baseline requested bytes | Candidate requested bytes |
| --- | ---: | ---: | ---: | ---: |
| Read | 8,239 | 49 | 624,208 | 313,012 |
| Write | 8,240 | 50 | 624,215 | 313,019 |

VM and JIT agree. The target covers widths 0/128/2,048/4,096/8,192/12,288/16,384,
read/write calls, recursion depths 0/1/4 and call counts 1/16/32. All 12 oversized
outer-activation groups reduce both requests and bytes. Total and per-call allocations
and recursion overhead do not regress in the target; existing pool regimes stay intact.
288 allocation observations and 432 semantic runs include the repeated width 8,192
cross-check (252 and 378 unique cases). Dense controls add 12 pairs and 18 semantic runs.

Current confirmed correctness gates: 1,226 library tests passed, eight existing ignores;
400 integration tests passed, one manual profiling ignore; the new regression passes
on baseline and candidate in all three runtimes; all 150 test-size LangArena checksum
runs passed. The new regression checks mutable/let/const names, independent closures,
aliases, value snapshots and writes before exceptions. Independent target identity
verification passed, and unchanged baseline data is rejected by the improvement gate.

All 22 prior control report groups passed, including collection, dictionary, callback,
cold-setup and ownership checks. Eight append comparisons and module boundary checks
passed. Independent evidence assembly verified artifact identities, fixture coverage,
diagnostic parity and checksum configurations. See [full evidence](compact-capture-names-2026-10-28.json).

These figures are allocation traffic, not retained memory or elapsed speed. They do
not establish competitive Goose/Rust scores on the busy local machine. Broader capture
traversal/materialization, collection costs, production-size runs, platform gates and
clean-host comparisons remain separate work; sections 3 and 4 are not closed here.

Verification setup note: the initial append check lacked its imported helper fixture.
The baseline probe stopped before measurement; the exact baseline helper was restored
and only unfinished checks were resumed. The failure and repair are preserved in the evidence.
