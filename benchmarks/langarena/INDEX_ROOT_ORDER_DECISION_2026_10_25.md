# Indexed assignment root order — approved decision, implementation pending

The user explicitly selected **“Update the replacement array (recommended)”**
when asked which root to use if an index expression replaces the target array.
This resolves the semantic decision previously left open in sections 3–4.

The intended rule is consistent with existing interpreter behavior and the
language specification's nested-assignment rule: evaluate the right-hand side,
then each target index once, then read and update the resulting root. Preserve
independent snapshots. A failed write leaves expression side effects intact and
does not publish a partially updated root. Compound assignment's separate read
and write evaluations are not implicitly redesigned by this decision.

## Reproduced baseline

Runtime `0d0f365`, frozen executable hash in
`index-root-order-decision-2026-10-25.json`. Five cases test replacement, a larger
replacement accepting a previously invalid index, a smaller replacement rejecting
a previously valid index, RHS/index ordering with a negative index, and dictionary
replacement as a consistency control.

All five pass in the interpreter. All five fail the approved-contract assertions
or bounds behavior in VM and JIT-enabled mode with runtime exit code 4. These are
intentional red reproductions of the newly approved behavior, not regressions in
the separately running native-default candidate. The simple JIT-enabled cases
report VM fallback; they do not independently establish native execution.

A separate 256-call hot global-write control compiles two functions and records
**314 native function entries**, returning the original-root result `[42, 2, 3]`
instead of the approved `[42, 9, 9]`. Raw counters, source and output are retained.
Native entries are execution evidence, not native CPU-time attribution.

## Required implementation and gates

- Lower direct indexed assignment so the root is loaded after index evaluation,
  including scoped globals/captures, while retaining efficient unique-owner writes.
- Align the existing reentrant-root regression with the approved result across
  VM/interpreter/JIT; add growth/shrink bounds, negative index, single evaluation,
  errors, immutable bindings and alias checks.
- Exercise actual native entries after warmup, not only JIT-enabled fallback.
- Check array/byte/dictionary representations, nested-path consistency and existing
  assignment optimizations; do not silently redesign compound assignment.
- Run all-runtime regressions, all 150 test-size checksums and paired allocation/
  workload comparisons. Investigate repeatable slowdowns above 5%.
- Update the language/parity documentation when implementation is verified.

The compiler/runtime fix has **not** landed. The native-default candidate remains
frozen for its own tests and measurements; this decision/evidence commit does not
change runtime behavior or claim completion of the remaining checklist.
