# Capture-path reachability census

This diagnostic overlay counts capture activation, compact-membership checks,
prefix snapshots, pending-publication retries, VM bridge snapshots and recursive
fallback snapshots. It does not measure CPU time, allocation traffic or retained
memory. Counters are process-wide atomics and include worker threads.

Apply `instrument-capture-census.rb ISOLATED_WORKTREE` only to a separate checkout
of its recorded baseline (`f58f0a9`). The script rejects the current checkout and
unexpected source anchors. Preserve its JSON output, source hashes, exact diff,
compiler version and build log. Do not install the instrumented runtime.

Build that checkout's release library, then link the probe against that exact
library and its matching dependency directory:

```sh
rustc --edition=2021 -O -C lto=thin \
  benchmarks/langarena/probes/capture_path_census.rs \
  --extern kujo=/absolute/path/to/instrumented/libkujo.rlib \
  -L dependency=/absolute/path/to/matching/release/deps \
  -o /absolute/path/to/capture-census
```

Run from the accepted repository root:

```sh
/absolute/path/to/capture-census vm 'Matmul::T4'
/absolute/path/to/capture-census interpreter 'Template::Regex'
/absolute/path/to/capture-census jit 'Compress::BWTEncode'
```

Each process selects one unchanged `test.json` case through the port's argument
binding. Kernel source, configuration, iterations and checksum remain unchanged.
The VM path matches the CLI's cooperative suspend/resume execution, including an
unbounded scheduler for task-based benchmarks. Calling `VM::execute` alone is
insufficient for this census: it returns a suspension marker for threaded Matmul.
That harness failure must not be reported as a language failure.

The probe also accepts a fixture path. Run imported fixtures from their module
directory. Positive controls must demonstrate that the relevant counters work;
zero benchmark counts alone are insufficient evidence of instrument validity.

Accept a census only after all 50 cases in each of VM/interpreter/JIT finish with
their exact expected checksum and `Summary: 1, 1, 0`. Preserve stderr counters,
stdout, exit codes, argv, source identities and positive controls. The accompanying
`verify-capture-census.rb AUDIT_DIRECTORY` validates the recorded report format,
source overlay, exact case membership, six membership controls and nine bridge
read/write/error controls.

`membership_names` counts submitted names, not exact hash lookups.
`prefix_snapshot_calls` includes the recursive retry after flushing pending
publication; it is not the number of distinct language calls. Test-size absence
of a path does not prove absence for every input size, arbitrary user program or
host callback. Attribution and before/after cost measurements are still needed
before proposing a runtime optimization.
