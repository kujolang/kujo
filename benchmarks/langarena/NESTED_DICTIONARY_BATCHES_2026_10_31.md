# Bounded nested dictionary collection batches

Runtime `b401dc6` extends bounded nested dictionary publication to collection,
string and byte payloads. Dictionary, struct-field and deeper dictionary owners
retain value semantics. The journal grows for distinct keys, reuses existing key
handles and preserves the compact same-key path. Admission rejects unproven
callbacks, owner changes and expressions; struct payloads retain lexical captures.

For two array writes under a dictionary owner of width 2,048, requested bytes
per added call fall from 634,020 to 6,804 over calls 1–8, and from 634,026 to
6,810 over calls 8–16. VM and JIT agree. The 624,240-byte owner-width penalty
is removed for the measured eligible array, mixed, struct and string payloads.
Fresh retained snapshots still require copying; their controls do not regress.
These are allocation results, not elapsed-time speedups.

Verification against accepted root-dictionary runtime `8ab7c6a`:

- 1,301 library tests passed (8 ignored), 424 integration tests passed (1 ignored).
- Two portable Rust contracts exercise dictionary, struct and deep owners in
  VM, interpreter and JIT, including aliases, errors, replacements and bytes.
- All 150 test-size checksum cases passed: 50 benchmarks in each runtime.
- 3,784 allocation pairs pass strict total and adjacent-call-slope gates.
  Another 72 boundary pairs received explicit review and 57 alternating
  follow-ups. Eleven initial JIT increases retain both signs and overlapping
  ranges on rechecks; this is not a strict universal nonregression claim.
- 5,784 semantic observations and 18 historical/extended boundary contract
  executions passed. Unchanged-baseline negative controls reject the old cost.
- 288 RSS pairs across 144 groups were reviewed; 96 initial group means are
  positive. The largest mean is +301,056 bytes (2.742%). Twelve additional
  pairs recheck the two largest percentage increases: their follow-up means
  remain positive at about 87,381 and 198,656 bytes. The largest follow-up
  pair is +647,168 bytes. These process-memory tradeoffs are explicitly accepted,
  not attributed to laptop load. All width-2,048 once-snapshot initial groups
  have lower mean RSS. No universal memory reduction or leak-freedom is claimed.
- Formatting, type checks, layout tests and all 901 frozen source hashes pass.

The rejected V1 candidate retained a cold-struct proof allocation and the string
width penalty. V2 avoids collecting unused capture-elision names for nested
struct writers, while retaining the capture, and admits text/bytes into the
bounded journal. A separate harness cwd mismatch caused a spurious 1,170-byte
comparison difference; accepted measurements use the same repository cwd.
Rejected evidence remains in the local audit directory and is not acceptance.

[Raw evidence and review receipts](nested-dictionary-batches-2026-10-31.json)
retain inputs, outputs, identities, every memory group and follow-up observations.
Sections 3 and 4 remain open for capture-storage boundaries, repeated-array
reservation costs and the final representation review. Production-size and
clean-host competitive acceptance remain separate.
