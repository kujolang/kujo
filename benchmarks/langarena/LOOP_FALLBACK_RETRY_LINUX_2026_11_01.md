# Native fallback retry: verified Linux gates

Candidate `348a62700179e59beccaefddc7ed4da30992d67b` passed run
41278373023's native regression job 124765077324: **1,350 passed, eight ignored**.
This comprises 1,313 library tests and 37 tests in eight integration suites.
Artifact 13072098051 matches the frozen commit and source hashes. Independent
verification matched every pass/ignore name and all nine suite totals.

The new tests verify exact bounded retry delays, reset after success, independent
chunk/loop sites, all callback effects, at most twelve native entries for the
three-scan fallback case, and recovery to two native completions after builtin
rebinding. Before-fix controls produce 801 entries for repeated callbacks and
503 for the recovery case. These are structural work counts, not timing ratios;
the Linux test records bounds rather than an exact post-fix entry count.
Existing reuse, nested snapshots, callback/error and recursion checks also pass.

Parity job 124765077597 independently passes 299 checks across default and
JIT-disabled configurations, including the new no-JIT hot-loop snapshot test.
Exact source identity, eight suite totals and that test's pass were verified.
These parity counts are not the 150 LangArena checksum runs.

The companion `loop-fallback-retry-linux-2026-11-01.json.gz` preserves 21
digest-verified artifacts: native source/log/membership records, parity receipts,
and the two before-control fixtures with all three runtimes' raw outputs.

The current Mac release test build and full release gate remain pending.
Acceptance additionally requires the final pinned CLI, identical callback and
paired cost probes, and the unchanged 150-checksum matrix. Accepted runtime
remains `fd7b764`; no production or competitor speed claim is established here.
