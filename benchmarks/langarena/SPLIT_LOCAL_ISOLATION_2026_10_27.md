# Shared-helper retained-local recursion — 2026-10-27

The accepted runtime `ce923d1` retains width-dependent allocation costs when a
captured closure calls a same-module helper, which calls a foreign module that
reenters the closure. The helper leaves globals and captured locals in different
suspended frames. This is a baseline characterization, not acceptance of the
in-progress same-frame foreign-prefix optimization.

All 144 allocation observations and 216 VM/interpreter/JIT semantic runs pass.
[Machine evidence](split-local-isolation-2026-10-27.json) preserves executable,
probe, generator, module and fixture identities and raw results.

## Requested allocation per added outer call

Depth-four results below subtract the 16-call run from the 32-call run and divide
by 16. VM and JIT agree in every measured group.

| Extra retained locals | Operation | Requests | Requested bytes |
| ---: | --- | ---: | ---: |
| 0 | read | 555 | 35408 |
| 0 | write | 560 | 35443 |
| 128 | read | 939 | 160250 |
| 128 | write | 944 | 160285 |
| 2048 | read | 6699 | 2069954 |
| 2048 | write | 6704 | 2069989 |
| 8192 | read | 41535 | 13630038 |
| 8192 | write | 41540 | 13630073 |

The matrix covers 0/128/2,048/8,192 locals, reads/writes, depths zero/one/four and
one/16/32 calls. Every retained local is checked; aliases share captured state,
independent closures stay separate, and the foreign invocation counter must equal
depth times calls. Depth zero measures ordinary activation on this same route.

These are allocation traffic measurements, not elapsed-speed or peak-memory
results. Do not interpret differences from other route fixtures as patch gains.
This run used the accepted baseline while the separate candidate release was
compiling; no wall-clock performance conclusion is drawn.

## Reproduction and next gate

Run `ruby benchmarks/langarena/probes/split-local-allocations.rb PROBE BINARY NEW_OUTPUT_DIR`
with matching accepted artifacts. An optional fourth `BASELINE_PROBE` argument
executes both probes against identical generated paths and rejects total or
added-call allocation increases. That paired check alone does not establish an
optimization; define and verify the targeted scaling reduction separately.

A future fix must preserve the direct and same-frame foreign paths, caller
parameters, live public owners, snapshots, immutable bindings, errors, task
ownership, cold/append controls and all-runtime checksums. Broader performance
work remains open in the [checklist](PERFORMANCE_CHECKLIST.md).
