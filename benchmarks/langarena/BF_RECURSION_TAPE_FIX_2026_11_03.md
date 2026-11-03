# Recursive Brainfuck: canonical one-cell initialization

The recursive port now starts with `bytes([0])`, matching the pinned Rust and
Goose implementations. Previously it constructed a 30,000-element vector and
converted it to bytes on every recursive-program invocation. The array variant
still starts with 30,000 cells. Growth, cell arithmetic, parsing, execution,
warmup, iteration counts and checksums are unchanged.

This is a port preparation correction, not a Kujo runtime optimization. It
eliminates 29,999 initial cells and the intermediate vector construction in this
wrapper; no measured elapsed-speed or allocation-byte improvement is claimed.

## Verification

- The old port rejects the new `--require-one-cell` regression gate; its failure
  is preserved. The earlier isolated audit independently reproduces its extra
  tape shape across all three runtimes.
- The corrected canonical port passes **96 complete tape/position/checksum
  observations** across VM, interpreter and JIT, including nested/skipped loops,
  wrapping bytes, left-edge clamping and grow-on-demand behavior through 33 cells.
- All **150 unchanged test-size checksum combinations** pass. A separate verifier
  checks exact case coverage/order, original configurations, exit status, full
  success summaries, every expected warmup/run phase and monotonic timestamps.
- Generated `port.kujo` exactly equals the ordered source fragments from
  `build.kujo`. The frozen accepted CLI and driver hashes are recorded.
- Ruby syntax, formatting and whitespace checks pass. No Rust runtime source
  changes were made, so a new runtime build is not substituted for the frozen CLI.

CLI SHA-256:
`b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.

The memory candidate's eleven-production-control sequence was fully verified and
archived **before** modifying the canonical port. Its old source receipts remain
historical, not rewritten. A separate all-150 refresh on that unaccepted memory
candidate has been started against this new port; its result is not assumed here.

## Remaining boundaries

Executed comparator state, official production sizes, growth beyond 30,000 cells,
malformed-input behavior, intermediate i64/u32 accumulator differences and complete
operation/allocation comparisons remain open. This correction does not complete
the Brainfuck structural audit or establish a competitive speed score.

`bf-recursion-tape-fix-2026-11-03.json.gz` preserves source/binary identities,
regression fixtures, baseline rejection, full-state outputs, checksum results,
runner/verifier and exact generated port. Every embedded artifact hash is checked.
