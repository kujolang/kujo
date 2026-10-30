# Checked arithmetic selectors for captured-array batches

Runtime `5458132` admits proven bounded addition/subtraction index expressions
into existing captured-array batches. Each intermediate expression is checked
for overflow at both endpoints of the single counted loop variable. Invariant
integer parameters, negative indices and nested addition/subtraction are covered.
Multiplication, callbacks and unsupported expressions retain the ordinary path.
Snapshot, error, callback and publication semantics remain authoritative.

## Matching release allocation results

Baseline runtime: `f92de4b`. Both versions use identical fixture paths and inputs.
The table reports requested bytes per additional helper call at array width 2,048
with one saved snapshot, in both VM and JIT. It does not report elapsed speedups.

| Form | Baseline bytes/call | Candidate bytes/call |
| --- | ---: | ---: |
| Literal-index control | 6,514 | 6,514 |
| Offset addition | 284,846 | 6,514 |
| Offset subtraction | 284,846 | 6,514 |
| Nested addition/subtraction | 284,846 | 6,514 |
| Negative-index addition | 284,846 | 6,514 |
| Counted-loop addition | 286,573 | 8,241 |
| Counted-loop subtraction | 286,573 | 8,241 |
| Negated counted-loop addition | 286,573 | 8,241 |

All seven arithmetic forms have flat once-snapshot allocation slopes between
widths 8 and 2,048. Fresh snapshots remain covered and retain required copies.
The target matrix covers 192 allocation pairs and 288 semantic checks. The
historical matrices cover 2,496 allocation pairs, 3,744 candidate semantic checks
and nine portable contracts. Neither matrix has total or slope allocation
increases: 2,688 allocation pairs and 4,032 semantic checks overall.

## Correctness and memory verification

- 1,271 library tests passed; eight ignored. The new proof test covers valid
  affine expressions and rejected overflow, bounds and unsupported operations.
- 419 integration tests passed; one ignored. The targeted arithmetic integration
  also passed separately in VM, interpreter and JIT. JIT mode may use fallback.
- All 150 test-size checksum cases passed: all 50 benchmarks in each runtime.
- Exact previous/current test sets, fixture coverage, raw outputs and artifact
  identities were independently checked. Formatting and diff checks passed.
- All 192 alternating peak-RSS pairs were reviewed across 96 groups. Twenty-seven
  groups have positive mean deltas; fifteen increase in both rounds. The largest
  mean increase is 253,952 bytes (3.30%), for loop-minus/width8/fresh/interpreter.
  All large-owner once-snapshot arithmetic groups decrease in both rounds.
  Small-owner, fresh-snapshot and literal-control results are mixed. These
  increases are retained explicitly; local peak RSS does not prove leak freedom
  or universal memory improvement.

## Remaining scope

This closes the measured bounded addition/subtraction selector cost. General
capture materialization, oversized fallback, dense or unsupported collection
paths and representation review still require evidence-backed dispositions.
Sections 3 and 4 remain open. Production-size, clean-host competitive timing and
cross-platform acceptance are not established by this patch.

[Raw evidence and reproduction sources](arithmetic-selector-fix-2026-10-30.json)
include matching baseline/candidate results, fixtures, verifiers, test/checksum
outputs, build commands and memory review.
