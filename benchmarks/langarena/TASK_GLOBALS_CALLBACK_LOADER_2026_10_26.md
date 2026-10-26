# Task globals and callback loader checkpoint — 2026-10-26

Runtime commits: `2b13158` (task globals) and `2e020ee` (callback loader).
[Structured evidence](task-globals-callback-loader-2026-10-26.json) includes
source/binary identities, raw completed observations, allocation counters and
checksum reports. Local audit originals remain under
`.audit-evidence/callback-loader-v7-2026-10-26/`.

## Changes

Task submissions refresh globals while retaining captured local state. Lexical
namespace provenance remains separate from execution ownership. Module globals,
callback globals, independent snapshots, nested reentry and caught-error writes
retain their respective ownership. Scoped borrowing avoids temporary environment
copies. The new regression suites cover VM, interpreter and JIT-enabled execution.

Callback VMs now use an empty loader placeholder before borrowing the caller's
loader. This removes unused working-directory and search-path discovery. Ordinary
VM initialization, module caches, capability checks, cancellation and error
recovery remain covered. A deterministic unit asserts that fresh and reused
callbacks do not initialize discarded loaders, including wrong-arity recovery.
JIT-enabled coverage includes fallback; it is not a native-only execution claim.

## Verification

- Formatting, diff whitespace and locked library check passed.
- 1,121 library tests passed; eight ignored.
- 276 integration tests passed; one ignored.
- Nine quick semantic cases passed across the three runtimes.
- All 150 test-size checksums passed: 50 each in VM, interpreter and JIT.
- Eight allocation-probe runs returned correct checksums.
- Source hashes were checked again before committing; no runtime edits followed
  the verified candidate.

The intermediate task-global commit also has preserved V6 evidence: 1,120
library passes, 276 integration passes and all 150 test-size checksums.
Neither checkpoint establishes full production-size or cross-platform acceptance.

## Measurements and limits

Compared with the immediately preceding V6 candidate, identical allocation probes
measured 10.89% fewer allocation requests and 10.84% fewer requested bytes in the
JIT task-state control. Sort callbacks used about 2.80% fewer requests in VM/JIT,
with requested bytes down 1.83%/1.79%. There were two observations per control;
these measure allocation traffic, not retained memory or overall language speed.
Task-state VM allocation changes below 0.10% remain inconclusive.

Completed diagnostic comparisons against accepted runtime `0b87ccb` showed a
candidate/baseline median elapsed ratio of 0.345 for task-state JIT, 0.549/0.521
for direct VM/JIT callbacks, and 0.679/0.721 for sort VM/JIT callbacks. Some helper
controls were slower (medians 1.061–1.110) with broad ranges. Preserve both wins
and losses; none establishes a repeatable speedup or regression on this host.

The user confirmed continuously changing heavy laptop workloads. Remaining
priority timings, memory comparisons, immediate-before timings and the queued
helper recheck were stopped or not started. They are cancelled/deferred, not
runtime failures. Partial observations remain in the structured evidence and
must not be presented as a completed comparison. No further local timing retry
is required to clear an arbitrary percentage threshold. Reliable performance
acceptance and Goose/Rust comparisons remain pending on a clean server.

A prior harness attempt omitted helper fixtures and failed before helper
measurements began. The exact fixtures were restored and completed correctness
results retained; recovery evidence distinguishes this setup issue from runtime
failure.

## Still open

- Channel-ordered captured-local visibility during active task/caller execution:
  scalar and collection repros pass VM/JIT but fail the interpreter. Independent
  collection snapshots remain intact. The separate fix is not included here.
- Imported user-defined binding/name/map materialization costs.
- Remaining profile-led representation and compiled-execution work.
- Unchanged production-size all-50 matrix, clean-host comparisons and supported
  platform/release acceptance.

No Kostya contact or upstream submission occurred.
