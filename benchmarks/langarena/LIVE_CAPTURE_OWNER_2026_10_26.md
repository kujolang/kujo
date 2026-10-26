# Ordered captured-local visibility: owner-backed storage

Verified runtime `b34a0b8`. The interpreter now observes channel-ordered writes through aliases that are
already executing the same closure. This fixes stale reads and stale writeback
without changing independent closure snapshots or task-global isolation.

Private invocations reference the captured environment's local values through a
weak owner link. The public owner's `Environment.scopes` maps retain actual values;
Rust hosts can still read them and write values that subsequent calls observe.
Runtime leaf mutations borrow the canonical value under its owner lock. Language
and public host callbacks do not run under that lock. Independent closures,
generators and visible-binding snapshots materialize their own values. Definitions
publish both values and binding permissions, including newly introduced names.
Concurrent read-modify-write remains non-atomic.

This design avoids the discarded prototype's second reference to every current
collection, which forced copy-on-write on every append. It also avoids exposing
internal markers in the public owner maps. The final definition test catches a
failure found during development: an invocation could introduce a name that its
owner-backed lookup could not subsequently retrieve.

## Verification

- 1,128 library tests passed, zero failed, eight ignored. Seven new storage/VM
  tests cover unique ownership, snapshots, immutable bindings, host callback
  reentry, helper scopes, rejected mutations, definitions and named VM captures.
- All 33 ordered scenarios passed across VM, interpreter and JIT: scalar, array,
  dictionary, nested-row and field state; both synchronization directions;
  independent snapshots, helper reentry, caught errors and generator suspension.
- The broader integration harness passed 281 tests with three ignored. Both
  pending interpreter capture tests then passed explicitly: 283 distinct passes
  overall, with one unrelated historical ignored test remaining.
- Those two interpreter tests are now enabled by default. Both standalone default
  harnesses passed again: six tests, zero ignored, exercising the same 33 scenarios.
- The Rust embedding regression passed: read the captured public map after a
  call, write `41` directly into it, then observe `42` after the next call.
- All 150 unchanged **test-size** LangArena checksums passed: 50 per runtime.
  This does not certify full production inputs.
- Formatting and diff checks passed. Runtime source hashes remained unchanged
  throughout the final verification pipeline; only test-ignore attributes were
  removed afterward and their default harnesses rerun.

## Allocation comparison

The same counting-allocator source and imported append fixtures were run against
the accepted baseline and candidate. Each of the eight runs per version returned
`PASS`. VM and JIT entry paths produced identical counts; this does not establish
native-only JIT execution.

| Appends | Baseline requests | Candidate requests | Baseline bytes | Candidate bytes |
| ---: | ---: | ---: | ---: | ---: |
| 512 | 6,819 | 6,818 | 1,184,497 | 1,184,553 |
| 1,024 | 8,868 | 8,867 | 1,467,633 | 1,467,689 |
| 2,048 | 12,965 | 12,964 | 2,033,905 | 2,033,961 |
| 4,096 | 21,158 | 21,157 | 3,166,449 | 3,166,505 |

The byte-growth slope is identical and linear. Each candidate run has a constant
56 additional requested bytes and one fewer allocation request. The prior
quadratic-copying regression is absent. These are allocation-traffic measurements,
excluding parse/setup, not retained-memory measurements or an overall speedup.

## Provenance and remaining work

[Structured evidence](live-capture-owner-2026-10-26.json) retains source, binary,
library and probe hashes, toolchain information, test outputs, allocation counts,
all 150 checksum outcomes and exact test configurations. Full local build/log
artifacts are under `.audit-evidence/live-capture-v4-2026-10-26/`.

The executable SHA-256 is
`2900e8e0c2c742b3c52cc4d4d1485d6889d62e6c8601dc1763427fd5e3286589`.
The normal release library SHA-256 is
`d92b2f0052a442941e792b9a32e036fd0274937466f7d58edc8cd83602c917de`.

The [rejected prototype report](LIVE_CAPTURE_REJECTED_2026_10_26.md) remains
historical evidence; its open visibility defect is resolved by this candidate.
Remaining user-binding environment materialization, broader representation and
compiled-execution work, full production verification, and platform/release gates
remain open in the implementation checklist. Local timing remains diagnostic;
clean-server Goose/Rust comparisons are deferred. No Kostya contact occurred.
