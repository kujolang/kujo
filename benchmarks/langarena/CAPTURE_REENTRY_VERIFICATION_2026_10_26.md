# Synchronous capture reentry verification

Runtime `45360b9` fixes stale captured-state reads and lost writes during
synchronous recursive calls, callbacks, global-helper/generator suspension, and
reentry through another VM. Separate closure snapshots, saved collections,
immutable bindings and error paths retain their tested behavior. Active calls
track capture ownership; bridges publish and refresh the latest active capture.
A conservative write observer skips publication only for proven read-only calls.
Unknown helper/bridge transitions retain full synchronization.

Baseline runtime: `3a9cbe4`. Evidence, exact fixtures, executable/source hashes,
individual timing pairs, allocation counts, outputs and checksum reports are in
[capture-reentry-verification-2026-10-26.json](capture-reentry-verification-2026-10-26.json).
Measurements used normal release binaries after compilation finished, on the
shared local Mac. These are local regression checks, not clean-host rankings,
production-size acceptance, or comparisons against Goose/Rust. JIT-enabled tests
include fallback execution; no native-entry claim is made for AST helper tests.

## Correctness gates

- Normal release build, `cargo check --locked -j2`, formatting and diff checks passed.
- All 48 focused reentry CLI cases passed across VM, interpreter and JIT-enabled
  modes. The baseline failures and independent snapshot controls are retained.
- 259 integration tests passed, with one ignored. This includes 12 captured-root
  cases covering arrays, dictionaries, bytes, throwing indexes and saved copies.
- Nine additional imported-callback tests passed, now including JIT mode alongside
  VM/interpreter (`c6a4a58`): imports, state, errors/arity, async results, capability
  restrictions and recursion-depth errors.
- 1,114 library tests passed, with eight ignored.
- All 150 test-size checksums passed: all 50 LangArena benchmarks in each of VM,
  interpreter and JIT-enabled modes. This does not replace production-size runs.

## Seven-pair timing results

Each cell is the median paired candidate/baseline elapsed-time ratio; below one
means faster. Runs alternated baseline/candidate order. All medians stayed below
the established 1.05 slowdown review threshold. Individual slower observations
remain in the evidence, including MergeSort VM's maximum ratio of 1.150845.

| Workload | VM | Interpreter | JIT-enabled |
| --- | ---: | ---: | ---: |
| Imported read helper | 1.011684 | 0.966242 | 0.956205 |
| Imported write helper | 0.979683 | 0.960628 | 1.024371 |
| Read-only captured recursion | 0.971694 | 0.952818 | 0.962190 |
| QuickSort | 1.000574 | Not measured here | 1.000729 |
| MergeSort | 1.029043 | Not measured here | 0.995854 |
| NeuralNet | 0.997277 | Not measured here | 1.000763 |

The helper suites contain 56/52/52 observations, including separate allocation
and resource diagnostics. Priority comparisons contain 84 timed observations.
NeuralNet did not cross the threshold requiring the additional 11-pair recheck.
QuickSort is essentially unchanged; this fix is not a claimed sorting speed win.

## Allocation and memory observations

Phase-4 requested allocation counts/bytes were measured separately from normal
release timings. They are allocator requests, not retained heap sizes.

| Workload, VM probe | Baseline requests / bytes | Candidate requests / bytes |
| --- | ---: | ---: |
| Imported read, width 128 or 2,048 | 3,353 / 291,973 | 3,354 / 292,581 |
| Imported write | 192,539 / 18,008,265 | 192,540 / 18,008,873 |
| Read-only recursion | 918,562 / 86,898,784 | 918,566 / 86,917,024 |

JIT-enabled probe deltas are also +1 request/+608 bytes for imported read/write
and +4 requests/+18,240 bytes for recursion; absolute counts are in the JSON.
The rejected V3 prototype added 65,540 recursion allocation requests and had
5.3–5.7% median slowdowns. V4 removes almost all that allocation overhead and
passes the repeated timing gate; V3 evidence remains explicitly rejected.

Separate NeuralNet peak RSS observations were 18,350,080 → 18,591,744 bytes in VM
and 35,643,392 → 36,724,736 bytes with JIT enabled. These are one observation per
binary/mode, excluded from paired timing medians; they do not establish a general
memory improvement or statistically significant memory regression.

## Still open

The [task-write evidence](CAPTURE_TASK_WRITE_PARITY_2026_10_26.md) still reproduces
ordinary-task lost writes, helper-parameter journal pollution and stale task
globals. This change addresses synchronous reentry, not those task ownership
issues. User-defined imported bindings still impose copying costs. Production,
clean-host competitive, supported-platform and broader representation gates
remain open in [the checklist](PERFORMANCE_CHECKLIST.md). No Kostya contact.
