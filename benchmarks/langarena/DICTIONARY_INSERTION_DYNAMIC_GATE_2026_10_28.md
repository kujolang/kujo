# Dynamic insertion batch regression gate

The [dynamic gate](probes/dictionary-insertion-dynamic-controls.rb) complements
[cold insertion controls](probes/dictionary-insertion-cold-controls.rb) by passing
the number of writes as a helper argument. It covers 1, 3, 4, 16, 64 and 257 writes;
initial dictionary widths 0, 2, 8, 128 and 2,048; one, two, 16 and 32 calls; and
no initial snapshot versus fresh snapshots.

Each run checks 480 paired VM/JIT allocation observations and 720 semantic
executions across VM, interpreter and JIT. Assertions verify the entire resulting
dictionary and the preservation of saved values. Total allocation requests,
requested bytes and per-call slopes are checked separately.

[Preserved results](dictionary-insertion-dynamic-gate-2026-10-28.json):

- The accepted `6d28861` baseline compared with itself passes with all 480 pairs
  exactly equal and all 720 semantic executions passing.
- The rejected first insertion candidate fails 20 allocation pairs, while all
  720 semantic executions pass. All regressions are first calls without a saved
  snapshot: 3/4 writes into an empty owner; 16 writes into widths 0/2; and 64/257
  writes into widths 0/2/8. VM and JIT agree on the affected cases.
- The largest observed increase is 28,913 requested bytes for 257 insertions
  into an empty dictionary. No per-call slope regressions appear in this matrix.

This verifies the gate, not a replacement runtime. The revised insertion
candidate remains under verification. Allocation traffic is neither retained
memory nor evidence of competitive speed. Sections 3 and 4 remain open.

Run with a fresh output directory:

```sh
ruby benchmarks/langarena/probes/dictionary-insertion-dynamic-controls.rb \
  BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT
```

A regression returns a nonzero exit and preserves the complete report. The JSON
evidence includes runner source, fixture contents, identities and both reports.
