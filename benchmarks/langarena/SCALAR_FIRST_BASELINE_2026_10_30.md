# Scalar-first captured array batch isolation

Accepted runtime `18ebd21` passes 96 allocation observations and 144 semantic
checks across VM, interpreter and JIT. The same three final values are written
with either scalar-first or collection-first ordering; a scalar-only control
isolates the compact scalar journal limit. Original and fresh-per-call snapshots
are checked separately, including every untouched array element.

Requested bytes per added helper call with one original snapshot; VM/JIT agree:

| Write sequence | Owner width 64 | Owner width 2,048 |
| --- | ---: | ---: |
| Scalar, array, dictionary | 14,759 | 284,583 |
| Array, dictionary, scalar | 6,251 | 6,251 |
| Three scalars | 13,425 | 283,249 |

Both scalar-first cases retain a 269,824-byte owner-width penalty. Cold one/two
call observations and fresh-snapshot controls are retained in the evidence; the
table uses the 16-to-32-call slope and does not claim elapsed-time performance.

Source inspection identifies two admission boundaries: collection batch storage
can promote existing collection journals but not the compact scalar journal;
the batch proof also requires at least one collection payload, leaving three
scalar writes on ordinary fallback. A correction must preserve the existing
one/two-entry compact path, snapshot publication, error ordering, and bounded
admission. Small-container and cold costs must be checked before acceptance.

This report reproduces the next gap; no runtime correction is claimed.
[Complete evidence](scalar-first-baseline-2026-10-30.json) preserves fixtures,
raw results, artifact hashes and the exact-matrix verification implementation.
