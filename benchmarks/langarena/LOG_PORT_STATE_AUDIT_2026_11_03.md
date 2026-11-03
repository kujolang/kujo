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

## Named-count work correction

The canonical port now materializes a dictionary containing all thirteen pinned
pattern names/counts, sums its values, then updates the wrapping result. This
supersedes the direct-accumulation difference noted above; it is a benchmark work
correction, not a runtime speed improvement. Helper invocation and values-list
materialization are Kujo implementation costs; exact allocation parity is not
claimed. Pattern setup/precompilation and preparation layouts still need review.

The refreshed audit verifies 144 full states including every named count, 1,872
match lists, input text/config snapshots, and count-dictionary copy independence
in VM/interpreter/JIT. Names and patterns both match pinned comparator source.
Six unchanged-checksum altered-match/count controls reject mismatches. The
separate named-count regression covers all thirteen keys, copied dictionary
mutation, wrapped accumulation and prior-state preservation. Its extracted Kujo
fixture and the official Etc::LogParser test checksum (117) pass all three modes
on the frozen accepted runtime. Rust integration execution and refreshed all150
CI remain pending; no runtime source changed.

Raw refreshed evidence is in log-named-counts-2026-11-03.json.gz, with originals
at /tmp/kujo-log-named-counts-review. Comparator execution, Unicode behavior,
remaining work/setup differences and production remain open.
