# Warmed indexed-read allocation isolation

Accepted runtime `26105f4` remains unchanged. The warmed cached native read helper
made zero allocation requests in all 12 measurements: three rounds of 100,000
reads for integer arrays, float arrays, a string-keyed float dictionary and nested
array results. Root setup and 100 warmup reads per case are excluded. Nested
results receive independent roots and are released each iteration. The scalar
checksum, returned kinds, successful releases and absence of runtime errors are
asserted. Nested payload equality is not checked by this diagnostic.

The counter's separate positive control records one request for 4,096 bytes.
The helper binary links against accepted library SHA-256
`8becd18b137d9267eddd620e6ecb69f2b01a5e449fe3cffce8c6614df64a09f5`.
Compilation uses `rustc --edition=2021 -O -C lto=thin --cfg cached`, the saved
library via `--extern kujo=.../libkujo.rlib`, and `-L dependency=target/release/deps`.
The saved compiled source differs from the committed probe only in a usage comment.

## Source-program controls

Four source fixtures compare repeated scalar-variable and integer-array reads at
200 and 2,000 iterations. Both shapes request exactly the same VM allocations at
each size: 446 requests / 34,948 bytes and 4,046 requests / 45,748 bytes. The eight
VM/JIT allocation runs and four interpreter executions pass their expected sums.
A separate eight-run CLI census verifies one native function entry and one compile
entry in every JIT case, with neither in the VM cases. This confirms execution of
the small source programs; it does not establish benchmark CPU-time coverage.

The original JIT whole-execution counts include compilation and setup. The helper
isolation excludes both, narrowing the next action: do not attempt an allocation
fix for these warmed cached read paths based on the earlier whole-process totals.
Hashing, root bookkeeping, indexed writes and call dispatch can still consume CPU
without allocating. Attribute those costs on a current workload before a fix.

## Verification and limits

The collector checks exact shape/count coverage, exit status and artifact hashes.
The evidence bundle preserves all raw outputs, source fixtures, counter control,
CLI census and original exploratory counts. Formatting and Ruby syntax checks pass.
No runtime source changed, so the full runtime suites were not rerun for this probe.

Only the cached helper variant was measured here. Setup, growing live-root sets,
uncached reads, mutations, escaping results and other value types are outside this
measurement. Diagnostic elapsed times include warmup, counter overhead and laptop
contention; they are not benchmark scores. Zero allocation does not mean zero CPU
cost, universal zero-allocation indexing or a speed improvement.

[Raw evidence](indexed-read-allocation-screen-2026-10-31.json.gz).
