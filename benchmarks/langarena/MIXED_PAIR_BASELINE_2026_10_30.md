# Mixed two-write array baseline

Accepted runtime `94ad8f5` passes 768 allocation observations and 1,152 semantic
checks across 12 ordered payload pairs, widths 2/3/8/2,048, once/fresh snapshots
and 1/2/16/32 calls. VM/JIT allocation slopes agree in every group; semantic
checks also run in the interpreter.

With one initial snapshot, every pair retains a 278,256-byte owner-width penalty
per added call between widths 2 and 2,048. Reversing either pair produces the same
measured slope:

| Payload pair, either order | Width 2 bytes/call | Width 2,048 bytes/call |
| --- | ---: | ---: |
| Scalar / array | 5,909 | 284,165 |
| Scalar / dictionary | 6,075 | 284,331 |
| Array / dictionary | 6,659 | 284,915 |
| String / array | 5,954 | 284,210 |
| Bytes / array | 5,909 | 284,165 |
| Struct / scalar | 6,070 | 284,326 |

All helpers receive the same numeric and byte arguments, including cases that
do not use the byte argument. Future comparisons must reuse these exact fixtures;
their fixed overhead is not interchangeable with earlier one-argument baselines.
Fresh-snapshot cases remain separate because their detachment can be necessary.

The existing compact array journals cover numeric scalar pairs and same-kind
array/dictionary pairs. General batch admission requires more than two writes.
Mixed two-write storage is the next confirmed optimization target. Any new storage
must preserve the capture-frame layout, small-owner costs, snapshots, repeated
indices, error publication and callback visibility; it must not replace the cheap
existing homogeneous paths unconditionally.

No runtime change accompanies this baseline. Allocation observations do not
establish elapsed-speed improvement or close sections 3 and 4.

[Evidence](mixed-pair-baseline-2026-10-30.json) includes raw outputs, exact source
and artifact hashes, fixtures, recomputed slopes and the independent verifier.
