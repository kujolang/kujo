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
