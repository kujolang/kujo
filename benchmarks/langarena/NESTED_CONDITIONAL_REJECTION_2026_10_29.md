# Conditional nested insertion candidate rejection

The nested insertion V3 runtime candidate is **rejected**, not shipped. Its
whole-body profitability guard disables previously available batching when one
possible first-write branch inserts a key, even when the executed branch updates
an existing key.

The [complete observations](nested-conditional-rejection-2026-10-29.json) contain
192 baseline/candidate allocation pairs, 288 VM/interpreter/JIT semantic checks,
and 1,728 allocation-probe executions. All semantic checks passed. Twelve total
allocation comparisons regressed: dictionary/struct/array owners, VM/JIT, one/two
calls, width two, five subsequent writes, existing-key branch, no retained snapshot.
The maximum excess was three requests and 1,118 requested bytes. Reported added-call
slope comparisons did not regress; that does not excuse the total-cost regressions.

At width two/five writes, each executable ran twelve times; other cases ran twice.
Samples alternate baseline/candidate order. Total comparisons use candidate maxima
against baseline minima. The earlier baseline's full-table replacement variation
remains visible in raw samples. No timing tolerance, discarded sample, or averaging
was used to accept a result. These are allocation measurements, not speed claims.

The accepted baseline remains root-insertion runtime `4cd3bf1`, with evidence in
`DICTIONARY_INSERTION_2026_10_28.md`. Candidate CLI SHA-256 is
`39dec7dd7f07b03f8aef7f5f0049a7a608f41b115188ffc5b3a790463499bd08`.
The report records all executable identities, fixture sources and individual samples.
Report SHA-256: `46628c5d98839c78fdd09c00219f82603df926c6480e168de7854d0f8681f9dd`.

The cold matrix was stopped incomplete after rejection; broad tests, checksums and
memory characterization were not accepted for this candidate. The proposed follow-up
separates the reserve hint from first-insertion admission and checks the actual
first key. It remains unverified. Existing-key batching must survive either branch,
and unprofitable first insertions must retain ordinary mutation.

`probes/nested-insertion-conditional-controls.rb` reproduces the rejection.
`probes/nested-insertion-mixed-repeated-controls.rb` provides equivalent conservative
sampling for unconditional mixed writes; that full matrix has not run on V3 because
the conditional gate rejected it first.
