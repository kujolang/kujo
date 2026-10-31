# Capture key-byte fallback isolation

Accepted runtime `b1bf786` passes 72 allocation observations and 108 semantic
runs. With 128 extra retained locals, all tested name lengths remain at
1,266/1,277 requested bytes per added read/write call. At 1,024 extra locals:

| Name prefix bytes (plus decimal suffix) | Read bytes/call | Write bytes/call |
| --- | ---: | ---: |
| 9 | 1266.0 | 1277.0 |
| 129 | 466483.0 | 466494.0 |
| 241 | 581171.0 | 581182.0 |

VM/JIT slopes agree; VM/interpreter/JIT validate closure snapshots, aliases,
mutation and retained values. Slopes compare 16→32 calls. A first overlong fixture
was rejected by the lexer-enforced 256-character identifier limit; it is retained as
fixture-generation error evidence and excluded from passing totals. The longest
accepted identifier has 245 characters.

This confirms the key-byte part of the [pool boundary gap](CAPTURE_POOL_BOUNDARY_2026_10_30.md):
compact metadata admission by binding count misses smaller captures whose names
exceed the idle pool's 128 KiB key budget. Raising only the count threshold cannot
address both demonstrated fallbacks. Preserve bounded retention, cold costs,
public host edits and snapshot semantics in the eventual fix. Combined-scope
capacity remains a separate control to add. No runtime change or speed claim.

[Evidence](capture-key-bytes-2026-10-30.json) includes source-deduplicated fixtures,
matching artifact identities, raw counters, semantic statuses, recomputed slopes,
verifiers and the rejected fixture log. Sections 3/4 remain open.
