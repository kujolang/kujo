# Accepted-runtime call scaling baseline

**102/102 uninstrumented samples verify**: seventeen kind/arity/width cells,
VM/JIT opt-in, three rotating mode rounds, 32,768 measured calls after 128 warmup
calls. The accepted CLI is `3d6b3ea8e621d6bf8f74ce2cd8f102bc2da0629221b6d3fdc091640ff4e790b3`.
No runtime source change accompanies these results. This is a loaded-laptop
diagnostic, not a candidate comparison, clean-host score or significance claim.

Each target returns its first argument (or seven at arity zero); additional
arguments alias that same value. Arrays retain a caller snapshot. Setup and
result/snapshot assertions are outside the timer. These controls isolate
read-only calls, not mutating recursion or arbitrary user programs.

Median JIT kernel microseconds divided by measured call count:

| Arity | Scalar | Array width 8 | Array width 256 | Array width 8192 |
| --- | ---: | ---: | ---: | ---: |
| 0 | 1.074 | — | — | — |
| 1 | 1.100 | 1.639 | 1.650 | 1.510 |
| 2 | 1.142 | 1.926 | 1.853 | 1.625 |
| 3 | 1.246 | 2.061 | 2.133 | 1.979 |
| 4 | 1.254 | 2.266 | 2.350 | 2.350 |

These numbers include loop, global callable lookup, argument transfer, dispatch
and return work. They are **not isolated frame costs**. The full archive also
retains VM medians/ranges and every sample. Width does not produce proportional
cost growth in these controls, so unconditional whole-array copying is not a
supported explanation here. This does not prove that mutating sort recursion
never detaches or copies. The decline in some larger-width observations must
not be interpreted as larger arrays inherently making calls faster.

The separate [entry census](probes/NATIVE_CALL_SCALING.md) verifies eighteen
instrumented controls: 768 extra checked calls yield exactly 768 additional
native function entries and dispatches, with zero extra compilation attempts in
each of nine width-8 arity/kind configurations. Owned-argument dispatches grow
by 768 for arrays and zero for scalars. Thus the measured family does reach
native functions while retaining dispatch. Counters do not supply CPU shares.

## Next implementation boundary

Existing positional arguments, reusable argument buffers and native-root pools
are already present. Investigate a guarded compiled-callee path that removes
redundant VM-stack transfers and ownership conversions. Dynamic target identity,
arity/representation, captured state and code lifetime must remain valid; retain
generic routes for unsupported calls. Merely fusing helper calls does not prove
that a direct known-callee native path exists.

Before editing, add controls for argument ordering, recursion/depth limits,
rebinding, field receivers, callbacks/errors, cancellation and mutating snapshots.
Prove the new generated path executes, compare scalar and owned controls, then
run unchanged target workloads and all 150 runtime/checksum combinations.
Repeat candidate comparisons in paired rounds; do not reuse these separately
collected medians as a controlled before/after speedup. Keep the existing
one-implementation/one-hypothesis-revision delivery limit.

## Retained evidence

`native-call-baseline-2026-11-02.json.gz` contains 209 hash-verified artifacts:
raw outputs, invocation/source/binary identities, verifier summary, probe and
runner/verifier scripts. Verification checks exact coverage/order, argv, raw
timers, outputs and exits. All 51 JIT samples retain the exact top-level
MakeClosure fallback notice; VM stderr is empty. Native participation is
established separately by the census, not inferred from that warning or a timer.
