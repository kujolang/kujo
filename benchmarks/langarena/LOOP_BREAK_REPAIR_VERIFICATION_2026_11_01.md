# Loop break repair verification

Repair `eb299e0a699560c80d87d0d81582914ba2c6d956` remains a candidate,
not the accepted runtime. It resumes the VM at each loop exit before consuming
post-loop instructions, restoring that exit's operand stack. This addresses the
[break regression](LOOP_BREAK_REGRESSION_2026_10_31.md).

The Mac release build passed all 33 tests in eight targeted suites, including
break and normal-condition exits, snapshots, callbacks, errors and rebinding.
Every reported test name was matched against its executable's test listing;
source hashes match the frozen build manifest. The exact tested CLI was copied
without rebuilding and pinned as SHA-256
`683f3f615380d27423126d3ff1dddea908643c2b7d31afd3d0f51d5846e61907`.
The previously failing JIT `Template::Parse` now passes unchanged test inputs
(count 10, iterations 2, checksum 917360148), exit zero and no output truncation.

Linux run `41272272135`, job `124747313908`, artifact `13070318176` passed
1,343 tests with eight ignored: 1,310 library tests and the same 33 integration
tests. The artifact records the exact repair commit and matching JIT/test source
hashes. The saved log reports successful compilation, nine successful test
summaries and unique test membership matching those totals. Integration test
names match the Mac suites. This is correctness evidence, not speed evidence.

The companion compressed JSON bundle preserves the source manifest, Mac log,
test identities, template report and Linux source/log/verification with SHA-256
checksums. The fresh 150-case checksum matrix is still pending, as are paired
cost measurements and the separate full compatibility gate. These partial
passes do not establish final acceptance. Accepted runtime remains
`fd7b7646e8adf99d1a798302ef9a18978e4c330d`; clean-host comparisons remain pending.
