# Kujo performance implementation plan

Aim: make unchanged LangArena workloads competitive with Goose and Rust while
preserving Kujo's language contracts. Beating both languages on every workload is
a target to test, not a guarantee. This document explains the execution order;
[performance-plan.json](performance-plan.json) tracks all 50 cases and evidence.

## Current evidence and limits

- The JSON serialization change has a measured paired median improvement of
  2.916x on this laptop. That is one workload, not a suite-wide speedup.
- Nested assignment now avoids reproduced Arc allocation churn. Its noisy paired
  timings do not establish an end-to-end speedup.
- The latest recorded complete test-size run passed 50 VM and 50 interpreter
  cases on candidate `ebe4258`. It predates subsequent runtime fixes and cannot
  certify current HEAD. No complete current production-size run exists.
- Experimental JIT integer argument guards, callback errors and whole-function
  integer global assignments have regression coverage. General-value global
  handling, loop-JIT bindings, Boolean result types, checked overflow and catchable
  division errors remain open. Native float/general-value compilation remains
  unfinished.
- Shared-laptop contention and throttling prevent reliable idle-machine score
  predictions. A clean measurement environment is an acceptance requirement.

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
