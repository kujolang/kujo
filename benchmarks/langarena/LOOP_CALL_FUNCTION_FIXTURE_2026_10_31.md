# Direct loop compiler regression fixture

Candidate `34bc5d1`, remote run `41265318868`, job `124726820712`,
passed 1,310 library tests (eight ignored) and 31 integration tests. Its eighth
new loop-condition test failed because the compiler-generated top-level loop
contains `PushScope`, which loop admission deliberately rejects. The test called
the compiler directly without checking admission. This is distinct from the
original missing `jit_call_function` registration.

The direct test now extracts a function-local loop from actual compiler output,
checks the same loop-admission predicate used by the VM, verifies that a `Call(1)`
remains in the loop, and requires compilation to succeed. No opcode is removed
or rewritten, and all seven runtime-based loop-condition tests remain unchanged.
This is a fixture correction only. The candidate still requires local and remote
regression gates, all 150 test-size checksums, and before/after measurements.

A standalone Rust probe using the corrected source, function extraction,
admission assertion and direct compilation passed against the completed local
candidate library (`libkujo-0204a58eed7d01d2.rlib`). The probe was linked with Rust
thin LTO, matching the release dependency artifacts. This focused check does not
replace the full integration suite. Test-file formatting and `git diff --check`
also pass.
