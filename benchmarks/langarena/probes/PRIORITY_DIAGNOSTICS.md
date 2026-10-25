# Priority workload diagnosis

Run from the repository root. This harness diagnoses the unchanged `test.json`
workloads; it does not certify production completion or clean-machine speed.
The selected cases are QuickSort, MergeSort, NeuralNet, CacheSimulation,
GameOfLife, both Brainfuck variants, TextRaytracer and Base64 Encode (small-JIT
control). Each runs in VM and JIT. Runtime implementation is unchanged.

Build before measurement, using a fresh output directory:

```sh
mkdir -p .audit-evidence/priority-NEW
cargo build --release --locked --lib -j 2
rustc --edition=2021 -O -C lto=thin \
  benchmarks/langarena/probes/priority_allocations.rs \
  --extern kujo=target/release/libkujo.rlib \
  -L dependency=target/release/deps \
  -o .audit-evidence/priority-NEW/priority-probe
bash benchmarks/langarena/probes/priority-diagnostics.sh \
  "$PWD/.audit-evidence/priority-NEW"
```

The explicit Rust LTO step consumes release dependency bitcode with the matching
Rust LLVM instead of handing it to Apple's older linker plugin. Preserve the
compiler version, build logs and source/binary/input hashes. Never infer a
binary's source revision solely from its filename or modification time.

## Three separate observations per case/mode

1. `count`: counts main-thread allocation/reallocation requests and requested
   bytes during `VM::execute`, including port initialization, setup, warmup,
   JIT compilation and checksum work. Parsing/bytecode compilation and VM
   construction happen before counting. Requested bytes are cumulative traffic,
   not live/retained/peak memory. Worker allocations are not counted.
2. `sample`: disables counting, runs the same source, and requests a three-second
   macOS `/usr/bin/sample` capture. This run is instrumented by sampling and is
   not a timing comparison. Short cases can exit before usable sampling; retain
   sampler logs and mark missing profiles rather than fabricate attribution.
   Samples can cover setup or JIT compilation rather than the hot kernel; inspect
   phase checkpoints and stacks before interpreting them.
3. `counters`: runs the ordinary CLI with runtime measurements and external
   `/usr/bin/time -l` resource accounting. Counters are events with different
   boundaries, not percentages of execution time. CLI wall time includes startup
   and setup; phase checkpoints distinguish creation, preparation, warmup and run.

The probe initializes the same constant globals as the CLI, including `null`.
An earlier attempt omitted this initialization; its failing probe observations
were rejected, while successful ordinary CLI observations were retained.

The diagnostic probe injects only fixed arguments into the existing port main
function. Kernel source, configuration, iterations and checksum checks remain
unchanged. Every accepted result must contain its exact expected checksum and
`Summary: 1, 1, 0`, a zero exit status, and appropriate accounting output.

The shell orchestrator is repository profiling maintenance: macOS PID sampling,
process deadlines and host telemetry need host tools; the language implementation
and benchmark kernels remain Rust/Kujo. Counts and CPU profiles are deliberately
separate. No compiler build may overlap diagnosis. Host telemetry is observed
every ten seconds and does not make the laptop a controlled environment.

The 180-second probe deadline is diagnostic only. A timeout remains incomplete;
do not promote it to a checksum failure. Subsequent timing comparisons must run
without sampling/allocation instrumentation, alternate variants, retain failures
and use the original baseline plus Goose/Rust on a quiet host.

## Follow-up gate

Aggregate counts identify candidates, not root causes. Before proposing a fix,
isolate a small reproducer, distinguish setup/JIT/compiler allocation from the
kernel, test shared and unique ownership, and measure scaling. An unsupported
frame or short sampling window must remain an explicit limitation.

## Sustained short-workload samples

After the main pass, use `sustained-priority-diagnostics.sh OUTPUT_DIRECTORY`
to sample short workloads. It creates an isolated copy of the exact port and
configuration, changing only the final entry call into repeated calls: ten for
CacheSimulation, 100 for GameOfLife, both Brainfuck variants and Base64 Encode.
Each repetition retains the original preparation, warmup, iteration count and
checksum. This is a diagnostic wrapper, not an official larger benchmark input.
Every repetition must pass; samples cannot establish production completion.

After both passes finish, audit all observations with:

```sh
ruby benchmarks/langarena/probes/audit-priority-diagnostics.rb OUTPUT_DIRECTORY
```

The audit checks every expected checksum and observation, measurement schema,
allocation-count mode, source/input hashes and positive external RSS. It preserves
raw outputs, sample stacks and host telemetry in the diagnosis JSON. It rejects
partial or failed observations rather than turning them into measurements.
