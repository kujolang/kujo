# Initial dictionary insertion: candidate rejected

The candidate is **not accepted**. Its runtime edits were restored to the accepted
source. The [evidence](dictionary-insertion-rejection-2026-10-28.json) preserves
the patch, artifact identities, test provenance and complete cold-control results.

The candidate allowed a first missing scalar key into the private publication
journal and promoted additional keys into sparse storage. Large dictionaries
benefited, and 1,230 library tests, 404 integrations and 150 test-size checksums
passed. Those correctness results do not establish allocation acceptance.

The earlier insertion fixtures saved an initial snapshot. That snapshot prevents
the first call from taking the new optimization, leaving an admission case
untested. Removing it exposed extra requested bytes on the first invocation:

| New keys inserted | Existing keys | Extra requested bytes | Request-count change |
| --- | ---: | ---: | ---: |
| Two direct writes | 0 | 118 | −1 |
| 64 writes in a loop | 0 | 7,204 | −58 |
| 64 writes in a loop | 1 or 2 | 6,576 | −59 |

VM and JIT agree on these deltas: eight rejected pairs across the two modes.
Fewer allocation requests do not compensate for the increased requested bytes
under this gate. These are allocation measurements, not elapsed-time results.

The permanent [cold-insertion gate](probes/dictionary-insertion-cold-controls.rb)
covers 160 paired allocation observations and 240 semantic executions across VM,
interpreter and JIT. It tests empty, tiny and larger owners; two and 64 new keys;
one, two, 16 and 32 calls; and no initial snapshot versus fresh snapshots.
It exits nonzero for this candidate and passes when comparing the accepted
baseline with itself. Both results are preserved in the evidence.

Two library tests originally asserted that missing keys could not enter the
optimization. Their revised assertions verified insertion, old-owner visibility,
snapshots and invalid-index/value rejection. Only those two test functions changed
after the release build; the production-source identity check is preserved.
Those changes are part of the rejected patch, not the current accepted runtime.

Next work must account for the cost of staging new-key batches relative to the
owner being copied, including cold calls without saved snapshots. Keep the large
owner gains while passing this new gate and all existing controls. No fixed small
owner threshold has been selected or validated. Nested initial insertion remains
a separate measured gap, alongside nonscalar replacements, broader loop admission
and capture materialization. Sections 3 and 4 remain open.
