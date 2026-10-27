# Foreign callback local-prefix transfer — 2026-10-27

Runtime `65a58f2` transfers private suspended capture maps for eligible synchronous
foreign callbacks whose globals and locals belong to the same live owner. It
restores those maps after the call while public values stay owner-backed. Split
owners, task routes and unproven ownership retain synchronization. Pool limits
and value semantics are unchanged.

Baseline runtime: `ce923d1d2668a593f890523cd6aa91fe37a2b17d`.
[Machine evidence](foreign-local-prefix-transfer-2026-10-27.json) includes source
manifest, compiler/build arguments, artifact hashes, paired measurements,
semantic outputs, checksum results and both rejected attempts.
Candidate binary SHA-256:
`65e3cd41c23db3207a466f68e59ae27846d824915ca394260b71f7dc236a8447`.
The local `target/release/kujo` from the rejected pool experiment is **not** this
candidate; use the recorded archived artifact or rebuild the committed source.

## Measured result and acceptance correction

The original exact-equality gate **failed** across the existing pool boundary.
Its failed state is preserved. Expanded measurements at 4,096, 8,192, 12,288
and 16,384 locals established constant extra recursion allocation within each
regime. The revised bounded criterion is explicit; it does not claim that the
original criterion passed or that the boundary cost disappeared.

Extra allocation per outer call after subtracting depth-zero activation:

| Retained locals | Read depth 1 requests / bytes | Read depth 4 | Write depth 1 | Write depth 4 |
|---|---:|---:|---:|---:|
| 0 / 128 / 2,048 / 4,096 | 72 / 4,548 | 298 / 20,112 | 78 / 4,639 | 322 / 20,476 |
| 8,192 / 12,288 / 16,384 | 84 / 6,562 | 308 / 21,742 | 90 / 6,653 | 332 / 22,106 |

VM and JIT allocation deltas agree. Combined reports contain 288 paired
observations and 432 semantic runs, with 252 unique allocation cases and 378
unique semantic cases because 8,192 locals were repeated. All 28 depth-four
groups strictly reduce requests and bytes; no tested total or added-call
allocation increases. At 8,192 locals, depth-four writes fall from 41,370
requests / 13,621,011 bytes to 8,566 / 2,743,383 per outer call. The remaining
outer activation still scales with retained names.

An attempted pool bypass removed part of the boundary cost but introduced 24
direct-path total-allocation regressions: +2 requests / 384 bytes at depth zero
and +4 / 768 at depth two. It was rejected and reverted. The no-regression gate
was retained. A synthetic unchanged-baseline control rejects width-growing
cost under the bounded gate; this is a verifier self-test, not a new runtime run.

## Verification

- 1,179 library tests; 21 targeted integration cases.
- 348 matching release integration cases; one manual profiling test ignored.
- Six new VM/interpreter/JIT regression contracts cover direct foreign callbacks
  and same-module bounce routes, retained locals, aliases, independent closures,
  snapshots, mutable/constant arrays, caught errors and foreign-module state.
  Validation counters prevent swallowed descendant assertion failures.
- 18 expanded baseline parity cases pass.
- Read, numeric-key, module, nested, direct recursion, ordinary local, cold,
  append and publication controls pass without allocation regressions.
- All 50 test-size checksums pass independently in VM, interpreter and JIT.
- Formatting, diff checks and matching frozen source identity pass.

## Reproduction and remaining work

Use the original `probes/indirect-local-allocations.rb` and the new
`probes/foreign-local-pool-boundary-allocations.rb`, each with matching candidate
probe, candidate binary, fresh output directory and baseline probe. Then run:

```sh
ruby benchmarks/langarena/probes/foreign-local-pool-boundary-gate.rb \
  ORIGINAL_REPORT_JSON EXTENDED_REPORT_JSON NEW_ACCEPTANCE_JSON
```

Keep `foreign-local-prefix-gate.rb` unchanged as the original rejected criterion.
Run the independent controls described in [gate documentation](probes/INDIRECT_LOCAL_GATES.md).
Machine evidence embeds the reports, identities and synthetic negative control.

This closes eligible same-owner foreign reentry copying only. Split-frame
reentry, oversized outer activation, broader captured environment and collection
costs remain on the checklist. These are allocation-traffic measurements, not
peak-memory or elapsed-speed claims. Production-size coverage and controlled
Goose/Rust comparisons remain pending. No Kostya contact occurred.
