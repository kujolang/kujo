# Loop cache statistics: Linux verification

Candidate `8064895bc10293275e4cd01f29642ee5cc5685bd` passed Linux run
41275112849, job 124755624740: **1,347 passed, eight ignored**. This covers
1,312 library tests and 35 tests across eight integration suites.

Artifact 13071650967 identifies the exact commit and source hashes. Independent
verification matched those hashes to immutable Git objects, checked all nine
suite totals and matched all 1,355 unique test names to their pass/ignore results.
The statistics regression checks repeated compilation reuse, public statistics
and distinct code identity after chunk mutation. The integration suites include
VM, interpreter and JIT behavior checks; these are not all 150 LangArena cases.

The companion `loop-stats-linux-2026-11-01.json.gz` preserves three artifacts:
the source/toolchain receipt, complete test log and membership verification.
Every decoded artifact matches its recorded SHA-256 digest.

This result does not certify later profile invalidation or integration commits.
The corrected combined candidate is `f1a59839`; its full CI, local runtime tests,
checksum matrix and paired cost measurements remain pending. Accepted runtime
remains `fd7b764`. No production or comparative speed claim follows from this gate.
