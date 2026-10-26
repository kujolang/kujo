# Remaining imported-helper costs after b34a0b8

This refresh measures the newly verified runtime without changing it. The previous
component counts predated native-default sharing and must not be treated as the
current baseline. All fixtures retain their snapshot and mutable-state assertions.

## Captured environment components

The existing `probes/import_environment_allocations.rs` loads the same four
historical module fixtures and measures 64 clones after setup and warming.

| Extra bindings / array width | Clone requests | Requested bytes |
| --- | ---: | ---: |
| 0 / 128 | 1,856 | 183,616 |
| 0 / 2,048 | 1,856 | 183,616 |
| 128 / 128 | 10,048 | 2,723,264 |
| 2,048 / 128 | 132,928 | 43,463,616 |

The base capture now contains 14 explicit bindings, including seven native values,
versus 472/465 in the historical measurement. Unchanged native defaults are shared;
the remaining explicit bindings cannot simply be discarded. Array width does not
affect this shallow clone control. User-binding count still affects map storage
and cloned binding names substantially.

## Actual calls: separate reads from writes

New controls call the same imported `first(items)` or `edit()` helper 64 and 128
times. `first` reads its parameter; `edit` increments the module's array element.
Each control retains an independent initial array snapshot and checks the final
result, module state and unchanged snapshot. The process-wide allocator measures
VM execution including imports, while excluding parse/compile. Differences between
the two call counts isolate the added-call slope from fixed setup in these fixtures.

The table shows VM differences per added call.

| Extra bindings / array width | Read requests/call | Read bytes/call | Write requests/call | Write bytes/call |
| --- | ---: | ---: | ---: | ---: |
| 0 / 128 | 57 | 4,538 | 47 | 21,728 |
| 0 / 2,048 | 57 | 4,538 | 47 | 282,848 |
| 128 / 128 | 185 | 44,220 | 175 | 61,410 |
| 2,048 / 128 | 2,105 | 680,788 | 2,095 | 697,978 |

JIT shows the same binding-count and array-width dependence, but its 64-to-128
call difference includes one additional request and 52 additional bytes beyond
VM in every control. These are measured differences, not a pure steady-state
per-call cost or proof of native-only JIT execution. All 32 allocation controls passed. The 16 new call
fixtures also passed in all three runtimes (48 executions), and the four original
snapshot/state fixtures passed in all three runtimes (12 executions).

Two distinct costs remain:

1. Even a helper that only reads its parameter pays for unrelated module bindings.
   The added-call allocation count rises by exactly the extra binding count.
2. Mutating the module array adds 136 requested bytes per extra array element per
   call in these controls. Captured global scope zero is still cloned when entering
   the call; the owner retains the previous array while the activation writes it.
   This differs from the newly fixed captured-local storage path. The source and
   allocation slope identify a repeated copy-on-write cost, not a timing guess.

## Next implementation gates

- Remove unnecessary environment materialization only where lexical behavior is
  proven unchanged. Parameter-only helpers are one candidate; do not prune globals
  merely because a small fixture does not access them.
- Preserve module state, native overrides, unknown-name errors, recursive and
  callback reentry, public `Environment.scopes` access and native map-pointer lifetime.
- Preserve independent snapshots while removing redundant references to current
  mutable module collections. Task globals must remain submission snapshots;
  extending captured-local sharing to scope zero indiscriminately is incorrect.
- Add positive and negative eligibility/ownership regressions in VM, interpreter
  and JIT, then repeat these exact component and call controls against the fix.
- Keep the remaining optimization checkbox open until the relevant cost is reduced
  and broad regression/checksum gates pass. Clean-host speed acceptance is separate.

[Structured evidence](import-cost-refresh-2026-10-26.json) records the frozen
runtime/library/probe identities, actual linker arguments, historical fixture
identity, new source text, all allocation observations, per-call deltas and all
60 semantic outcomes. Raw local artifacts are under
`.audit-evidence/import-cost-refresh-2026-10-26/`.

These measurements report requested allocation traffic, not retained memory or
elapsed-speed improvement. The earlier 150 test-size checksum pass remains the
runtime's correctness checkpoint; this diagnostic makes no new production claim.
