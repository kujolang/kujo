# Repeated loop execution: measured remaining gap

Candidate eb299e0 passed the full unchanged 150-checksum matrix, but remains
unaccepted pending compatibility and performance review. Its exact binary is
683f3f615380d27423126d3ff1dddea908643c2b7d31afd3d0f51d5846e61907.
The accepted fd7b764 baseline binary is
45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f.

All 108 paired observations passed their exact output checks. Six synthetic
fixtures isolate condition calls, body calls and precomputed bounds at widths
300/3000, each repeated 200 times. Three pairs alternate baseline/candidate order
in VM, interpreter and JIT, with whole-function JIT disabled. These are diagnostic
probes, not altered LangArena workloads. Timings include startup and compilation
on the loaded laptop; they do not establish competitive or clean-host speed.

| Runtime | Shape | Width | Median baseline/candidate | Three paired ratios |
|---|---|---:|---:|---|
| vm | condition | 300 | 1.006 | 1.083, 1.006, 1.006 |
| vm | body | 300 | 0.987 | 1.066, 0.987, 0.965 |
| vm | precomputed | 300 | 0.976 | 0.936, 0.987, 0.976 |
| vm | condition | 3000 | 0.985 | 0.985, 0.837, 1.092 |
| vm | body | 3000 | 1.014 | 0.961, 1.014, 1.045 |
| vm | precomputed | 3000 | 1.022 | 1.022, 1.023, 1.009 |
| interpreter | condition | 300 | 1.041 | 1.041, 1.003, 1.092 |
| interpreter | body | 300 | 0.986 | 0.978, 0.999, 0.986 |
| interpreter | precomputed | 300 | 1.032 | 0.965, 1.032, 1.057 |
| interpreter | condition | 3000 | 0.998 | 0.911, 1.079, 0.998 |
| interpreter | body | 3000 | 1.013 | 0.939, 1.015, 1.013 |
| interpreter | precomputed | 3000 | 1.020 | 1.041, 1.020, 0.987 |
| jit | condition | 300 | 0.951 | 0.976, 0.951, 0.949 |
| jit | body | 300 | 1.007 | 0.995, 1.094, 1.007 |
| jit | precomputed | 300 | 1.033 | 1.033, 1.111, 1.010 |
| jit | condition | 3000 | 1.055 | 1.044, 1.075, 1.055 |
| jit | body | 3000 | 0.966 | 1.082, 0.966, 0.947 |
| jit | precomputed | 3000 | 0.987 | 1.035, 0.984, 0.987 |

Ratios above one favor the candidate. Changes are modest and mixed; in particular,
the small condition-call JIT fixture has a median ratio near 0.951. This loaded
host cannot establish a repeatable slowdown threshold, and no performance win is
claimed from this batch.

## Mechanism evidence

A separate instrumented pass of the same six JIT fixtures records VM call counts:

| Shape | Width | Baseline calls | Candidate calls |
|---|---:|---:|---:|
| condition | 300 | 60602 | 60401 |
| body | 300 | 60402 | 60202 |
| precomputed | 300 | 402 | 402 |
| condition | 3000 | 600602 | 597701 |
| body | 3000 | 600402 | 597502 |
| precomputed | 3000 | 402 | 402 |

The 3000-wide condition fixture saves only 2,901 calls across 200 scans. A debug
trace confirms exactly one native loop entry and one completed native execution,
while the output remains PASS 600000. This supports a one-invocation benefit,
not recurring acceleration.

Source inspection explains the limitation: VM back-edge handling enters native
execution only through JitCompiler::should_compile; that method rejects an offset
already present in compiled_cache. The loop code is inserted into that cache,
but subsequent back edges do not execute it. Existing counters, blacklist and
compiled lookup are offset-keyed, so blindly enabling reuse would risk mixing
chunks that share offsets or mismatching specialized collection assumptions.

Next work: separate chunk-aware loop cache/admission identity, safely execute
reusable native loops, and preserve dynamic collection admission/ownership guards.
Regression coverage must include repeated calls, distinct chunks with identical
loop offsets, captured environments, changing collection representations and
shared snapshots, callbacks/errors, and both break and normal exits. Repeat the
same correctness matrix and measurements; do not weaken checks to force a win.

The companion bundle preserves all fixture sources, raw outputs, timing/counter
reports, trace and runners with SHA-256 digests. Full compatibility run
41273341615 remains pending. This is an owned active next task, not a release claim.
