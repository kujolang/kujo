# Nested collection replacement baseline

The accepted runtime still copies width-dependent storage when an imported helper
replaces a collection inside a nested dictionary. This occurs under dictionary,
struct and array roots. Root-only replacement coverage cannot close this gap.

The [complete baseline](nested-collection-replacement-baseline-2026-10-29.json)
records 96 allocation observations and 144 semantic checks. It covers array and
dictionary replacement values, three root shapes, leaf widths 128/2,048,
initial-only and fresh snapshots, and 16/32 helper calls. VM and JIT added-call
allocation slopes agree. VM, interpreter and JIT preserve original/final contents
and the tested snapshots.

Requested bytes per added helper call with an initial retained snapshot:

| Replacement value | Root | Width 128 | Width 2,048 |
| --- | --- | ---: | ---: |
| Array | Dictionary | 45,552 | 633,072 |
| Array | Struct | 45,633 | 633,153 |
| Array | Array | 45,009 | 632,529 |
| Dictionary | Dictionary | 45,718 | 633,238 |
| Dictionary | Struct | 45,799 | 633,319 |
| Dictionary | Array | 45,175 | 632,695 |

Every tested width increase adds 587,520 requested bytes per added call, including
the fresh-snapshot cases. Fresh snapshots require isolation; this result does not
imply all their copying can be removed.

`probes/nested-collection-replacement-isolation.rb` reproduces the characterization
using a matching allocation probe and CLI. The accepted baseline is runtime
`2c91c49`, with CLI SHA-256
`b71d8c253604b65945d603dab366fe7f58a8897c2489b17dc45f1b57495fc747`.
The report embeds fixture source and executable/runner identities. Independent
verification checked exact case coverage, actual fixture hashes and slope arithmetic.
Use identical fixture paths for future allocation comparisons because path storage
contributes to totals.

This is baseline allocation characterization, not candidate acceptance or an
elapsed-time speed measurement. Deeper nesting, cold/tiny owners, mixed writes,
callbacks and error paths remain part of subsequent fix-specific verification.
