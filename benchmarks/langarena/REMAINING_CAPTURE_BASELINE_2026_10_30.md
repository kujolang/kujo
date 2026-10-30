# Remaining capture costs: accepted-runtime refresh

Runtime `7724558` passes 80 allocation observations in VM/JIT and 120 semantic
checks across VM, interpreter and JIT. Existing capture-allocation improvements
remain intact. This refresh does not close the remaining capture work.

| Retained extra locals | Read bytes per added call | Write bytes per added call |
| --- | ---: | ---: |
| 0 | 1,266 | 1,277 |
| 128 | 1,266 | 1,277 |
| 2,048 | 1,266 | 1,277 |
| 8,192 | 1,650 | 1,661 |

VM and JIT agree. The oversized retained-local path still has a small fixed
allocation premium, rather than its historical multi-megabyte copying cost.
Separate imported-read controls stay flat across 0/128/8,192 unrelated bindings
and collection widths 128/2,048: 1,086 bytes per added call in VM and 1,086.8125
in JIT. The fractional JIT slope reflects the aggregate difference between
64-call and 128-call runs, not fractional individual allocations.

A 100,000-call read workload retaining 8,192 locals passed on both the normal
release CLI and the matching allocation probe. Five-second local samples show:

- The stripped release CLI lacks names for its hot runtime frames; this sample
  cannot establish source-level attribution.
- The symbol-bearing allocation probe repeatedly samples
  `Environment::capture_activation_with_pool` calling `BindingMap::get`.
- Source inspection finds exact membership validation at that boundary:
  matching name counts plus a lookup for every owner key. It protects public
  host edits, including same-size name replacements; it cannot simply be skipped.

The next experiment should reduce repeated lookup work while retaining exact
membership validation, public-host mutation behavior and snapshot isolation.
Allocation counters alone cannot verify this traversal improvement. Instrumented
samples identify a path to investigate; they are not production CPU percentages,
clean-host speed comparisons or proof that all remaining costs share this cause.
General capture materialization, exceptional fallback, scalar-first collection
journals and broader representation review remain open.

[Evidence](remaining-capture-baseline-2026-10-30.json) preserves fixture sources,
artifact identities, raw allocation rows, recomputed slopes, sampling output and
audit implementations. Both profiling executions returned the expected result.
No runtime changes accompany this baseline.
