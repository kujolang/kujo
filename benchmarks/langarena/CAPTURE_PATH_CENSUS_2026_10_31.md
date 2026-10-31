# Capture fallback reachability across all 50 benchmarks

Accepted runtime baseline: `f58f0a9` (runtime fix `5a0632d`). A separate diagnostic
checkout adds counters only; the accepted runtime is unchanged.

All **150 test-size checksum runs** pass: 50 each in VM, interpreter and JIT.
Fifteen positive controls also pass. Exact case membership, checksums, outputs,
counter records, source overlay and binary identities are verified.

## Observed paths

| Case | Runtime | Capture activations | Global bindings submitted | Local bindings submitted |
| --- | --- | ---: | ---: | ---: |
| Template::Regex | Interpreter | 111 | 333 | 111 |
| Compress::BWTEncode | Interpreter | 4,184 | 0 | 4,184 |
| Compress::BWTDecode | Interpreter | 1,016 | 0 | 1,016 |
| Compress::HuffEncode | Interpreter | 32 | 0 | 0 |
| Compress::HuffDecode | Interpreter | 35 | 0 | 0 |

Every other case/runtime combination records zero activation events in the
instrumented interpreter path. All 150 runs record zero compact-membership scans,
prefix snapshots, pending-prefix clones, bridge snapshots and reentry fallback
snapshots. These are event counts, not timings or allocation totals.

Six oversized-capture controls demonstrate nonzero membership checks in all
three runtimes. Nine bridge controls preserve reads, writes, snapshots and errors
while exercising prefix snapshots: VM/JIT reach the bridge path and interpreter
reaches the reentry fallback. The pending-publication clone counter has no
positive control in this corpus; its zero observation has that limitation.

The initial probe incorrectly called `VM::execute` alone and stopped on
Matmul::T4's suspension marker. That was a diagnostic harness defect, not a Kujo
failure. The corrected probe uses the CLI's `execute_until_suspend` and unbounded
scheduler path. The rejected run, source and successful initial controls are
retained separately; final evidence reruns the complete matrix.

## Section 3/4 disposition

This completes the requested bounded review of whether these remaining capture
fallbacks are reached by the test-size suite. It does **not** justify another
fallback refactor for benchmark performance: the targeted paths were not reached.
The five ordinary-activation cases may warrant cost attribution if interpreter
performance becomes the priority, but counts alone do not establish a defect.

The [prior residual review](SECTIONS_3_4_REVIEW_2026_10_31.md) remains accurate for
general user programs and host callbacks. Membership validation protects public
host-map edits; fallback snapshots protect reentry and publication semantics.
Fresh observable snapshots, payload allocation and unsupported collection-proof
shapes remain broader costs. This census does not measure them or prove them fixed.

The next benchmark performance investigation should follow actual VM/JIT hot
paths and allocation attribution, rather than assume these interpreter fallback
functions explain the remaining gap. Production-size reachability, clean-host
Goose/Rust comparisons, supported-platform acceptance and section 5 compiled
execution remain separate requirements. No claim of universal section 3/4
optimization, production completion or competitive speed follows.

## Reproduction and evidence

See [probe instructions](probes/CAPTURE_CENSUS.md) and the
[compressed evidence bundle](capture-path-census-2026-10-31.json.gz). The bundle
retains observations, positive-control fixtures, overlay hashes, source manifest,
verifier, build provenance and the rejected harness run. Historical handoff notes
describe intermediate states; `verified.json` is the final result.

Counters cover the named **Interpreter environment paths**, not every closure or
capture operation in the bytecode VM. They include worker threads and initialization
inside each selected benchmark process. Test-size absence does not prove absence
at every production input size. The loaded laptop does not support speed claims.
