# Nested full-table replacement diagnosis

The mixed existing/new-key control passes 540 semantic invocations but rejects
its own accepted-baseline comparison on three call slopes. There are no total
regressions: three first-call samples differ by one allocation and 2,464 requested
bytes, causing misleading slopes when subtracted from stable second-call totals.

Repeating eight exact fixture/mode/call combinations 12 times (96 observations)
reproduces two allocation totals in three first-call groups. Second-call groups
and the 64-key control remain stable in these observations. This is allocation
variation, not an elapsed-time effect from laptop load.

A separate Rust reproducer confirms that this toolchain's `HashMap::insert` grows
a full table when replacing an existing key: capacities 3/7/14/28 become
7/14/28/56. Replacing through `get_mut` preserves all four capacities. Nested
inline and sparse publication currently use `insert` for replacements. The
observations are consistent with unnecessary growth depending on journal iteration
order; the candidate still needs repeated measurements to verify that attribution.

The candidate now updates existing slots directly and inserts only missing keys.
A new regression test covers full leaves in inline and sparse publication. The
insertion profitability guard is also limited to paths that may start with a new
key, preserving the previously supported existing-first route. Neither change
has completed candidate verification yet.

Do not discard the strict gate failure or widen a tolerance. Before accepting the
candidate, compare repeated same-path baseline/candidate observations conservatively,
verify the capacity regression and semantic tests, and complete the other gates.
The original mixed runner remains a reproducible negative control for baseline
allocation stability; it is not documented as a passing baseline gate.

[Measured evidence](nested-full-table-replacement-2026-10-29.json) includes the
96 observations, exact baseline identities, reproducer source/output and toolchain.
