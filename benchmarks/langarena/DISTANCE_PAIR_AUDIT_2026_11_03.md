# Distance kernels: complete pair-score comparison

On frozen accepted Kujo CLI `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`,
the audit checks both Jaro and NGram for all 400 ordered pairs drawn from twenty
strings, twice in VM, interpreter and JIT modes: **4,800 score observations**.
The exact CLI hash is authoritative in the accompanying manifest; this is not a
new runtime build or a timing result.

The executed Rust reference includes the unchanged pinned upstream
`rust/src/benchmarks/distance.rs` inside a standalone module. The wrapper calls
the actual private kernels; dummy configuration/RNG functions panic if called,
so this explicitly excludes preparation rather than pretending to test it.
Upstream commit is `3c9f84ddaeec834c0631747cb789e8cfd602f993`. The corresponding
Goose source is pinned and inspected, but Goose is not executed in this audit.

Each unmodified Kujo kernel's floating result matches Rust within absolute
`1e-12`; its integer checksum contribution matches exactly. Controls include
empty/short strings, unequal lengths, permutations, repeated grams, embedded NUL,
accented UTF-8, combining characters and an emoji. Input construction uses
`utf8_decode(bytes(...))`, preserving exact bytes without assuming JSON escapes
are valid Kujo source escapes.

An independent byte-window multiset oracle also checks all Rust NGram results.
The independent verifier checks every row/order, both repetitions, runtime exits,
raw output hashes and frozen identities. Three negative controls alter a float
score by 0.0001 while retaining its integer contribution; score verification
rejects each, demonstrating coverage beyond matching aggregate checksums.

## Behavior and work differences retained

- The benchmark's NGram denominator counts distinct gram keys, while its
  numerator counts the multiset intersection. `aaaaaa` against itself therefore
  scores 1.5. All compared kernels agree; do not replace this with a conventional
  normalized similarity formula.
- Jaro uses bytes, including for non-ASCII input, and integer-halves its
  transposition count. This audit preserves those comparator semantics.
- Kujo calls `utf8_encode` on both strings inside each kernel invocation. Its
  native implementation allocates byte storage. Rust uses borrowed `as_bytes()`;
  Goose prepares byte arrays once and passes slices. This is a source-confirmed
  representation/work difference, not measured allocation attribution or speed.
- Rust/Goose NGram return immediately when either string is shorter than four
  bytes. Kujo may build the other side's gram dictionary before returning the
  same zero score. Official generation uses strings of at least four bytes,
  so the short-string extra work is an adversarial-input observation, not an
  explanation of official benchmark timings.

## Remaining gates

Both ledger rows remain partial. Verify complete generated pair lists and RNG
state, wrapper iteration/checksum accumulation, traversal/conversion/allocation
counts, executed Goose controls and production iterations. Measure any proposed
byte-preparation change separately as a port improvement; do not present it as a
runtime optimization. JIT mode passes do not prove native participation here.
Malformed byte streams are outside these string-input kernels' UTF-8 contract.

The first diagnostic fixture used JSON's `\u0000` source escape, rejected by
Kujo's lexer. That fixture failure and original collector are retained; no
language behavior was changed. The corrected fixture constructs the same NUL
input with the documented byte-decoding builtin.

Reproduction:

```text
ruby benchmarks/langarena/probes/audit-distance-pairs.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT
ruby benchmarks/langarena/probes/verify-distance-pairs.rb NEW_OUTPUT
```

`distance-pairs-2026-11-03.json.gz` retains source/input identities, comparator
sources, generated Rust/Kujo fixtures, compiler diagnostics, all raw scores,
collectors, verifiers and the initial fixture failure. Binary identities are
recorded by hash; executables are not embedded. No canonical port/runtime changes,
production pass, Goose execution, elapsed-speed or final equivalence claim.
