# Direct recursive capture-prefix transfer — 2026-10-27

Runtime `ce923d1d2668a593f890523cd6aa91fe37a2b17d` transfers private captured-prefix maps into a direct synchronous
recursive call and restores them on return. Caller parameters remain outside
the transferred prefix. Live captured values remain in the public owner, and
retained value snapshots remain independent. No pool bounds are increased.

Admission requires direct active reentry with global-only or live-local captures.
Tasks and indirect local reentry retain their existing paths. The existing
foreign-global transfer path is unchanged.

Baseline runtime: `e9e5695c38a33a1b178376c559467c71a7142f7a`.
Exact sources, artifacts, build arguments and raw results are in
[the machine evidence](recursive-prefix-transfer-2026-10-27.json).

## Allocation results

Requested allocations per added outer call, with recursion depth two and writes
to a captured counter. Values are the difference between 16-call and 32-call
runs divided by 16. All extra local names are read and checked by a final call.

| Runtime | Extra locals | Baseline requests | Candidate requests | Baseline requested bytes | Candidate requested bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| jit | 128 | 81 | 81 | 4302 | 4302 |
| jit | 2048 | 2133 | 81 | 683535 | 4302 |
| jit | 8192 | 24669 | 8277 | 8161425 | 2723343 |
| vm | 128 | 81 | 81 | 4302 | 4302 |
| vm | 2048 | 2133 | 81 | 683535 | 4302 |
| vm | 8192 | 24669 | 8277 | 8161425 | 2723343 |

All 96 paired allocation cases and 144 semantic cases complete without regressions. The matrix covers
zero, 128, 2,048 and 8,192 extra locals, reads and writes, recursion depths zero
and two, and one, 16 and 32 calls. No total or per-added-call request/byte
regression occurred. The runner's historical unchanged-baseline calibration is retained with its
original artifact identities. The restoration-specific gate rejected unchanged
128-local allocation; that raw rejection is retained. The corrected
[prefix gate](probes/RECURSIVE_PREFIX_GATES.md) verifies constant extra recursion
cost across all four widths and strict reductions in all eight larger
read/write groups. It also rejects the prior accepted runtime's measurements.
No raw measurement or prior gate status was changed.

Subtracting the depth-zero activation cost leaves the same added recursion
overhead at every tested width: 48 requests / 2,802 bytes for reads and
50 requests / 2,816 bytes for writes. Oversized outer activation costs remain.

These are requested allocation counts and bytes, not elapsed-speed or
peak-memory measurements. Local timing is diagnostic on the loaded laptop.

## Verification

- 1,179 library tests pass; eight existing tests remain ignored.
- 15 targeted integration tests pass, including expanded VM/interpreter/JIT contracts.
- The expanded depth-16 CLI fixture passes the unchanged baseline in all three runtimes.
- 342 matching release integration tests pass; one manual profile remains ignored.
- 18 direct baseline/candidate nested-assignment parity cases pass.
- 48 ordinary-local allocation pairs and 72 semantic controls pass without regression.
- 128 module allocation pairs and 192 semantic cases pass, preserving accepted
  width scaling for all four collection shapes at one and eight writes.
- 72 additional nested allocation pairs and 108 semantic cases pass.
- 16 read-key pairs, 12 numeric-key pairs, 36 cold comparisons, eight append
  pairs and three publication-boundary controls pass.
- All 150 test-size checksums pass: 50 each in VM, interpreter and JIT.
- Format, diff, source and artifact identity checks pass.

Regression coverage checks depth-16 recursive writes and caught errors, caller
parameters, aliases, independent closures, immutable bindings and retained
arrays. A unit test verifies that private map keys move without cloning, that
owner-backed updates remain visible, and that the caller regains its prefix.

The append gate initially stopped because its imported fixture module had not
been copied into the new evidence directory. Restoring the unchanged baseline
module resolved the setup failure; its identity and the original failure are
retained with the passing measurements.

## Remaining scope

Oversized activation setup, indirect captured-prefix materialization, captured-name
traversal and broader collection costs remain open in the
[full checklist](PERFORMANCE_CHECKLIST.md). This change does not establish
production-size, supported-platform or clean-host Goose/Rust acceptance.
No Kostya contact occurred.
