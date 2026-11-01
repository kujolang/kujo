# Depth-cache maintenance isolation, 2026-11-01

The current-source MergeSort profile reaches depth-cache maintenance. This diagnostic isolates the existing cache methods using real Kujo `Value` objects and the accepted source `348a6270`. No runtime change or alternative cache is included.

All **72 observations** and **7.2 million operations** pass independent verification. Four collection types, three retained-owner counts, two operation shapes and three repetitions have exact ordered coverage and checksum checks. Input, generated-source, executable and observation hashes are verified.

| Collection | Retained owners | Lookup median, ns | Release + remember + lookup median, ns |
|---|---:|---:|---:|
| Array | 1 | 38.54 | 120.14 |
| Array | 64 | 40.06 | 120.25 |
| Array | 512 | 38.82 | 131.41 |
| Dictionary | 1 | 37.84 | 116.66 |
| Dictionary | 64 | 37.91 | 120.34 |
| Dictionary | 512 | 39.85 | 123.96 |
| Integer-dictionary | 1 | 39.06 | 115.50 |
| Integer-dictionary | 64 | 38.92 | 116.69 |
| Integer-dictionary | 512 | 48.48 | 125.03 |
| Struct | 1 | 38.12 | 119.44 |
| Struct | 64 | 38.29 | 120.88 |
| Struct | 512 | 38.54 | 128.27 |

These are local component timings, including probe loop/index/checksum overhead. They do not measure a speedup, allocation volume, full assignment, end-to-end MergeSort or clean-host performance. Three repetitions on a loaded laptop cannot establish precise regression thresholds. The results justify comparing a narrow alternative while retaining whole-workload gates; they do not establish that hashing alone explains the difference between operation shapes.

## Semantic and extraction boundaries

Controls verify array host copy-on-write invalidates the old cache identity while a retained snapshot still has its bound, path release removes weak references before mutation, mixed collection entries clear at the 1,024-entry cap, repeated replacement remains consistent, and measured cache sizes exactly match retained owners.

The builder extracts the real cache struct and lookup/release/remember methods without changing their bodies. It omits the private depth-traversal method: timed reads use cached bounds directly. Root-only paths are used. The diagnostic declares only the Index/Field access variants and copies the canonical decimal-index helper as a free function to cross the library's private-API boundary. The generated source is independently reconstructed byte-for-byte by the verifier. Full nested assignment and all runtime semantics are **not** certified by these component controls. The initial attempt to call private depth/index methods failed at compilation before any measurement; its receipt and generated source are retained.

The linked diagnostic library is the [verified current-source library](ACCEPTED_LOOP_PROFILES_2026_11_01.md), SHA-256 `701a889210a3d0b67945661e641f845d3f4dd16ec3a22ba9d07f923ca0c27e5a`. This executable is not the shipped CLI.

## Next gate

Compare a narrowly scoped internal-identity hash/cache alternative with this unchanged control, including lookup-heavy cases and multiple owners. Do not remove weak-reference invalidation, depth checks, ownership checks or snapshot preservation. Any runtime candidate requires VM/interpreter/JIT regressions, unchanged workload checksums, boundary/retention review and before/after workload evidence. Prior root-hash regressions remain a reason to reject helper-only acceptance.

[Evidence bundle](depth-cache-controls-2026-11-01.json.gz) contains 12 digest-verified artifacts. Reproduce using `probes/build-depth-cache-controls.rb OUTPUT_DIRECTORY ACCEPTED_LIBRARY` and verify using `probes/verify-depth-cache-controls.rb OUTPUT_DIRECTORY`. No runtime optimization is accepted from this baseline.
