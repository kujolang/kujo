# Lexical scope distribution control

Accepted runtime `b1bf786` passes 36 allocation observations and 54 semantic
runs. The same 6,144 extra retained locals are distributed across one, two or
three nested lexical blocks. Separate closures, aliases, mutations and every
retained value are checked across VM/interpreter/JIT.

| Lexical blocks | Read bytes per added call | Write bytes per added call |
| --- | ---: | ---: |
| 1 | 1266.0 | 1277.0 |
| 2 | 1266.0 | 1277.0 |
| 3 | 2042119.0 | 2042130.0 |

The three-block shape confirms an additional aggregate-budget fallback:
identical total binding count and values allocate about 2.04 MB per added call
instead of roughly 1.3 KB. The capture fix must cover this distribution, not
only single-scope counts or long names.

Slopes compare 16 and 32 calls; VM/JIT agree. Lexical blocks are source shapes,
not proof of the physical captured-map layout. These controls cannot establish
that arbitrary public-host environments obey the same allocation scaling.
Add direct multi-map capacity/retention tests when implementing the capture
fallback fix. Existing [count boundary](CAPTURE_POOL_BOUNDARY_2026_10_30.md)
and [key-byte](CAPTURE_KEY_BYTES_2026_10_30.md) gaps remain separate confirmed
fix targets. No runtime change or elapsed-speed claim accompanies this control.

[Evidence](capture-scope-budget-2026-10-30.json) preserves deduplicated source,
matching artifacts, allocation rows, semantic statuses and independent verifiers.
