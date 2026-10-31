# Repeated array-position allocation baseline

Updating two fixed array positions repeatedly still requests memory proportional
to the larger owner and loop bound. This reproduces on accepted runtime `b1bf786`
and the pending root-dictionary V4 candidate. Their measured call slopes match;
the root-dictionary fix does not resolve this separate collection cost.

| Loop iterations per call | Width 8: bytes/call | Width 2,048: bytes/call | Extra bytes/call |
| --- | ---: | ---: | ---: |
| 64 | 91,624 | 101,464 | 9,840 |
| 1,024 | 1,365,544 | 1,532,824 | 167,280 |

VM and JIT agree. Allocation requests per call stay at 636 and 9,276,
respectively, across both widths. These are requested allocation bytes, not
retained memory, peak RSS, or elapsed-time speed measurements.

Each runtime artifact passes 16 allocation observations and 24 semantic checks
across VM, interpreter and JIT. Eight fixtures vary owner width (8/2,048), loop
iterations (64/1,024), and helper calls (1/8). The helper overwrites positions
0 and 1 with array and dictionary payloads. A saved original and untouched
entries are checked after the call loop. Slopes use `(eight calls - one call)/7`;
width-dependent final verification is constant between those two observations.
Call slopes still include payload construction and helper execution.

The array proof currently bounds reservation using executed write count and
owner length. That is a source-supported explanation for repeated-position
overreservation; these whole-fixture counters alone do not attribute every
byte to a particular allocation. The fix must preserve bounds, negative-index
aliases, snapshots, error ordering and fallback behavior, and must verify
distinct-position capacity independently of its semantic output tests.

[Raw evidence and reproduction scripts](repeated-array-reservation-2026-10-31.json)
preserve fixture sources, artifact hashes, exact arguments, outputs, counters,
and independent verifier receipts. V4 remains unaccepted pending its other
gates. These additional observations are separate from V4 acceptance totals.
Section 4 remains open; no runtime change is included in this baseline.
