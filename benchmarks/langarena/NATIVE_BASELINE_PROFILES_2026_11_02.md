# Accepted native performance baseline profiles

Accepted runtime: `782c712d`; campaign branch starts from `66a59e4e`.
All 1,390 frozen source/build/test/vendor hashes still match. The release library
build, symbolized harness link and sequential six-profile collection completed
successfully. QuickSort, MergeSort and NeuralNet passed their unchanged test-size
checksums in VM and JIT modes. No runtime change is included here.

The library SHA-256 is
`1bfa0858e8cf27bf72f0300c27948632a06683ad0ea3f1c42e951d85ea01451b`;
the symbolized harness SHA-256 is
`4c42b444bf33ddad63d356fe4d744ba58e57557facd9cbd187109bf40976fe6d`.
The harness is a diagnostic executable, not the frozen competitive CLI.

## Observations and next experiment

Samples begin after warmup and cover at most three seconds. Attribution selects
only the named `kujo-kernel-profile` thread, excluding the waiting parent. Each
stack node's immediate children are subtracted to obtain exclusive sampled
weights; nonnegative weights and conservation against the thread total pass for
all six profiles. Generated frames without symbols remain unresolved. These are
sampled wall-stack occupancies, **not CPU self-time fractions** or speed estimates.

| Workload | VM thread samples | JIT thread samples | JIT memory-movement samples nearest `NativeCallRoots::encode` |
|---|---:|---:|---:|
| QuickSort | 1,095 | 867 | 33 |
| MergeSort | 1,108 | 1,065 | 55 |
| NeuralNet | 1,106 | 1,100 | 60 |

The QuickSort JIT window can be partial because the process ends. Sampling
frequency and host contention preclude comparing these counts as elapsed speed.
The ancestor column describes the nearest symbolized Kujo frame above
`memmove`, `memcpy` or `bzero`; it is not proof of avoidable allocations, deep
copies or causal CPU share.

Collection read/write helpers, root encoding, VM/native calls and Value movement
remain visible in current accepted code. VM samples prominently include dispatch
and instruction cloning. This supports investigating collection access and root
transfers; it does not justify substituting an easy getter-call cleanup for the
planned guarded native-access experiment. `jit_return_scalar_kind` has only six
QuickSort and seven MergeSort exclusive samples in this window.

Source review confirms ordinary integer array reads already borrow their root,
encode immediate scalars directly, and optionally validate a repeated-root slot
cache. Reintroducing those optimizations would duplicate accepted work. The next
candidate must eliminate remaining helper crossings within a proven safe region,
or demonstrate a separate material transfer cost with measured evidence. It must
preserve negative/mixed indices, snapshots, exceptions and callbacks. Raw array
pointers cannot remain valid merely because a root handle stays unchanged.

The report's duplicate `DEBUG_JIT_CALL` lookup proposal is excluded: the existing
debug-environment disposition preserves concurrent changes between observations.
The pending code-lifetime candidate remains separate.

## Reproduction and evidence

`native-baseline-profiles-2026-11-02.json.gz` contains 19 hash-verified artifacts:
the source manifest and pipeline receipt, toolchain/build/link logs, six raw
profiles, collector output, attribution, harness/config and maintenance scripts.
Executable binaries are identified by hash rather than embedded. The source-bound
build/link commands are retained in `state.json` inside the archive.

Run the attribution tool with a directory containing `samples/*.sample.txt`:

```sh
ruby benchmarks/langarena/probes/attribute-kernel-stacks.rb EVIDENCE_DIRECTORY
```

The archive collector independently verifies source/input/binary hashes, exact
case/mode coverage, checksums, sampler exit codes and warmup triggers before
packaging; it then decodes and verifies every embedded hash. It deliberately
refuses to overwrite an existing archive. Ruby is repository-maintenance glue;
measured runtime behavior remains Kujo/Rust.

No production-size, clean-host, all-50 candidate or speedup claim follows from
these six diagnostic runs. All candidate acceptance gates remain open.
