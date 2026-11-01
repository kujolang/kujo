# Accepted depth-cache runtime profiles, 2026-11-01

Six checksum-bearing post-warmup samples pass independent verification on source equivalent to accepted `55e2f683`: QuickSort, MergeSort and NeuralNet in VM and JIT. All 890 tracked runtime/test/build manifest files match. The locked release library build passed; its frozen SHA-256 is `6f522349c74340a4ef55ff08d27bd4c346c144cc2feef95138b57255f1bda880`. The diagnostic harness links that library with optimization, thin LTO and retained symbols. It is a separate artifact from the accepted CLI, not a competitive timing run.

| Workload | VM observations | JIT observations |
|---|---|---|
| QuickSort | Dispatch, opcode/value cloning, memory movement and environment lookup | Environment lookup, indexed reads/writes, native root encoding and callback dispatch |
| MergeSort | Dispatch, opcode/value copying and depth-cache maintenance | Binding-path mutation, indexed reads, root encoding, depth-cache release/remember and environment lookup |
| NeuralNet | Dispatch, copying and allocation/free activity | Root encoding, payload release, field reads, copying and allocation/free activity |

The QuickSort raw stack includes `jit_call_function` → Rust environment lookup → `getenv` → `__findenv_locked`. Source inspection confirms two pre-invocation `DEBUG_JIT_CALL` reads plus a post-callback read. The existing `jit_debug_environment` regression requires callback changes to be visible immediately on return; caching the entry flag across that boundary would regress behavior. Next isolate the repeated pre-invocation check before proposing a change, with callback enable/disable/error and malformed-stack controls. No runtime optimization follows merely from its presence in a sample.

VM opcode/scalar and native-root representation alternatives previously deferred for clean-host review remain deferred. These profiles do not establish precise native execution coverage, quantify a causal speedup, or close section 5.

Sampling runs for three seconds at two-millisecond intervals after unchanged warmup markers. Collapsed totals include waiting threads, so they are not CPU percentages. Generated native frames can remain unresolved. Whole-production and clean-host comparisons remain open.

[21-artifact evidence bundle](accepted-depth-profiles-2026-11-01.json.gz) preserves source/build identity, raw samples, symbols, checksum outputs, collector and independent verifier. The pipeline is terminal and successful; no profiling/build job from this refresh remains active.
