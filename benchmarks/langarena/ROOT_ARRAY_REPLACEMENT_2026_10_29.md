# Direct root-array collection replacement remains open

The struct-literal correction in `3ac637e` improves dictionary and eligible nested
replacement paths. A direct root-array element replacement remains on the ordinary
mutation path: `IndexedPublication::encode` admits numeric scalar values only.

Isolation against `2a5ffbc` retains an original snapshot, calls an imported helper
16/32 times, and replaces `state[0]` with a newly constructed struct. Both runtimes
have these requested-byte slopes; VM and JIT agree:

| Array width | Before, bytes per added call | After, bytes per added call |
| --- | ---: | ---: |
| 2 | 5,740 | 5,740 |
| 2,048 | 292,180 | 292,180 |

The unchanged 286,440-byte width penalty remains a target for the next collection
fix. This is allocation evidence, not elapsed-time performance.

The [raw report](root-array-replacement-2026-10-29.json) retains 32 allocation
observations and 48 passing semantic checks across both binaries, VM/interpreter/JIT,
widths, call counts and initial-only/fresh snapshots. All original/final elements
and retained snapshots are asserted. Fixture sources, runner/verifier sources and
artifact hashes are included. The candidate was unaccepted when measured and was
subsequently accepted as `3ac637e` without changing its executable.

Fresh snapshots require separate copying. Their observations are preserved, not
used as the optimization target: at width 2,048 the baseline JIT slope is 9,193,070
bytes per call versus 9,193,715 in baseline VM and candidate VM/JIT. This 645-byte
JIT offset comes from one extra 10,320-byte request in the initial baseline
16-call observation; candidate totals never exceed baseline totals. Three further
alternating rounds (12 executions) match exactly between binaries at each call
count and do not reproduce that extra request. Both have a 9,193,715-byte slope
in those rounds. The original observation remains in the report; its allocation
source is not attributed. No runtime change follows from this isolation alone.
