# Complete sort output oracle

The unchanged pinned Rust and Kujo QuickSort/MergeSort kernels produce identical
complete sorted arrays for 49 nonempty diagnostic inputs. Each engine produced
98 arrays: Rust, Kujo VM, interpreter and JIT mode all matched an independent
integer-sort reference. Kujo also preserved its original input and both rows of
the initial MergeSort state after each call.

Inputs cover lengths 1, 2, 3, 31, 64 and 257; ascending, descending, equal,
alternating, absolute-value-ordered and three seeded duplicate-heavy patterns;
plus a case containing both i32 extrema. This validates integer output and
snapshots, not stable ordering of distinguishable equal-key objects.

The oracle embeds entire unchanged Rust sort modules and the sort base module
from LangArena `3c9f84ddaeec834c0631747cb789e8cfd602f993`, adding sibling diagnostic
entrypoints. It includes the unchanged RNG prefix for compilation, but supplies
identical explicit input arrays to both languages. Kujo's common/sort sources
are unchanged. No official benchmark inputs or implementations were edited.

The tested Kujo binary is the **subsequently rejected guarded-call candidate**, SHA-256
`2ebf39e3cd41f78fe7bf5fc4a7d751b77e57760cebd626d94fdb4131fd3f8c04`.
Its runtime base and patch are archived. This is additional candidate correctness
evidence, not acceptance of its performance or proof of native kernel coverage.

JIT mode prints its normal whole-script `MakeClosure(0)` admission warning; the
exact warning is retained. The first oracle attempt rejected that warning after
the complete output comparisons passed. The corrected oracle permits only that
specific diagnostic or empty stderr; all original evidence remains archived.

Empty input is excluded because the pinned Rust MergeSort entrypoint subtracts
one from the length. Official sort sizes are nonempty. Operation, partition,
call and copying counts remain open, as do production-size and Goose comparisons.
Matching complete outputs does not establish equal work between ports.

Run `ruby benchmarks/langarena/probes/audit-sort-state.rb UPSTREAM FROZEN_KUJO
NEW_OUTPUT`. The [archive](sort-state-audit-2026-11-02.json.gz) contains 34
SHA-256-verified artifacts: generated sources, command receipts, complete raw
outputs, source identities, the warning-policy attempt and candidate provenance.
