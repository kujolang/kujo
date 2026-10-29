# Direct array collection publication

Runtime f415211 retains eligible single-index collection replacements until
publication, preserving value semantics, snapshots, index evaluation, callbacks,
immutable bindings and errors. Array/dictionary payloads retain their existing
Arc handles. Larger payloads use bounded storage only for a direct tail assignment
with no later publication boundary; other bodies retain ordinary mutation.
Permissions share the existing flag byte, with unchanged-layout tests retained.
Capture-map refill also clears deleted-bucket markers after singleton projections
by moving the surviving owned key/value through drain and reinsertion. The
regression test checks retained key storage, values and table capacity.

Requested bytes per added helper call, width 2,048, 16-to-32 calls, VM repeat case:

| Payload | Before | After |
| --- | ---: | ---: |
| array | 282472 | 3904 |
| dictionary | 282638 | 4070 |
| struct | 290821 | 4089 |

All 18 target groups (three payloads, repeat/shared/argument forms, VM/JIT) have
flat hot-call slopes across widths 2/2,048 and strictly lower requested bytes at
width 2,048. These are allocation measurements, not elapsed-time speed claims.

Verified against accepted runtime 3ac637e:

- 1254 library tests passed; 8 ignored.
- 409 integration tests passed; 1 ignored.
- All 150 test-size checksums across VM, interpreter and JIT.
- 1232 allocation pairs / 1848 semantic checks, including historical dictionary/nested controls, without total or slope regressions.
- 54 paired memory observations / 108 executions reviewed.
- 15 additional scalar/compound contracts across all three runtimes, covering dynamic operators, failed writes and selector snapshot ordering.
- Exact source/artifact identities, test names, fixture/configuration coverage, formatting and diff checks.

Memory assessment: Reviewed all 54 pairs across 27 groups. At width 32768 without retained snapshots, all nine payload/runtime groups have lower mean peak RSS by 4366336 to 6299648 bytes (9.97% to 13.11%). Small and retained-snapshot cases have mixed differences: five groups have positive means, with the largest +198656 bytes (+0.333%) for retained dictionary payloads in VM; the largest individual increase is +266240 bytes. These increases are retained and are not declared zero or proven measurement noise. They are small in these samples; source review shows bounded one-entry publication storage and no new persistent retention, while all allocation and ownership gates pass. Accept this candidate with those measured limitations. Two alternating local process-RSS observations per case do not establish retained-heap bounds, leak absence or clean-host memory/speed superiority.

[Full observations](root-array-collection-publication-2026-10-29.json) retain all
results and remaining measured width costs. Multiple-index updates and broader
boxed-payload helper bodies are not closed by this change. Some observed copies
are required for real snapshots; others need further isolation and optimization.
Compound/captured-scalar batching, capture materialization and final representation
review remain open. Sections 3/4 are not complete. Production-size, platform and
clean-host competitive acceptance remain separate.
