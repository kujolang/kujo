# Native execution census — October 23, 2026

All 50 unchanged **test-size** cases passed on runtime source `0c77115`, with
JIT enabled and VM fallback allowed. All 50 original measurement sidecars passed
the committed Draft 2020-12 schema and their recorded SHA-256 hashes matched.
This does not close production, interpreter, full-native or competitive gates.

[Raw cases, counters, build identity and hashes](native-execution-census-2026-10-23.json)
retain the exact configuration and checksum output for every workload. The build
uses default features, thin LTO and one codegen unit, with final-binary symbols
retained for sampling. It ran on the shared Intel macOS laptop. Instrumentation
and sampling affect elapsed time; these numbers are not competitive samples.

## What executed

34 cases entered native whole functions; 16 recorded no native entries. A zero
count may mean a cold function or unsupported code, not necessarily one specific
compiler defect. A positive count does not establish that the benchmark's main
work ran natively. Native boundary and function counts were equal in this run.

| Case | Native function entries | Dispatches with owned arguments | All JIT call dispatches |
| --- | ---: | ---: | ---: |
| TextRaytracer | 0 | 742,468 | 1,110,426 |
| MergeSort | 399,807 | 600,011 | 1,000,035 |
| QuickSort | 366,708 | 533,813 | 900,735 |
| NeuralNet | 0 | 156,020 | 156,082 |
| Maze AStar | 133,064 | 38,739 | 172,123 |
| Calculator Interpreter | 1,246 | 25,476 | 26,939 |

Dispatch counts include compilation retries and callback observations. Owned
arguments may coexist with other compilation exclusions. These are **not** native
time-coverage percentages, and their ratio does not predict a possible speedup.
They do show that collection/string call boundaries are exercised extensively.
The separate [eight-probe audit](jit-owned-values-2026-10-23.json) establishes the
correctness failures that must be repaired before widening those native paths.

## QuickSort CPU evidence

The [symbolized sample](quicksort-cpu-sample-2026-10-23.txt) contains 2,307 samples
for the executing `kujo-vm-runner` thread over a requested three-second interval.
It shows 890 leaf samples in VM dispatch and 221 in opcode cloning (about 38.6%
and 9.6% of that thread's sampled stacks). It also shows call-frame setup,
allocation/free, value copying and environment lookup. Two waiting threads each
contribute 2,307 join-wait samples; including them in the denominator would
misrepresent the executing thread's profile. This is a single interval, not a
whole-workload attribution or a predicted speedup.

Code inspection identifies an avoidable empty `BytecodeChunk` construction in
`VM::call_bytecode_function`: it saves caller code by replacing it with a new
empty chunk, then immediately replaces that placeholder with callee code. Measure
and remove this allocation independently of the larger native ownership change.
Opcode copying and diagnostic environment checks remain measured candidates;
their changes still need independent semantic/cost evidence.

TextRaytracer exited before its sample began. The NeuralNet sampling attempt
returned no call graph. Neither supplies usable CPU attribution.

## Reproduction

Use a fresh output directory and the frozen binary recorded in the JSON:

```sh
kujo run --scheduler-no-timeout benchmarks/langarena/profile.kujo \
  BINARY benchmarks/langarena/test.json OUTPUT 600000 jit 0 1 '' measurements
```

Validate each original sidecar against `schemas/runtime-measurements-v1.schema.json`
and compare its hash, case config and checksum output. Five embedded runner
`cpu_seconds` values differed from their original sidecars by floating-point
reserialization below 1e-12; all other fields matched exactly. The committed
census preserves the authoritative sidecar values. Raw sidecar SHA hashes apply
to original file bytes, not to reserialized embedded JSON.
