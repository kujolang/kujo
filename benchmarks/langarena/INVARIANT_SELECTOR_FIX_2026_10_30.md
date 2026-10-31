# Checked invariant arithmetic selectors for captured-array batches

Runtime `f1893ea` admits bounded integer multiplication/division of invariant
operands into captured-array batch eligibility. Intermediate arithmetic uses
checked operations. Counter-dependent products/quotients, callbacks, floating
operands and unsupported expressions keep ordinary execution. Addition/subtraction
may combine a proven invariant product with the counted loop variable.
No persistent cache or collection representation was added.

## Matching release allocation results

Baseline runtime: `5458132`. Identical fixture paths and inputs are used for both
versions. At width 2,048 with one retained snapshot, VM/JIT requested allocation
per additional helper call is:

| Selector | Baseline bytes/call | Candidate bytes/call |
| --- | ---: | ---: |
| Literal control | 6,514 | 6,514 |
| Invariant multiplication | 284,846 | 6,514 |
| Invariant division | 284,846 | 6,514 |

Candidate once-snapshot slopes are flat across widths 8/2,048. Fresh snapshots
remain separately covered and retain required copies. The target matrix passes
72 allocation comparisons and 108 semantic checks. Historical controls pass
2,496 allocation comparisons, 3,744 semantic checks and nine portable contracts;
prior arithmetic controls pass another 192 comparisons and 288 semantic checks.
All 2,760 allocation comparisons and 4,140 semantic checks pass, with no measured
allocation-total or slope increases. These are allocation results, not speedups.

## Correctness and memory verification

- 1,272 library tests passed; eight ignored. Exact previous/current test sets match
  plus the new invariant-expression proof regression.
- 420 integration tests passed; one ignored. The new regression also passed
  separately against the frozen release CLI in VM, interpreter and JIT mode.
  JIT mode may use fallback; this does not prove native execution of every case.
- All 150 test-size benchmark checksums passed: 50 in each runtime.
- Regression coverage includes snapshots, signed/truncated division, intermediate
  overflow, division by zero, bounds errors, prior-write publication, callbacks
  and rejection of unsafe counter-dependent arithmetic.
- Exact coverage, raw counters/outputs, source and artifact identities, formatting
  and diff checks passed independent verification.
- All 72 alternating peak-RSS pairs were reviewed across 36 groups. Twenty-two
  groups have positive mean deltas; seventeen increase in both rounds. The largest
  mean increase is 182,272 bytes (2.09%) for literal-control/width2,048/fresh/JIT.
  All six large-owner once-snapshot target groups decrease in both rounds.
  Small-owner and fresh-snapshot results are mixed. The bounded observed increase
  is retained explicitly; its cause is not established and it is not dismissed
  as proven noise. This does not establish leak freedom or universal memory gains.

The initial Cargo targeted-test command was deliberately stopped while rebuilding
an unnecessary second runtime. The exact same test source was compiled as a
standalone test harness against the frozen release executable. Full release
library tests and the integration harness still ran; no failing test was skipped.
The command/log and workflow adjustment are retained in the evidence.

## Remaining scope

This closes the measured invariant multiplication/division selector cost.
Small mixed and dense nested-row copying remains confirmed in the
[nested-row baseline](NESTED_ROW_BATCH_BASELINE_2026_10_30.md). Remaining capture,
unsupported/deeper/escaping paths and representation dispositions still require
review. Sections 3/4 remain open. Production-size, cross-platform and clean-host
competitive performance are not established by this patch.

[Raw evidence and reproduction sources](invariant-selector-fix-2026-10-30.json)
include matching before/after data, fixtures, verifiers, build commands, raw
correctness/checksum outputs and memory review.
