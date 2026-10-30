# Mixed two-write array batches

Runtime `f92de4b` stores eligible mixed two-write captured-array updates in a
bounded two-entry journal. Scalar-first and collection-first updates transfer
their pending writes without materializing the full owner. Numeric pairs and
same-kind array/dictionary pairs retain their compact storage. The new mixed
journal is boxed to preserve the capture-frame layout.

Admission still checks bounds, ownership, supported values and observation
boundaries. Real snapshots, callbacks, errors and ordinary publication remain
authoritative. Two-element owners are included; no general aliasing or value
semantics rule is weakened.

## Allocation results

VM and JIT agree. With one initial snapshot, requested bytes per added call are:

| Ordered payload pair | Baseline width 2 | Baseline width 2,048 | Candidate widths 2/3/8/2,048 |
| --- | ---: | ---: | ---: |
| scalar-array | 5,909 | 284,165 | 5,693 |
| array-scalar | 5,909 | 284,165 | 5,693 |
| scalar-dict | 6,075 | 284,331 | 5,859 |
| dict-scalar | 6,075 | 284,331 | 5,859 |
| array-dict | 6,659 | 284,915 | 6,443 |
| dict-array | 6,659 | 284,915 | 6,443 |
| string-array | 5,954 | 284,210 | 5,738 |
| array-string | 5,954 | 284,210 | 5,738 |
| bytes-array | 5,909 | 284,165 | 5,693 |
| array-bytes | 5,909 | 284,165 | 5,693 |
| struct-scalar | 6,070 | 284,326 | 5,846 |
| scalar-struct | 6,070 | 284,326 | 5,846 |

Baseline is `94ad8f5`. Candidate slopes are flat across all four widths for
every listed form. Fresh-snapshot cases are retained as separate controls;
required snapshot copying is not claimed as removed. Most forms save one
allocation request per added call; struct/scalar forms save three.
These are allocation improvements, not elapsed-speed or clean-host claims.

## Verification

- Target: 768 allocation pairs, 1,152 semantic checks and three portable
  runtime contracts.
- Historical controls: 384 shape pairs / 576 semantic checks; 192 scalar pairs /
  288 checks; 288 batch pairs / 432 checks; 432 boundary pairs / 1,296 checks;
  432 fallback pairs / 648 checks.
- Total: 2,496 allocation pairs and 4,392 semantic checks, with no total or
  slope allocation increases in these matrices, plus nine portable contracts.
  Seven corrupted target reports are rejected by the independent verifier.
- 1,270 library tests passed, eight ignored; 418 integrations passed, one
  ignored. Exact prior test sets and the added tests are verified. The original harness
  passes 417 tests; a standalone integration adds one test for full two-entry
  promotion, plus six direct baseline/candidate runtime contracts.
- All 150 test-size checksums passed: all 50 benchmarks in VM, interpreter
  and JIT. Formatting and diff checks passed.

The first release attempt failed because the disk was full while writing a
compiler object. Removing 2 GiB of reproducible debug artifacts allowed the
unchanged source to build successfully. Original failure logs, commands,
states and recovery evidence are retained. The extra promotion test initially omitted thin LTO; the linker rejected Rust
dependency bitcode. Correcting that test-build flag allowed the unchanged test
to link. Its first fixture was strengthened to ensure a second call exercises
unique-owner promotion; original fixtures and linkage logs are retained.
A later library-test build stopped without a terminal result; OS inspection
confirmed its compiler and orchestration processes absent. Its cause is unknown.
The incomplete attempt was archived and the gate resumed from source validation;
no incomplete attempt is counted as a test pass.
Baseline binaries were preserved;
normal release dependency flags match the baseline.

## Memory and remaining scope

All 288 alternating peak-RSS pairs are retained: 12 forms, widths 2/2,048,
once/fresh snapshots, three runtimes and two rounds. Reviewed all 144 groups (288 paired observations). 58 groups have positive mean RSS deltas, including 36 increasing in both rounds; 61 decrease in both. The largest positive mean is 159,744 bytes (2.09%) for array-dict/width2/once/JIT. All width2048/once groups decrease in both rounds; fresh snapshots and small owners show mixed changes. Accept these measured small peak-RSS increases alongside flat activation layouts and zero requested-allocation increases in the control matrices. They are retained explicitly, not dismissed as proven noise. This bounded two-entry journal has no growing retained cache; these observations do not prove absence of leaks or universal memory improvement. Loaded-host timings establish no competitive speed claim.

Arithmetic selectors retain their separately measured copying cost. Broader
capture materialization, unsupported/dense update paths and representation
review remain open. This patch does not close sections 3/4 or establish
production-size, competitive-speed or cross-platform acceptance.

[Evidence](mixed-pair-batches-2026-10-30.json) includes source/artifact identities,
raw allocation and semantic results, independent verifiers, exact test/checksum
outputs, memory review and the disk-full recovery. The reproduction baseline is
[mixed-pair characterization](MIXED_PAIR_BASELINE_2026_10_30.md).
