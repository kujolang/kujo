# Imported module collection ownership isolation

Runtime: `2c342ef1817bc6931feb84cbfc039f3dc1f26f1f` (the accepted independent-helper optimization). This diagnostic identifies remaining work; it does not claim a runtime fix or a speedup.

[Machine-readable evidence](module-owner-isolation-2026-10-26.json) records the binary/probe hashes, all source fixtures, deduplicated module sources, allocation counts, semantic outcomes and computed deltas. The probe source is [allocation_probe.rs](probes/captured-append-scaling/allocation_probe.rs).

## Controls and results

The matrix varies array, dictionary, struct-with-array and dictionary-with-array state; 128/2,048 elements; one/eight element updates per invocation; 32/64 invocations; and either one initial snapshot or a fresh snapshot immediately before every call. Dictionary keys have fixed widths to avoid string-length confounding. Every case checks the saved snapshot, untouched last element and final current state.

All **128 VM/JIT allocation controls** and **192 VM/interpreter/JIT semantic controls** pass. These are diagnostic cases, not additional LangArena benchmarks.

Per-added-call allocation traffic is calculated as `(64-call total - 32-call total) / 32`. Increasing collection width by 1,920 elements adds:

| State shape | Additional requested bytes per invocation | Additional allocation requests |
| --- | ---: | ---: |
| Array | 261,120 | 0 |
| Struct containing array | 261,120 | 0 |
| Dictionary containing array | 261,120 | 0 |
| Direct dictionary | 587,520 | 0 |

These width slopes match in VM and JIT, with both one and eight updates, and with both snapshot policies. This isolates collection copying at the invocation boundary: repeated writes within an invocation already reuse the detached collection. It is not evidence of copying on every element write.

A snapshot freshly retained before each call legitimately requires separation on each mutation. A single original snapshot does not by itself require detaching the updated current collection on every later invocation. However, eliminating that repeated copying must also preserve publicly observable module-owner state.

## Publication boundary that fixes must preserve

[The public-owner regression tests](../../tests/imported_global_owner_contracts.rs), committed as `394f675`, pass on this baseline in VM, interpreter and JIT. A host thread reads the exported function's public owner map while the helper waits inside a channel callback. The map remains accessible, contains a real array, and retains its pre-return value; the updated value appears when the helper returns.

A fix cannot simply expose private markers in that map, publish mutations early, or hold the owner lock across arbitrary user code. The tests use ordered channel synchronization rather than speed assumptions.

## Remaining work and measurement limits

Reduce captured-frame map/name setup allocations separately from the collection-width cost. Any frame reuse must release retained values and identities, bound idle memory, and consult actual public owner keys and mutability metadata on each invocation. Then address collection copying while retaining snapshot, callback, reentry, task and public-owner contracts.

The laptop is variably loaded. This report uses requested allocation traffic and semantic assertions, not elapsed-time rankings, peak resident memory, or a claimed Goose/Rust speed comparison. Full production-size and clean-server comparisons remain separate gates.
