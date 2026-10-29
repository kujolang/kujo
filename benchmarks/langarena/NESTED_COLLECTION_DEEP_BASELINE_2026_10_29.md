# Deep and small-container collection replacement baseline

The accepted runtime `2c91c49` still copies container-width-dependent storage when
imported helpers replace collections through deep dictionary paths, mixed
struct/array/dictionary paths, and array leaves. This extends the
[earlier nested baseline](NESTED_COLLECTION_REPLACEMENT_BASELINE_2026_10_29.md).
It does not certify the uncommitted root replacement candidate.

The [complete evidence](nested-collection-deep-baseline-2026-10-29.json) contains
192 allocation observations and 288 successful semantic checks. Coverage includes
array/dictionary replacement values, three path shapes, widths 2/2,048,
initial-only/fresh snapshots and 1/2/16/32 calls. VM, interpreter and JIT check full
original and final contents, plus the changed entry in each fresh snapshot.

Requested bytes per added call over the 16-to-32-call interval, retaining an
initial snapshot:

| Replacement | Path | Width 2 | Width 2,048 |
| --- | --- | ---: | ---: |
| Array | Deep dictionary | 9,683 | 635,759 |
| Array | Struct/array/dictionary | 8,248 | 634,324 |
| Array | Array leaf | 6,632 | 284,888 |
| Dictionary | Deep dictionary | 9,849 | 635,925 |
| Dictionary | Struct/array/dictionary | 8,414 | 634,490 |
| Dictionary | Array leaf | 6,798 | 285,054 |

VM/JIT added-call slopes agree. Their **total** allocations differ: JIT uses
90 additional requests / 10,165 bytes at width 2 and 91 / 10,217 at width 2,048.
Those offsets are constant across the tested call counts. An initial verifier
incorrectly required equal totals; its rejection is retained locally. The corrected
verifier checks equal slopes and preserves all total differences in the report.
Future candidate gates must compare each runtime against its own baseline.

`probes/nested-collection-deep-isolation.rb` reproduces the fixtures and measurement.
The evidence records executable identities, source hashes, exact fixture paths,
coverage verification and independently recalculated slopes. Use the original
fixture paths for future comparisons; path storage affects allocation totals.

This is allocation characterization, not elapsed-time evidence. Initial snapshots
must remain isolated; fresh snapshots may require copying on every call.
Zero-snapshot paths, mixed writes, callbacks, errors and alias transitions still
need fix-specific verification. No nested replacement fix is accepted yet.
