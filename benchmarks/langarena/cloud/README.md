# Clean Linux benchmark handoff

The full performance objective remains open. This directory prepares an exact
Linux comparison set; it does not provision resources or claim Linux validation.
The original Kujo baseline is `f037705`, and the candidate is `44b2fb1` (task/callback and nested-worker
JIT/cancellation fixes). Both use the pinned Rust toolchain and separate target folders.
The earlier preparation pin `9be62a6` remains historical evidence, not the
original baseline. Update candidate pins explicitly after accepting further fixes;
never silently substitute a floating branch or overwrite a prior run directory.

## Access and machine

Use a dedicated-CPU Linux x86-64 server. Start with 8 vCPUs/16 GiB RAM and
100 GB disk; prefer 16 vCPUs/32 GiB when evaluating the 16-thread workloads.
Actual production memory requirements and runtimes must be measured. Obtain
access through SSH keys or a connected provider; never put credentials in Git
or benchmark reports. No cloud server has been provisioned by this setup.

## Preparation

On Ubuntu, install prerequisites through the normal administrator workflow:

```sh
sudo apt-get update
sudo apt-get install -y build-essential git curl ca-certificates clang cmake \
  pkg-config libssl-dev zlib1g-dev libzstd-dev perl time sysstat ruby unzip xz-utils
```

Install rustup using its official installer, then open a shell with its tools on
PATH. Complete OS updates and any required reboot before measurements. Run as
an ordinary user, choosing a new directory and a conservative build concurrency:

```sh
bash benchmarks/langarena/cloud/prepare-linux.sh /home/benchmark/langa-comparison 2
```

The script installs Rust 1.96.0, checks out immutable Kujo/LangArena/Goose
revisions (including both Kujo versions), builds release binaries with locked Rust dependencies, uses Goose's
upstream `prod` recipe, and records machine/compiler identities and hashes.
The candidate's `build.kujo` assembles the ignored `port.kujo` from pinned source
fragments before its hash is recorded. The candidate passed 150 local test-size
checksums; see `../task-workers-release-2026-10-24.json` for the bounded evidence.
Its shell bootstrap is necessary before a Kujo executable exists; subsequent
orchestration uses the repository's Kujo runners. A failed build leaves logs
and no completion marker. `BUILT_NOT_VERIFIED` means only that builds finished.

Run all comparisons from `$workspace/kujo` using its unchanged benchmark port
and configuration for **both** Kujo executables:

- Original: `$workspace/kujo-baseline/target/release/kujo`, VM and JIT.
- Candidate: `$workspace/kujo/target/release/kujo`, VM and JIT; interpreter for parity.
- Goose: `$workspace/arena/goose/target/benchmark`.
- Rust: `$workspace/arena/rust/target/release/benchmarks`.

Source revisions, executable hashes and the candidate port/configuration hashes
are recorded together. Baseline failures must remain visible; do not repair the
historical runtime or label missing timings as successful comparisons.

## Required execution gates

1. Verify all 50 `test.json` checksums for Kujo VM/interpreter/JIT and both
   competitor binaries. Record failures explicitly. Use the pinned benchmark
   sources and unchanged configurations; do not substitute reduced workloads.
2. Run existing focused seven-round comparisons before broad optimization.
   Invoke frozen executables directly: upstream convenience scripts rebuild.
   Complete all builds first, execute one benchmark process at a time, alternate
   variant order, and retain raw measured-loop and total-process times.
3. Record peak RSS, CPU steal, swap activity and available memory. Keep thread
   counts identical; do not pin a multithreaded benchmark to a single vCPU.
4. Do one production-size pass with explicit per-case deadlines and incremental
   results before estimating the cost of repeated production runs. Timeout is
   incomplete, never a pass. Keep compiler/JIT warmup treatment comparable.
5. Inspect remaining allocation/call and indexing costs, implement measured
   improvements with VM/interpreter/JIT semantic regression tests, and repeat
   the affected gates. Compact arrays remain a separately gated investigation.
6. Complete supported-platform verification separately; Linux success does not
   establish macOS/Windows or all-architecture acceptance.
7. Export reports, logs, source revisions, configuration and executable hashes
   before destroying the server. A stopped server may still incur charges.

Existing entry points: `../profile.kujo`, `../compare_revisions.kujo`,
`../compare_quicksort.kujo`, and `../probes/paired_root_pool.kujo`. Paths in these
runners are relative to the Kujo checkout; run from its repository root.

## Current validation boundary

The bootstrap has local shell syntax and fail-closed preflight validation only.
Linux downloads/builds, cloud performance, production completion and platform
acceptance remain unverified until the server run supplies evidence. Cloud
access is currently required; it does not block independent local investigation.

## Full-registry comparison runner

Use `compare_suite.kujo` from this tooling checkout (the pinned runtime checkout
can predate this runner). Run it with the candidate executable, from the
candidate repository root, after generating `port.kujo`. Create a fresh output
directory first. The arguments are:

```text
BASELINE LATEST RUST GOOSE NEW_OUTPUT_DIRECTORY CONFIG [ROUNDS] [TIMEOUT_MS] [EXACT_NAME_OR_STAR] [resources]
```

It runs original VM/JIT, candidate VM/JIT, Rust and Goose sequentially over every
entry in the supplied unchanged `test.json` or `run.json`, rotating variant order.
Use one round for the initial full-registry correctness pass. Seven rounds are
the default for subsequent comparisons. The default per-process deadline is ten
minutes; set a longer deadline explicitly for production retries. A local tree
case needed roughly 35 seconds per iteration and did not finish within ten
minutes; that timeout does not establish a correctness failure.

The schema-v2 `samples.json` manifest lists relative `result_files` in execution
order. Each sample JSON contains measurements, status, configuration and relative
`stdout_path`/`stderr_path` files with the captured output. Read those files
relative to the manifest directory. This replaces the old inline `results` array:
large output and multi-round resource snapshots cannot overflow a cumulative
JSON write. Every referenced sample and output file is written before the
manifest checkpoint; a failed write cannot announce a missing sample.

The runner freezes the configuration and instrumented port in the output folder,
records their hashes, checkpoints every result, retains failures and exits
nonzero if any variant fails. It refuses existing evidence files. Kujo output
must contain the exact expected checksum; Rust/Goose use their pinned upstream
runner's checksum verdict. Binary/source provenance must accompany these reports
from the bootstrap manifest. Add `* resources` after rounds and timeout to
measure all cases with `/usr/bin/time`, or use an exact case name instead of `*`.
Resource mode records `peak_rss_kib` (macOS bytes are converted to KiB), preserves
the raw time output in stderr, and requires a positive measurement for a pass.
On Linux it also saves before/after aggregate CPU jiffies (including steal),
`/proc/meminfo`, load average and swap-page counters for each sample. These
host-wide counters describe interference; they are not per-process CPU usage.
On macOS only peak RSS is collected; CPU pressure remains a separate gate.
Missing measurements, including those from killed processes, remain null and
are never treated as zero. Truncated output is rejected. Wrapper startup is
included in total wall time, while the benchmark measured loop stays unchanged.
Cancellation stops
with partial evidence retained. Do not interpret incomplete rows as timings.

The lightweight host-fixture contract can be checked without real benchmark
work using `bash benchmarks/langarena/tests/compare-suite.sh /absolute/kujo`.
It exercises all 50 synthetic registry entries in VM, interpreter and JIT,
wrong checksums, malformed times, deadlines, output preservation and selection.
It also checks resource mode on the current host, both RSS parser formats and
truncated output rejection. Linux resource collection still needs a real Linux
run; passing macOS fixtures does not establish that gate.
These are runner tests, not LangArena correctness or speed results.
