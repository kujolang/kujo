# QuickSort: native call costs, 2026-10-24

Two confirmed runtime costs are fixed, with **150/150 unchanged test-size
checksums passing** in VM, interpreter and opt-in JIT. A reliable elapsed-time
speedup is **not established** on this thermally throttled host. QuickSort remains
far from Goose and Rust.

## Changes and reproductions

- `9b5694f`: native callbacks now contribute to the existing hot-call policy.
  Before the fix, unchanged QuickSort compiled but its `idiv` pivot helper never
  did. A regression failed at the promotion threshold. Afterward, release debug
  evidence confirms `idiv` and `random_int` promotion from native callbacks.
  Existing closure, arity, unsupported-code and disabled-JIT guards remain.
- `50f8357`: valid native calls move their ordered argument suffix with one
  exact-sized allocation instead of repeatedly growing and reversing a vector.
  The pre-fix allocation regression failed at eight arguments. The extraction
  now uses one allocation rather than two at arity eight or four at arity 32.
  At QuickSort's arity three, extraction requests 408 rather than 544 bytes on
  this build; complete warmed bridge calls request 1,324 rather than 1,460 bytes.
  Existing malformed-stack consumption, rollback order and error codes remain.

The kernel, random inputs, warmups, iterations and checksums are unchanged. These
are general runtime changes, with no benchmark-name dispatch or native sorting
shortcut. Regressions cover retained snapshots, duplicate/negative elements,
empty/singleton arrays, argument evaluation order, bounds errors and recovery.

## Measured results

Before: frozen `faf9f95` (the pre-fix snapshot, **not** original `f037705`).
After: frozen `50f8357`, release with the same default features and thin LTO.
QuickSort uses unchanged `test.json`: 100,000 elements and two measured iterations.

Seven rotating rounds, six variants, 42/42 checksum passes:

| Variant | Median loop seconds | Median whole-process seconds |
|---|---:|---:|
| Before VM | 9.978 | 16.011 |
| After VM | 9.778 | 16.177 |
| Before JIT | 3.798 | 6.952 |
| After JIT | 3.449 | 6.201 |
| Goose | 0.022 | 0.054 |
| Rust | 0.019 | 0.048 |

The median **within-round before/after ratio** is 1.099x for JIT, with a
0.944–1.196x range; five of seven pairs favor the candidate. VM's control ratio
is 0.978x, with a 0.898–1.131x range. Paired ratios differ from the ratio of the
independent median times above.

A second seven-round run places only before/after JIT adjacent to reduce drift:
14/14 checksum passes, paired median **1.015x**, range **0.941–1.074x**; again five
of seven pairs favor the candidate. Independent medians are 3.615s before and
3.460s after. The smaller repeat does not substantiate a dependable 10% speedup.

Reported CPU speed limits fell from 71% to 57% during the six-variant run and
68% to 51% during the adjacent run. No builds, profiling or other benchmark runs
from this task overlapped either timing experiment. The laptop remained shared.
The structural allocation/coverage improvements are verified independently of
these noisy elapsed measurements.

In this session, the candidate JIT takes approximately **157x Goose's time and
182x Rust's time** for the measured loop. Comparator times have millisecond
precision. This is neither a controlled-host ranking nor a production result.
The timer excludes preparation, warmup, compilation and checksum work; whole
process times include them and are retained separately.

## Remaining work

A new three-second sample, verified against the candidate executable UUID and
linker map, still shows allocation/freeing, value movement, indexed-read helpers,
root bookkeeping and environment lookups. It includes startup/warmup and idle
threads; sample counts are not native CPU percentages or speed ratios. Memory
movement alone does not prove unnecessary whole-array copying.

The next candidates are repeated function-property scans during callbacks,
repeated debug environment reads, call/frame allocation and guarded indexed
access/root handling. Reproduce and measure each before changing it. Cache
invalidation, runtime environment changes, value semantics, errors and capability
behavior must remain correct. Full production, controlled competitive and
supported-platform acceptance remain open.

## Verification and evidence

- 1,086 library tests pass, eight ignored; 19 targeted integrations pass.
- Both new reproductions fail on their respective pre-fix implementations and
  pass after the fixes. Semantic fixtures run in VM, interpreter and JIT.
- Frozen release: all 50 test-size checksums pass in each mode, **150/150** total.
  Names, configurations, expected checksums and source/binary hashes are verified.
  JIT fallback is allowed; this does not claim all kernels execute natively.
- All 56 comparative samples pass. Formatting and diff checks pass.
- No production-size or cross-platform acceptance is claimed for this candidate.

[Raw results, source identities, profiles and verification](native-call-performance-2026-10-24.json)
retain all samples and all 150 checksum results. The six-variant runner is
[compare_quicksort.kujo](compare_quicksort.kujo); the exact executed sources for
both comparison drivers are retained in the JSON. No Kostya contact or upstream
submission occurred.
