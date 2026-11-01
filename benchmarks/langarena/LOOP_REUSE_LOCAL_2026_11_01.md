# Loop reuse local verification and callback re-entry cost

Mac release build for `aae3919f` completed successfully. All 35 tests across
eight integration suites pass. Each suite's pass names match its executable's
`--list` inventory; source hashes match the frozen manifest. The pinned CLI is
SHA-256 `cb17c863223bd39108954d5d819e0e1319c575ad7720c28d033a96a07e5286dd`.

The unchanged `probes/loop_reuse/callback-condition.kujo` passes VM, interpreter
and JIT with exactly `PASS 903`. With whole-function JIT disabled, the reused-loop
candidate enters native code **801 times with zero completed loops**. The pinned
eb299e0 control enters once with zero completed loops. Both preserve all 903
callback effects. This confirms repeated native-to-VM fallback work, not a wrong
result or a measured wall-clock slowdown. Loaded-host timings remain unsuitable
for a competitive speed claim.

The candidate is not accepted. Resolve excessive fallback re-entry without
permanently blocking eligible later invocations, preserve independent loops and
recursion, and rerun the native-reuse and all-runtime semantic regressions before
the final checksum and paired-cost gates. Later integration changes through
5fce889b have not changed this native loop-admission behavior.

The companion `loop-reuse-local-2026-11-01.json.gz` contains eleven digest-verified
artifacts: source identity, complete test log and membership verification, both
callback reports and candidate stdout/stderr for each runtime. The frozen binary
remains local under `.audit-evidence/loop-reuse-2026-11-01/candidate-kujo`.
