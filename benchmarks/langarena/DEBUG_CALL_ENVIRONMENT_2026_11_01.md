# Callback debug environment lookup isolation, 2026-11-01

The accepted runtime `55e2f683` profiles reach environment lookup through `jit_call_function`. Its source performs two debug checks before invoking the callee and a separate check after it returns. This probe isolates three `std::env::var(...).is_ok()` reads versus reusing the first result for the second pre-call check, retaining the post-callback read.

Seven alternating pairs in each of five environment states produce 70 verified observations. Each observation includes 200,000 iterations and a checked result sum. Fifty ordered transition controls verify both strategies across absent, empty, enabled, Unicode and invalid-UTF-8 values; a callback changes the environment between the pre-call and return checks. Invalid UTF-8 remains disabled, matching `env::var`, unlike a simple `var_os().is_some()` substitution.

| State | Median baseline/candidate lookup-time ratio |
|---|---:|
| Absent | 1.488 |
| Empty | 1.492 |
| Enabled | 1.491 |
| Unicode | 1.491 |
| Invalid UTF-8 | 1.524 |

These are standalone lookup results on the loaded laptop. The probe excludes marshalling, diagnostics formatting, VM callbacks and concurrent environment writers. It does not establish callback or whole-language speedup. It supports evaluating a narrow candidate, not integration. Any runtime change must preserve post-callback flag visibility and verify debug output, callback success/error paths, malformed stacks and all runtimes. Concurrent writers can change the flag between reads; this probe does not certify their diagnostic interleavings.

No runtime source changed. The [four-artifact bundle](debug-call-environment-2026-11-01.json.gz) preserves raw observations, exact source, binary hash and verifier. The [probe](probes/debug_call_environment.rs) and [verifier](probes/verify-debug-call-environment.rb) are reusable. Build with `rustc --edition=2021 -O`, run the resulting probe to `observations.txt`, then pass its directory to the verifier.

## Disposition

Do not integrate entry-flag reuse under the current strict behavior-preservation scope. Concurrent environment writers can change the second pre-call diagnostic independently of the first; reusing the flag removes that observation point. The isolated speed ratio does not justify changing that behavior. Keep this evidence for a future explicit diagnostic-contract decision, without treating it as an accepted fix. The next independent target is repeated path validation in `assignment::update_checked`, which calls `validate` and then `update_in_place` (which validates again) before writing. A test-only traversal counter is being used to reproduce that cost before any runtime change.
