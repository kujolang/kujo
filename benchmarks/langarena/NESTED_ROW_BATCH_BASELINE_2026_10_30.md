# Nested nonscalar row batches: remaining width-copying baseline

Accepted runtime `5458132`. Imported helpers perform counted replacements with
one-element array payloads inside dictionary- and struct-owned rows. Both once
and fresh snapshots are retained; final, untouched and original cells are checked.

| Owner | Writes/call | Width 128 bytes/call | Width 2,048 bytes/call | Width penalty |
| --- | ---: | ---: | ---: | ---: |
| Dictionary | 32 | 55,221 | 316,341 | 261,120 |
| Dictionary | 128 | 147,765 | 408,885 | 261,120 |
| Struct | 32 | 54,058 | 315,178 | 261,120 |
| Struct | 128 | 142,762 | 403,882 | 261,120 |

These once-snapshot slopes use 8/16 helper calls and agree in VM/JIT. The
unchanged write count isolates row-width cost. Fresh-snapshot groups retain the
same width penalty, with additional fixed observation costs. A separate dense
dictionary matrix writes one quarter/all of each row; allocation is linear in
writes with a retained width term. It does not show quadratic per-element copying.

All three matrices pass independent verification: 144 allocation observations,
216 semantic checks across VM/interpreter/JIT, exact coverage, artifact/source
identities, raw counters, outputs and slopes. This baseline uses the accepted
runtime, not the in-progress invariant-selector candidate. An initial discarded
fixture used struct field syntax on a dictionary and failed before measurement;
the verified dictionary matrices use string-key indexing.

Nine additional baseline boundary runs pass across dictionary, struct and deep
rows in VM/interpreter/JIT. They cover full batches, mixed scalar/array/dictionary
payloads, negative aliases, overwrite after a full batch, callback observation,
prior writes before a bounds error, and retained original/intermediate snapshots.
These contracts establish behavior to preserve; the follow-up candidate must
repeat them. Their fixtures and raw outputs are included in the evidence.

Source attribution: `src/interpreter/row_spill_proof.rs` admits scalar row
expressions, while these payloads are arrays. Admission and row storage must be
reviewed together before choosing a fix; widening a guard alone is not proven
safe. Existing root-array batching does not establish nested-row eligibility.

Disposition: confirmed remaining cost, not fixed. A follow-up must retain nested
owner snapshots, callbacks, bounds/overflow and prior-write publication, including
small/cold fallback controls. Repeat matching allocation/scaling, all-runtime
regressions and checksum gates before acceptance. No elapsed-speed claim follows.

[Raw reports, fixtures and reproduction](NESTED_ROW_BATCH_BASELINE_2026_10_30.json).
