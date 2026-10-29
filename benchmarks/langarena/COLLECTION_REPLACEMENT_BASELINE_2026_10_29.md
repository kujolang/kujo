# Collection-valued replacement baseline

Accepted runtime `2c91c49` still copies the captured root dictionary when replacing
an entry with an array or dictionary. The [verified observations](collection-replacement-baseline-2026-10-29.json)
cover 64 allocation measurements and 96 semantic checks across VM/interpreter/JIT,
two owner widths, captured/argument counters and initial/fresh snapshots. Original
and final collections are checked in full, including unchanged entries.

VM and JIT agree on added-call requested-byte slopes (16 versus 32 calls), with
an initial snapshot only:

| Replacement | Counter | Width 128 | Width 2,048 |
| --- | --- | ---: | ---: |
| Array | Captured | 44,547 | 632,067 |
| Array | Argument | 44,540 | 632,060 |
| Dictionary | Captured | 44,713 | 632,233 |
| Dictionary | Argument | 44,706 | 632,226 |

Each retains a 587,520-byte width penalty per additional helper call. Removing the
captured counter does not remove this cost. Fresh snapshots remain separate
semantic controls because their observable old values can require copying.

Source review points to scalar-only first-entry publication admission. Supporting
collection values must preserve snapshot reads, aliases, callback publication,
error/discard behavior, value-type transitions and the existing journal layout.
Sparse scalar promotion must not receive an unsupported collection value. Rejected
admission must not allocate a replacement journal before falling back.

`probes/collection-replacement-isolation.rb` is the reusable baseline runner.
Compare future candidates on these exact fixture paths as well as matching source
and executable identities. Path-length differences can affect allocation totals.
These results establish an allocation gap; they do not measure competitive speed
or justify skipping the wider regression and checksum gates.
