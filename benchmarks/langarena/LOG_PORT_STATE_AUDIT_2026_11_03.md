# Log parser: complete ASCII state and match-list audit

The accepted frozen runtime (`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`)
matches an independent Ruby model in VM, interpreter and JIT modes for 144
complete states: sixteen inputs, three successive runs and three runtimes.
The checks include 1,872 complete per-pattern match lists, generated text,
configuration and wrapping accumulated checksums. This is not a speed result or
proof that every JIT region executed natively; the normal top-level fallback
notice is preserved in the JIT stderr evidence.

Preparation sizes are 0, 1, 2, 3, 7, 16, 60 and 256, crossing the IP rollover and
all generated method/path/status/agent selections. Eight ASCII boundary inputs
cover empty/unstructured text, case folding, substring matches, adjacent status
matches, authentication paths, secrets/session token delimiters, peak-hour
boundaries and deliberately non-validating IP/method patterns. Boundary counters
start near u32 overflow and are checked after each of three calls.

All thirteen pattern strings are byte-equal in Kujo and pinned Rust/Goose source
at `3c9f84ddaeec834c0631747cb789e8cfd602f993`. Comparator files were read and
byte-checked against the pin, not executed. The Ruby model is restricted to
ASCII here; Unicode regex-class behavior remains outside this evidence.

A work-layout difference remains: both comparators insert thirteen named counts
into a dictionary and then sum it on each run. Kujo adds each count directly to
the result. Matching outputs do not erase that allocation/work difference. This
audit does not change the canonical port or silently normalize that difference.

The verifier checks binary/input/source identities, ordered state coverage,
runtime commands and raw outputs. Three altered-match controls retain the old
checksum but fail full-state equality. The 17-file archive
`log-state-2026-11-03.json.gz` includes raw outputs, manifests, comparator sources,
fixtures, oracle and verifier; every archived content hash was rechecked.

The first attempt stopped because the collector incorrectly required empty JIT
stderr; the runtime exited successfully and emitted its normal fallback notice.
The corrected run preserves stderr and verifies every full state. The separate
elapsed collector was corrected to admit only that specific normal notice,
while still rejecting arbitrary stderr; no failed speed observation is scored.

Executed comparator state, Unicode boundaries, allocation/work normalization and
production coverage remain open. The all-50 ledger row remains partial.
