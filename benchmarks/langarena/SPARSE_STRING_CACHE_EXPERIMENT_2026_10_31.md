# Sparse string-cache experiment: not accepted

The sparse cache removes the dense layout's unused-constant allocation penalty
and passes correctness checks, but this version is **not accepted** because the
MergeSort VM slowdown repeated in the follow-up. The revised dispatch is a new,
unverified candidate; none of these results establish its performance.

## Verified behavior and allocation results

- 1,312 library tests and 17 targeted integrations pass; eight existing library
  tests remain ignored. The tests cover VM/interpreter/JIT, recursive/imported
  calls, callbacks, copied strings, host bytecode edits and concurrent code users.
- All 150 test-size benchmark checksums and 60 comparison-run checksums pass.
- 24 independent semantic runs and 186 allocation observations cover hot reads,
  cold reads, append and 1/4/16-key access. Warm counts repeat exactly.
- VM hot literal loads save two requests and key-width + 40 requested bytes per
  read. Native JIT has no corresponding per-read allocation difference.
- Cold phases remain flat across 0/128/2,048 unused integer or string constants.
  The measured phase uses 4,552 requested bytes versus 4,613 baseline; the fixed
  marker saving must not be attributed to the workload's literal reads.
- The 16-key, eight-byte cold phase adds 335 requested bytes for its first two
  loop iterations. This phase includes marker/call costs. With 2,000 iterations,
  the same case saves 1,536,000 bytes attributable to literal loads, plus 61 bytes
  of fixed phase overhead. Cached entries grow with accessed strings.
- Empty bytecode owners request and retain 472 bytes versus 432 baseline: +40
  bytes per owner, including code that never reads strings. All tracked bytes
  are released on drop in the 0/1/128/2,048-owner controls. These are requested
  allocator bytes, not RSS or fragmentation measurements.

Accessed string payloads remain cached with their code owners. Public constant
representation stays unchanged; public bytecode mutation invalidates the cache.
The first string is stored inline, with a lazy locked hash map for further
strings. These lifetime and locking costs remain explicit tradeoffs.

## Paired workload results

Four alternating pairs per workload/runtime; signed medians show candidate
change relative to accepted `fd7b764` (negative means faster):

| Workload | Runtime | Warm kernel wall | Whole-process-tree CPU |
| --- | --- | ---: | ---: |
| QuickSort | VM | -3.2% | -2.2% |
| QuickSort | JIT | +0.6% | -1.4% |
| MergeSort | VM | +2.2% | +1.7% |
| MergeSort | JIT | -2.9% | -4.3% |
| NeuralNet | VM | -12.8% | -12.2% |
| NeuralNet | JIT | +0.1% | +0.3% |

NeuralNet VM improves in all four pairs. MergeSort VM's initial first pair is
+39.0% wall/+44.0% CPU. Six additional alternating MergeSort VM pairs are all
slower in warm wall time; five are slower in whole-process CPU. Their warm-time
median is +10.7%. Combining **all ten pairs**, including the original outlier,
gives +7.6% warm wall and +8.9% CPU medians (nine wall and seven CPU slowdowns).

The overloaded laptop limits causal conclusions; the repeatable adverse pattern
is nevertheless sufficient to withhold acceptance. CPU counters include startup,
compilation, warmup and execution, unlike the warm-kernel wall interval. No
clean-host, production-size or competitor claim follows from these observations.

## Attribution and next candidate

The symbolized allocation probe linked to this candidate library retains an
outlined `load_constant_value` wrapper. Its non-string branch sets up an
88-byte stack frame and saved registers before restoring them and tail-calling
the original converter. The disassembly is preserved. MergeSort's inner loops
load numeric constants repeatedly. This identifies unnecessary machinery worth
removing; it does not prove that it explains the entire observed slowdown.

The next candidate moves the kind check into a small forced-inline dispatch
helper and keeps string cache machinery out of line. It requires fresh tests,
allocation checks and workload comparisons before acceptance.

## Provenance

Candidate CLI SHA-256:
`6aca57c5d7f12210b52bbbebad089c1ad03a688cf86e4fd0b89c1ca86bb26d24`.
Library SHA-256:
`49c9b3f7564a8ce63fde3cecbb558536cb4375b50415a9f56ef553e68c1c34d1`.
The evidence bundle preserves the patch against `f347d35`, source manifests,
test membership, allocation controls, all raw paired outputs, footprint evidence,
disassembly and verifiers. Runtime verifiers require the matching candidate
source; the current dispatch revision intentionally differs. The CPU-accounting
control was reused from the verified same-host experiment, with bundle digests
and explicit provenance; it was not represented as a newly run control.
