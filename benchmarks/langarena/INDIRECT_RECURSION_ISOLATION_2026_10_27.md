# Indirect recursion isolation — 2026-10-27

The next confirmed allocation target is **foreign-module callback reentry**.
Ordinary same-module helper bridges already have flat allocation traffic across
0/128/2,048 unrelated bindings. No runtime change was made in this experiment.

The matching release binary and allocation probe are the frozen artifacts from
runtime commit `2b83f1e`. The [machine evidence](indirect-recursion-isolation-2026-10-27.json)
includes their hashes, generated source, raw measurements and per-call deltas.
All 216 allocation controls (VM/JIT) and 324 semantic controls
(VM/interpreter/JIT) passed. Passing means the fixtures return correct totals and
module state; it does **not** mean the callback allocation cost is fixed.

## Isolation

Three routes use the same read/write recurrence, depths 0/1/4 and 16/32 outer
calls. Subtracting the smaller run isolates incremental traffic per outer call.

- `direct`: recursive exported helper calls itself.
- `shared`: a non-exported same-module function bridges each recursive call.
- `callback`: another module exports `invoke(f, n)`, which calls the supplied
  recursive function. This introduces a foreign capture between reentries.

VM depth-four traffic per added outer call:

| Route / operation | Extra bindings | Requests | Requested bytes |
| --- | ---: | ---: | ---: |
| Direct read | 0 or 2,048 | 148 | 12,783 |
| Same-module bridge read | 0 or 2,048 | 217 | 16,872 |
| Foreign callback read | 0 | 452 | 42,182 |
| Foreign callback read | 128 | 836 | 153,500 |
| Foreign callback read | 2,048 | 6,596 | 2,063,204 |
| Foreign callback write | 0 | 529 | 42,898 |
| Foreign callback write | 128 | 913 | 154,216 |
| Foreign callback write | 2,048 | 6,673 | 2,063,920 |

JIT allocation results match these rows. All routes remain flat at depth one;
the foreign route becomes binding-dependent at depth four. This makes shallow
recursion insufficient as the sole acceptance gate for the next fix.

The routes have different source/module layouts, so cross-route constants are
descriptive, not attribution of a runtime patch. A future before/after test must
use identical fixture paths and matching build profiles. These are allocation
requests/bytes, not peak memory or speed estimates on the loaded laptop.

## Reproduce and next acceptance gates

Run `probes/recursive-helper-allocations.rb PROBE BINARY NEW_OUTPUT_DIR` with
`KUJO_RECURSIVE_CAPTURE_ROUTE=direct`, `shared`, or `callback`. The default remains
direct and continues to enforce its existing depth-one/depth-four flatness gates.
The other routes report flatness without rejecting the known baseline cost.
An optional fourth baseline probe retains paired non-increasing allocation checks.

Before accepting a callback optimization, reproduce this growth, enforce a
candidate scaling gate, and verify live suspended globals, callback publication,
escaped snapshots, completed writes before errors, and parameter/local ownership
in all three runtimes. Preserve the direct and ordinary-helper controls. General
captured locals, publication traversal, collection-width copying and the remaining
performance plan are still open. Clean-server speed comparisons remain pending.
