# Memory candidate: eleven production controls verified

The frozen memory candidate completes all eleven selected unchanged production
controls: JSON Generate, ParseDom and ParseMapping in VM, interpreter and JIT,
plus QuickSort and MergeSort in JIT. Every result passes independent source,
binary, configuration, exit, checksum, phase-completion and resource verification.

| Workload | Runtimes | Checksum |
| --- | --- | ---: |
| Json::Generate | VM, interpreter, JIT | 120 |
| Json::ParseDom | VM, interpreter, JIT | 3989922952 |
| Json::ParseMapping | VM, interpreter, JIT | 3826684672 |
| Sort::Quick | JIT | 61757772 |
| Sort::Merge | JIT | 61757772 |

The verifier checks all expected warmup/run progress markers and monotonic phase
timestamps, exact unchanged run.json rows, complete output, zero exit status and
exact success summaries. VM measurement counters contain zero native entries;
all five JIT cases report native execution. Interpreter uses supported output
and process-resource observations without unsupported measurement flags.

Measured maximum process RSS ranges from 57,901,056 to 325,070,848 bytes across
these controls. These are individual loaded-laptop observations, not matched
memory comparisons, retained-heap measurements or a proof of a memory improvement.
Concurrent diagnostic work and host activity preclude speed comparisons. The
one-hour outer deadline was unchanged; no timeout or failed case was discarded.

## Identity and scope

- Candidate source: `754453061a98fe3c994b89f1c9308326a50c544d`.
- Candidate CLI SHA-256:
  `17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`.
- run.json SHA-256:
  `8fe7baed41e8eb0a233057c4406446029f7e5d792cf3a3ac6fc7e714964a155f`.
- Port SHA-256:
  `435d32deb5eaba5e58f011049b3bd07c4050cb5a12f09d9dc990c67767ced2ab`.

`vm-memory-production-2026-11-03.json.gz` retains the manifest, runner, independent
verifier, complete outputs, measurements, process/phase receipts and source
snapshots with verified per-artifact hashes. Binary identity is retained without
embedding the executable. The local full verifier was run successfully before
any subsequent canonical port changes; archived source identities remain tied to
this historical result and must not be rewritten to match later changes.

This is **11/150 production combinations**, not all-suite production acceptance.
It does not certify future source revisions, every lifecycle shape, competitor
parity or Windows reclamation. The separate memory runtime remains unaccepted
pending the remaining platform checks and final evidence review.
