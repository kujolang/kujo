# Constant-site array reservation

Runtime commit: `5a0632d`; baseline: `1ed1866`; candidate parent: `79cf3d6`.

The array proof retains its eight-entry exact destination union. When that fills,
constant assignment-site count now bounds reservation instead of total loop
writes. Any varying selector disables this bound, even after the union fills.
Raw endpoint, bounds, overflow, depth and visit checks remain unchanged. No new
statement forms are admitted and no heap set is added.

## Measured benefit

Requested bytes per added call at owner width 2,048 (VM and JIT agree):

| Destinations | Iterations | Baseline | Candidate |
| --- | ---: | ---: | ---: |
| 9 | 64 | 424,920 | 383,592 |
| 9 | 1,024 | 6,204,312 | 6,037,032 |
| 16 | 64 | 768,984 | 686,328 |
| 16 | 1,024 | 11,037,592 | 10,870,968 |

The width 32-to-2,048 allocation penalty becomes zero in all measured nine- and
sixteen-destination cases. Eight-destination controls are unchanged. All 48
target pairs and 24 added-call slopes are nonincreasing; 72 semantic runs pass.
A synthetic unchanged-counter control is rejected by the benefit assertion.
These are allocation measurements, not elapsed-time speedups.

## Verification and tradeoffs

- 1,309 library tests pass; eight remain ignored.
- 430 integration tests pass; one remains ignored.
- All 50 test-size checksums pass in each runtime: VM, interpreter and JIT.
- Three portable regression tests cover 24 runtime/shape combinations, including
  imported state, alias calls, retained snapshots, ordered errors and untouched
  elements. Baseline and candidate both pass these semantic tests.
- 3,880 strict allocation pairs pass totals and scaling gates; 5,928 candidate
  semantic observations and 18 host contracts pass.
- All 72 allocation boundary observations are retained. The 36 JIT cases receive
  108 follow-up pairs; three initially consistently positive cases receive 21
  additional pairs. Those focused repeats include nonpositive differences and
  overlapping ranges for bytes and requests.
- 144 peak-RSS pairs cover 72 groups. All 38 initially positive mean groups receive
  five more pairs each (190). Seventeen retain positive means; the largest is
  72,908.8 bytes, or 0.929%. No follow-up group increases in every pair, and all
  positive groups have overlapping ranges. These observations are accepted for
  this bounded change, not discarded or presented as zero memory regression.
- Formatting, diff checks, exact test membership, binary identities and 882
  frozen source hashes pass verification.

The [compressed evidence bundle](constant-site-fix-2026-10-31.json.gz) contains
raw observations, verifiers, source snapshots, negative control, explicit memory
reviews and the acceptance receipt. The bundled draft is historical progress;
the final acceptance receipt and this report supersede its pending statements.

## Remaining scope

This closes the reproduced greater-than-eight constant-destination reservation
cost. Varying selectors and unsupported expressions still use conservative
bounds. The [sections 3/4 review](SECTIONS_3_4_REVIEW_2026_10_31.md) continues to
identify capture membership traversal, reachable fallback materialization, and
fresh observable snapshot/payload representation as broader residual costs.
It does not define an unlimited queue of hypothetical optimizations.

Loaded-host RSS and timing cannot establish competitive throughput. Production
sizes, clean-host Goose/Rust comparisons, platform acceptance and section 5
compiled-execution work remain separate. No Kostya contact or publication occurred.
