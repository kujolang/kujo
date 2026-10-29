# Nested dictionary insertion — accepted local checkpoint

Runtime `2c91c49` avoids whole-owner copies for eligible initial scalar insertion
into uniquely owned, nonempty nested dictionaries. The first key remains inline;
proven larger batches use bounded sparse storage. Admission checks the actual
first key, preserving batching when a conditional branch updates an existing key.
Empty leaves, aliases, uncertain paths and unprofitable initial insertions retain
ordinary mutation. Existing-key publication uses replacement without unnecessary
full-table growth. Value semantics and call-frame size are preserved.

## Measured allocation result

VM and JIT agree on these added-call requested-byte slopes (16 versus 32 calls,
no retained snapshot):

| Owner | Width 128, before → after | Width 2,048, before → after |
| --- | ---: | ---: |
| Dictionary | 46,885 → 6,977 | 634,405 → 6,977 |
| Struct | 46,931 → 6,977 | 634,451 → 6,977 |
| Array | 46,299 → 6,891 | 633,819 → 6,891 |

The dictionary control's width-2,048 requested bytes fall about 98.9%. This is
allocation traffic, not an overall elapsed-time speedup or Goose/Rust comparison.

## Verification

- 1,239 library tests passed, eight ignored; both activation-layout tests passed.
- 405 integration tests passed, one profiling test ignored.
- All 50 canonical test-size checksums passed in VM, interpreter and JIT (150).
- 3,360 allocation pairs and 5,040 semantic checks passed across cold, conditional,
  mixed-write, batching, fallback, error, deep-owner and root-insertion controls.
- The nine-invocation insertion contract and six standalone conditional snapshot
  checks passed. Exact test sets, source/executable identities, fixture coverage,
  allocation totals/slopes, formatting and diff checks were independently verified.
- All 54 paired memory observations / 108 executions completed and were verified.

Large unaliased-owner cases show mean peak RSS reductions of 25.82–26.22%.
Empty/retained controls range from -1.724% to +1.523% in mean peak RSS; the largest
positive mean was 501,760 bytes. One retained-array VM pair increased 1,388,544
bytes while the other decreased 1,089,536 bytes. All observations are preserved.
Two local observations per case cannot prove zero memory regression or attribute
retained heap. Acceptance covers correctness and measured allocation behavior;
clean-host performance remains pending.

V1's small-map regression and [V3's conditional regression](NESTED_CONDITIONAL_REJECTION_2026_10_29.md)
remain documented rejections. V4 passes all twelve formerly failing conditional
comparisons with zero extra requests/bytes against the baseline. Repeated controls
compare candidate maxima against baseline minima and conservative added-call slope
bounds; no tolerance was added and no samples were discarded.

[Machine-readable evidence](nested-dictionary-insertion-2026-10-29.json) records
artifact hashes, verification receipts, target slopes, raw memory observations,
limits and the local audit directory containing full allocation reports.

## Remaining sections 3 and 4

1. Collection-valued replacement while retaining observable old collections.
2. Broader compound-index and captured-scalar batching.
3. Capture traversal/materialization and oversized fallback: remove avoidable work
   and document necessary copying.
4. Collection representation review and final combined-candidate verification.

This completes the nested-insertion review cutoff, not sections 3 and 4 as a whole.
No production-size rerun, platform acceptance or clean-host speed claim follows.
