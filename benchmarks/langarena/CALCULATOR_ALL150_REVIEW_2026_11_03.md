# Calculator correction: refreshed all-150 verification

Candidate source be92de2954848617c8690f177bbefe67b7df1909; successful CI run
41651148226, artifact 13238583472 (`langarena-all150-linux`). Independently checked
repository/run/job/artifact provenance, optimized build records, all 916 source
hashes against pinned Git blobs, uploaded executable bytes and post-test rechecks.
Binary SHA-256: bd4d08a04a7986929ee28f5eedf78aa382013cbb44db8f82c1e88a5c14f3f14c.

All 50 named upstream test-size checksums passed in each of VM, interpreter and
JIT (150 outcomes). The separate calculator accumulator regression passed; its
pinned Rust test loops through all three runtimes. Both CLI and test build records
include runtime-jit, ruling out the test's conditional JIT skip. Assertions cover
signed extrema, wrapping accumulation and snapshot preservation as recorded in
CALCULATOR_STATE_AUDIT_2026_11_03.md.

The verifier now explicitly requires JIT build features. Eleven verifier controls
cover valid evidence, duplicate/wrong checksums, missing runtime/source, changed
binary, failed build, absent JIT feature, and missing/wrong/failed calculator
regression evidence. All controls passed. Formatting passed.

Raw nonbinary evidence and review controls are retained in
calculator-all150-review-2026-11-03.json.gz; actual uploaded executable remains at
/tmp/kujo-calculator-all150-review/artifact/kujo. This closes the refreshed
calculator/test-size checksum gate only. It does not accept worker reuse, prove
native-region coverage or structural equivalence, or pass full production sizes.
Corrected elapsed comparison run 41654661543 remains separate.
