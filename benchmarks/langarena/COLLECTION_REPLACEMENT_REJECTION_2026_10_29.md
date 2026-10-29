# Collection replacement candidate rejection

The first collection replacement candidate is **rejected**, not shipped. It
preserves the tested semantics but does not reduce the copying it was intended
to remove.

The [complete observations](collection-replacement-rejection-2026-10-29.json)
record 64 targeted baseline/candidate allocation pairs and 96 semantic checks,
plus 96 two-key fallback pairs and 144 semantic checks. Semantic checks cover VM,
interpreter and JIT; allocation comparisons cover VM and JIT. Independent checks
verify case coverage, fixture and executable identities, totals, slopes and
the rejection decision.

All targeted allocation totals and added-call slopes exactly equal the accepted
baseline. Increasing owner width from 128 to 2,048 still adds 587,520 requested
bytes per added call with an initial retained snapshot. The two-key fallback
matrix has no total or slope regressions, but this cannot establish that an
inactive optimization is safe once enabled.

Source inspection identifies the missing connection: `eval_expr` flushes private
publication state for array and dictionary literals, including an empty admission
slot. Assignment therefore cannot use the new collection entry. The next candidate
preserves admission through literal construction while retaining each operand's
publication boundary. It must repeat the target and fallback measurements and
verify callbacks, aliases, spreads and errors inside literals before acceptance.

The accepted baseline remains nested-insertion runtime `2c91c49`. The rejected
candidate CLI SHA-256 is
`b401c86276bfabee60e67988387f5205d3e8a1c8cddefe23c70fc094f8880f15`.
The accompanying JSON preserves executable and evidence hashes. Broader library,
integration, checksum and memory acceptance did not run after this rejection.
These observations measure requested allocation traffic, not elapsed speed.
