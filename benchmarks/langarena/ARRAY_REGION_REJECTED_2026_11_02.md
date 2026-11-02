# Borrowed array regions: rejected and removed

The tactical helper-view experiment is closed without an accepted runtime change.
Its sole revision passed semantic gates but did not establish repeatable target
gains and consistently slowed fallback cases. Both candidate patches/binaries
and evidence remain preserved. No third tactical revision is planned.

The candidate changes to `src/jit.rs`, `src/native_owned.rs` and `CHANGELOG.md`
are removed. All **1,390 frozen source/build/test/vendor identities** match the
accepted baseline again, and `cargo fmt --check` passes. Committed fixtures and
measurement tooling remain. **Local `target/release/kujo` still contains the built
candidate**: use the frozen accepted CLI for baseline work, and rebuild before
treating target output as current source. Broader native-access work remains open.

## Verified fallback sweep

**504/504 samples** form 252 rotating pairs: six cases, three widths, VM/JIT
opt-in, seven pairs per cell. Each measured kernel performs 32,768 groups of
eight reads. Setup, warmup and result/snapshot assertions lie outside the timer.
These are synthetic diagnostics, not official suite scores.

Median accepted/candidate paired speedup (below one means slower candidate):

| Case | JIT width 8 | JIT width 256 | JIT width 8192 | Candidate-faster pairs |
| --- | ---: | ---: | ---: | ---: |
| Integer array | 1.089x | 1.109x | 1.183x | 21/21 |
| Negative-index array | 1.109x | 1.118x | 1.132x | 21/21 |
| Float array | 1.132x | 1.131x | 1.144x | 21/21 |
| Bytes | 0.869x | 0.872x | 0.880x | 0/21 |
| Integer dictionary | 0.894x | 0.885x | 0.889x | 0/21 |
| String dictionary | 0.966x | 0.953x | 0.990x | 1/21 |

All eighteen VM cell medians lie between 0.996x and 1.011x. Near-baseline VM
controls and consistent JIT fallback losses support rejection; host variability
does not justify disregarding them. No clean-host or significance claim is made.

For integer-indexed nonarrays, the candidate first validates a root and rejects
its nonarray representation, then generic fallback validates the cached root
again. Both validations reuse the slot hint, so the hash-lookup unit test passes
without eliminating repeated validation/branching. This is a source-confirmed
mechanism, not a measured CPU-time fraction. Do not assign the string-key result
the same cause without evidence.

The [target comparison](ARRAY_REGION_V2_COMPARISON_2026_11_02.md) likewise did not
establish repeatable gains: paired JIT medians were 0.914x QuickSort, 0.964x
MergeSort and 1.009x NeuralNet. Synthetic array gains do not override those
results or the fallback losses.

## Evidence

`probes/verify-native-region-scaling.rb` verifies exact coverage/order, invocation,
identities, raw timers, exits and output. Its first attempt rejected stderr:
all 252 JIT samples carry the exact top-level MakeClosure fallback notice, while
VM stderr is empty. The corrected verifier admits only that exact JIT notice
and rejects additional diagnostics. No measured sample was changed or discarded.

Twelve separate controls (six cases per binary) pass and show `repeated_reads`
compiling successfully and the current call retrying with native code. They are
not timing samples, CPU-share attribution or proof of whole-script native
execution. The debug label says "100 calls" even on first-call loop admission;
it is not a measured call count.

`array-region-v2-scaling-2026-11-02.json.gz` retains **1,014 hash-verified
artifacts**: raw outputs, original report, verified pairs, separate traces and
probe/runner/verifier sources. The [correctness receipt](ARRAY_REGION_V2_CORRECTNESS_2026_11_02.md)
retains the candidate patch and 150/150 unchanged test-size checksum results.

Next is a new bounded call-overhead experiment. Existing positional arguments,
argument buffers and native-root pools already provide reuse; target remaining
marshalling and VM transitions, without calling existing pooling a new fix.
