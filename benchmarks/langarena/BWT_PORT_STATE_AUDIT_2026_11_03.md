# BWT: complete byte and wrapper-state audit

The accepted frozen CLI
`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`
passes 432 complete BWT wrapper-state observations: 24 inputs, encode/decode,
three successive iterations and VM/interpreter/JIT. This is correctness evidence,
not an elapsed speed measurement or proof that all JIT regions executed natively.

The independent oracle sorts complete cyclic rotations with original-offset tie
breaking. The canonical port remains its rank-doubling implementation; the
oracle does not replace benchmark work. Each observation checks every input and
transformed byte, the original-row index, decoded bytes where applicable, and
wrapping result/checksum. Input snapshots must survive each run unchanged.

Fourteen prepared ABRACADABRA sizes cover 0, 1, 2, 3, 7, 8, 9, 10, 11, 16, 31,
32, 33 and 64. Ten supplied byte inputs cover empty, singleton zero/255, periodic
and repeated data, ascending and descending complete byte domains, and a
deterministic mixed 65-byte sequence. Accumulators start near u32 overflow.
Six negative controls change transformed bytes or original-row indices while
preserving old checksums and are rejected by full-state equality.

Rust `compress.rs`, Goose `bwt.goose` and `compress_data.goose` were pinned and
byte-checked at `3c9f84ddaeec834c0631747cb789e8cfd602f993`, not executed.
Source review confirms stable rank-pair sorting and the same generated pattern.
Representation/work differences remain: Kujo uses nested dynamic pair arrays
and a captured comparator where Goose uses typed RankPair structs; Kujo's empty
inverse still allocates/counts its fixed 256-entry tables while Goose returns
immediately. These are not quantified runtime costs or permission to alter
benchmark phase boundaries.

`bwt-state-2026-11-03.json.gz` preserves eighteen files including raw output,
oracle/fixture/verifier, pinned comparator sources and manifests. All content
hashes were rechecked. Initial collection stopped before execution with ENOSPC;
the successful `bwt-state-v2` run followed a fresh space check. Only redundant
session-created download ZIPs were removed; frozen executables and extracted
evidence were retained. The verifier was adjusted for the laptop's Ruby 2.6.

Executed Rust/Goose controls, malformed encoded-index policy, intermediate
rank/transition work counts, allocation attribution and production inputs remain
open. Both all-50 ledger entries remain partial; no canonical port or runtime
source changed.
