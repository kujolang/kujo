# Alternative root hash mechanism screen

No alternative from this screen is selected for runtime integration. The cheaper
mixes reduce some insert/get/remove costs but retain lookup penalties; none
dominates the identity baseline across the tested shapes. This narrows the next
lookup investigation without another full runtime build.

The standalone Rust control keeps the root map's `u64 -> usize` shape and compares
identity hashing, multiplication, `value ^ (value << 7)`,
`value ^ (value << 13)`, and `value ^ value.rotate_left(7)`.
It does not modify Kujo or model frame/slot ownership.

## Coverage and verification

Seven rounds cover five hashers, three retained-key counts (1/64/2,048), three
handle bases (0, 2^32, 2^63), and three modes: stable lookup, rotating lookup and
transient insert/get/remove. Width/base order reverses on alternate rounds;
hasher order rotates and reverses. Every case warms 4,096 insert/remove cycles,
then measures 200,000 operations. All 945 observations and 189 million measured
operations verify against the exact inventory and independently calculated sums.

Retained values are checked after each run. Removed, never-issued and boundary
keys remain absent after bucket reuse. These checks establish map behavior;
runtime stale-handle, frame and value-semantic tests would still be required for
any integrated change. Source, dependency, compiler identity, binary and raw output
hashes are recorded. No compiler was running during measurements.

## Local results

Each cell is the range of seven-round paired median ratios against identity,
across all nine base/width combinations. Below 1 means less elapsed time.
Ratios are computed per round before taking medians, not by dividing marginal
elapsed-time medians. All observations are retained.

| Hasher | Stable lookup | Rotating lookup | Insert/get/remove |
| --- | ---: | ---: | ---: |
| Multiply | 1.395–1.401 | 0.794–0.921 | 0.097–0.921 |
| XOR shift 7 | 1.100–1.104 | 1.109–1.162 | 0.210–1.012 |
| XOR shift 13 | 1.297–1.298 | 1.003–1.122 | 0.900–1.009 |
| XOR rotate 7 | 1.057–1.087 | 1.066–1.149 | 0.212–1.005 |

Multiplication improves every measured lifecycle pair but slows every stable
lookup pair. The two seven-bit mixes reduce the stable-read penalty relative to
multiplication, but rotating reads are slower in 62/63 and 61/63 pairs respectively.
The thirteen-bit mix retains most of the wide lifecycle cost. These are map-control
results on the loaded laptop, not integrated-runtime speed predictions.

The probe also records `HashMap::capacity()` before warmup, after warmup and after
measurement. This API reports insertion capacity and can decrease with tombstones;
it is not allocated bytes, retained heap or RSS. No memory saving is claimed from
those values.

## Evidence and next constraint

The next design must address read cost as well as retained-key churn and must
cover reused/high handles. Merely replacing multiplication with a cheaper mixer
does not meet the observed requirements. The prior integrated candidate remains
unaccepted; the working runtime remains `fd7b764`.

Source: `probes/native_root_hash_alternatives.rs`.
Evidence: `native-root-hash-alternatives-2026-10-31.json.gz`, including build/run
receipts, raw observations, verifier and source with per-artifact SHA-256 digests.
Local originals: `.audit-evidence/native-root-hash-alternatives-2026-10-31/`.
This screen does not establish production, platform or clean-host acceptance.
