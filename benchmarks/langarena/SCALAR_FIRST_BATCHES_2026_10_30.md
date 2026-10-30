# Scalar-first captured-array batches

Runtime `ac7f234` removes owner-width copying from eligible scalar-first mixed
updates and bounded scalar-only batches. It transfers the existing two-entry
scalar journal into batch storage without publishing or detaching the array.
One/two scalar writes retain their compact storage. Admission still requires a
bounded, observation-free body; ordinary fallback publication remains authoritative.

Normal release artifacts and dependency flags match the `18ebd21` baseline.
VM and JIT agree on these requested bytes per added call, at width 2,048 with
one initial snapshot:

| Update order | Baseline | Candidate |
| --- | ---: | ---: |
| Scalar, array, dictionary | 284,583 | 6,251 |
| Three scalars | 283,249 | 4,917 |
| Array, dictionary, scalar control | 6,251 | 6,251 |

Candidate slopes are flat across widths 3/8/64/2,048. Fresh snapshots remain
separate controls; their required copy costs are not claimed as removed.
These are allocation measurements, not elapsed-speed improvements.

## Verification

- 192 targeted allocation pairs, 288 semantic checks and three portable contracts.
- 288 historical batch pairs / 432 semantic checks, 432 boundary pairs / 1,296
  semantic checks, and 432 fallback pairs / 648 semantic checks.
- Combined: 1,344 allocation pairs and 2,664 semantic checks, with no total or
  slope allocation regressions. All 18 historical hot batch groups remain flat.
- 1,266 library tests passed, eight ignored; 415 integration tests passed, one
  ignored. Exact prior test-name/status sets were checked, not just pass counts.
- All 50 test-size benchmarks pass in VM, interpreter and JIT: 150 checksums.
- Formatting and diff checks pass. Seven deliberately corrupted target reports
  are rejected by the independent verifier.

Regression coverage includes scalar bit preservation, Boolean/null values,
negative and repeated indices, saved snapshots, replaced owners, invalid bounds,
error publication and callback fallback across all three runtimes.

## Memory and limitations

All 72 alternating peak-RSS comparisons are retained: three shapes, two widths,
once/fresh snapshots, three runtimes and two rounds. Large-owner once-snapshot
scalar-first/scalar-only groups decrease in both observations across all runtimes.
Other groups vary in both directions. The largest positive group mean is
114,688 bytes (approximately 1.501%) for width-three fresh-snapshot scalar-only
interpreter execution. This small increase is retained rather than hidden.
Two local observations do not prove retained-heap behavior, absence of leaks or
universal memory improvement. Clean-host speed comparisons remain pending.

The reused historical batch verifier initially demanded another strict improvement
over its already optimized baseline and rejected the run despite zero allocation
regressions. Its control-only condition was corrected to require preserved flat
scaling and no regressions. The new scalar-first improvement has a separate
target gate. The original failure, correction and unchanged observations are
preserved in the evidence; no runtime change was made to bypass that gate.

General compound selectors, unsupported collection shapes, remaining capture
materialization and the broader representation review remain open. This patch
does not close sections 3 and 4 or establish production-size/competitive acceptance.

[Evidence](scalar-first-batches-2026-10-30.json) includes matching artifact and
source identities, raw observations, exact fixture sources, verification scripts,
full test/checksum results and the memory review.
