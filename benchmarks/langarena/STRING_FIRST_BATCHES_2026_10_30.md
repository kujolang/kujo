# Initial string and byte array batches

Runtime `94ad8f5` routes an eligible initial string or byte payload into the
existing bounded array batch storage. Previously a string-first update reached
numeric scalar journal preparation and fell back to copying; the same string
placed last could use batch promotion. The change preserves admission bounds,
compact scalar storage, snapshots and ordinary publication boundaries.

VM and JIT agree on requested bytes per added call for string-first updates
with one initial snapshot:

| Owner width | Baseline `ac7f234` | Candidate |
| --- | ---: | ---: |
| 3 | 5,472 | 5,260 |
| 4 | 5,608 | 5,260 |
| 8 | 6,152 | 5,260 |
| 2,048 | 283,592 | 5,260 |

Requests remain 66 per added call. Fresh snapshots are separate controls;
required copying is not claimed as removed. This is an allocation improvement,
not an elapsed-speed or clean-host comparison.

## Verification

- 384 shape allocation pairs / 576 semantic checks; 192 prior scalar pairs /
  288 semantic checks; 288 historical batch pairs / 432 semantic checks;
  432 boundary pairs / 1,296 semantic checks; 432 fallback pairs / 648 checks.
- Total: 1,728 allocation pairs and 3,240 semantic checks with no total or slope
  allocation regressions, plus six portable contracts. Seven corrupted target
  reports are rejected by the independent verifier.
- Six additional baseline/candidate contracts cover byte arguments, saved
  snapshots, multiple strings, invalid indices, errors and callback visibility
  across VM, interpreter and JIT. The expanded fixture is also in integration tests.
- 1,267 library tests passed, eight ignored; 416 integrations passed, one ignored.
  Exact prior test sets and new tests were checked, including source identities.
- All 150 test-size checksums pass: all 50 benchmarks in each runtime.
  Formatting and diff checks pass.

The first library-test build failed because the new test compared whole `Value`
objects with Rust `PartialEq`, which they do not implement. The corrected test
compares string/byte payloads directly. Original source, manifest and failure
output are preserved. An exact source comparison verifies that only the
`cfg(test)` module changed; normal release CLI/library/probe artifacts exclude
that module. Their measurements remain applicable, and the full library suite
subsequently compiled and passed with the corrected test.

## Memory and remaining scope

All 144 alternating peak-RSS pairs are retained: six shapes, widths 3/2,048,
once/fresh snapshots, three runtimes and two rounds. All 72 groups were reviewed. Large-owner string-first cases with one initial snapshot decrease in both observations across all three runtimes. Other groups show small changes in both directions; the largest positive mean is 159744 bytes (about 1.819 percent) for the unchanged large-owner fresh-snapshot array/scalar pair in JIT. All increases are retained. Accept this bounded routing change with these local uncertainties documented; two observations do not prove leak freedom, retained-heap behavior or universal memory improvement. Allocation totals and slopes show no regressions across 1728 comparisons.

Mixed two-write updates and arithmetic selectors still have their measured
owner-width penalty. Remaining capture materialization and representation
review also remain open. This patch does not close sections 3/4, establish
production-size acceptance or supply competitive speed measurements.

[Evidence](string-first-batches-2026-10-30.json) preserves exact artifacts,
fixtures, raw comparisons, verification scripts, test/checksum outputs, the
test-only correction proof and memory review. The accepted baseline is
[the shape refresh](ARRAY_SHAPE_REFRESH_2026_10_30.md).
