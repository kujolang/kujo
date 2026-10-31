# Native root read/lifecycle baseline

Accepted runtime `fd7b764` remains unchanged. A new direct-helper diagnostic links
the frozen accepted library (`ffce3ed37b9cd1935ac3898ce042c354024c5b64e21148a255636dabde8bbd2a`),
with 1/64/2,048 independent live roots holding the same payload. It compares cached
and uncached reads, stable and rotating root selection, scalar results and owned
row results. Setup, 4,096 warmup reads and teardown are outside timing.

All 72 observations (7.2 million outer indexed reads) pass exact inventory,
checksums, kind/error assertions and post-frame reference-count checks. Owned rows
also validate their nested value and release each temporary root. This is full
read/validate/release cost, not isolated map CPU time. The hint remains initialized
and aligned according to the existing 16-byte ABI. No handles are fabricated.

Scalar median times at 2,048 roots versus one are approximately 1.00–1.04×.
Owned-row lifecycle medians grow 3.19–3.75× across all four cache/selection shapes.
Rotating selection includes modulo and index-selection costs. Three repeats on a
loaded laptop establish a diagnostic lead, not a complexity proof or speed score.

## Root-table mechanism control

The runtime uses `HashMap<u64, usize, BuildNoHashHasher<u64>>` for issued handles.
An independent control reproduces monotonically issued transient handles while
retaining 1/64/2,048 earlier live handles. It measures lookup-only and
insert/get/remove cycles using the current identity hasher, a one-multiply u64
hasher, and deterministic `DefaultHasher`. All 126 observations pass membership,
checksum and retained-entry checks; seven repeats reverse width and hasher order.

Median times for 200,000 operations:

| Hasher | Operation | 1 live | 64 live | 2,048 live |
| --- | --- | ---: | ---: | ---: |
| Identity | Lookup | 0.523 ms | 0.516 ms | 0.547 ms |
| Multiply | Lookup | 0.691 ms | 0.682 ms | 0.803 ms |
| Standard | Lookup | 2.076 ms | 2.029 ms | 2.224 ms |
| Identity | Insert/get/remove | 5.058 ms | 9.162 ms | 39.194 ms |
| Multiply | Insert/get/remove | 4.549 ms | 4.069 ms | 4.155 ms |
| Standard | Insert/get/remove | 7.436 ms | 7.878 ms | 10.939 ms |

The identity-hash lifecycle grows about 7.75×, whereas the multiplied control
stays roughly flat. This supports root-table maintenance as a contributor to the
owned-result growth. It does not isolate the entire runtime slowdown or establish
CPU percentages. Multiplication costs more on isolated lookups; a runtime
candidate must retain that tradeoff and test real helper/workload behavior.

Next: test a private root-table hashing change without changing handle identity,
frame ownership, stale-handle detection, cache validation, bounds or snapshots.
Existing lifecycle contracts plus targeted VM/interpreter/JIT checks are required.
Do not claim success from the synthetic map control alone.

## Reproduction and limits

Sources: `probes/native_root_read_scaling.rs` and
`probes/native_root_hash_control.rs`. The helper uses release thin LTO linked to
the frozen accepted library; the map control uses the same built nohash dependency.
`native-root-read-scaling-2026-10-31.json.gz` preserves sources, identities, build
commands, raw output and verifiers. Both compiled successfully. Targeted rustfmt
checks pass. No runtime changed, so full runtime tests were not rerun for these
probes. No clean-host, production, competitor or general speedup claim follows.
