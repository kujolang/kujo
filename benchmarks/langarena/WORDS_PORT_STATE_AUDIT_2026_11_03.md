# Words port state audit — partial

The unchanged Words kernel passes **288 complete-state observations** in VM,
interpreter and JIT. Every word frequency, first-seen order, winning word/count
and accumulated checksum is checked. Original and diagnostic input strings and
retained snapshots must remain unchanged. No runtime or port changed.

Sixteen inputs cover unique and tied maxima, reversed tie order, repeated words,
1,024-character words, Unicode/emoji, and generated sequences of 1/8/64/257 words.
Each input runs two checksum seeds and three iterations in all three runtimes.
All inputs have nonempty words separated by single ASCII spaces. The collector
uses a diagnostic clone exposing state after the unchanged checksum calculation;
it agrees with the original kernel. An independent verifier counts each distinct
word separately and calculates the checksum using a modular polynomial rather
than the collector's iterative multiply/add recurrence.

## Comparator policy remains unresolved

Kujo selects the first-seen word among equal maximum counts. The pinned Rust
source calls `max_by_key` over a `HashMap` iterator, so its source does not specify
the same first-seen policy. Goose selects the first maximum visited by its
dictionary iterator; this audit does not establish that iterator's ordering.
The `a b` and `b a` controls verify Kujo's differing winners. They do not prove
which winner an executed comparator would choose or that official generated
inputs encounter a relevant tie. No comparator execution or mismatch is claimed.

Tokenization also has a boundary: Kujo explicitly splits on ASCII space, Rust
uses `split_whitespace`, and Goose calls `each_word`. The current controls stay
within their single-space input domain. Empty text, repeated spaces, tabs and
newlines need separate policy controls. Do not silently change the port to guess
at a universal tie rule. Execute pinned comparators and inspect official generated
inputs first; any unresolved upstream tie/whitespace contract belongs in the
maintainer-question list, without contacting Kostya under current authorization.

Random preparation, work/allocation counts, executed Rust/Goose full-state checks
and production coverage remain open. The structural ledger row remains partial.
These observations establish neither native participation nor elapsed speed.

## Evidence

- Comparator source revision: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Accepted loop-fix CLI SHA-256:
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
  The separate memory candidate is not used. Runs overlap its no-JIT build.
- `probes/audit-words-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT` reproduces the
  collector. The archive includes the independent verifier.
- `words-state-audit-2026-11-03.json.gz` retains 19 hash-verified artifacts,
  including complete outputs, fixtures, source identities and comparator sources.
