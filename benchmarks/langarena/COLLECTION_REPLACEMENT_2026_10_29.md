# Collection-valued root dictionary replacement

Runtime `d853be4` retains eligible array, dictionary and other collection
replacements in a compact private entry. It preserves public snapshots, callback
and error boundaries, scalar/collection transitions, and replacement-root assignment
semantics. Literal operands still apply their normal publication boundaries.

Array/dictionary payloads retain existing shared handles directly. Larger collection
representations use a compact payload, avoiding the second candidate's fallback
allocation regression. Scalar batching policy and activation layout are unchanged.

Requested bytes per added helper call at 2,048 owner keys, retaining an initial
snapshot (16-to-32-call interval):

| Replacement | Counter | Before | After |
| --- | --- | ---: | ---: |
| array | captured | 632067 | 5272 |
| array | argument | 632060 | 5265 |
| dictionary | captured | 632233 | 5438 |
| dictionary | argument | 632226 | 5431 |

Candidate slopes are flat across 128 and 2,048 owner keys; VM/JIT slopes agree.
This measures allocation traffic, not elapsed speed. Fresh snapshots retain their
required copying. The earlier ineffective and fallback-regressing candidates remain
recorded as rejections.

Verification at this exact candidate:

- 1244 library tests passed; 8 ignored.
- 406 integration tests passed; 1 manual profiling test ignored.
- All 150 test-size checksums passed across VM, interpreter and JIT.
- 3760 allocation pairs and 5640 associated semantic checks passed.
- 54 paired memory observations completed; review and all raw observations are
  retained in the machine-readable evidence. Local peak RSS does not measure
  retained heap or establish clean-host acceptance.
- Exact source identities, test sets, fixture coverage, allocation totals/slopes,
  layout guards, formatting and diff checks were verified.

Large unaliased array/dictionary cases reduced mean peak RSS by 15.69–17.23%.
Other two-observation means ranged from -1.18% to +0.77%; the largest positive
individual change was 770,048 bytes. Six exact-path struct allocation pairs were
request/byte-identical to baseline. This does not prove zero memory regression;
clean-host memory verification remains pending.

[Machine-readable evidence](collection-replacement-2026-10-29.json) records the
artifact identities, target/fallback data, independent verification receipts and
memory review. Full supplementary reports remain in the recorded audit directory.

This accepts the measured root array/dictionary replacement optimization.
The current candidate exactly matches
all 192 deep/mixed/array-leaf allocation baselines: nested replacements still need
optimization. Struct-literal replacements also retain their earlier allocation
costs and remain in scope. Compound indexed arithmetic, captured-scalar batching,
remaining
capture traversal/fallback costs, and final collection review remain open in
sections 3 and 4. Production-size, platform and clean-host comparison gates are
separate and are not certified by these results.
