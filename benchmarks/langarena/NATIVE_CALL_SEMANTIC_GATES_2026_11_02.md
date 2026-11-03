# Native-call semantic entry gates

The invalid-call repair passes **155 unique regression tests and 150/150 unchanged
test-size checksums**. VM arity/non-callable errors now reach ordinary exception
handlers; interpreter non-callable invocation raises an error rather than
silently returning zero. This is a correctness prerequisite, not a speed win.
Runtime repair commit: `5fe52fc5`.

Baseline executable SHA-256:
`3d6b3ea8e621d6bf8f74ce2cd8f102bc2da0629221b6d3fdc091640ff4e790b3`.
The unchanged frozen CLI was used, not the stale array-candidate release binary.

## Reproduced failures

- VM: a too-few-arguments call inside a relay bypasses its caller's exception
  handler and terminates with exit 4.
- Interpreter: calling an integer returns zero and exits successfully instead
  of raising a runtime error. The hot fixture consequently catches four errors
  rather than the expected five.
- JIT opt-in: the hot fixture reaches its direct wrong-arity call and terminates
  instead of catching it. Compiled helper calls and VM fallback must both recover.

All three new Rust regression tests fail against the accepted CLI: cold caller
recovery, hot semantic controls, and uncaught invalid-call exit status. The
standalone test harness uses the same `tests/jit_call_contracts.rs` source and
explicitly points `CARGO_BIN_EXE_kujo` at the frozen baseline.

The separate baseline debug trace confirms successful compilation of `indirect`,
`ordered_call`, `apply`, `field_call`, `mutate` and `recurse`. Compilation logs
are not self-CPU measurements or proof of a future direct-callee optimization.

Local raw evidence is under
`.audit-evidence/native-performance-campaign-2026-11-02/`:
`call-contract-entry.json`, `call-contract-baseline-tests.log`, and
`call-contract-baseline-trace.stdout` / `.stderr`.

## Verified repair and remaining work

The candidate routes VM arity/non-callable errors through normal exception
handling and makes interpreter non-callable invocation return an error value.
The hot fixture covers argument order, dynamic rebinding, closures, field
receivers, nested snapshots, recursive transfer and recovery. Cold controls
separately check caller-frame unwinding and uncaught exit code 4 in all engines.

- [x] Reproduce failures on the frozen accepted runtime.
- [x] Add cold and hot regression controls.
- [x] Build the corrected CLI and pass these controls in all three runtimes.
- [x] Verify all 150 unchanged test-size checksum combinations.
- [x] Pass existing immediate/owned-call and runtime parity regression gates.
- [x] Verify source/binary identities and archive final outcomes.
- [ ] Resume guarded compiled-callee performance implementation and measurement.

The corrected release CLI was frozen with SHA-256
`bf5bd666143cabb2339a72a5635702a6c735639e25d12663396aea7a87abaed4`.
The manifest verifies 1,392 source identities. Only `src/vm.rs`,
`src/interpreter/mod.rs` and the existing loop-test expectation differ among the
baseline manifest's files; the new test and hot fixture are also bound by hash.

| Suite | Unique passing tests |
| --- | ---: |
| New call contracts | 3 |
| Owned arguments/imported callbacks | 2 |
| Native environment/capabilities | 3 |
| Task policy/cancellation | 3 |
| Immediate arguments/recursion depth | 8 |
| Loop-condition calls/fallback | 13 |
| VM/interpreter parity surfaces | 123 |
| **Total** | **155** |

The new call-contract suite and the last three suites pass through Cargo release
test runs. The owned/environment/task suites use their unchanged CLI-test sources
compiled independently against the frozen CLI. Standalone focused tests and
early parity/loop runs also pass, but their Cargo reruns are not counted twice.
Formatting and staged-diff checks pass. This is targeted coverage, not a claim
that the entire repository's test inventory or production benchmarks ran.

An exploratory standalone library-test link failed on Apple/Rust LLVM bitcode
compatibility; its retained build log is a tooling failure, not a runtime test
result. The matching Cargo build and parity tests subsequently passed.

All **150/150 unchanged test-size checksum combinations** pass with exact coverage,
exit status and untruncated-output checks. Candidate/source identities were
checked between modes. Seven benchmark/driver identities were checked against the
frozen checkpoint and pinned profile source during and after the run; this is
not a before-run input-hash receipt. Checks overlapped compilation and supply no
speed evidence.

`native-call-semantics-2026-11-02.json.gz` retains baseline failures, source and
binary identities, candidate patch, test sources, raw checksum/test outputs,
runner and verifier, with per-artifact SHA-256 checks. The verifier rejects
incomplete suites, ignored/filtered tests, missing cases, truncation and identity
drift. Local evidence is under
`.audit-evidence/native-performance-campaign-2026-11-02/call-semantic-v1/`.

Reproduce the receipt verification with
`ruby benchmarks/langarena/probes/verify-native-call-semantics.rb EVIDENCE_DIRECTORY`.
For fresh test execution, run Cargo release tests for the seven named suites;
the all-50 runner and actual execution commands are retained in the archive.
Guarded compiled-callee implementation, before/after speed measurements,
production coverage and clean-host comparisons remain open.
