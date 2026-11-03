# Executed Goose distance-kernel comparison

The existing local Goose compiler built the unchanged pinned distance kernels
for the same 400 ordered string pairs used in the Rust/Kujo audit. All **800
Goose scores** match executed Rust within absolute `1e-12`, with exact integer
checksum contributions. Together with the previous 4,800 Kujo observations,
this establishes the recorded pair-score agreement across the three languages,
not full benchmark equivalence or a speed result.

The collector copies `distance.goose`, `helper.goose` and `bench.goose` verbatim
from upstream commit `3c9f84ddaeec834c0631747cb789e8cfd602f993`, verified against
Git before compilation. A separate main constructs exact byte inputs and invokes
the original `distance::jaro` and `distance::ngram`. The compiler emits standalone
C with `-O2`; Apple clang builds it with `-O2 -w -lm -pthread`. Compiler and
executable identities, generated C, toolchain versions and commands are retained.

Goose floats are rendered with the unchanged upstream fixed-format helper at
15 decimal places. Comparisons retain the existing `1e-12` tolerance; integer
contributions are calculated before formatting and compared exactly. All pairs,
including NUL and UTF-8 byte sequences, have the same bytes as the Rust/Kujo
fixtures. Repeated grams preserve the benchmark's intentional score above one.

The independent verifier checks build/run exits, raw hashes, frozen identities,
row order, all scores and integer contributions. A deliberately altered score
with unchanged checksum contribution is rejected. The previous Rust/Kujo pair
evidence was reverified before accepting this comparison.

The first diagnostic main attempted random access into an array of variable-size
Pair elements, rejected by Goose's type checker. The corrected main iterates
those elements directly. Both fixtures/diagnostics are preserved; benchmark
kernels and language semantics were not changed.

## Remaining scope

Executed Goose pair scores are now covered. Goose input generation, complete
prepare/warmup/reset/run state, production iterations, operation/copy/conversion
counts and wider boundaries remain open. The earlier generated-state comparison
against Rust remains separate. No new Kujo runtime or canonical port changes,
native-participation claim, timings or competitive score are introduced.

Reproduce with `probes/audit-distance-goose.rb UPSTREAM GOOSE_COMPILER VERIFIED_PAIR_EVIDENCE NEW_OUTPUT`
then `probes/verify-distance-goose.rb NEW_OUTPUT VERIFIED_PAIR_EVIDENCE`.
`distance-goose-2026-11-03.json.gz` retains original/generated source, raw
outputs, compiler diagnostics, identity receipts, verifiers and the initial
fixture failure. Executables are identified by hashes, not embedded.
