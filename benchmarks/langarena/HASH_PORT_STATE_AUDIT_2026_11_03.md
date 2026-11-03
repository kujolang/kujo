# Hash port state audit — partial

CRC32 and the benchmark's toy SHA256 pass **2,052 complete hash/checksum
observations** across VM, interpreter and JIT. Every toy-hash digest byte is
checked, rather than only the four bytes used by its checksum. No runtime or
port changed. This toy function is not cryptographic SHA-256 and must not be
replaced with a standard cryptographic implementation for benchmark comparisons.

The 57 byte inputs cover lengths 0, 1, 2, 7, 8, 9, 15, 16, 31, 32, 33, 255,
256 and 257 with zero, all-255, ascending and deterministic mixed bytes, plus
ASCII `123456789`. Every input runs both hash paths, two initial checksum seeds
(0 and 4294967294), three consecutive iterations and three runtimes. The 256-byte
cases cover the entire byte domain. Empty buffers exercise initial digest lanes.

The unchanged kernel executes beside a diagnostic clone exposing the calculated
hash and complete digest. Their accumulated results must agree. Original inputs,
diagnostic inputs and retained snapshots must remain unchanged after every call.
The collector uses zlib CRC32 and a toy-hash reference. An independent verifier
uses bitwise CRC recurrence (including the standard 0xcbf43926 reference vector),
separately processes each toy-hash lane and extracts bytes arithmetically. Both
verify exact case coverage, raw output and source identities.

## Remaining scope

Pinned Rust and Goose sources implement the same CRC polynomial and toy-hash
recurrence, with big-endian digest lanes and little-endian extraction of the
first four bytes for accumulation. They are inspected, not executed here.
Random input preparation, operation/conversion/copy counts, production sizes,
executed Rust/Goose state and malformed non-byte inputs remain open. Both
structural ledger rows stay partial. No timing or native-participation claim.

## Evidence

- Comparator revision: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Accepted loop-fix CLI SHA-256:
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
  The separate memory candidate is not used; runs overlap its no-JIT build.
- `probes/audit-hash-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT` reproduces the
  collector. The archive includes the independent verifier.
- `hash-state-audit-2026-11-03.json.gz` retains 20 verified artifacts,
  including full output, fixtures, source identities and comparator sources.
