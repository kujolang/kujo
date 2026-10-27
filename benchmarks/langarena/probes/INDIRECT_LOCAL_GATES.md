# Indirect retained-local recursion isolation

```sh
ruby benchmarks/langarena/probes/indirect-local-allocations.rb \
  MATCHING_PROBE MATCHING_BINARY NEW_OUTPUT_DIR [BASELINE_PROBE]
```

The runner measures a returned closure that reenters through another module's
exported `invoke` function. That function writes its own invocation counter;
the main program checks the exact count to verify the foreign route executed.
The retained closure has 0/128/2,048/8,192 extra local bindings, all used and
checked by a final call. Aliases must share state, while independently created
closures remain independent.

Read/write operations, depths 0/1/4 and one/16/32 outer calls produce 144 VM/JIT
allocation cases and 216 VM/interpreter/JIT semantic cases. Subtracting the
16-call run from the 32-call run gives added-call allocation. Depth zero is the
ordinary activation control. Generated owner modules, the bridge module, source
hashes, artifact identities and all measurements are retained.

Without a baseline the result is `characterized`. With a baseline probe, both
artifacts run identical fixture paths and any total or per-call request/byte
increase fails. A non-increasing paired result is `compared_without_regressions`;
it is not acceptance of an optimization. A subsequent fix needs an explicit
scaling criterion tied to the reproduced indirect-entry cost, plus independent
direct-recursion, task, callback, error, snapshot and cold/append controls.

This suite does not measure elapsed-speed improvements or peak memory, and it
does not establish full LangArena production-size compatibility. Do not relax
snapshot or public-owner semantics to reduce measured allocation.

## Foreign-prefix optimization acceptance

After a paired measurement, run:

```sh
ruby benchmarks/langarena/probes/foreign-local-prefix-gate.rb \
  NEW_OUTPUT_DIR/report.json NEW_ACCEPTANCE_JSON
```

The gate implements criteria defined before candidate measurements. It verifies
exact coverage, recomputes added-call deltas, and rejects total or per-call
request/byte increases. After subtracting depth-zero activation, extra recursion
cost must be constant across all four widths at both depths one and four,
independently for reads/writes and VM/JIT. All twelve depth-four groups with
nonempty retained locals must strictly reduce both requests and bytes against
the reproduced baseline. A paired non-regression report alone cannot pass.

A synthetic unchanged-pair control constructed from the recorded baseline is
rejected because its recursion overhead scales with width. This tests the gate;
it must not be described as a newly executed baseline/candidate measurement.

## Preserve the accepted direct path

Run `direct-local-controls.rb PROBE BINARY NEW_DIR BASELINE_PROBE` separately.
It uses the same direct-recursion fixture matrix as the restoration runner:
96 allocation pairs and 144 semantic runs. It requires no total/per-added-call
regression and constant extra depth-two recursion overhead across all widths.
It does not demand a new direct-path improvement when evaluating a foreign-path
fix. Keep this control distinct from the strict foreign-prefix acceptance gate.
