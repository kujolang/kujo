# Current-source kernel profiles, 2026-11-01

Six post-warmup samples pass checksum and independent sampling verification: QuickSort, MergeSort and NeuralNet in VM and JIT. Runtime source is `348a6270`, integrated in `0523c2dc`. No runtime code changed for this collection.

The frozen worktree library build completed successfully with locked dependencies. The diagnostic library SHA-256 is `701a889210a3d0b67945661e641f845d3f4dd16ec3a22ba9d07f923ca0c27e5a`; 648 tracked source/Cargo/vendor files are hash-bound and source-equivalent to the accepted revision. The symbolized harness links with optimization, thin LTO and no symbol stripping. It is a separately linked diagnostic, not the accepted CLI artifact. The initial Ruby preparation failure was corrected with `map.compact` before linking or sampling; the completed library build was reused.

| Workload | VM observations | JIT observations |
|---|---|---|
| QuickSort | Opcode cloning/dropping, dispatch, value copies and memory moves | Indexed reads/writes, native root encoding, hashing, value movement and environment lookup |
| MergeSort | Dispatch, opcode and value copies, memory moves | Binding-path mutation, indexed access, root encoding, hashing and depth-cache maintenance |
| NeuralNet | Dispatch, opcode/value copying, allocation/free activity | Root encoding, indexed/field reads, value copies and allocation/free activity |

QuickSort's environment lookup stacks again include `jit_call_function` through Rust environment lookup to `getenv`. Existing callback-visible debug flag semantics must remain; these samples do not justify caching away observable changes.

## Next isolated target

MergeSort reaches `DepthCache::remember` during binding-path mutation. Its four maps currently hash internal Arc addresses with the standard hasher and preserve weak references for mutation invalidation. Isolate cache lookup/remember/release cost across realistic retained-owner counts before proposing a change. Any alternative must preserve weak-reference invalidation, depth limits, pointer reuse safety, snapshots, host mutation and mixed collection types. A faster isolated hash is insufficient: prior root-table experiments demonstrated whole-workload regressions despite some helper gains. Require matched helper and workload evidence before accepting a candidate.

VM scalar/opcode and string-cache candidates already deferred for clean-host review remain deferred. This profile refresh does not rehabilitate rejected experiments or close all compiled-execution requirements.

## Limits and evidence

Three-second samples at two-millisecond intervals begin after the unchanged benchmark warmup marker. Waiting threads appear in collapsed totals; those totals are not CPU percentages. Generated native frames can remain unresolved. These samples identify reachable work, not precise native/fallback time, causality, speed improvements or competitor rankings. Compilation/cache lifetimes, full production coverage and clean-host comparisons remain separate open gates.

[Evidence bundle](accepted-loop-profiles-2026-11-01.json.gz) contains 21 hash-verified artifacts: raw samples, collector/verifier, source/build identities, symbol table, checksums and the rejected preparation receipt. The independent verifier passed all six samples against exact binaries, source hashes, invocation/PID receipts and checksum outputs.
