# Remaining callback allocations — 2026-10-24

Runtime `93e7caa`, default-feature debug library, macOS x86_64. This follows
the dead-local-map fix. No new runtime change or speedup is claimed here.

The same QuickSort probe still passes sorted-output, snapshot, backing-storage
and allocation-budget assertions. At 8,192 unique elements, JIT records 58,534
allocation requests and 9,278,520 cumulative requested bytes. A temporary
diagnostic captured the first allocation of each selected size, disabling
accounting during stack capture. Counts remain identical to the untraced run.
The source, identities and complete traces are retained in
`remaining-call-allocations-2026-10-24.json`.

| Size | Requests in unique JIT trial | First sampled allocation site |
| ---: | ---: | --- |
| 6 | 14,632 | Builtin name clone in `jit_load_operand`, `src/jit.rs:1017` |
| 16 | 14,632 | `same_storage` traversal vector, `src/interpreter/assignment.rs:971` |
| 136 | 7,316 | `Vec::split_off` in `jit_call_function`, `src/jit.rs:4430` |
| 272 | 7,316 | Same argument-vector splitting site |
| 408 | 14,632 | Same argument-vector splitting site |

The 136/272/408-byte buckets total 8,954,784 bytes, approximately 96.5% of all
requested bytes. These are one-, two- and three-element `Value` vector sizes
on this build. First-allocation samples are not exhaustive attribution of every
allocation in those buckets. The traces do not support attributing the 16-byte
bucket to environment-variable lookup: its sampled source is storage identity.

## Implementation candidates and required gates

1. **Storage identity:** array, byte and dictionary root comparisons can use
   pointer identity before allocating traversal storage. Keep the existing
   scalar rejection, type mismatch behavior and nested-struct traversal rules.
   Verify identical/different storage, nested structs, snapshots and full outputs
   in all runtimes, plus a structural allocation comparison.
2. **Argument buffer reuse:** retain only empty, bounded buffers. Each active
   call must own its buffer; recursive calls cannot borrow an active parent's
   storage. Preserve argument order, wrong-arity and malformed-stack behavior,
   return ownership, exceptions and the non-JIT bridge. Immediate calls can
   recycle a buffer only after their values have been copied into the call
   context. Positional calls must finish consuming or dropping arguments before
   recycling. The current `IntoIter<Value>` drops its vector allocation after
   the final argument; retaining that capacity requires explicit lifecycle work.
3. **Builtin names:** the sampled clone is real, but changing callable name
   representation has wider API implications. Avoid caching away name lookup or
   shadowing behavior merely to remove the string allocation. Profile again
   after the larger argument costs are addressed.

A VM argument pool accessed from a native helper must borrow only its own field,
not create an overlapping mutable borrow of the whole VM while a stack reference
is live. Bound retained capacity and pool length; test unfinished initialization,
errors, recursive calls and teardown for retained values. These are candidates,
not implemented mechanisms. The ongoing `93e7caa` frozen-release job must finish
its source checks before further runtime edits.
