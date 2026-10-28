# Collection costs after array-row publication — 2026-10-27

The unchanged ten-shape matrix was rerun against accepted runtime `1c3acd4`.
All 240 allocation measurements and 360 semantic checks pass; an independent
verifier checks exact coverage, fixture/artifact identities and recomputed deltas.
[Machine evidence](collection-gap-after-array-2026-10-27.json) retains the data and verifier.

Requested bytes per added helper call, VM, initial snapshot retained:

| Shape | Width 128 | Width 2,048 |
| --- | ---: | ---: |
| dict_one_int | 4382 | 4382 |
| dict_two_int | 43657 | 631177 |
| dict_one_float | 4382 | 4382 |
| struct_row_one | 4802 | 4802 |
| struct_row_two | 5141 | 5141 |
| dict_row_two | 5183 | 5183 |
| array_row_one | 4718 | 4718 |
| deep_row_one | 23918 | 285038 |
| dict_dict_one | 44755 | 632275 |
| struct_dict_one | 44760 | 632280 |

The array-owned row path is now flat. Multiple dictionary keys, deeper rows and
nested dictionaries remain width-dependent. Fresh-per-call snapshots remain
separate controls because their required copies preserve value semantics. The
matrix also covers widths 2 and 16/32 calls, with VM/JIT allocations and all three
runtimes for semantic checks. These are allocation counts, not elapsed-speed or
peak-memory claims. No candidate optimization is accepted by this characterization.
