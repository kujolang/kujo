# Bounded nested collection row batches

Runtime `b1bf786` retains eligible mixed and collection-valued nested array-row
updates in bounded private storage. It preserves compact scalar and single-entry
paths, snapshots, negative-index aliases, callbacks, bounds errors and publication
ordering. Admission rejects uncertain selectors and observing expressions; ordinary
mutation remains the fallback. This accepts the measured row scope, not all
collection operations or completion of sections 3 and 4.

Compared with accepted `f1893ea`, requested bytes per added call at row width 2,048,
with an initial retained snapshot, are:

| Workload | Before | After |
| --- | ---: | ---: |
| Dictionary row, three arrays | 286,779 | 7,715 |
| Dictionary row, three mixed values | 286,353 | 7,289 |
| Struct row, three arrays | 286,776 | 7,666 |
| Dictionary row, 32 writes | 316,341 | 39,737 |
| Struct row, 32 writes | 315,178 | 38,528 |
| Dictionary row, 128 writes | 408,885 | 140,153 |
| Dense dictionary row, 512 writes | 779,061 | 541,817 |
| Dense dictionary row, 2,048 writes | 2,259,765 | 2,148,473 |

VM and JIT agree in these groups. Fixed-write width penalties are removed across
the tested widths; dense controls retain work proportional to their write count.
All target cold/fresh-snapshot totals and per-call allocation slopes are no worse.
These are allocation-traffic results, not elapsed-time speedups or Goose/Rust comparisons.

Verification:

- 1,276 library tests passed; eight ignored. Exact prior test set plus four new
  tests verified against the final frozen source.
- 421 integration tests passed; one ignored. The new portable contract runs
  dictionary, struct and deeper fallback cases in VM, interpreter and JIT.
- All 150 test-size benchmark checksum cases passed across the three runtimes.
- 3,048 allocation comparisons, 4,572 associated semantic runs and 18 boundary
  runs passed across target, historical and selector matrices; totals and slopes
  have no measured allocation regressions.
- 288 alternating peak-RSS pairs in 144 groups were verified and individually
  reviewed. 30 groups have a positive mean delta; 9 increased in both rounds. The largest positive group mean is 1,372,160 bytes (10.046%).
- Source/artifact identities, exact fixture coverage, storage-layout tests,
  formatting and diff checks passed.

Memory review retains every increase and its disposition in the evidence.
A focused six-pair follow-up of the largest dense workload retains positive
fresh-snapshot RSS means: VM +669,696 bytes, interpreter +247,808 and JIT
+535,893.33. These are accepted observed process-memory tradeoffs, not dismissed
as laptop load and not a claim of memory reduction.

An additional 288 paired measurements across the same 144 groups count live Rust
global allocations, including successful allocation, resize and free operations.
After normalizing for the one-byte executable-name difference at measurement
entry, ending live growth matches in every pair. The largest positive execution
peak-growth difference is 50 bytes; the largest dense fresh-snapshot workload
matches exactly in all three runtimes. The probe checks its allocation accounting
before each run, and both probes link the verified frozen runtime libraries.
These whole-execution peaks include module setup; they do not separately identify
the peak of every helper call.

This rules out additional ending requested-live Rust data in these observations.
It does not identify the RSS cause: native allocations, mmap, code pages and
allocator arenas are outside these counters. The private batch has bounded
capacity and is released at publication; no general leak-freedom, identical RSS,
production-size or clean-host memory guarantee follows. All positive RSS groups
have explicit review dispositions in the evidence.

The first candidate failed a new full-table overwrite test: HashMap insertion
increased capacity from three to seven. The correction updates existing entries
with `get_mut` before considering insertion. The failed patch and log remain in
the evidence. A later formatting-only line wrap is also recorded; the full library
suite and release build were rerun against final formatted source.

[Complete evidence](nested-row-batches-2026-10-30.json) preserves raw results,
fixtures, identities, independent verifiers, memory review and rejected-attempt
provenance. The separate confirmed multi-key dictionary copying cost, broader
capture/fallback and representation review remain open. Production-size, platform
and clean-host competitive gates remain separate; no upstream contact occurred.
