# Paired task-owner elapsed diagnostic

The baseline is accepted campaign revision
`cd5f2b207a8f907d6c263f5a2e405412965344ca`. The candidate is the exact dispatched
workflow revision. Both optimized, locked builds use the same Linux runner and
Rust toolchain, and are copied to distinct frozen executables before sampling.
No builds run during samples. This is a hosted CI diagnostic, not a dedicated
clean-host result or a Kujo/Rust/Goose comparison.

`probes/compare-owner-elapsed.rb` executes the same checked-in task fixture with
four repetitions of sixteen equal jobs. It covers VM, interpreter and JIT, each
with direct serial execution and parallel execution at 4, 8 and 16 workers.
Every sample checks task results, totals and input snapshots through the fixture,
then requires successful exit, its exact success line and empty stderr.

One warmup per variant/cell is excluded. Seven subsequent rounds alternate pair
order and rotate cell order. The report retains all 192 observations (24 warmups,
168 measured), commands, stdout/stderr, source/executable hashes, paired ratios,
medians and ranges. Hashes are checked before and after each sample. Runtime
measurement flags are absent and known diagnostic environment overrides are
rejected. Each child has a 120-second wall deadline and a 60-second scheduler
deadline; failure produces no completed comparison table.

Wall time includes process startup, compilation and checksum assertions. The
collector polls at 1 ms, so submillisecond differences cannot be resolved by
this protocol. Samples are paired observations, not independent clean-host
replicates. The VM/interpreter cells are controls for unrelated overhead; JIT
ratios alone cannot establish causality or an overall language speedup. Existing
instrumented compile-count evidence provides a separate mechanism check.

The collector controls verify complete pairing/warmup coverage, order variation,
absence of instrumentation, rejection of wrong output and diagnostic overrides.
CI preserves both executables, build records, source revisions, toolchain,
host/load/memory snapshots and raw samples. Artifact identity and the raw ratios
must be independently reviewed after completion before accepting a speed claim.

Dispatch the registered `vm-code-lifetime.yml` with `elapsed_only: true` and
`checksum_only: false`. This selects only the reusable elapsed workflow; it does
not rerun the platform matrix. This diagnostic is one candidate acceptance input;
memory, platform and production gates remain separate requirements.
