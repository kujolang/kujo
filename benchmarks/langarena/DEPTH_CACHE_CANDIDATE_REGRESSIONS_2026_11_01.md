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

## Memory and retention review

Source review against accepted `348a6270` confirms the complete `DepthCache` implementation is unchanged. The four map values still hold `Weak` identities and scalar bounds, not strong collection owners. These weak references keep allocation control blocks alive until removal; they do not keep collection payloads alive after the final strong owner is dropped. Releasing the mutation path before `Arc::make_mut` and subsequent publication are unchanged, with new passing owner-kind regressions.

Publication still checks aggregate entry count at 1,024, removes dead identities, and clears the maps if the remaining count reaches that threshold. This is a publication-time policy, not a universal hard cap: direct depth scans can insert entries before publication. `clear` retains map capacity, as before. No payload-retention or capacity-shrinking improvement is claimed. The component layout measurement shows 192 to 128 bytes for the empty cache object on this host; heap allocation and whole-process RSS were not measured. Hash-table bucket placement can differ, but key/value types, key equality, weak-owner lifetime and retention decisions are unchanged. This review supports unchanged ownership policy, not a new global memory bound.

## Outstanding acceptance

All **150 unchanged test-size checksum cases pass** on the frozen candidate: 50 each in VM, interpreter and JIT. Independent verification checks exact ordered case membership, unchanged configs, exit status, nontruncated output, unique checksum and summary lines, and frozen source/binary/report hashes. The [ten-artifact checksum bundle](depth-cache-candidate-checksums-2026-11-01.json.gz) includes raw reports/logs and the verifier. This does not certify production-size inputs. Four alternating workload pairs for QuickSort, MergeSort and NeuralNet in VM/interpreter/JIT are now running and require independent verification. Broader [Linux CI run 41287402564](https://github.com/kujolang/kujo/actions/runs/41287402564) was dispatched at this exact candidate revision; no CI pass is claimed here. Whole-workload performance and candidate integration remain pending; the source-level memory/retention review above is complete. Loaded-laptop timings remain diagnostic; clean-host comparisons are separate.

The [component comparison](DEPTH_CACHE_HASH_COMPARISON_2026_11_01.md) qualified this candidate for testing. Its helper ratios do not establish a whole-language or competitive speedup, and passing the selected regression suites does not certify the unfinished gates.
