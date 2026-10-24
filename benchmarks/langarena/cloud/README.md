# Clean Linux benchmark handoff

The full performance objective remains open. This directory prepares an exact
Linux comparison set; it does not provision resources or claim Linux validation.
The original Kujo baseline is `f037705`, and the candidate is `aa30071` (builtin
argument reuse). Both use the pinned Rust toolchain and separate target folders.
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
