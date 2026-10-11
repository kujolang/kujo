# Release performance and resource campaign

Repository: `kujolang/kujo`, the Rust VM/interpreter and CLI for the Kujo language.
This campaign exercises native data/AI mechanisms, Tokio-backed task execution
and CLI/static analysis. It uses the existing locked dependency graph and adds
no runtime, build or npm dependency.

Roadmap item 3. Runtime baseline: `42269d9b3f924736a2f995e8334ec2e7025cc1d2`
on `main`, immediately after the native representation parity campaign. Stable
version remains 1.8.1; no new release is published here. The preceding full
release gate established 3,041 passing test receipts, zero failures and 18
existing ignores, with all three runtime fixture sweeps passing. See
[native representation evidence](native-api-representation-parity.md).

Status: completed for roadmap item 3; item 4 is ready. Accepted controls and
comparisons pass in Linux x64, local macOS x64, hosted macOS arm64 and Windows x64.

## Measured outcome

The retained ASCII estimator shortcut avoids repeating weighted scalar work after
character-limit validation. This workload performs 100 counts of 262,144 ASCII
characters in a fresh process; both binaries return 65,536. Values below are
whole-workload medians, not per-call latency or provider billing.

| Platform | Baseline ms | Candidate ms | Median paired delta | A/A median paired delta |
| --- | ---: | ---: | ---: | ---: |
| Linux x64 hosted | 21.517 | 6.531 | -69.83% | +0.14% |
| macOS x64 local | 50.539 | 24.367 | -52.50% | +0.83% |
| macOS arm64 hosted | 33.735 | 10.199 | -69.58% | +1.91% |
| Windows x64 hosted | 27.446 | 13.743 | -50.19% | +0.39% |

See [all workloads and resource observations](artifacts/release-performance-2026-10-10/measurements.md)
and the [durable raw evidence](artifacts/release-performance-2026-10-10/README.md).
Small positive and negative changes outside the ASCII case remain visible; they
are not promoted to improvements or hidden behind an aggregate score. For
example, local task-admission rejection has a +9.59% paired median change while
its A/A control has -11.56%, illustrating why this campaign does not impose a
short-process timing budget. No measured outlier was discarded.

A local supplemental probe with 262,144 ASCII characters followed by one
non-ASCII character retained the expected result (65,537) in all 11 measured
pairs; medians were 49.522 ms and 43.506 ms. Its source, raw observations and
binary hashes are preserved. This does not establish a universal Unicode speedup.

Instrumented ASCII CPU medians were 18.002 to 2.951 ms on Linux and 30.345 to
5.323 ms locally; these are separate instrumented series, not the paired
uninstrumented timings. Peak RSS was essentially unchanged; no retained-memory
reduction is claimed. Binary size was unchanged locally, +384 bytes on Linux
and +512 bytes on Windows; it was also unchanged on hosted macOS arm64. No dependency or model-context reduction is claimed.

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
CI and evidence only. No runtime public API, CLI flag/exit code, configuration, environment
variable, serialization format, tool schema or dependency changed. New maintenance
commands and workflow inputs are documented in the benchmark guide. Token counts
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
Nine Node tests cover statistics, missing/corrupt/failed reports, unsupported
resources, driver failures/deadlines, provenance validation, evidence-directory
preservation and exact disposable-artifact cleanup. Clippy initially found one unnecessary clone in a new test; the
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

| ID | Priority | Area | Finding | Evidence | Action | Status |
| --- | --- | --- | --- | --- | --- | --- |
| P01 | P2 | Native tokens | Repeated ASCII weighted-scalar work after validation | Four paired release series and independent oracle tests | Reuse the validated count for ASCII | Fixed |
| P02 | P1 | Verification | Prior no-JIT CLI command omits a required binary feature | Fresh build exits 101; explicit corrected build passes 149 tests | Rebuild the intended CLI before parity tests | Fixed |
| P03 | P2 | Performance evidence | Native paths and combined provenance/resource receipt absent from the old corpus | Seven added runtime families, 21 total plus three static families | Extend existing runners and add platform driver/contracts | Complete |
| P04 | P2 | Windows build | Git Bash selects incomplete Perl for vendored OpenSSL | Initial failure log; successful native Strawberry Perl preflight/build | Use native-shell build helper | Fixed |
| P05 | P2 | Scheduling | Hosted arm64 immediate-async workloads spend much more wall time than CPU time | Accepted A/A: 1,000 calls, 9.696 s wall median, 207.428 ms instrumented CPU median, 1,001 scheduler rounds | Profile wake/poll behavior before changing semantics | Needs more evidence; not an established regression |
| P06 | P2 | Evidence output | Successful Windows runners retain disposable PDBs | Four files, 6,733,824 raw bytes; exact cleanup test | Remove only owned symbols after successful runner execution | Fixed |

## Changes implemented

- P01: `src/interpreter/native_functions/token.rs` reuses validated counts only
  for ASCII. `tests/token_estimation_release_contract.rs` adds the scalar oracle
  and exact-limit/error checks. Native result and error contracts are unchanged.
- P02: `.github/workflows/ci-release-gate.yml` explicitly builds the no-JIT CLI
  with its required network feature; the prior native parity audit is corrected.
- P03: `scripts/runtime_measurement_release_bench.rs` adds seven families and
  independent output oracles. `scripts/release_performance_campaign.js`, its Node
  tests and `.github/workflows/release-performance.yml` add provenance,
  statistics, resource receipts, bounded execution and three-platform checks.
  `benches/README.md` documents reproduction and baseline updates.
- P04: `scripts/build_release_comparison.js` uses structured subprocess argv,
  preserves build logs and checks native Windows Perl before locked builds.
- P06: the same campaign driver removes successful runner PDBs; its ninth Node
  test protects measurement/unrelated files and optional-symbol handling.
  The reviewed archive records its explicit symbol omissions.

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
| `node --test tests/release_performance_campaign.test.js` | Passed, 9 tests |
| `cargo fmt --check` and `rustfmt --check scripts/runtime_measurement_release_bench.rs` | Passed; unchanged static runner’s pre-existing standalone formatting difference was left untouched |
| `cargo test --test readme_contracts --test benchmark_publication_policy_contract --test markdown_unchecked_contract --test repo_hygiene_contract` | Passed, 14 final documentation/hygiene tests |
| `bash scripts/release_candidate_gate.sh --roadmap-only` | Passed |
| `shasum -a 256 -c SHA256SUMS` in the artifact directory | Passed, all five archives |
| `git diff --check` | Passed |

The same locked release build command passed for the local candidate (14m47s;
dependencies reused) and both binaries on each accepted hosted platform. Exact
local campaign commands, run from the repository root:

```bash
node scripts/release_performance_campaign.js \
  .audit-evidence/release-performance/kujo-baseline \
  .audit-evidence/release-performance/kujo-baseline \
  42269d9b3f924736a2f995e8334ec2e7025cc1d2 \
  42269d9b3f924736a2f995e8334ec2e7025cc1d2 \
  .audit-evidence/release-performance/baseline-aa-final
node scripts/release_performance_campaign.js \
  .audit-evidence/release-performance/kujo-baseline \
  .audit-evidence/release-performance/kujo-candidate \
  42269d9b3f924736a2f995e8334ec2e7025cc1d2 \
  f4abf5dd265d5b866a2cfc99d82344c3a8ebbd5f \
  .audit-evidence/release-performance/paired-final
node .audit-evidence/release-performance/late-unicode-probe.js
```

All three commands passed. The supplemental probe source is inside the local
archive. Hosted workflow runs use these same immutable source inputs. The current
workflow centralizes the equivalent locked Cargo/Git build steps in
`node scripts/build_release_comparison.js`.

## Provenance notes

Ending implementation/verification SHA:
`bb6fd8208f08cc2100c71a78b9a322602067ae50`. Runtime source matches measured
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

The nine harness contract tests also passed independently on Linux, macOS and
Windows in [run 38096097835](https://github.com/kujolang/kujo/actions/runs/38096097835),
including the platform-specific deadline cleanup path. The hosted full release
gate passed in [run 38092538128](https://github.com/kujolang/kujo/actions/runs/38092538128).

Windows checkout uses CRLF for the Rust runner files, so its recorded byte hashes
correctly differ from Unix hashes. Review verified each archived file against
its own raw manifest/hash, then confirmed that only CRLF/LF differs from the
committed runner text. All 21 generated workload sources and iteration counts
match Linux exactly. Raw hashes were not rewritten or normalized.

## Evidence size correction

The Windows archive contained four throwaway Rust runner PDBs totaling 6,733,824
uncompressed bytes. They carry no runtime samples or source evidence. Successful
runner cleanup now removes those symbols as well as the executable; a ninth
Node test verifies exact-file cleanup, missing optional symbols and preservation
of measurement/unrelated files. Runner failures retain generated debug artifacts.

The reviewed Windows archive is 104,200 bytes versus the original 1,868,856 bytes.
Only those four PDBs were omitted; each evidence folder records their original
names, sizes and hashes in `omitted-runner-symbols.json`, alongside the original
artifact/archive identity. Original raw hash manifests remain unchanged. All
retained raw samples, sources, reports and build logs were reverified after
repacking. This is a measured artifact-size reduction, not a runtime-memory or
provider-token saving.

## Final platform verification and remaining work

The complete hosted arm64 retry passed both full series in
[run 38094214685](https://github.com/kujolang/kujo/actions/runs/38094214685),
validating the calibrated campaign budget without reducing any workload. Linux
passed its job in run 38091114929; Windows passed run 38092543276. The archive
index preserves exact artifact IDs. All eight accepted series have 21 runtime
and three static families; every retained file hash and statistical summary
was reviewed.

- P0/P1: no confirmed unresolved correctness/security defect or introduced
  behavioral regression remains in this campaign.
- P2: roadmap item 4 is ready for its separate token/payload/output-efficiency
  pass. No broader scheduler change is bundled into this estimator optimization.
- Needs more evidence: P05 is repeatable baseline latency, not a new regression.
  The accepted hosted arm64 A/A series again spends about 9.7 seconds on 1,000
  immediate sequential async calls while using about 0.21 seconds of CPU in
  separate instrumented samples. Existing scheduler loops in `src/vm.rs` sleep
  for 1 ms when no context completes. Timer/coalescing or wake behavior is a
  code-supported hypothesis, not a proven cause; profile before any event-driven
  change and preserve cancellation, fairness and timeout contracts.
- Not worth changing from this evidence: wall-time CI budgets for short noisy
  processes, or build-speed comparisons across different dependency cache states.
  Allocation/retained-heap and syscall profiling remain outside the measured
  dimensions; no corresponding savings are claimed.

Cross-repository follow-ups: none required by these changes.
