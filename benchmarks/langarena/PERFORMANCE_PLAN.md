# Kujo performance implementation plan

Aim: make unchanged LangArena workloads competitive with Goose and Rust while
preserving Kujo's language contracts. Beating both languages on every workload is
a target to test, not a guarantee. This document explains the execution order;
[performance-plan.json](performance-plan.json) tracks all 50 cases and evidence.

## Current evidence and limits

- The [tagged-owned candidate](checksums-2026-10-23-owned-values-jit.json) at
  `e2797da` passed all 50 test-size JIT checksums, with frozen input/binary identity,
  raw report hashes and measurement schemas verified. 34 cases recorded native
  entries; fallback remained allowed. This debug diagnostic run predates operand
  cleanup, overlapped local tests/builds and provides no competitive timing evidence.
  Its separate VM/interpreter runs are not included in that result.

- The JSON serialization change has a measured paired median improvement of
  2.916x on this laptop. That is one workload, not a suite-wide speedup.
- Nested assignment now avoids reproduced Arc allocation churn. Its noisy paired
  timings do not establish an end-to-end speedup.
- The latest verified test-size VM and JIT runs passed all 50 cases at `739dd89`;
  the latest complete interpreter run is `e2797da`. These frozen candidates
  predate subsequent constructor/operator fixes and cannot certify current HEAD.
  No complete current production-size run exists.
- All 50 test-size cases passed with JIT enabled at `33a85f9` using a ten-minute
  per-case debug deadline. VM fallback was allowed. This establishes neither full
  native coverage nor speed acceptance. The earlier `c4a07cd` run and its MergeSort
  timeout/retry remain in the evidence history.
- Experimental JIT immediate argument guards, callback errors, whole-function
  integer global assignments and Boolean return/callback tags have regression
  coverage. Native scalar operand type guards also have regression coverage.
  Native short-circuit joins now carry operand payloads and type tags; regressions
  cover native execution, mixed stack joins and direct recursion. Cold callbacks
  now restore recursion depth after return. Boolean/integer named global loads and
  whole-function stores preserve scalar types. Float/null immediate values now
  have native payloads, float arithmetic/comparisons and typed loop-local
  persistence. Function arguments now carry immediate kinds through cached calls,
  callbacks and direct recursion. String/object values, collection helper boundaries
  and full loop/script binding parity remain open. Checked native
  integer arithmetic now has overflow/divisor guards and regression coverage.
  General-value compilation and owned-value native entry remain unfinished.
- Shared-laptop contention and throttling prevent reliable idle-machine score
  predictions. A clean measurement environment is an acceptance requirement.
- The [native census](NATIVE_EXECUTION_CENSUS.md) at `0c77115` passed all 50
  test-size checksums with validated measurement reports. 34 cases recorded native
  function entries and 16 did not; this is not full native coverage. A QuickSort
  CPU sample identifies VM dispatch, opcode cloning and call-frame setup costs.
- The [native lifetime-hook run](checksums-2026-10-23-native-root-lifetimes-jit.json)
  at `7f89e13` passed all 50 test-size checksums. Binary/config/port/runner hashes,
  all raw report hashes and all measurement schemas were verified. Four native
  entry/dispatch counters match the earlier census for every case. This debug
  run allowed fallback and overlapped local builds; it supplies no speed evidence.
  It predates the `0f4934f` lazy-scope guard and does not close the eight owned-value
  failures, current-HEAD verification or production gates.

## Non-negotiable rules

Keep algorithms, inputs, iterations, warmups, RNG behavior and checksums unchanged.
No benchmark-name special cases, precomputed answers, omitted work, or native
kernels written just for a benchmark. General-purpose optimizations must have
independent semantic and cost tests. Preserve value semantics, alias isolation,
binding rules, errors, cancellation and capability enforcement.

Keep VM, interpreter and JIT evidence separate. A fallback may preserve correctness
but is not evidence of faster compiled execution. Timeouts and crashes stay visible.
No Kostya contact or upstream submission is authorized by this plan.

## 1. Establish a trustworthy baseline

- Pin source commits, executable hashes, compiler versions, build flags and the
  unchanged upstream suite revision for Kujo, Goose and Rust.
- Complete the current candidate's 50 test-size cases in both runtimes before
  interpreting its timings. Record explicit JIT results separately.
- Run all 50 production cases with recorded deadlines and memory limits. Save
  partial results; distinguish wrong result, crash, timeout and successful run.
- Use a quiet, adequately cooled host. Run samples sequentially, without our own
  build or profiling jobs. Record power, throttling and competing load.
- Collect at least seven valid samples per comparison, alternate baseline and
  candidate order, and retain raw measurements. Separate startup, preparation,
  warmup, measured work, total elapsed time and peak resident memory.

Gate: reproducible reports for every case, with missing results explicitly marked.
No estimated idle-host timings qualify as a baseline or an acceptance result.

## 2. Find where time and allocations actually go

- Profile representative numeric loops, recursion, sorting, collections, strings,
  parsing, serialization, graphs and parallel work. Extend to every outlier.
- Attribute CPU time to actual execution paths; measure allocation count/bytes,
  copying, reference-count traffic and peak memory separately.
- Measure successful native compilations, native entries, fallback reasons and
  coverage by function/loop. Compilation attempts alone do not prove JIT coverage.
- Add isolated reproducers and input-size scaling measurements for each suspected
  cost. Distinguish linear work from accidental quadratic copying.

Gate: every optimization has an observed hotspot, a failing semantic or cost
reproducer where applicable, and an unchanged end-to-end workload to validate it.
Prioritize by measured potential benefit and implementation risk.

## 3. Finish the compiled-execution correctness foundation

Work in this order, with a separate small commit for each coherent change:

1. Fix JIT global stores through checked binding resolution. Cover mutable globals,
   local shadowing, readonly globals, changing value types and nested callbacks.
2. Preserve result types across native boundaries, including Boolean versus integer
   results. Audit all supported argument and return representations.
3. Add checked native arithmetic and catchable errors: overflow, division/modulo
   by zero and signed boundary cases. Preserve effects before the failure, prevent
   effects after it, and never replay failed work.
4. Audit every native helper's status/error path, stack/frame state and callback
   boundary. Verify locks, captures, recursion and imports under repeated calls.
5. Add differential generated programs comparing values, types, output and errors
   across interpreter, VM and JIT, including failures after hot-loop compilation.

Gate: new regressions fail on the baseline and pass on the fix; existing relevant
contracts and the library suite pass. JIT stays opt-in until broader acceptance.

The [dictionary constructor audit](native-dictionary-constructors-2026-10-23.json)
adds native dynamic-key and unique fixed-key construction with tagged operands,
retained key-vector lifetime and VM nesting/error checks. Repeated constant keys
now preserve source-order evaluation and last-value-wins semantics in the VM;
that form still falls back because its string constants are not native operands.
Numeric literal-key coercion remains a separate VM/interpreter compatibility gap
(recorded in the audit); no suite speedup or production acceptance is claimed.

The [owned operator audit](native-owned-operators-2026-10-23.json) fixes native
string arithmetic/ordering and preserves struct operator dispatch and errors
through the reference VM. Immediate numeric paths remain native. Constants remain
gated until retained literal lifetimes and specialized local-helper behavior are
verified; bounded root storage is not evidence of reduced string copying or speed.

### Remaining native ownership gate

Converted operand paths now release Pop, overwritten SSA locals, VM-marshaled
callable/arguments and consumed equality/logical operands. Dup and SSA local loads
receive distinct roots. Compiled discard/overwrite/callback tests keep root-slot
capacity at most four at both 8 and 128 repetitions. This is deterministic storage
evidence, not a suite speedup or proof of every opcode's lifetime behavior.
`ReleaseDeadLocal` now has native lowering, caller-slot isolation, Null readback
and copied-parameter compilation regressions. The bounded-storage test also covers
dead-local release while a separate operand copy remains live. Legacy collection
helpers, loop-entry/local persistence and wider owned-entry coverage still require
their own checks before the gate closes.

The latest [borrowed-equality audit](jit-owned-equality-2026-10-23.json) passes all
11 focused probes in interpreter, VM and JIT modes with native entries verified.
Owned comparisons now use `Value::equals` instead of handle identity. This closes
the expanded equality reproductions below, but does not establish general native
coverage, dead-operand lifetime bounds, production correctness or speed acceptance.

Earlier [named-operand audit](jit-owned-loads-2026-10-23.json): ordinary named loads,
callable marshaling, collection callback arguments and named assignments now use
tagged owned values. All eight original probes agree across the three modes.
Three added equal-value probes still fail: comparing handles happens to pass the
original unequal-string check but breaks identical strings and structurally equal
arrays/dictionaries. Equality remains open, alongside dead-operand lifetime work,
legacy specialized collection paths and broader native coverage. Older audit
counts below describe their recorded candidates, not this latest state.

The [owned-value lifetime foundation](../../docs/NATIVE_OWNERSHIP.md) now provides
recyclable roots, frame-local cleanup and explicit return promotion. Native
boundary/function lifetime hooks are connected, with lazy storage for immediate
paths. Generated owned operations still need integration; mechanism and ABI-probe
tests are not a language-level parity pass. Tagged VM-stack conversion now uses
the codec for callback results and argument marshaling. The refreshed
[owned-stack audit](jit-owned-stack-2026-10-23.json) repairs the native callback
return probe; seven original mismatches remain. Dead operand and overwritten-local
release is still required before claiming bounded storage for long owned loops.

The [owned-value audit](jit-owned-values-2026-10-23.json) reproduces eight native
mismatches on `0c77115`, with VM/interpreter agreement and measured native entries
for every probe: string/array/dictionary returns, native callback returns and
arguments, bytecode callback arguments, string equality and empty-string truth.
The [reproducer](probes/native_owned_values.kujo) records failures rather than
treating its own successful exit as a parity pass. These are current correctness
gaps in that baseline, even though the earlier all-50 benchmark checksum run passed
with fallback. The refreshed audit above tracks the one repaired case separately.

The string-global audit still returns an integer marker after compilation. Fix the
complete owned-value path before widening owned argument guards or claiming typed
native parity: named loads, callable values, locals, operand-stack joins, operators,
callback arguments/results, assignments and returns must agree on representation.
Use the reference VM for operator/capability/error contracts that need a runtime
helper; retain native immediate operations where their guards prove the types.

Ownership storage must scale with live values, not loop iterations or the total
number of loads. Verify release at pops, overwritten/dead locals, normal returns
and every error exit, including recursion and nested callbacks. An append-only
handle vector lasting for a whole invocation is not an acceptable final design.
Keep alias snapshots and copy-on-write behavior intact across those boundaries.
The dead-local path now implements lifetime semantics and copied-parameter loops
have native compilation coverage; wider owned entry remains gated. Add bounded-live-storage and allocation/scaling
checks alongside both-runtime semantic tests before claiming a performance win.

## 4. Remove measured VM and representation costs

Investigate these as candidates, not as assumed defects:

- Call setup, argument preparation, local/global lookup and redundant validation.
- Collection ownership across calls/imports; field/index updates; append/pop/delete;
  nested rows and structs. Retain copy-on-write isolation for aliases and snapshots.
- Numeric dispatch and boxing in tight loops; byte access and string scanning.
- JSON parsing/mapping/serialization, temporary buffers and repeated conversions.
- Allocation lifetimes, reference counting and worker scheduling overhead.

Use borrowing, ownership transfer, reusable storage, specialized bytecode or compact
representations only where profiling and semantics justify them. Do not introduce
an arena, collector or broad representation rewrite without evidence that its
benefit exceeds its complexity and compatibility cost.

Gate: deterministic cost/scaling improvement plus unchanged semantics. Claim a
speedup only after paired release measurements; retain negative results too.

## 5. Expand fast compiled execution

- Add guarded typed paths for the numerics and collections found in real profiles,
  including floats where workloads require them.
- Reduce VM/native crossings, redundant loads and boxing on proven hot paths.
- Extend calls, recursion, imports and captures with correct ownership and errors.
- Evaluate inlining and loop optimizations after type guards and exit state work.
- Measure compilation/startup costs and native coverage. Consider caching or AOT
  only if those costs or platform constraints justify the additional system.

Gate: hot paths demonstrably execute native code, pass differential tests and show
repeatable end-to-end improvement. Correct fallback must resume from valid state
without duplicating side effects. Never remove safety checks merely to win a score.

## 6. Verify all 50 production workloads

For each candidate milestone, update the existing 50-row checklist with source and
binary identity, mode, checksum, sample count, elapsed time, peak memory, comparator
ratios and unresolved limitations. Re-run all test-size cases in VM and interpreter;
complete production VM coverage and explicit interpreter/JIT coverage separately.

Gate: 50/50 unchanged production checksums in each mode claimed as verified, no
unclassified failure, and no substitution of historical evidence for current code.
Production interpreter timeouts remain incomplete coverage even if VM succeeds.

## 7. Enforce competitive and regression gates

For each case, divide Kujo's time by the faster measured Goose/Rust time. Track
milestones of at most 2x, then 1.25x, then below 1x with repeatable evidence.
Publish the entire distribution and worst cases; an aggregate cannot hide failures.

- Investigate repeatable slowdowns above 5% on controlled-host measurements.
  Treat noisy measurements as inconclusive, not as automatic passes or failures.
- Track startup and peak memory alongside execution time. Record and justify any
  memory tradeoff; do not exchange unbounded memory growth for lower elapsed time.
- Keep quick semantic/cost gates in ordinary CI and the full performance matrix on
  a controlled scheduled runner. Recheck apparent wins in independent sessions.

Gate: no unexplained correctness failure or repeatable regression; competitive
claims identify exactly which cases, modes, builds and hosts support them.

## 8. Release and integration acceptance

- Run formatting, build checks, library and relevant integration suites, language
  and CLI contracts, runtime parity and all-fifty checks on the final candidate.
- Verify release builds on supported operating systems and architectures. A macOS
  Intel result does not certify Linux, ARM or Windows behavior/performance.
- Validate local official-driver timing protocol and packaging before submission.
- Keep maintainer decisions separate: typed JSON mapping versus DOM conversion,
  indexed graph representations, worker-pool lifecycle and accepted build flags.
  Prepare concrete evidence and questions; obtain authorization before contact.

Gate: evidence attached to the release candidate, remaining limitations explicit,
and experimental features promoted only after their own reliability/platform gates.

## Work contract for each implementation batch

Reproduce → add regression → change the smallest responsible path → run both-runtime
and affected JIT checks → measure deterministic costs → build and measure release
candidates → update the checklist → commit and push a small reviewed change.

Batch expensive release builds across coherent tested fixes, but never label an
older binary as the current candidate. If a hypothesis fails, record the result and
choose the next measured hotspot. Finishing the plan means satisfying its gates,
not merely landing code in every area.
