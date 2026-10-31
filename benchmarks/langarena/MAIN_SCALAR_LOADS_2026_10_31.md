# Main-dispatch scalar loads: preserved for clean-host review

The main-dispatch-only variant is **deferred, not accepted**. It consistently
improved the three measured VM workloads, but QuickSort JIT remains uncertain.
The tested code is preserved on `codex/main-scalar-load-experiment-20261031`,
commit `036032535ad4bdf540e9c1b180c33dae3f34d517`. The accepted-runtime branch
retains the runtime from `fd7b764`; this report does not promote the experiment
or claim a release/performance guarantee.

## Change and correctness

The experiment borrows numeric operands of `LoadLocal` and `LoadConst` before
main-dispatch opcode cloning. Loaded values still use their original cloning or
constant-conversion paths, including collection values. Instruction-pointer
advancement before failures, stack behavior and cancellation polling are retained.
The compact callback dispatcher is byte-for-byte unchanged from base
`f010aa6244ac0822f5cda912fa41fa8f91d2d6aa`; its hash is recorded separately.

The candidate passed:

- 1,310 library tests and 22 targeted integration tests. Exact membership was
  checked against all seven executables. Eight existing manual ignores match
  the accepted baseline exactly; no new tests were ignored.
- All 150 test-size benchmark checksums: 50 each in VM, interpreter and JIT modes.
- All 72 comparison-run checksums, with unchanged configurations and workload sources.
- Formatting, diff and source/artifact identity checks.

Coverage includes owned-value independence, fresh literals, callback copies,
error recovery, actual JIT native entry, and malformed-bytecode error/IP/stack
contracts. The full repository integration suite was not rerun. The new low-level
unit test is part of the experimental commit; the portable regression fixture
was already committed with the preceding combined-dispatch experiment.

## Paired results

Each pair alternated baseline/candidate order. Three workloads ran four pairs
per mode. NeuralNet JIT also had six preceding screening pairs; QuickSort JIT had
six subsequent follow-up pairs. All observations are retained. The table uses
medians of paired candidate/baseline changes, not pooled time ratios.

| Case | Mode | Pairs | Warm-kernel wall change | Process-tree CPU change |
| --- | --- | ---: | ---: | ---: |
| QuickSort | VM | 4 | -16.0% | -14.6% |
| MergeSort | VM | 4 | -15.4% | -15.4% |
| NeuralNet | VM | 4 | -7.6% | -9.2% |
| QuickSort | JIT | 10 | +3.0% | +1.2% |
| MergeSort | JIT | 4 | -1.7% | -0.4% |
| NeuralNet | JIT | 10 | -0.3% | -1.5% |

All 12 VM pairs improved in both metrics. QuickSort JIT was slower in six of ten
pairs in both metrics: wall changes range from -8.3% to +12.1%, and CPU changes
from -3.5% to +8.2%. NeuralNet JIT has five faster/five slower wall pairs and six
faster/four slower CPU pairs. Its +11.3% wall/+8.9% CPU screening outlier remains
in the combined results. MergeSort JIT has one slower pair out of four.

Warm-kernel wall time spans `warmup-done` through the final run checkpoint.
Process-tree CPU time includes the runner and child startup, compilation, warmup
and execution. These are different measurement scopes. Direct and nested child
controls verify CPU accounting against the children's own CPU totals; the earlier
weaker nonzero-only control is retained as superseded. CPU data helps distinguish
descheduling from execution, but does not remove thermal, cache or host-load
variation. JIT means the CLI's opt-in mode, including supported VM fallback;
these runs do not quantify native CPU-time coverage.

## Disposition and remaining gate

The VM benefit is promising. The QuickSort JIT result is insufficient to claim a
regression-free improvement, and this overloaded laptop is also insufficient to
establish a universal regression. Preserve the variant for comparison with the
accepted baseline on the planned clean host. Do not discard slower observations
or silently trade JIT performance for VM gains.

The prior combined-dispatch candidate remains rejected on its own evidence.
This experiment does not prove that callback dispatch alone caused its slowdown:
compiled code layout and measurements from different periods remain confounders.
Other work can continue from the accepted runtime while this variant waits for
clean-host review. Sections 3/4 retain their completed bounded cutoff; broader
section 5, production/platform checks and competitor comparisons remain open.

## Evidence and reproduction

The [190-artifact bundle](main-scalar-loads-2026-10-31.json.gz) contains the exact
candidate patch, source manifests, tests, all raw comparison/checksum reports,
CPU controls, collectors, verifiers and disposition bound to the experimental
commit. Its membership and content digests were checked. The package verifier
reads the candidate source from the immutable commit, so it works after returning
to the accepted-runtime branch. Runtime-result verifiers require the matching
experimental checkout to validate the build source manifest.

`run-tests.rb` records the release-test command. `freeze-after-tests.rb` verifies
suite membership and freezes artifacts. `neural-followup.rb` performs the initial
screen, `measure-workloads.rb` runs all 150 checksums and 48 broader comparisons,
and `quick-followup.rb` addresses the subsequent QuickSort concern. The independent
result verifiers and both combined-review scripts preserve every paired ratio.

Accepted CLI SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.
Experimental CLI SHA-256:
`9803f7a7d1e92b1ea49827cc18e616b6131626c5cb0bdbac20364517480bf996`.
Experimental library SHA-256:
`95e710808af1a279fb7758db1bc8b8e4df70c9ec867dcad87f6fb08507b66a7b`.

`target/release` still contains experimental build artifacts. Use the saved
accepted CLI/library in `.audit-evidence/native-field-abi-2026-10-31` as the
accepted baseline, not whatever binary happens to occupy that build directory.
No clean-host, production-size, cross-platform or Goose/Rust score is claimed.
