# Depth-cache runtime candidate regressions, 2026-11-01

Candidate `9fb0ed705c59a8da6c6eab36e9f50c3a0722f728` is committed and pushed on `codex/depth-cache-hash-20261101`. It is **not yet accepted** into the working runtime baseline. The current accepted source remains `348a6270`.

Only the four internal depth-cache address maps change hash builder, to `BuildHasherDefault<ahash::AHasher>`. The cache maintenance method bodies are byte-identical to the accepted source. Weak-reference ownership, host mutation invalidation, depth checks and existing publication retention limits remain unchanged. User dictionary hash types are unchanged.

## Local regression gate

The release test command exits 0. Exact executable test inventories, every pass/ignore and suite totals verify independently against the frozen source manifest:

| Suite | Passed | Existing ignores |
|---|---:|---:|
| library | 1315 | 8 |
| jit_callback_assignment | 2 | 0 |
| jit_callback_depth | 1 | 0 |
| jit_nested_paths | 17 | 0 |
| nested_assignment | 9 | 0 |
| vm_interpreter_parity_surfaces | 123 | 0 |

Total: **1,467 passed, zero failures, eight existing manual ignores**. New unit regressions cover all four owner types under host writes with and without snapshots, weak-identity release before unique-storage writes, and mixed-owner publication clearing at the existing limit. The new 1,100-iteration mixed-owner integration fixture passes in interpreter, VM and JIT. Its expected behavior also passes on the frozen accepted CLI before candidate execution. Existing native-path and callback-depth suites remain included.

The copied candidate CLI SHA-256 is `344736925b2c03b2f22fc78ff5d44f798a5e95ff852956821feba24b2bfb49a5`. Commit binding verifies the exact runtime/test source hashes after commit; the binary was frozen after the targeted gate completed. Formatting and diff checks pass. The [nine-artifact evidence bundle](depth-cache-candidate-regressions-2026-11-01.json.gz) preserves raw tests, executable inventories/hashes, source and binary manifests, baseline fixture receipt and source patch.

## Outstanding acceptance

All 150 unchanged test-size checksum cases are running on the frozen candidate. Four alternating workload pairs for QuickSort, MergeSort and NeuralNet in VM/interpreter/JIT are queued behind checksum completion and require independent verification. Broader [Linux CI run 41287402564](https://github.com/kujolang/kujo/actions/runs/41287402564) was dispatched at this exact candidate revision; no CI pass is claimed here. Whole-workload performance, memory/retention review and candidate integration remain pending. Loaded-laptop timings remain diagnostic; clean-host comparisons are separate.

The [component comparison](DEPTH_CACHE_HASH_COMPARISON_2026_11_01.md) qualified this candidate for testing. Its helper ratios do not establish a whole-language or competitive speedup, and passing the selected regression suites does not certify the unfinished gates.
