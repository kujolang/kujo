# Compound indexed arithmetic: rejected publication strategy

The first compound-arithmetic candidate is rejected. It broadened eligible
indexed operands and published pending dictionary writes before every binary or
unary struct operator callback. Targeted arithmetic benefited, but callbacks that
did not observe the dictionary paid avoidable copying costs.

Against accepted runtime `30e1511`, the candidate passed 128 target allocation
comparisons and 192 semantic checks. All eight compound target groups had flat
width scaling at widths 128 and 2,048. Requested bytes per added call fell from
roughly 633 KB to 7 KB at width 2,048. These are allocation results, not elapsed
speedups, and did not establish acceptance.

The original operator controls found two additional allocation requests and 43
requested bytes in all 16 comparisons. Call-count isolation attributed that cost
to the first actual operator callback; integer-only controls had no delta. A
nonallocating size histogram reproduced one additional 19-byte allocation and one
24-byte allocation. It did not establish their source-level origin.

Further callback-body isolation exposed larger regressions:

| Callback body | Calls | Additional requests | Additional requested bytes |
| --- | ---: | ---: | ---: |
| Return only | 16 | 45 | 19,425 |
| Write without reading a snapshot | 16 | 75 | 20,070 |
| Read a snapshot | 16 | 0 | 0 |
| Assert only | 16 | 0 | 0 |

VM and JIT results agreed. One-call controls had no delta for these four variants,
and their outputs passed. Unconditional publication therefore imposed repeated
copying even when the callback did not need to observe the collection.

The revised candidate suspends the caller's publication permissions while an
operator body executes. Existing reads and call boundaries determine when pending
writes become observable. The caller's permissions are restored afterward. A
separate regression test covers a callback whose selector replaces the root;
reading the indexed value must preserve the pre-selector object snapshot.

The revision is **not yet accepted**. Its target and callback controls pass,
including zero allocation deltas in all 16 callback-body comparisons. Broad
regression, checksum and memory gates remain necessary. No runtime commit is
represented by this report.

## Reproduction and provenance

- Accepted baseline: `30e1511`.
- Rejected candidate CLI SHA-256:
  `e7da45d2d5a99498a9a59c9bdff9f16c99696c87c54a5224e24869b282301e57`.
- Rejected source and patch:
  `.audit-evidence/compound-batch-2026-10-29/rejected-source/`.
- Target receipt: `candidate-verified.json` in that audit directory.
- Initial rejection: `candidate-rejection.json` in that directory.
- Attribution: `operator-attribution/{call-count,numeric-return,callback-body}/report.json`.
- Revised candidate evidence: `.audit-evidence/compound-batch-v2-2026-10-29/`.
- Callback contracts: test commits `fd2dd6d` and `ba642b4`.

Audit artifacts are local evidence, not published release artifacts. Matching
normal-release dependencies are required for allocation comparisons. The separate
`cargo test --release` dependency graph must not substitute for those artifacts.
