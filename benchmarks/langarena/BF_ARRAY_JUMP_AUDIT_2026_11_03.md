# Array Brainfuck: extra bracket dispatch reproduced

The canonical Kujo array interpreter sets `pc` to the matching bracket and
immediately continues its dispatch loop. Pinned Rust and Goose set that target
and then increment `pc`, continuing **after** the matching bracket. Kujo therefore
dispatches one extra matching bracket for every taken loop jump. That bracket
does not change state: a skipped loop arrives at `]` with a zero cell, and a
backward jump arrives at `[` with a nonzero cell.

An isolated variant changes the two jump expressions to
`pc = jumps[pc] + 1 continue`. The canonical port is unchanged by this audit.
The memory-candidate acceptance sequence retains its corrected recursive-tape
port identity; integrate further port changes only after that checkpoint review.

## Verification

Sixteen balanced, nonempty-command programs run in VM, interpreter and JIT.
Controls cover nested/repeated/skipped loops, output, movement, left-edge clamping,
byte overflow/underflow and straight-line cases. Both observed variants must
match the original function's checksum. An independent iterative Ruby oracle
checks every final tape byte, pointer, wrapped checksum and command-dispatch count.

All **96 complete state/count observations** pass. Seven programs take loop jumps;
nine are unchanged controls. Per pass through the sixteen-program fixture:

| Variant | Dispatched Brainfuck commands |
| --- | ---: |
| Canonical port | 711 |
| Isolated jump correction | 666 |
| Independent target model | 666 |

The baseline count equals the target count plus exactly one dispatch per taken
jump in every case and runtime. All final states and checksums agree. These are
Brainfuck command counts, **not Kujo VM instructions, CPU instructions, CPU-time
fractions or elapsed-speed measurements**. The fixture mix is not an official
benchmark workload. Actual Rust/Goose execution remains unverified here.

## Remaining scope

The canonical fix, official input/warmup checks, all-150 checksum refresh and
production remain pending. Invalid bracket/input handling, comma behavior,
empty-program preparation, growth beyond the initial tape and full comparator
work/layout equality remain separate audit boundaries. Do not generalize these
valid-program state checks into a complete Brainfuck-equivalence claim.

## Evidence

- Upstream revision: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Frozen accepted CLI SHA-256:
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
- Reproduce with `probes/audit-bf-array-jumps.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT`.
- `bf-array-jump-audit-2026-11-03.json.gz` preserves source identities, comparator
  sources, generated fixture, oracle expectations, complete raw outputs and
  verification summary. All embedded artifact hashes are verified.
