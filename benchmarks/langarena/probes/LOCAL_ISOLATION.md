# Local isolation follow-up

This round changes diagnostic tooling only. The candidate runtime is frozen at
`ea0ea3e`, the original at `f037705`; no runtime fix or benchmark algorithm change
is included. Clean-host comparisons remain pending.

## Reproduce

Build the original from `git archive f037705` in an isolated directory. Preserve
the current candidate binary before any build, verify its prior SHA256, and copy
the rebuilt original to a separate immutable filename. Build with the same Rust
1.96.0 toolchain, locked dependencies, release profile and two build jobs. Do not
run measurements while any build is active. The baseline Cargo configuration is
preserved, including its original serde_json features.

Build the diagnostic library from the current unchanged runtime, then:

```sh
rustc --edition=2021 -O -C lto=thin \
  benchmarks/langarena/probes/local_isolation_allocations.rs \
  --extern kujo=target/release/libkujo.rlib -L dependency=target/release/deps \
  -o "$OUTPUT/isolation-probe"
ruby benchmarks/langarena/probes/local-isolation-generate.rb "$OUTPUT"
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" micro
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" ownership
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" phase
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" neural
```

`OUTPUT` contains `candidate-kujo`, `original-kujo`, and the diagnostic probe.
Each stage refuses to overwrite previous evidence and records explicit outcomes,
source/binary hashes, stdout/stderr and continuous host telemetry. The shell/Ruby
programs are repository profiling maintenance, not language adapters. Kernel and
ownership workloads remain Kujo; Rust controls call the actual runtime types.

## Interpretation boundaries

- Allocation probes are not timing comparisons. A single-thread allocator reads
  a private environment phase marker without allocating and counts requests and
  requested bytes. This adds lookup overhead deliberately; never infer elapsed
  speed from these runs. Marker-setting and assertions themselves allocate.
- Phases: 0 port initialization; 1 create; 2 prepare; 3 warmup; 4 measured loop;
  5 checksum/report. Marker overhead near transitions is charged to the previous
  phase. Counts include phase-local instrumentation and JIT compilation; setup
  and frontend compilation outside `VM::execute` are not counted.
- The phase fixtures preserve benchmark algorithms/configuration/checksums and
  add only private `env_set` markers. They are instrumented diagnostic variants,
  not official production results. Exact checksums must still pass.
- Ownership controls use 64 distinct indexed mutations at sizes 128/512/2048, plus 256 and 1024 mutations at size 2048.
  Unique, one retained snapshot, and a new snapshot before each write distinguish
  required detachment. Assertions verify current values and retained snapshots.
  The setup array owner is explicitly released before the measured loop.
- Array, dictionary, nested-row, struct-row and helper-return controls isolate different
  paths. Scaling differences are evidence about these reproducers, not automatic
  causal attribution of every cost in the full benchmark.
- Direct Rust controls count actual opcode clones, environment lookups, unique
  and shared array detachment. Call controls separately vary arity and call count
  with construction/compilation/warmup outside the measured region.
- NeuralNet runs use the unchanged port and test configuration, seven paired
  rounds with alternating original/candidate order in VM and JIT. Compare loop
  checkpoint deltas and whole-process time separately. Host noise prevents a
  clean-machine speed or universal regression claim even with consistent signs.
- CPU sample stacks and runtime counters from the priority diagnosis identify
  dispatch, compilation, helper and native-boundary costs. Event counts are not
  CPU-time percentages; unsymbolized generated code remains unattributed.

## Small-workload compilation payback

The `break_even` stage runs the unchanged port with Base64 Encode size 30,
one warmup and 32/512/8192/131072 measured iterations, seven alternating VM/JIT
pairs per size. These are explicitly diagnostic configurations. Checksums are
independently calculated from the expected encoded byte count and the documented
DJB2 checksum; the same calculation reproduces the official test checksum at
its original iteration/warmup counts. Full-process time includes compilation;
loop checkpoint deltas are reported separately. Results can bracket a local
crossing, not establish a universal JIT admission threshold.

```sh
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" break_even
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" break_even_confirmation
ruby benchmarks/langarena/probes/local-isolation-audit.rb "$OUTPUT"
```

Instruments `xctrace` is absent on this host and DTrace cannot initialize with
current privileges/SIP. The report therefore retains sampled helper/dispatch
counts and an unattributed category; precise generated-native/fallback CPU-time
coverage is an instrumentation limitation, not a claimed completed percentage.

The first three small-workload rounds placed the largest-input difference near
the noise threshold. Four confirmation rounds extend the same alternating order
to seven pairs; the initial observations remain preserved separately.

`local_isolation_layout.rs` can be compiled with `--crate-type lib --emit=llvm-ir
-O` against either release library to record `size_of` constants without running
an executable or altering the library. Layout differences are hypotheses to
investigate, not causal proof of a timing regression.

## Ownership observer/scope controls

The initial 225 ownership observations run at top level and call `assert_equal`
on every successful iteration. “Unique” here means no explicit user snapshot;
it does not prove the runtime holds only one owner. Native calls can synchronize
global environments and retain internal owners, so a size slope cannot be
attributed to the assignment alone.

Generate 270 additional observations for the same five shapes/three ownership
states at (128,64), (2048,64), (2048,1024). Keep failure assertions, but execute
no native assertion on a successful loop iteration; compare global scope with a
function-local `exercise()` scope. Snapshot reads remain in every iteration.

```sh
ruby benchmarks/langarena/probes/ownership-controls-generate.rb "$OUTPUT"
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" ownership_controls
```

These controls distinguish effects of assertion calls/global synchronization
from local-state ownership. JIT-enabled runs must be interpreted with their
native-entry counters: enabling JIT alone does not prove a loop compiled.

After the paired timing run, `neural_resources` performs four separate
`/usr/bin/time -l` observations (original/candidate × VM/JIT), requiring the same
exact checksum. Peak RSS is retained separately and these runs are excluded
from the seven-round timing statistics.

## Allocation/collection fix comparison

The `priority_calls` stage runs seven alternating before/after pairs of unchanged
QuickSort, MergeSort and NeuralNet test configurations in VM and JIT-enabled
execution. It obtains each expected checksum from `test.json`, requires the exact
checksum and summary lines, records phase checkpoints, and rejects timeouts or
nonzero exits. Supply frozen executables as `$OUTPUT/original-kujo` and
`$OUTPUT/candidate-kujo`; the report hashes both. In the October 25
allocation/collection round, `original` is **ea0ea3e**, not the older f037705
initial baseline. Run sequentially after builds and correctness checks:

```sh
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" priority_calls
ruby benchmarks/langarena/probes/local-isolation-run.rb "$OUTPUT" neural_resources
```

A report path must be fresh; the runner refuses to overwrite prior observations.
Compare paired ratios and all sample ranges. Total process time includes startup,
preparation, warmup and measured loops. The measured-loop duration is the last
`run-N` timestamp minus `warmup-done`; these checkpoints are not independent CPU
samples. Resource-accounting runs remain separate from paired timing.

`call_validation.kujo` checks 10,000 eight-argument calls.
`collection_reuse.kujo` checks dictionary deletion, stack pop and queue dequeue,
with the mutation region marked as phase 4. Generate separate copies changing
only `n` for width/scaling diagnostics, preserving the source and hash in evidence.
These diagnostic kernels are not Arena workload substitutions. Run with the
allocation probe for phase traffic and the standard CLI for wall/RSS observations.

See [the fix report](../ALLOCATION_COLLECTION_FIXES_2026_10_25.md) for exact
allocation observations, ownership restrictions and remaining gates.

## Imported setup, parameter slots and representation controls

`call-slots-generate.rb OUTPUT` creates zero-, one-, eight-, 32- and 128-argument
kernels and refuses to overwrite files. Small arities run 200,000 calls; wider
arities run 20,000. Each verifies an exact sum of distinct arguments. This measures
frame metadata scaling, not an Arena algorithm. Run frozen before/after binaries
sequentially, alternate order for seven pairs, and retain all samples.

`representation_costs.kujo` checks `bytes`, `utf8`, `byte_index` or
`iterator_snapshot`; generate separate copies with widths 128 and 2,048. Phase 4
excludes initial collection construction. Preserve source hashes and ordered
validation. Timing controls use 4,096 byte conversions; allocation controls use
64 operations. Interpreter checks semantics/timing; the allocation probe measures
VM/JIT main-thread requested traffic. RSS is collected separately.

The [follow-up report](../IMPORTED_SETUP_AND_BYTES_2026_10_25.md) preserves all
356 observations plus targeted repeats of initially slower cases. Exact drivers,
commands and sources are embedded in its JSON evidence. Compilation must finish
before timing; test-size and diagnostic results do not establish clean-host or
competitive acceptance.
