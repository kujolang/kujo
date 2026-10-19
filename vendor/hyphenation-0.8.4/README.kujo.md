# Kujo patch notes

Source: crates.io `hyphenation` 0.8.4, checksum
`bcf4dd4c44ae85155502a52c48739c8a48185d1449fff1963cffee63c28a50f0`.
Upstream: https://github.com/tapeinosyne/hyphenation (MIT OR Apache-2.0).
All original source, licenses, patterns and dictionary bytes are retained.

Kujo changes only the normal/build codec dependency and its calls:

- Replace retired `bincode` 1.3.3 with `bincode-next` 2.1.0, using only `std`
  and `serde`. Pin the Rust-1.85-compatible release; 3.1.1 requires Rust 1.90,
  beyond Kujo's Rust 1.89 minimum. No custom codec implementation or renamed
  copy of the retired library is substituted.
- Explicit `config::legacy()` retains little-endian, fixed-width integer
  dictionary encoding. Continue allowing trailing bytes and consuming exactly
  one dictionary from readers. Retain serde rather than changing data derives.
- Internal load/build error payload types now use the fork's DecodeError and
  EncodeError. Kujo does not expose these transitive types or diagnostics;
  standalone consumers of this patched hyphenation crate must account for that
  difference. No compatibility claim is made for directly replacing a public
  hyphenation dependency outside this repository.

`tests/fixtures/hyphenation_compat` verifies every shipped dictionary against
complete decoded-content hashes captured using the original codec. It also
exercises embedded resources, writing, truncation, language mismatch and reader
boundaries. The optional dictionary-building path is tested separately.
Original upstream formatting is preserved to make the patch easy to review.
