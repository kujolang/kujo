# Direct array collection replacement: first candidate rejected

The first direct-array candidate passes its three targeted regression suites
across VM, interpreter and JIT, plus all 648 fallback semantic checks. It is
rejected for allocation regressions, not accepted as a runtime improvement.

Against accepted runtime 3ac637e, 64 of 432 paired allocation totals and 32 slopes
regress. Struct replacements followed by another index, a scalar transition,
whole-root observation or a caught error add 28 requested bytes per helper call
at widths 2 and 2,048 in VM/JIT. Request counts are unchanged: boxed payload
storage replaces one small struct-name copy with a larger allocation before
ordinary mutation becomes necessary. Array and dictionary payload controls have
no higher totals in this matrix.

The correction must avoid paying for boxed storage on these fallback paths,
then repeat target and fallback acceptance. Passing semantics or reducing the
main width penalty alone is insufficient. No threshold was relaxed.

[Raw observations and rejection](root-array-rejection-2026-10-29.json) retain
source/artifact hashes and all pairs. Candidate source is frozen under
.audit-evidence/root-array-v1-2026-10-29/rejected-source. No candidate broad
library/integration/all-50 or memory gates ran after rejection. The accepted
baseline remains 3ac637e; regression tests were separately pushed as 7297c63.
