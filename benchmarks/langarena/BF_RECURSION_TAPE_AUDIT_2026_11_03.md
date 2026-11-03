# Recursive Brainfuck: initial tape mismatch reproduced

Pinned Rust and Goose start the recursive Brainfuck tape with **one zero byte**
and grow on rightward movement. The Kujo port starts that variant with **30,000
zero bytes**, sharing the array variant's initial size. This is a confirmed port
work/layout difference, not a demonstrated defect in the Kujo runtime.

The isolated one-cell variant changes only the recursive wrapper's initial tape
expression from `bytes(vector(30000, 0))` to `bytes([0])`. It is not integrated
into the canonical port: the memory candidate's production sequence currently
depends on frozen port identities. Finish and archive that sequence before
changing those sources, then run the final-port checksum matrix.

## Verified controls

Sixteen programs cover empty/comment-only input, ignored comma, byte overflow and
underflow, left-edge clamping, movement through 33 cells, output, repeated loops,
nested loops and skipped loops. All run in VM, interpreter and JIT. An independent
iterative Ruby machine supplies the oracle for Kujo's recursive implementation.

For every case, the diagnostic baseline and isolated candidate must match the
original `bf_text` checksum. Their complete tapes (losslessly encoded in Base64),
positions and wrapped checksums are checked against the independent model.
All **96 complete tape observations** pass their respective shape checks:

- Baseline: exact visited bytes plus zero padding to 30,000 cells.
- Candidate: exact one-cell/grow-on-demand model, including length and every byte.
- Against the comparator's initial/growth model, baseline has **48 shape
  mismatches**, candidate **zero**. Every program visits fewer than 30,000 cells,
  so the baseline's extra padding cannot accidentally match the intended shape.

This removes 29,999 initial cells in the isolated source variant. No allocator
traffic or elapsed-speed improvement is inferred from that cell count. The runs
overlap production verification and are correctness-only.

## Remaining scope

The array variant is not changed. Executed Rust/Goose state, official program and
warmup coverage, growth beyond 30,000 cells, malformed bracket behavior, operation
counts and production remain open. Rust/Goose recursive accumulation uses i64
before the final u32 conversion; Kujo reduces each output accumulation to u32.
The tested low-32-bit checksums agree, but this audit does not claim identical
intermediate accumulators or full structural equivalence. Retain that distinction
when extending the oracle.

## Evidence

- Upstream source: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Frozen accepted CLI SHA-256:
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
- Reproduce with `probes/audit-bf-recursion-tape.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT`.
- `bf-recursion-tape-audit-2026-11-03.json.gz` retains source identities, pinned
  comparator sources, generated fixture, inputs, independent oracle expectations,
  raw three-runtime results and verification summary; embedded hashes verified.

No runtime fix, canonical port change or competitive speed claim is made here.
