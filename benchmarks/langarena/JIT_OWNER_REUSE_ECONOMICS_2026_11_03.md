# Task-owner reuse: compilation work verified

The first actual async reuse economics run passes forty diagnostic cells across
VM, interpreter, JIT and loop-only JIT. Serial jobs compile once per process
instead of the historical baseline's once per job. Outputs, task counts and native
participation are preserved. These are compilation-event savings, not elapsed-speed
or CPU-time gains; observations are instrumented and scheduling-dependent.

| Route | Jobs | Historical baseline compiles | Candidate JIT | Candidate loop-only |
| --- | ---: | ---: | ---: | ---: |
| Direct serial | 16 | 16 | 1 | 1 |
| Parallel, 1 worker | 16 | 16 | 1 | 1 |
| Parallel, 4 workers | 16 | 16 | 4 | 5 |
| Parallel, 8 workers | 16 | 16 | 6 | 7 |
| Parallel, 16 workers | 16 | 16 | 9 | 11 |
| Direct serial | 64 | 64 | 1 | 1 |
| Parallel, 1 worker | 64 | 64 | 1 | 1 |
| Parallel, 4 workers | 64 | 64 | 10 | 10 |
| Parallel, 8 workers | 64 | 64 | 17 | 15 |
| Parallel, 16 workers | 64 | 64 | 16 | 15 |

The historical baseline is the established per-job oracle in
`JIT_ASYNC_ECONOMICS_2026_11_03.md`, not a fresh paired timing run. Every JIT and
loop-only cell records one native boundary entry per job; loop-only cache misses
match compilation counts and hits match jobs minus compilations. VM compiles zero;
interpreter outcomes use exact output rather than unsupported measurement flags.

## Provenance and verification limits

- Tested source: `d9803cb6fffb7189d6d2367ced4fe4182710292b`.
- Run `41636858839`, job `125920131386`, artifact `13232242081`.
- Independently reviewed targeted lifetime/semantic checks: 79 (eleven owner,
  sixty-two integration, two minimal-feature and four accounting/reset).
- Earlier unwind revision `1910a3639f76e3de73867d7644e8fb5a5365340c` also passes
  independent review: run `41636226800`, job `125918030391`, artifact `13231848210`,
  the same 79 targeted tests. Counts overlap; do not add them as unique tests.
- Exact run/head/job/artifact identity, source/host, build results and source-derived
  membership pass. Six membership-verifier controls pass for each artifact.
- Economics review verifies forty exact cells, raw stdout/stderr, measurement
  bytes/hashes, task/native/cache counters, argv/environment and pinned source.
  Generated port bytes match the supplied canonical port hash.
- CI rehashed the executable before/after collection. Its bytes were not uploaded,
  so off-host review preserves its SHA as CI-receipt evidence, not an independent
  executable rehash. Archive verification has a distinct bounded status and does
  not overwrite the original CI verification receipt.
- Six synthetic economics parser controls and five archive controls pass. Archive
  controls reject source drift, port drift, failed outcomes and changed invocation.

`jit-owner-reuse-economics-2026-11-03.json.gz` preserves raw receipts, provenance,
controls and verifier source. Native elapsed speed, resource acceptance, later
bytecode/type regression execution, final-source all150 and platform acceptance
remain open. The candidate is not accepted or merged by this result.
