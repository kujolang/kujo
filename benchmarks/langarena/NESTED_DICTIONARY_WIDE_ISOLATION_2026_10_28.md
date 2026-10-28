# Larger nested dictionary batches — 2026-10-28

Accepted runtime `21a0907` still copies a wide nested dictionary when a helper
updates more than four distinct scalar keys. This report isolates that boundary;
it does not claim a new runtime fix or elapsed-speed improvement.
[Machine evidence](nested-dictionary-wide-isolation-2026-10-28.json) includes exact
fixtures, artifact identities, runner/verifier source and recomputed deltas.

## Measurements

Requested bytes per added helper call, numeric dictionary owner, VM mode,
initial-only snapshot:

| Distinct updated keys | Dictionary width 128 | Dictionary width 2,048 |
| ---: | ---: | ---: |
| 4 | 6,320 | 6,320 |
| 5 | 46,693 | 634,213 |
| 16 | 51,335 | 638,855 |
| 64 | 71,591 | 659,111 |

Crossing the four-key boundary introduces 587,520 additional bytes of
width-dependent traffic between the two dictionary sizes. That width penalty
is unchanged at 5, 16 and 64 writes. Additional writes add modest linear traffic
in this measured range. This supports targeting overflow publication rather
than claiming every subsequent write copies the whole dictionary.

The maintained runner covers dictionary, struct and array owners; numeric and
string keys; 4/5/16/64 writes; widths 128/2,048; initial-only and fresh-per-call
snapshots; and 16/32 helper calls. All 384 VM/JIT allocation measurements and
576 VM/interpreter/JIT semantic cases pass. Original snapshots, fresh snapshots
of the first two keys and final values of every updated key are checked. This
baseline is not exhaustive fresh-snapshot verification for all larger-batch keys.
The independent verifier checks exact coverage, fixture/artifact hashes and deltas.

## Next implementation and acceptance gates

The next candidate should preserve the existing inline path for up to four keys
and provide spill storage proportional to actual distinct writes on overflow.
Simply increasing the fixed key limit would move the same failure boundary.
An ordered sparse table can preserve insertion order and repeated-key coalescing;
its ownership and lifetime must be explicit. No runtime implementation is included
in this report.

Required acceptance:

- Remove width-dependent added-call traffic for eligible 5/16/64-key cases across
  all tested owners and key forms, with total and added-call nonregression checks.
- Preserve the accepted one/four-key allocation controls and assignment/capture
  storage layout; allocate spill storage only when needed.
- Check all retained fresh-snapshot keys, repeated numeric/string aliases,
  insertion order, exact scalar bits, errors, callbacks, root replacement and
  publication after overflow in all three runtime modes.
- Check mixed scalar values and larger batches beyond 64 keys; the measured sizes
  must not become semantic or implementation limits.
- Exercise full spill journals before nonscalar/deeper-path fallback and escaping
  reads, including independent real snapshots and shared leaves.
- Verify journal storage is released after publication/discard and follows actual
  pending writes rather than dictionary width or a persistent unbounded cache.
- Run matching-release integration, historical allocation controls and all 150
  unchanged test-size checksums before acceptance, then record evidence and push.

Broader capture materialization, deeper paths, nonscalar leaves and larger array
rows remain open. Clean-host elapsed-speed, production-size and platform gates are
separate work; these local allocation measurements do not establish Goose/Rust parity.
