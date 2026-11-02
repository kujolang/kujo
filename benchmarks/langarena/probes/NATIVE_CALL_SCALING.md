# Native call scaling controls

`native-call-scaling.kujo KIND ARITY WIDTH CALLS verify|measure` prepares the next
call-overhead experiment. It is not a runtime change or accepted performance
result. The frozen accepted CLI passes **51/51 verification controls** across
VM/interpreter/JIT: scalar arities 0–4 at width 8, and array arities 1–4 at widths
8/256/8192, each with 128 warmup calls and 256 checked calls. Exact invocations,
identities and raw output are in `../native-call-entry-controls-2026-11-02.json.gz`.
Separate native-entry census controls also pass: increasing checked calls from
256 to 1,024 adds exactly 768 native function entries and 768 call dispatches in
each of nine configurations (five scalar arities, four array arities at width 8),
with zero additional compilation attempts. Array configurations additionally
add 768 owned-argument dispatches; scalar configurations add zero. All eighteen
instrumented controls pass. The census demonstrates native participation and
remaining dispatch, not CPU-time share or a speedup. Raw counters and source
identities are in `../native-call-entry-census-2026-11-02.json.gz`.
[Uninstrumented baseline scaling](../NATIVE_CALL_BASELINE_2026_11_02.md) now
verifies 102 samples. Candidate timing and optimized-path verification remain
pending.

`KIND` is `scalar` or `array`. Scalar arities are 0–4; array arities are 1–4.
Each kernel iteration calls the corresponding named target once. Targets return
their first argument (or seven at arity zero); additional arguments deliberately
isolate transfer/disposal overhead. The caller retains an array snapshot. Array
widths should include 8/256/8192 to separate call overhead from payload scaling.
The scalar one-argument control can exercise an existing specialized route;
it must not stand in for owned arguments or multiple arguments.

Setup, 128-call warmup, full result and snapshot checks are outside the timer.
The warmup crosses the current function-call threshold; the separate census
above verifies native entries for the width-8 controls. Verify the future
optimized generated path separately. This wrapper is
an altered diagnostic workload and must never replace official LangArena inputs.

Current source already has positional owned arguments, reusable argument vectors
and a bounded native-root pool. The generic generated Call still transfers the
callable and each argument through helpers to the VM stack, invokes
`jit_call_function`, then retrieves the result through another helper. The next
experiment must quantify and reduce remaining transfers/bridges rather than
claiming existing pooling as new work. Fusing transfers alone does not establish
a direct native known-callee path.

Before runtime edits, measure arity/width scaling with builds stopped. Add
separate argument-order/arity-error, recursion-depth, dynamic rebinding, field receiver,
callback/exception, cancellation and mutating-snapshot controls for the proposed
optimization. These read-only identity targets do not cover those contracts.
