# Dictionary batching with primitive captured dependencies

Runtime `30e1511` lets eligible imported helpers batch dictionary writes while
reading or updating primitive captured values. Admission checks actual values in
the entered capture on every invocation, without copying them for the check. The
parameter fast path and task/reentry exclusions remain. Unknown or collection-valued
dependencies retain ordinary fallback; a struct operator introduced between calls
is not mistaken for the earlier number. Snapshots, callback visibility, failed
constant-assignment publication and replacement-array ordering remain covered.

Requested bytes per added call at dictionary width 2,048 (VM and JIT agree):

| Dependency | Baseline `268add2` | After |
| --- | ---: | ---: |
| Captured read | 633,043 | 7,061 |
| Captured write | 633,051 | 7,069 |
| Captured increment and return | 633,051 | 7,069 |
| Parameter-only control | 7,062 | 7,062 |

All six captured-scalar target groups have flat owner-width scaling across widths
128 and 2,048. These are allocation slopes, not elapsed-time speedups. The baseline
refresh reused the same fixture paths as the earlier compound/capture isolation;
all 128 baseline allocation totals matched the earlier accepted runtime.

Verified acceptance:

- 1,262 library passes, eight existing ignored tests.
- 412 integration passes, one existing ignored test.
- All 150 test-size checksums: 50 benchmarks in VM, interpreter and JIT.
- 576 paired allocation comparisons and 864 semantic checks: 128 target, 64 cold,
  288 historical array-batch and 96 dictionary fallback/boundary/operator/dense
  comparisons, with no allocation-total or added-call slope regressions.
- 15 additional scalar/compound contracts, including dynamic operators and root
  replacement; all 18 historical array-batch hot groups remain flat.
- 72 paired peak-memory observations across 36 reviewed groups.
- Exact source/artifact identities, complete test-name coverage, formatting and
  diff checks. The acceptance receipt and audit implementations are preserved.

All nine large-dictionary groups retaining only the original snapshot reduced mean
peak RSS by 20.40–20.76%, about 10 MB per process. Small and retained-version cases
had mixed signs. The largest small mean increase was 75,776 bytes/0.986%; the largest
large retained-version mean increase was 438,272 bytes/0.626%, positive in both
observations. The largest individual increase was 1,200,128 bytes, paired with a
716,800-byte decrease in the other observation. These are recorded tradeoffs, not
established noise. Two local samples do not prove universal memory neutrality,
retained-heap bounds, leak freedom or clean-host performance.

Two audit-tooling failures are retained: the standalone regression harness needed
matching thin LTO to consume release dependencies, and system Ruby 2.6 required
`map`/`compact` instead of `filter_map`. Neither correction changed runtime code,
comparison thresholds or completed allocation observations.

[Complete evidence](captured-scalar-batches-2026-10-29.json) includes observations,
fixtures, hashes, scripts, tests, checksums, memory review and those tooling failures.
Compound indexed arithmetic retains the measured 587,520-byte width penalty in all
four counter variants; all compound slopes are unchanged. Broader capture
materialization, unsupported collection paths (including initial scalar-first array
journals) and representation review remain open. Sections 3/4 are not complete.
Production-size, platform and competitive comparisons remain separate.
