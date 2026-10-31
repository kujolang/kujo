# Borrowed scalar-load experiment: rejected

The combined main-dispatch and compact-callback candidate is **not accepted**.
It removed `LoadLocal`/`LoadConst` opcode copies without changing value cloning,
but repeated NeuralNet JIT measurements showed a slowdown. Runtime source has
been restored to the accepted implementation. No runtime fix or release is
claimed by this experiment.

## Correctness and scope

The candidate passed 1,310 library tests, 22 targeted integration tests, and all
150 test-size checksum cases (50 each VM/interpreter/JIT). Exact test membership
was checked against all seven test executables. Eight manual tests were ignored;
their exact names match the accepted baseline. Formatting and diff checks passed.
The entire repository integration suite was not rerun.

The retained regression fixture checks scalar/owned values, independent nested
copies, fresh literals, callback copies, error recovery and actual JIT native
entry. It passed in all three runtimes on both the candidate and accepted CLI.
The standalone Rust regression executable was also linked to run the accepted
CLI directly. The candidate-only low-level instruction-position test is retained
in the archived patch, not added to accepted runtime source in this change.

## Initial paired measurements

Four alternating baseline/candidate process pairs per workload/runtime measured
the interval from `warmup-done` through the final run checkpoint. Inputs and
checksums are unchanged `test.json` cases. Percentages below are medians of paired
candidate/baseline changes, not ratios of pooled medians.

| Case | VM paired median change | JIT paired median change |
| --- | ---: | ---: |
| QuickSort | -10.8% | -4.9% |
| MergeSort | -11.9% | -3.0% |
| NeuralNet | -0.2% | +2.1% |

QuickSort improved in all four pairs in both modes. MergeSort VM had one slower
pair (+1.4%); NeuralNet VM had two slower pairs (up to +2.1%). NeuralNet JIT was
slower in three of four pairs (up to +8.3%). All raw observations and ranges are
retained; these are loaded-laptop diagnostics, not clean-host speed claims.

## NeuralNet JIT follow-up and disposition

Six additional alternating pairs all showed a slower candidate:
**+8.6%, +16.3%, +11.4%, +9.8%, +3.0%, +1.2%**. Combining the original four and
follow-up six gives nine slower pairs out of ten, with a **+5.7% median paired
increase**. The follow-up does not replace or discard the initial measurements.
All 60 comparison runs passed their workload checksums.

Variable laptop load prevents a universal regression estimate. It does not
justify accepting this candidate in the face of repeated adverse measurements
in both execution orders. The smaller opcode-extraction microbenchmark did not
establish whole-runtime improvement; this experiment demonstrates why the
whole-workload gate is necessary.

The cause is not yet isolated. Main dispatch, compact callback dispatch, compiled
code layout and host variation remain possibilities. Next, test the main-dispatch
change independently of the compact-callback change against the same accepted
baseline. Preserve the ownership/error controls and repeat the adverse workload
alongside QuickSort before accepting any variant. Do not treat the rejected
candidate binary in `target/release` as the accepted baseline.

## Reproduction and evidence

Base commit: `6cd96c07aaa6a04a8f25277f42e559182b689670` (runtime `fd7b764`).
Accepted CLI SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.
Rejected CLI SHA-256:
`3eb8bc8abe2d906ae6069f1a6c3bb1a3ba09788afdecdb322eeb7e142c4760af`.

The [evidence bundle](borrowed-scalar-loads-experiment-2026-10-31.json.gz) contains
source manifests, the rejected runtime patch, test output, baseline regression
receipts, artifact identities, all raw workload reports, collectors and independent
verifiers. `run-tests.rb` records the exact release-test command; `after-tests.rb`
collects checksums and initial pairs, and `neural-followup.rb` collects the follow-up.
`verify-results.rb` and `verify-neural.rb` independently check raw coverage,
checksums, provenance and timing calculations. The original candidate source
manifest describes the rejected build, not the restored working tree.

No production-size, cross-platform, Goose/Rust or clean-host result is claimed.
Sections 3/4 remain at their completed bounded cutoff; section 5 remains open.
