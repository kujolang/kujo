# Live captured-local prototype: rejected on allocation scaling

The experimental shared-map implementation fixed the known visibility cases,
but it is **not accepted**. Both runtime source files were restored to `2e020ee`.
The interpreter defect remains open. No full correctness acceptance or release
claim is made for the discarded prototype.

The expanded regression gate now covers 27 cases. VM/JIT pass all 18 cases on
the accepted runtime. Interpreter fails nine. The new case creates an independent
snapshot closure after a worker update, calls a helper with a shadowing parameter,
then lets the worker update the original again. Expected output is `9,11,11,1`;
the accepted interpreter returns `1,1,1,1`. This exposes stale writeback as well
as an early stale read. The prototype passes all 27 targeted cases. The
interpreter gate remains explicitly ignored by default and is run with
`--include-ignored`; an ordinary green run cannot close this issue.

## Allocation evidence

An imported factory returns a closure that appends integers to its captured
array. Every run asserts the final array length. The same process-wide counting
allocator source measures both versions; parse/setup are excluded. VM hosts the
imported AST closure. All eight allocation runs pass their output assertions.
These counts measure allocation traffic, not peak memory or wall-clock speed.

| Appends | Accepted runtime requested bytes | Prototype requested bytes |
| ---: | ---: | ---: |
| 512 | 1,184,497 | 54,441,034 |
| 1,024 | 1,467,633 | 214,933,066 |
| 2,048 | 2,033,905 | 856,781,386 |
| 4,096 | 3,166,449 | 3,423,935,050 |

The prototype approaches four times the bytes when input doubles. At 4,096
appends it requests approximately 1,081 times the baseline bytes. The baseline
increments scale linearly. This is sufficient to reject the prototype without
using noisy elapsed timings. There is one allocation observation per size;
these are diagnostic controls, not LangArena score measurements.

The implementation retained the current collection in both the activation map
and shared storage. Mutation cloned the shared value before publishing it,
forcing repeated copy-on-write of growing arrays. Shared visibility alone is
insufficient: the next implementation needs one authoritative mutable collection
location and deliberate snapshot boundaries. It must preserve efficient native
mutation without holding shared locks across language callbacks. Public host
mutation callbacks, raw scope consumers, helper scope-index reuse, generator
suspension and owner lifetime still require explicit handling.

## Retained artifacts and next gate

- [Structured evidence and rejected patch](live-capture-rejected-2026-10-26.json)
  records exact source, binary, library and probe hashes, build arguments,
  allocation counts and test output.
- `tests/ordered_task_capture_contracts.rs` and
  `tests/fixtures/ordered_capture_snapshot_helper.kujo` retain the expanded gate.
- `probes/captured-append-scaling/` retains all four input sizes, imported module
  and allocator probe source. Build the probe against the exact runtime library
  and its matching dependency/native linker flags; record those arguments and
  artifact hashes for each version. Do not substitute a different feature graph.
- Local full artifacts remain under
  `.audit-evidence/live-capture-v1-2026-10-26/`. Its binaries contain the rejected
  prototype, even though tracked runtime sources have been restored. Likewise,
  `target/release` must be rebuilt before using it as an accepted runtime.

The next candidate must pass the ordered/snapshot gate and preserve linear
captured-append allocation scaling before broader runtime and all-50 checks.
Existing requirements for globals, imports, callbacks, immutable bindings,
rejected mutations and independent value snapshots remain unchanged. Clean-host
speed comparisons remain deferred; no Kostya contact occurred.
