# LZW port equivalence: source review and full-byte controls

Scope: `Compress::LZWEncode` and `Compress::LZWDecode`. This extends the
representative port audit, not the all-50 completion claim. Runtime baseline is
`782c712d`; upstream is `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
The oracle verifies the Rust and Goose files against their pinned git blobs.

## Confirmed correspondence

All three implementations initialize 256 single-symbol dictionary entries,
start new codes at 256, and emit two-byte big-endian codes. Their decode paths
handle the special next-code case by appending the previous phrase's first
symbol. Benchmark inputs repeat `ABRACADABRA`; they are not RNG-dependent.
Encode checksums accumulate encoded lengths. Decode checksums additionally
compare the final decoded bytes with the original input. These checksums do not
prove identical intermediate dictionary work.

The extracted, unchanged Rust encode/decode kernels and the assembled Kujo port
agree on **all encoded bytes and all decoded bytes for eight controls**: empty,
single ASCII, repeated high byte, mixed NUL/high bytes, all 256 byte values,
two repetitions of all byte values, and 50/1,000-byte benchmark-pattern inputs.
All controls pass in baseline VM, interpreter and JIT modes. This establishes
bounded differential correctness; it is not a production-size pass, a native-path
coverage claim, or an executed Goose full-output comparison.

## Work differences to keep separate from runtime speed

| Path | Kujo | Pinned comparators |
| --- | --- | --- |
| Encode keys | `byte_character` constructs UTF-8 strings; phrase concatenation and string-key lookup | Rust extends byte vectors and moves a phrase into the dictionary on a miss; Goose copies byte slices into an arena |
| Initial capacity | Dictionary and result start empty; output bytes first accumulate as an ordinary array, then convert to Bytes | Rust reserves 4,096 dictionary entries and twice input length for encoded output; Goose reserves 4,096 dictionary entries |
| Decode storage | Bytes phrases in an ordinary array, Bytes concatenation and value-state updates | Rust owns byte-vector phrases and extends a byte output vector; Goose keeps phrase slices in its byte arena and appends to byte output |
| Decode preparation | Generates input once and encodes that same data | Rust and Goose generate the same deterministic pattern again through an encoder's prepare/run sequence |

For bytes 128–255, Kujo's key representation uses multiple UTF-8 bytes per symbol.
The mapping is injective and concatenation preserves symbol sequences; the
full-byte controls exercise that mapping. It changes allocation/hash work, not
the dictionary code sequence in these controls. Do not label it corruption or
claim a measured speed penalty from source inspection alone.

Preparation differs outside the measured loop, so process-wall comparisons also
include unequal setup work. No port change is made here. Potential diagnostic
variants include reusing single-symbol keys and reducing intermediate output
conversion. They must preserve complete output and keep setup inside its proper
benchmark phase; report port gains separately from runtime gains. Dictionary
capacity APIs or byte-key representations require their own design review.

## Remaining evidence

- Full-output comparison against executed Goose, broader adversarial inputs and
  dictionary growth boundaries remain open.
- Production sizes are 200,006 bytes/85 iterations for encode and
  1,000,007 bytes/1,600 iterations for decode; these eight controls do not cover
  those runs. Test sizes are 60 and 61 bytes, respectively.
- Attribute string conversion, hashing, growth and copying costs before ranking
  any port or library optimization. No timing was collected during this audit.
- Malformed-input behavior differs (Rust explicitly checks odd encoded length);
  this audit covers valid encoded streams, not a common public decoder contract.

## Reproduction

`probes/audit-lzw-equivalence.rb UPSTREAM_CHECKOUT KUJO_BINARY NEW_OUTPUT_DIRECTORY`
extracts pinned Rust kernels, compiles a standalone correctness oracle, generates
Kujo assertions from its full outputs and executes all three runtimes. The
maintenance script does not change the benchmark implementation or configuration.
The brace extractor targets these inspected functions; it is not a general Rust
parser. `lzw-port-audit-2026-11-02.json.gz` retains the script, extracted oracle,
generated Kujo assertions and result/identity report with verified embedded hashes.

Source: `src/53_lzw.kujo`, `src/50_bwt.kujo`, upstream
`rust/src/benchmarks/compress.rs` (LZW section), `goose/src/lzw.goose` and
`goose/src/compress_data.goose`. Existing roundtrip coverage also remains in
`tests/langarena_port_contracts.rs`.
