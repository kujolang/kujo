# CSV port state audit — partial

The existing CSV kernel passes **288 complete-state controls** across VM,
interpreter and JIT. Every parsed field, converted coordinate, accumulated result,
final checksum and preserved input string is checked. No runtime or port kernel
changed; these are correctness controls, not speed measurements.

The matrix covers 0, 1, 2, 3, 10, 11, 32 and 128 rows, both LF and CRLF record
endings, result seeds 0 and 4,294,967,294, and three consecutive iterations.
Eleven is the registered test-size row count, but the control contents are
crafted rather than the benchmark's random input. Fields include escaped quotes,
literal backslash-n, quoted embedded newlines, Unicode/emoji, tabs, backslashes,
empty unquoted fields and positive/negative dyadic coordinates. Empty input
preserves the accumulated result and still includes the input-text checksum.

The original `csv_run` executes unchanged alongside a diagnostic clone that
exports its final field rows and coordinate structs after checksum calculation.
Both results must agree. The external oracle compares every field and x/y/z value
and checks the source text after each iteration. An independent verifier reparses
the CSV with Ruby CSV and computes averages using rational decimal coordinates
before the seven-decimal checksum formatting. It verifies all 288 case IDs and
raw-output/source identities, not just aggregate checksums.

## Source-level work differences

Kujo's `parse_csv_rows` first validates quoting, then uses `csv::Reader` and
materializes a full array of rows with owned strings for all fields. The benchmark
then traverses those rows to allocate `Coordinate` structs. Strict quote
validation is part of the existing builtin's error behavior and cannot simply be
removed for speed.

The pinned Rust benchmark consumes records through `simd_csv::ZeroCopyReader`
and converts the three numeric fields while building its point vector. Goose's
benchmark consumes records through `csv::each_record` and likewise builds points
inside the callback. Those source-level interfaces differ from Kujo's full field
matrix. This is not an audit of the comparator libraries' internal allocations,
nor evidence assigning a fraction of the timing gap to this difference.

## Remaining gates and evidence

- Random input preparation, malformed quotes/field counts/numbers, numeric
  extremes, complete library allocation/work counts, executed Rust/Goose
  comparison and production coverage remain open.
- Valid control agreement does not imply a shared malformed-input policy.
- Both comparator source files match upstream
  `3c9f84ddaeec834c0631747cb789e8cfd602f993`; they were inspected, not executed.
  The inspected Kujo builtin source matches accepted loop-fix commit `537406d8`.
- The frozen accepted Kujo CLI hash is
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
  The separate memory candidate is not used. Runs overlap the no-JIT build;
  neither speed nor native-path participation is claimed.

`probes/audit-csv-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT` reproduces the
controls. `csv-state-audit-2026-11-03.json.gz` retains full inputs/outputs,
collector, diagnostic source, independent verifier, identity receipts and pinned
source evidence. The CSV structural-equivalence ledger row remains partial.
