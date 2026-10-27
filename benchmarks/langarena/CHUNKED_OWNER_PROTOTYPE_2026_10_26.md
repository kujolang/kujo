# Chunked array ownership experiment

Outcome: **do not integrate this design under the current contiguous public-owner contract**. Rebuilding the public `Value::Array(Arc<Vec<Value>>)` after every call makes allocation traffic worse in all 27 matched controls. This is a negative exploratory result, not a runtime change or speed comparison.

The [probe](probes/chunked_owner.rs) uses real Kujo `Value` from verified runtime `c0e45b3`. Its [machine-readable results](chunked-owner-prototype-2026-10-26.json) retain source/binary hashes, build arguments, 63 rows and 54 comparisons. Build dependencies are the same frozen normal-release library used in recursive-capture verification.

## What was measured

The matrix covers lengths 128/2,048/32,768; 1/8/128 updates per call; chunk widths 32/128/512; and 32 calls. Update indices vary across chunks. An independent numeric model checks every element after each call, along with both the initial snapshot and the previous call's snapshot. All 63 cases passed these assertions.

Three storage models are compared:

- Contiguous copy-on-write array, retaining the prior public owner during each call.
- Chunked array with shared directory and chunks, allowing its owner to use the new representation.
- The same chunked array, with a contiguous public array materialized at each return.

The second model changes the public representation contract and cannot be counted as an available runtime optimization. The third includes that missing publication cost. Initial chunk construction is reported separately from repeated updates/publication.

| Model, 32,768 elements, 128-element chunks | Writes per call | Requests over 32 calls | Requested bytes over 32 calls |
| --- | ---: | ---: | ---: |
| Contiguous | 1 | 64 | 142,607,616 |
| Chunked, changed owner representation | 1 | 128 | 625,152 |
| Chunked plus contiguous publication | 1 | 192 | 143,232,768 |
| Contiguous | 128 | 64 | 142,607,616 |
| Chunked, changed owner representation | 128 | 2,096 | 17,793,984 |
| Chunked plus contiguous publication | 128 | 2,160 | 160,401,600 |

For these chunked cases, initial construction additionally costs 514 requests and 4,468,776 bytes. Across all 27 private chunked comparisons, 16 reduce requested bytes and 11 increase them; dense updates or small arrays can erase the benefit. Adding contiguous publication increases requested bytes in all 27 cases, ranging from about 1.003 to 2.011 times the contiguous baseline, before initial construction costs.

## Limits and next constraint

This standalone Rust model checks integer payloads and numeric snapshots. It does not implement Kujo indexing, errors, nested values, callbacks, task scheduling, native APIs, iteration or JIT operations. No all-runtime acceptance follows from it. No elapsed-time or peak-memory claim is made on the loaded laptop.

The measured module collection-width cost remains open. A useful fix must reduce the complete update-and-publication cost while preserving real pre-return public owner values and escaped snapshots. Moving the full copy to publication is not a solution. A future representation change must also justify construction, dense-update, traversal and host-compatibility costs before integration.
