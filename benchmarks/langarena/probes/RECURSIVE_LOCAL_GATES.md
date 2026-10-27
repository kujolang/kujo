# Recursive local restoration gates

```text
ruby benchmarks/langarena/probes/recursive-local-allocations.rb CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR BASELINE_PROBE
```

Use allocation probes linked against matching baseline/candidate runtime libraries
with recorded executable, source, library and compiler identities. Both probes
execute each identical fixture path. Omitting the baseline argument performs
characterization only; it does not accept an optimization.

The matrix uses 0/128/2,048/8,192 retained extra locals, read/write helpers,
recursion depths zero and two, and 1/16/32 outer calls. Every retained name is
referenced and validated by a final flag-controlled invocation. Aliases share
the selected closure's state, while a separate factory instance stays independent.
Subtracting the 16-call total from the 32-call total separates repeated call costs
from module creation and the final full-name check. One-call cases retain cold
setup coverage; depth zero isolates ordinary activation from recursion.

Acceptance requires all 96 allocation pairs and 144 VM/interpreter/JIT semantic
runs, no increase in total or per-call allocation requests/bytes, and a strict
request/byte reduction for every nonempty depth-two writing case. Passing the
same probe as both baseline and candidate must fail the strict reduction gate.
Elapsed timings on a loaded machine are not acceptance evidence.

Also run `local-capture-allocations.rb` with the same baseline/candidate artifacts
to protect ordinary activation, and retain the cold, module, nested-publication,
append, ownership/reentry, and benchmark checksum gates. Reusing a suspended
caller's map must preserve changed names and binding kinds, caller parameters,
live owner values, snapshots, errors, and release of stale temporary values.
Pool bounds must remain unchanged. This target does not close general oversized
activation, captured-name copying, or broader capture traversal costs.
