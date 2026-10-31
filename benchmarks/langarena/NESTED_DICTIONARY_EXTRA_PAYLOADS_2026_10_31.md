# Nested dictionary struct and string payload baseline

Accepted runtime `8ab7c6a` passes all 216 semantic checks in this matrix. Its
144 VM/JIT allocation observations confirm a remaining nested dictionary copying
cost for both struct and string payloads.

The matrix covers dictionary, struct-field and deeper dictionary owners; struct
and string payloads; leaf widths 8/2,048; an initial snapshot or a fresh snapshot
per call; and 1/8/16 calls. Each update replaces two existing keys. Assertions
check updated values, untouched entries, the original snapshot and, when present,
the snapshot retained before each call.

With only the initial snapshot, increasing leaf width from 8 to 2,048 adds
**624,240 requested bytes per added call** for every owner/payload combination
in both VM and JIT. This holds independently over calls 1→8 and 8→16. These are
allocation measurements, not elapsed-time speedups. Fresh-snapshot results are
retained as separate regression controls.

[Raw evidence](nested-dictionary-extra-payloads-2026-10-31.json) includes fixture
sources, artifact hashes, invocations, outputs, counters, the runner and the
independent verifier. Verification checks exact matrix coverage, fixture and
artifact identity, arguments, successful outputs and counter consistency.

The pending nested-dictionary candidate has not yet been measured against these
cases. Acceptance requires candidate comparison, preservation of snapshot/value
semantics in all three runtimes, and review of allocation totals and call slopes.
This baseline does not close sections 3 or 4, and is not evidence of a candidate
regression or improvement.
