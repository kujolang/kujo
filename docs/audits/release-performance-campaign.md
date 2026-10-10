# Release performance and resource campaign

Roadmap item 3. Runtime baseline: `42269d9b3f924736a2f995e8334ec2e7025cc1d2`
on `main`, immediately after the native representation parity campaign. Stable
version remains 1.8.1; no new release is published here. The preceding full
release gate established 3,041 passing test receipts, zero failures and 18
existing ignores, with all three runtime fixture sweeps passing. See
[native representation evidence](native-api-representation-parity.md).

## Reproducible apparatus

The existing Rust release runtime runner now includes 21 workload families;
the existing static runner contributes startup, import-aware project checking
and LSP diagnostics. Each runtime sample checks an independent expected stdout,
empty stderr and exit success before accepting timing. Existing baseline versus
candidate byte equality remains an additional check.

The Node standard-library maintenance wrapper compiles those Rust runners,
records exact binary and workload SHA-256 hashes, runtime source commits, compiler,
CPU, OS, architecture, profile declaration, raw observations and statistics.
It preserves evidence in a new directory and emits one small receipt. Sources,
raw samples, instrumented reports and environment metadata remain inspectable.
No production dependency was added. [Commands and methodology](../../benches/README.md)
are the operational source of truth.

Runtime sampling rotates baseline, candidate and instrumented candidate through
two discarded warmups and eleven measured rounds. Static sampling alternates
baseline/candidate through two warmups and twenty-one measured rounds. Statistics
retain all measured samples and report min, median, nearest-rank p90, max, mean
and population standard deviation. Paired candidate deltas use the same round.
An A/A control runs the same immutable binary in both slots to expose noise;
its differences are never called improvements.

These are warm-cache fresh processes. Sustained loops amortize startup but do
not remove it from reported process latency. Instrumented phase and inclusive
VM counters provide a separate view; inclusive VM time may count nested work
more than once. Neither result is an exclusive per-function CPU profile.

CPU and process-lifetime peak RSS are read through the existing collector on
supported Unix platforms; Windows null values are explicitly unavailable.
They describe instrumented runs, not the uninstrumented timing samples. Total
heap allocations, retained heap, syscall counts, network-request counts and
child-process counts remain unmeasured. Shallow capture counters do not establish
retained-memory size. These controlled workloads use no live model provider.

Builds use `cargo build --release --locked --bin kujo`, the existing thin-LTO,
single-codegen-unit profile, and `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0`.
The local baseline required dependency rebuilding after the prior disk recovery;
additional regenerable dev package artifacts were cleaned before this build.
Build logs are retained, but build duration is not compared across differing
cache states. No build-speed claim is made. Hosted and local hardware/power/storage
are not normalized, so results are internal regression evidence, not universal
performance promises or cross-platform rankings.

The first control run exposed an undersized five-minute whole-corpus driver
budget. For example, the pilot's sustained calls and generator cases took about
2.5 and 1.6 seconds per sample, respectively, before the remaining workload
families. The driver timed out, terminated its owned process tree, preserved
partial evidence, and emitted no successful receipt. The campaign budget was
corrected to twenty minutes for the complete 819-process runtime series;
workloads, sample counts and correctness assertions were not reduced. Pilot
samples are calibration evidence only and are excluded from final comparisons.

## Candidate under verification

The token estimator already counts characters to enforce its text limit, then
scans them again to assign ASCII weight 1 and non-ASCII weight 2. An exploratory
optimized probe compared that path with an ASCII-only shortcut using the first
count. This identifies a candidate; only full-runtime paired release measurements
and contract tests can justify retaining it. Unicode estimation, model overhead,
character limits, error text and fitting policy must remain unchanged.

## Regression policy

The manual `release-performance` workflow builds exact commits on Linux x64,
macOS arm64 and Windows x64. Comparisons build baseline then candidate with the
same toolchain and profile, retain both binary hashes, run a baseline A/A control,
and then run the paired comparison. The contract job rejects failed/corrupt
measurement input and tests the statistics without timing thresholds.

The campaign also found a pre-existing no-JIT verification gap: `kujo` declares
`runtime-network` as a required Cargo feature, while the previous parity command
omitted it. A fresh target-directory build reproduced Cargo's required-feature
error. In a reused target directory, CLI tests could instead execute the earlier
default-feature binary. CI now explicitly builds the CLI with
`--no-default-features --features runtime-network,runtime-archive,runtime-image`
before running parity and token contracts. That configuration passed 149 tests
(123 existing parity, 6 direct native, 18 CLI and 2 token boundary tests).
The earlier audit is corrected to distinguish its valid native/in-process
results from the insufficient evidence for its CLI build configuration.

## Functional verification

The default-feature full release gate passed: formatting, all-target/all-feature
Clippy, 3,043 passing test receipts, zero failures and 18 existing ignores,
focused security/package/parity/socket suites, dual/interpreter fixtures and
dependency audit. The test total includes one nested subprocess receipt, as in
the 3,041-receipt starting baseline. A separate VM-only sweep passed 150 fixtures;
dual and interpreter also passed 150 each, with 11 explicit skips per sweep and
zero dual fallback. Optional cargo-deny remains unavailable; the existing
documented build-only bincode audit exception is unchanged.

The new token tests compare an independent weighted-scalar oracle across ASCII,
control characters, multibyte text, combining marks, model families and string
lengths. Exact-limit and over-limit cases retain character-based bounds and
exact errors, including messages context fitting would otherwise discard.
Seven Node tests cover statistics, missing/corrupt/failed reports, unsupported
resources, driver failures/deadlines, provenance validation and evidence-directory
preservation. Clippy initially found one unnecessary clone in a new test; the
test now borrows that input, with no assertion change.

No hosted-runner time budget is promoted to a regression assertion from one
campaign. Future corpus changes require reviewed expected results and a new
checksum-identified series; no timing result permits weakening correctness.
Raw workflow artifacts have 90-day retention, so reviewed evidence must also be
preserved in the repository's audit artifacts before expiry.
