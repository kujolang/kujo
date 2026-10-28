# Deeper collection isolation — 2026-10-28

Accepted runtime: `a5d07bb`. This is baseline characterization, not a runtime fix.
[Machine evidence](deep-collection-isolation-2026-10-28.json) preserves fixtures,
artifact identities, exact coverage, measurements and independently recomputed deltas.

The matrix covers dictionary, struct and array roots; array/dictionary leaves;
paths with two/four accesses; widths 128/2,048; one/eight repeated writes to the
same leaf key per helper call; initial-only/fresh snapshots; and 16/32 calls.
There are 384 VM/JIT allocation measurements and 576 VM/interpreter/JIT semantic
runs. All semantic checks pass, including every untouched leaf value and the
original snapshot. VM/JIT allocation deltas agree.

## Measured costs

Requested bytes per added VM call, dictionary root, initial-only snapshot:

| Leaf | Path accesses | Writes | Width 128 | Width 2,048 |
| --- | ---: | ---: | ---: | ---: |
| Array | 2 | 1 | 4,807 | 4,807 |
| Array | 4 | 1 | 24,868 | 285,988 |
| Array | 4 | 8 | 30,993 | 292,113 |
| Dictionary | 2 | 1 | 4,807 | 4,807 |
| Dictionary | 4 | 1 | 46,652 | 634,172 |
| Dictionary | 4 | 8 | 52,777 | 640,297 |

Across all three root types, the deeper array width penalty is 261,120 bytes per
added call; the deeper dictionary penalty is 587,520. Both penalties remain the
same for one and eight writes. This supports one width-dependent copy per helper
invocation, not one full copy per element write. Shallow controls remain flat.
Fresh snapshots are measured separately; their required copies are not defects.
These are requested-allocation measurements, not elapsed speed or retained memory.

## Next acceptance requirements

Deferred publication currently admits a single owner selector before the leaf,
and its fast reads recognize that same shallow shape. Deeper updates need both
safe publication and read handling. Extending only admission risks materializing
a pending snapshot on subsequent reads.

Any fix must preserve unique/shared owners, independently aliased intermediate
containers, original/fresh snapshots, numeric/string/negative index behavior,
root replacement, callbacks, errors and evaluation order. Reject regressions in
shallow paths and escape cases. Verify matching release artifacts and all three
runtimes before accepting a runtime change. Depth two/four is measurement coverage,
not a proposed language depth limit. Larger row batches and broader capture costs
remain separate open work; sections 3/4 are not complete.
