# Compound-update type transition contract

This fixture calls the same imported helper with numeric dictionary entries,
replaces one entry with a struct implementing `op_add`, then calls it again after
the callback has returned a number. It checks callback visibility of earlier
writes, callback mutation of another key, and preservation of every saved version.

A compound-update optimization must revalidate dynamic values or retain the normal
operator/publication path; cached function shape alone cannot establish that
indexed operands remain primitive between calls.

Run from the repository root:

```sh
kujo run benchmarks/langarena/probes/compound-type-transition/main.kujo --untrusted
kujo run benchmarks/langarena/probes/compound-type-transition/main.kujo --untrusted --interpreter
kujo run benchmarks/langarena/probes/compound-type-transition/main.kujo --untrusted --jit
```

Expected stdout: `COMPOUND TYPE TRANSITION PASS` and a successful exit.
The contract passed in all three modes on accepted runtime `e445f4a` and the
unaccepted general-array-batch V2 candidate. Raw evidence is retained under
`.audit-evidence/compound-type-transition-2026-10-29/`. This is a regression fixture,
not a LangArena benchmark or evidence of a compound-update performance fix.
