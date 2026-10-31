# Multiple collection-valued dictionary replacements

Accepted runtime `f1893ea` retains a dictionary-width copying cost when an imported
helper replaces three existing root dictionary keys with arrays. The same helper
writing scalar values is flat. This is separate from the nested-array-row candidate
currently under verification.

| Three updates, initial snapshot only | Width 8 | Width 2,048 | Width penalty |
| --- | ---: | ---: | ---: |
| Scalar values | 4,989 | 4,989 | 0 |
| Array values | 9,099 | 633,339 | 624,240 |

Numbers are requested bytes per additional call between 16 and 32 calls. VM and
JIT agree. The matrix covers 1/16/32 calls, widths 8/2,048, initial and fresh
snapshots, and both payload forms: 48 allocation observations and 72 semantic
runs across VM, interpreter and JIT. Original dictionaries, changed entries,
retained fresh snapshots and an untouched final entry are checked. Fresh-snapshot
copying is kept separate because observable old values can require it.

Source inspection finds a single-key collection publication path, while the
multi-key publication path stores scalar payloads. The measured multi-key case
therefore warrants a bounded collection-payload extension; this report does not
claim such a fix exists. It must preserve snapshots, mixed value transitions,
existing/new keys, callbacks, error publication, compact single-entry paths and
cold/fallback costs. Nested dictionary leaves and other payload forms require
separate characterization before broadening the claim.

[Evidence](multi-key-collection-baseline-2026-10-30.json) includes all fixture
sources, exact invocations, raw outputs and counters, artifact hashes and the
independent coverage/scaling verifier. No elapsed-time or competitive speed claim
follows. Sections 3 and 4 remain open.

## Payload and same-key controls

A second verified matrix adds 144 allocation observations and 216 runtime checks
on the same accepted runtime. Two array replacements, three dictionary values,
three struct values, scalar-first mixed values and collection-first mixed values
all retain the same 624,240-byte per-call width penalty. Three mixed replacements
of a single key remain flat at 6,152 bytes per call. VM and JIT agree.

These controls check the full final dictionary by resetting changed slots in an
independent value copy and comparing it with the original, then rechecking the
unmodified final snapshot. Initial and fresh snapshots remain separate. Combined
coverage is 192 allocation observations and 288 runtime checks. Evidence is under
`payload_controls` in the linked JSON; it includes sources and a separate verifier.

The follow-up must support mixed payload order and preserve the compact same-key
path. It must also extend collection-literal eligibility: `SparseRegion::expression`
currently rejects array and dictionary literals even though the broader capture
analyzer accepts them. Storage changes alone will not admit this workload. New-key
and nested dictionary-leaf cases remain outside this matrix.
