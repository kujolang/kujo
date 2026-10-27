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
