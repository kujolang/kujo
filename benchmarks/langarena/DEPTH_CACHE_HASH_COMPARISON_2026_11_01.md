# Depth-cache hash component comparison, 2026-11-01

Seven alternating same-process pairs verify **336 component observations**, 14 cold-construction observations and 14 executions of the existing semantic controls. Source reconstruction verifies the baseline against the prior accepted-source probe and confirms that only four internal-identity map hash builders differ in the candidate.

The candidate uses `BuildHasherDefault<ahash::AHasher>`, already a dependency of Kujo, for internal Arc-address keys. It preserves map operations, weak-reference ownership, invalidation, retention limits and all method bodies. User dictionary hashing is unchanged.

| Collection | Owners | Lookup median ratio | Release/remember/lookup median ratio |
|---|---:|---:|---:|
| Array | 1 | 1.970× | 1.925× |
| Array | 64 | 1.921× | 1.826× |
| Array | 512 | 1.916× | 1.725× |
| Dictionary | 1 | 1.915× | 1.801× |
| Dictionary | 64 | 1.925× | 1.708× |
| Dictionary | 512 | 1.920× | 1.749× |
| Integer-dictionary | 1 | 1.928× | 1.746× |
| Integer-dictionary | 64 | 1.926× | 1.707× |
| Integer-dictionary | 512 | 1.933× | 1.686× |
| Struct | 1 | 1.973× | 1.785× |
| Struct | 64 | 2.027× | 1.855× |
| Struct | 512 | 1.994× | 1.694× |

Ratios are paired baseline elapsed time divided by candidate elapsed time; above 1 favors the candidate. Median cold empty-cache construction ratio is 1.462×. On this host the cache object shrinks from 192 to 128 bytes, with alignment 8 unchanged. These sizes exclude map allocations and are not whole-process memory savings. Cold construction excludes first insertion and payload allocation.

## Admission decision and limits

Proceed to an isolated runtime candidate and full semantic/workload gates; do not accept a runtime speedup yet. This is a component experiment with real Values and extracted cache maintenance methods, root-only paths and no full depth traversal. The prior baseline report documents those extraction and semantic-control limits. Local elapsed times remain diagnostic on this loaded laptop. Earlier root-table hash regressions demonstrate why this result must not substitute for whole-workload verification.

Required next gates: preserve host mutation and snapshots for all four map types, weak-reference release and cache bounds; test VM/interpreter/JIT, unchanged benchmark checksums, small-workload setup and whole-workload before/after behavior. Accepted runtime remains `348a6270`.

[Evidence bundle](depth-cache-hash-comparison-2026-11-01.json.gz) contains 8 hash-verified artifacts, including exact generated code, build inputs, raw observations, verifier and independent verification receipt. The original [component baseline](DEPTH_CACHE_CONTROLS_2026_11_01.md) also re-verifies unchanged.
