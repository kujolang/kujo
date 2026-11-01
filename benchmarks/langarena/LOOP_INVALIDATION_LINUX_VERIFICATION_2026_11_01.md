# Loop profile invalidation: Linux verification

Candidate `17c3ccd46a685ac25a53e8f0f1028e3de7524931` passed run
41276390377, job 124759320099: **1,347 tests passed, eight ignored**.
The nine suites comprise 1,312 library passes and 35 integration passes.
Artifact 13071657847 preserves matching source hashes and the exact commit.

Independent verification checked immutable source hashes, all suite totals and
all pass/ignore names against the preceding statistics revision's inventory.
The existing guard-failure regression was extended in this revision: removing
an offset's specialization invalidates matching compiled loop variants across
chunks and dense-slot layouts while retaining another offset's cached variant.
Its pass is present in the log. This covers the public profiling API; no current
CLI wrong-result reproduction was claimed for that API path.

The companion `loop-invalidation-linux-2026-11-01.json.gz` contains the source
receipt, complete test log and verification manifest. All three decoded SHA-256
digests were checked. The test-name count includes eight explicit ignores;
ignored tests are not counted as passes.

Later integrated candidate `5fce889b` remains unaccepted pending its own gates.
This result does not establish all 150 checksum passes, production coverage,
callback fallback cost or comparative speed for that candidate. The accepted
runtime remains `fd7b764`.
