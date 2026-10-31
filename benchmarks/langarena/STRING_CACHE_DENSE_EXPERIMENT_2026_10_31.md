# Dense string-cache experiment: rejected

The candidate preserves immutable string values per bytecode owner and shares
those cached values across calls. It removes the reproduced VM literal-read
allocation slope, but its first string read allocates a cache slot for every
constant in that code owner. **This layout is rejected**, despite passing its
correctness tests. It is not the accepted runtime.

| Check | Result |
| --- | --- |
| Library tests | 1,310 passed; eight existing ignores |
| Targeted integration tests | 17 passed, including VM/interpreter/JIT fixture |
| Test-size benchmark checksums | 150/150 passed across VM/interpreter/JIT |
| Hot literal versus bound-key difference | Removed in VM; JIT remains equal |
| Warm and append allocation controls | No increases in measured requests/bytes |
| Cold read with 128 unused constants | +2,035 requested bytes |
| Cold read with 2,048 unused constants | +32,755 requested bytes |

Cold growth is the same with unused integer and unused string constants. The
accepted baseline is flat across those pool widths. The candidate's cache table
accounts for 16 extra requested bytes per unused entry. These are allocator
request measurements, not retained heap or timing measurements.

For 2,000 reads of a 256-byte literal key, the VM candidate requests 10,336 bytes
versus 602,397 on the baseline. The matched bound-key control also drops by
61 bytes from fixed phase costs; the attributable literal difference is therefore
592,000 bytes, or 296 per read. The comparison verifier retains all absolute
counts rather than attributing fixed marker savings to the hot loop.

Evidence includes 24 candidate semantic runs, 48 hot observations, 18 cold/warm
observations and 48 append observations. Every allocation group has three
identical repetitions. JIT hot/append cases record two native function entries;
VM cases record zero. The unchanged-baseline negative control correctly fails
the hot-allocation removal gate while passing the no-increase gates.

The initial test run passed, but its source manifest caught a formatting change
to the new integration test at startup. The original text was reconstructed and
matched to its recorded digest; the only difference was whitespace. Both forms
are preserved. All six suites were rerun against a fresh manifest, and exact
executable/test membership was verified before freezing candidate artifacts.

Candidate CLI SHA-256:
`27ecf2bf54e8b069dc7859cbb8e5b400721225231797e58415da35445833e206`.
Candidate library SHA-256:
`3551c4b3049c63f63d02498f2d4f3b97c23509f89ccacd7d778f6600a01c5dab`.
Accepted baseline remains `fd7b764`; use its frozen artifacts documented in
[the baseline report](STRING_CONSTANT_LOADS_2026_10_31.md).

The compressed evidence bundle preserves the candidate patch, original fixture,
source manifests, build/test receipts, checksum outputs and allocation controls.
`probes/compare-string-constant-allocations.rb` validates case/repeat membership,
raw counts, outputs and native coverage before reporting the separate gates.
No whole-workload speed claim follows from these concurrent checksum runs.

Next revision: store the first accessed string inline, with sparse spill entries
only for other accessed strings. That revision is unverified and must repeat
correctness, hot/cold/append, spill-path and workload checks. Cold metadata and
retention tradeoffs still require review; passing this rejected experiment does
not establish the revision's behavior.
