# Remaining arithmetic selector baseline

Accepted runtime `5458132`, following the bounded addition/subtraction fix.
This audit measures invariant integer-parameter multiplication and division in
three-write captured-array helpers. A literal-index helper is the control.

| Selector | Width 8 bytes/additional call | Width 2,048 bytes/additional call |
| --- | ---: | ---: |
| Literal control | 6,514 | 6,514 |
| Parameter multiplication | 7,406 | 284,846 |
| Parameter division | 7,406 | 284,846 |

These are requested-allocation slopes between 16 and 32 calls with one saved
snapshot, identical in VM/JIT. The 277,440-byte width penalty remains for both
unsupported forms. This does not establish their frequency in production
benchmarks, a speed ratio, or a defect in value semantics.

All 72 allocation observations and 108 semantic checks passed. The matrix
includes widths 8/2,048, 1/16/32 calls, once/fresh snapshots, VM/JIT allocation
counters and VM/interpreter/JIT output assertions. Every original and untouched
array element is checked. Exact source/binary identities, raw counters, matrix
coverage and recomputed slopes passed independent verification.

The next candidate must distinguish invariant integer expressions from
counter-dependent multiplication/division. Endpoint checks alone cannot prove
a nonlinear expression safe. Overflow, division by zero, signed integer division,
bounds, callbacks, prior-write publication and snapshot preservation remain
required regression boundaries. No runtime fix is included in this baseline.

[Raw measurements, fixtures and reproduction scripts](REMAINING_SELECTOR_BASELINE_2026_10_30.json).
