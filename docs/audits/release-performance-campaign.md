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
initially corrected to twenty minutes for the complete 819-process runtime series;
workloads, sample counts and correctness assertions were not reduced. Pilot
samples are calibration evidence only and are excluded from final comparisons.

## Retained implementation and compatibility

The token estimator already counts characters to enforce its text limit, then
scans them again to assign ASCII weight 1 and non-ASCII weight 2. An exploratory
optimized probe compared that path with an ASCII-only shortcut using the first
count. The implementation reuses the validated character count only when
`text.is_ascii()` is true; all other text retains the existing scalar weighting.
Paired release measurements and the independent oracle tests evaluate that change.
Unicode estimation, model overhead, character limits, error text and fitting
policy remain unchanged.

Runtime source is pinned to candidate
`f4abf5dd265d5b866a2cfc99d82344c3a8ebbd5f`; subsequent changes affect tests,
CI and evidence only. No public API, CLI flag/exit code, configuration, environment
variable, serialization format, tool schema or dependency changed. Token counts
and context-selection results are identical; this is less estimator work, not
a reduction in model context or provider billing. No sibling repository change
is required.

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
Eight Node tests cover statistics, missing/corrupt/failed reports, unsupported
resources, driver failures/deadlines, provenance validation and evidence-directory
preservation. Clippy initially found one unnecessary clone in a new test; the
test now borrows that input, with no assertion change.

The first Windows campaign failed in the baseline's vendored OpenSSL dependency:
Git Bash selected its Perl, which lacked `Locale::Maketext::Simple`. The build
driver now runs under the runner's native shell environment, verifies Perl's
`IPC::Cmd` import and records the executable before invoking Cargo. The source,
locked dependencies and release profile are unchanged. A platform selector permits
a Windows-only retry while preserving completed Linux/macOS work; the failed
build log is evidence of the harness correction, not a runtime result.

No hosted-runner time budget is promoted to a regression assertion from one
campaign. Future corpus changes require reviewed expected results and a new
checksum-identified series; no timing result permits weakening correctness.
Raw workflow artifacts have 90-day retention, so reviewed evidence must also be
preserved in the repository's audit artifacts before expiry.

## Findings and disposition

| ID | Priority | Finding and evidence | Action | Status |
| --- | --- | --- | --- | --- |
| P01 | P2 | ASCII estimation repeats weighted scalar work after validation | Reuse the validated count for ASCII; preserve Unicode and bounds | Implemented; paired release evidence required |
| P02 | P1 | Prior no-JIT CLI command omits the binary’s required network feature; fresh build rejects it | Explicitly build the intended configuration before CLI parity tests | Fixed; 149 tests pass |
| P03 | P2 | Existing release runners lack a combined provenance/resource receipt and coverage for new native paths | Extend runtime corpus, preserve samples/checksums, add cross-platform driver and contract gate | Implemented |
| P04 | P2 | Initial Windows benchmark build selects incomplete Git Bash Perl | Native-shell build helper and Perl import preflight | Fixed in harness; Windows retry pending |

## Verification receipt

All local Cargo commands below use `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0`.
Debug/test commands additionally use `CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0`. Logs live in ignored `.audit-evidence/release-performance/`;
reviewed measurement archives below are durable repository evidence.

| Command | Outcome |
| --- | --- |
| `cargo build --release --locked --bin kujo` at baseline SHA | Passed; dependency rebuild 36m04s, not a comparable build-time measurement |
| `cargo test --test native_representation_matrix --test native_representation_parity --test token_estimation_release_contract` | Passed, 26 tests |
| `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_gate.sh --full` | Passed after removing one unnecessary clone in a new test; 3,043 passing receipts |
| `cargo run -- test --runtime vm` | Passed, 150 fixtures / 11 explicit skips |
| `CARGO_TARGET_DIR=.audit-evidence/release-performance/no-jit-feature-probe cargo build --locked --no-default-features --features runtime-archive,runtime-image --bin kujo` | Expected reproduction: exit 101, required `runtime-network` missing |
| `cargo build --no-default-features --features runtime-network,runtime-archive,runtime-image --bin kujo` | Passed; actually rebuilds no-JIT CLI |
| `cargo test --no-default-features --features runtime-network,runtime-archive,runtime-image --test vm_interpreter_parity_surfaces --test native_representation_matrix --test native_representation_parity --test token_estimation_release_contract` | Passed, 149 tests |
| `cargo build --bin kujo` | Passed; default debug configuration restored |
| `node --test tests/release_performance_campaign.test.js` | Passed, 8 tests |
| `cargo fmt --check` and `rustfmt --check scripts/runtime_measurement_release_bench.rs` | Passed; unchanged static runner’s pre-existing standalone formatting difference was left untouched |
| `git diff --check` | Passed |

## Provenance notes

Ending implementation/verification SHA:
`5a2521cf39b9bcecdc097c3ee13b2c2b58187b9a`. Runtime source matches measured
candidate `f4abf5dd265d5b866a2cfc99d82344c3a8ebbd5f`; the final evidence/roadmap
commit is identifiable in Git history. Initial branch and ending branch are `main`.

A hosted paired run can report `corpus_dirty: true` because the preceding A/A
run creates untracked evidence directories. Local final runs also see the audit
write-up in progress. The runner/driver hashes identify the exact apparatus;
no runtime or workload source changed between paired measurements. This flag is
retained rather than rewritten to imply a clean checkout. Empty compiler/stderr
files may be omitted by artifact upload; their original manifest hashes still
identify empty content. All present evidence hashes and reconstructed summaries
are checked before admitting an archive.

## Security and resource boundaries

The estimator still validates the two-million-character ceiling before its
shortcut, including text that fitting would drop. No capability gate, secret
handling or error contract is relaxed. Regression tests cover exact-limit and
over-limit ASCII and multibyte input.

Campaign inputs are trusted executables and authenticated workflow source SHAs;
this harness is not a sandbox. SHAs are validated, subprocesses use argument
arrays, evidence directories are exclusively created, binary hashes are checked
again after sampling, and failed/expired runs cannot produce success receipts.
Timeout cleanup targets the runner's owned process group/tree. The corpus uses
fixed work bounds and no live provider. Retained evidence must still be reviewed
before publication when future workloads contain private inputs.

The hosted macOS arm64 A/A run then provided a second concrete calibration:
its first ten workload families completed correctly in 847.58 seconds, including
375.0 seconds for sustained tasks and 376.5 seconds for promise observers.
It reached the similarly sized scheduler/channel workload when the twenty-minute
whole-corpus deadline expired. Baseline sustained-task samples were about
9.45 seconds each, so this deadline could not accommodate all three scheduling
families plus the rest of the fixed corpus. The final whole-corpus budget is
forty-five minutes, with a two-hour job budget for two builds and two full series.
This is a measured campaign-duration correction; no runtime timeout, task wait,
workload, sample count or assertion changed. The partial arm64 control remains
failed calibration evidence, excluded from accepted performance tables.
