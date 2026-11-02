# Borrowed array-region candidate: correctness receipt

This is an unaccepted runtime candidate against accepted runtime `782c712d`.
The candidate patch is retained in the accompanying archive. It changes only
`src/jit.rs` among the 1,390 frozen source/build/test/vendor files; new regression
fixtures and their CLI test were separately committed as `4412ba67`.

The candidate release CLI SHA-256 is
`c95dfe17e7b9637b5b566e05777ca94feea62491f83ca5f1d0722ef10aa8cdb1`;
the source `src/jit.rs` SHA-256 is
`16ff1d5f5f45bb69d114873201209244e0b30e25e763897fe61adf8ac66de9d2`.
Release library/CLI build exited successfully. All input and binary identities
were rechecked independently after the checksum runs.

| Gate | Result |
| --- | --- |
| Expanded snapshot/mutation/callback/mixed-index fixtures | 6/6 CLI outcomes pass: two fixtures in VM, interpreter and JIT |
| Unchanged `test.json` workloads | 150/150 pass: all 50 in each runtime |
| Exact registry, configuration, checksum, exit and truncation verification | Pass; no omitted or truncated case |
| Generated repeated-read counter test | Pass: 9/129 reads use one region validation, without per-read roots |
| Broader JIT unit tests in the first candidate test binary | 91 pass, 7 ignored; known harness failure excluded |
| Corrected all-engine library unit test | Pending rerun |
| Candidate integration test executable | Pending; equivalent six CLI invocations passed |
| Paired speed, memory/resource, production and clean-host gates | Pending |

The initial library run selected two new tests: the barrier test passed, while
the all-engine fixture failed because its bare VM lacked the CLI's `null`
binding. The interpreter portion had passed. The test harness was corrected to
install builtins and `null`; its rebuilt unit result is still pending. The
broader unit binary predates only that `cfg(test)` correction, not a production
runtime change. An initial unqualified `--exact` counter-test invocation selected
zero tests; only the subsequent qualified one-test pass is counted.

The candidate reuses a borrowed array-buffer view without retaining an extra
Arc. It invalidates before mutation/rebinding, calls, unknown bytecodes and
overloaded operators. It still calls a Rust read helper; these results do not
establish direct generated scalar loads or completion of the native-access
architecture work. Bounds, mixed-type and generic error fallbacks remain.

The all-50 runner enables phase markers, so its elapsed values are diagnostic
and must not be used as competitive scores. No speedup or production-size pass
is claimed. A three-round paired screening comparison is a separate next gate;
it cannot replace independent-session or clean-host acceptance.

`array-region-correctness-2026-11-02.json.gz` retains 13 hash-verified files:
build identity/log, candidate patch, semantic controls, three complete checksum
reports, independent verification, original unit failure and broader unit log.
The verifier is `probes/verify-region-checksums.rb EVIDENCE_DIRECTORY` and requires
all 150 exact outcomes and unchanged identities, not merely a successful runner
exit. Frozen baseline profiles and their manifest remain in
`native-baseline-profiles-2026-11-02.json.gz`.
