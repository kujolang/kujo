# Symbolized kernel attribution

All six post-warmup diagnostic runs pass their checksums and independent profile
verification: QuickSort, MergeSort and NeuralNet in VM and JIT. Each profile
contains resolved Kujo runtime functions. No runtime implementation changed.

## Artifact identity and scope

The saved accepted library could not resolve a dependency's metadata after an
isolated build reused the shared target cache. The accepted runtime source was
rebuilt with the locked dependency configuration; all 882 frozen source/test
hashes matched. Runtime source, vendored dependencies and Cargo manifests also
match `5a0632d`. The rebuilt library SHA-256 is
`fe71551bc3571a96308a0f7b344be1fe8fcadfe85186a7f12e4d04d3a31abe52`.
This is a source-equivalent diagnostic library, **not the original accepted
executable or library artifact**.

The plain Rust harness has no allocation wrapper or capture instrumentation.
It injects only the port's test configuration, selected case and profile flag,
initializes CLI constant globals, and uses cooperative suspend/resume execution.
It links with `-O -C lto=thin -C strip=none`. The successful retry uses the
canonical archive basename `libkujo.rlib`; earlier metadata, filename and Apple
LLVM bitcode-link failures are retained in the evidence.

The sampler starts at `@profile warmup-done`, requests three seconds at a
two-millisecond interval, and retains its command, PID, output and call graph.
Source hashes, exact case coverage, checksums, symbol-table receipt and binary
identity pass verification. The verifier receipt binds its own source digest.
Formatting and diff checks pass.

## Findings

| Workload | VM observations | JIT observations |
| --- | --- | --- |
| QuickSort | `VM::execute_dispatch`, opcode clone/drop, value clone/drop, memory copies | Indexed reads/writes, `NativeCallRoots::encode`, ownership detach/take, hashing, environment lookup |
| MergeSort | Dispatch, opcode clone/drop, value handling, depth-cache operations | Binding-path mutation, indexed reads, root encoding, hashing and depth-cache operations |
| NeuralNet | Dispatch, opcode cloning, value handling and allocation/free activity | Root encoding, field reads, value release/drop and allocation/free activity |

QuickSort's environment-lookup stacks include `jit_call_function` calling
`std::env` and then `getenv`. This narrows the earlier system-frame observation to
an actual runtime caller. It does not justify globally caching debug flags or
changing when language/host environment mutations become visible.

The VM source clones the current opcode in `execute_dispatch` before its main
match, except for the already-specialized nested assignment path. The observed
clone/drop activity therefore has a concrete implementation target. These samples
do not establish the allocation cost of each variant or prove that replacing the
entire dispatch architecture would improve performance.

## Next bounded implementation gate

1. Isolate opcode/name-copy costs with real compiler-emitted global/local reads,
   varying name length and repetition count. Retain the original compiled source
   and verify the same result in VM, interpreter and JIT.
2. If a cost is reproduced, remove only the demonstrated unnecessary copying.
   Preserve global resolution, lazy builtins, snapshots, error ordering, callback
   behavior and chunk/frame lifetimes. Avoid a broad dispatch rewrite by default.
3. Require matching before/after allocation and scaling results, targeted runtime
   regressions, relevant broad checks and unchanged benchmark checksums before
   accepting a fix. Local elapsed time remains diagnostic.

Native-value root encoding/indexed-access overhead and debug environment lookups
remain separate candidates. They need their own reproducer and semantic review.
No shortcut or runtime change has been accepted from these profiles alone.

## Limitations and evidence

These are finite diagnostic samples on a loaded laptop. Waiting threads appear
in collapsed totals and cannot be used as CPU-time percentages. Generated JIT
frames can remain unresolved, so precise native/fallback time attribution is
still incomplete. The separately linked harness is not suitable for a direct
speed comparison with the accepted CLI, Goose or Rust.

[Evidence bundle](symbolized-kernels-2026-10-31.json.gz) contains 28 digest-verified
artifacts: raw profiles, symbol table, source/build identities, checksums,
verifier, harness, and rejected link attempts. Historical handoff notes describe
intermediate states; `samples/verified.json` is the final verification receipt.
Clean-host comparisons, production sizes and platform acceptance remain separate.
No Kostya contact, release publication or algorithm changes occurred.
