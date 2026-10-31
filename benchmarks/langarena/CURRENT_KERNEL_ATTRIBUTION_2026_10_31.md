# Kernel attribution after local-field and named-load fixes

Six fresh symbolized samples on accepted runtime `26105f4` pass checksums and
independent verification: QuickSort, MergeSort and NeuralNet in VM and JIT.
The harness links directly against the saved accepted library, SHA-256
`8becd18b137d9267eddd620e6ecb69f2b01a5e449fe3cffce8c6614df64a09f5`.
Profile collection required no library rebuild; runtime implementation is unchanged. The separately linked
harness is a diagnostic executable, not the accepted CLI artifact.

## Remaining attributed costs

| Workload | VM | JIT |
| --- | --- | --- |
| QuickSort | Dispatch, opcode clone/drop, value copies and memory moves | Environment lookup, indexed reads/writes, root encoding, argument movement, ownership detach/take and hashing |
| MergeSort | Dispatch, opcode clone/drop and value movement | Binding-path mutation, indexed reads, hashing, root encoding and depth-cache operations |
| NeuralNet | Dispatch, value copies, opcode cloning and allocator activity | Root encoding, field reads, value copies/drop and allocator activity |

These confirm that the remaining candidates still occur after the accepted fixes.
They do not quantify a speed change against the earlier profiles. Sample counts
are not CPU percentages: waiting threads and unresolved generated frames remain.
No clean-host or competitive performance claim follows.

The preceding [warmed-read allocation isolation](INDEXED_READ_ALLOCATION_SCREEN_2026_10_31.md)
found no allocation requests in four cached helper shapes. That result and these
CPU samples are compatible: indexed reads and root bookkeeping still perform
work without allocating. Further allocation-only screens cannot establish their
CPU benefit. Next experiments should isolate dispatch and root bookkeeping costs
with matched semantics and current workload attribution.

## Debug environment semantics

`jit_call_function` reads `DEBUG_JIT_CALL` before dispatch and again after returning
from user code. A native caller can invoke a VM callback which enables that flag.
The return diagnostic must then appear even though the flag was absent at entry.
The new `jit_debug_environment` regression runs the same source in VM, interpreter
and JIT, checks the result, requires one actual JIT native function entry, and
asserts the return diagnostic appears without a pre-call stack diagnostic.

Initial tiny-function and top-level-loop controls passed functionally but recorded
zero native entries. They are retained as rejected native-coverage controls; they
are not evidence for the JIT callback behavior. Adding a loop inside the caller
triggers the documented early compilation path and establishes the intended case.
An initial diagnostic invocation also discovered that the CLI disallows combining
`--interpreter` and `--measurements`; interpreter verification uses normal output.

Process-wide or call-entry caching of the debug flag would violate this reproduced
behavior. Any optimization must preserve re-reading after callbacks. This evidence
does not authorize assuming that external environment mutations cannot occur.

## Evidence

[Evidence bundle](current-kernel-attribution-2026-10-31.json.gz) contains raw samples,
checksums, exact identities, verifier output, source controls and regression-test
verification. Sampling begins at `@profile warmup-done`, lasts up to three seconds,
and uses a two-millisecond interval. Runtime source hashes match the accepted
manifest. Existing full-runtime results are not presented as newly rerun here.
