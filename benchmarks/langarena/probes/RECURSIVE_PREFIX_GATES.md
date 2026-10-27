# Direct recursive prefix transfer gate

Run the existing measurement matrix first:

```sh
ruby benchmarks/langarena/probes/recursive-local-allocations.rb PROBE BINARY NEW_DIR BASELINE_PROBE
ruby benchmarks/langarena/probes/recursive-prefix-gate.rb NEW_DIR/report.json NEW_ACCEPTANCE_JSON
```

The first runner was introduced for restoration-map reuse. Its improvement gate
requires strictly fewer requests and bytes at every nonempty local width,
including 128. The subsequent prefix-transfer change addresses a different cost:
map copying on recursive entry after the bounded pool is exhausted. At 128
locals the previous runtime already reuses enough storage, so unchanged
allocation there is a control rather than evidence of missing improvement.

The prefix gate preserves the original measurement report and any restoration
gate rejection. It accepts only a completed report: either passed or rejected
specifically for missing restoration reduction. Other failures remain failures.
It requires:

- Exact coverage of 96 baseline/candidate allocation pairs and 144 semantic runs.
- No request or requested-byte regression in any total or added-call result.
- Recomputed added-call deltas matching the original report.
- Constant extra depth-two recursion cost across 0/128/2,048/8,192 retained locals,
  independently for reads/writes and VM/JIT. Ordinary activation cost is subtracted.
- Strict request and byte reductions in all eight previously width-dependent
  groups: two larger widths, two operations and two runtimes.
- A baseline that actually exhibits width-dependent recursive overhead.

This is not an elapsed-time, peak-memory or ordinary activation improvement
claim. In particular, the 8,192-local outer activation can still allocate with
width even when its added recursive work is constant-cost.

For the initial prefix-transfer candidate, the earlier restoration gate rejected
unchanged 128-local allocation despite improvement at larger widths. That raw
rejection is retained. The new gate also rejects the prior accepted runtime
`e9e5695` using its preserved measurements, because its recursion overhead still
scales with width. Evidence reports must disclose this gate correction and
preserve both results; do not rewrite the old report status to passed.
