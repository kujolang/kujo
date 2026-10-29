# Struct-literal candidate rejected

Allowing struct construction through the outer expression publication boundary
removes the target owner-width copying cost, but the first candidate is rejected
for fallback allocation regressions. Its runtime changes are not committed.

The target's 64 allocation pairs and 96 semantic checks pass. At width 2,048,
root/nested literal payload costs fall from 640,407/642,273 to 5,448/5,985 requested
bytes per added call. Argument controls remain unchanged.

All 720 fallback semantic checks pass, but 128 allocation totals and 64 slopes
regress. Deep dictionary and array-leaf paths each add two requests/eight bytes
per call for different-key writes, scalar transitions, owner exposure and field
errors. The increase is independent of the tested owner widths (2 and 2,048).

[Rejection evidence](struct-literal-rejection-2026-10-29.json) records the candidate
identity and affected groups. Full library/integration/checksum/memory gates did
not run after rejection. The next correction must remove the extra copies while
preserving snapshots and failure atomicity, then repeat target and fallback gates.
