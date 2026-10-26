# Ordinary task captured-write verification

Accepted runtime: `0b87ccb`; baseline: `15a225d`. Ordinary interpreter task
callbacks previously published their entire captured environment on completion.
Two aliases writing different bindings could therefore overwrite each other's
updates. They now merge only written captured bindings. Separate closures and
saved collections retain value semantics; same-binding concurrent read-modify-write
is still non-atomic.

The write journal stores zero or one binding inline and promotes to a hash set
for distinct bindings. While it contains one binding, repeated writes reuse its
recorded name;
merging into an existing binding avoids an unnecessary key copy. Nested calls,
callbacks, helper scopes, exceptions and snapshot controls remain covered.

## Correctness and identity gates

- Baseline: five interpreter failures among fifteen lost-write cases; VM/JIT pass.
- Candidate: all fifteen lost-write cases plus twelve recursion/callback/snapshot
  controls pass across VM, interpreter and JIT-enabled execution.
- 271 integration tests pass, one ignored, including imported module-global guards.
- 1,116 library tests pass, eight ignored, including the new journal scope test.
- All fifty test-size LangArena checksums pass in each runtime: 150 successes.
- Release build, formatting and diff checks pass. Frozen runtime/test hashes,
  supplemental test hash, binary identities and benchmark inputs match.

[Machine-readable evidence](ordinary-task-write-verification-2026-10-26.json)
contains test logs, exact fixtures, source/binary hashes, all checksum reports,
all 232 observations (210 timings and 22 separate RSS measurements), and the
rejected prototype's source patch and timing evidence. Raw local artifacts are
in `.audit-evidence/ordinary-task-writes-v2-2026-10-26/`.

## Local timing results and costs

Each cell is the median of seven alternating paired candidate/baseline elapsed-time
ratios. Lower is faster. Builds and correctness runs finished before timing;
measurement groups ran sequentially. All pairs, including outliers, are retained.

| Control | VM | Interpreter | JIT-enabled |
| --- | ---: | ---: | ---: |
| Ordinary global helper | 1.00852 | 0.99243 | 0.99450 |
| Imported ordinary helper inside a task | 1.00695 | 1.01150 | 1.01093 |
| Repeated mutable captured calls inside one task | 0.99472 | 1.04024 | 0.99546 |
| QuickSort | 1.02365 | — | 1.01821 |
| MergeSort | 0.96805 | — | 0.99155 |
| NeuralNet | 0.99404 | — | 1.01162 |

All fifteen medians meet the predeclared 1.05 slowdown gate. This correctness fix
still costs about 4.0% on the interpreter captured-write control and 2.4% on
QuickSort VM locally. It is not a general speed improvement. NeuralNet medians
remain within the gate, so the conditional eleven-pair recheck was not triggered.
The priority timing controls cover VM/JIT; interpreter coverage for those
benchmarks is checksum coverage, not a priority timing comparison.

The first candidate passed correctness and the ordinary-helper timing gate but
failed the repeated captured-write control: interpreter median 1.09610, with all
seven pairs slower. It was rejected. The accepted revision's interpreter median
is 1.04024 against the same baseline; these are separate local measurement rounds,
not a direct paired V1-versus-V2 speedup claim.

RSS observations remain single observations, not retention trends. NeuralNet VM
is 18,169,856 → 18,247,680 bytes and JIT is 33,136,640 → 34,873,344 bytes.
All other RSS values are retained in the JSON. No broad memory improvement,
native-JIT-entry, production-size, clean-host or Goose/Rust ranking claim follows
from these measurements.

## Remaining work

Fresh controls on `0b87ccb` reproduce stale submission globals for ordinary and
async tasks, and stale ordered reads between a task and its active caller in the
interpreter. VM/JIT pass those controls; all twelve executions exit successfully
without a timeout. Exact outputs are in `remaining_gaps` in the JSON.

Those task ownership defects, unused imported binding materialization, broader
profile-led representation work, production-size acceptance and clean-host
comparisons remain open in the performance checklist. No maintainer contact was
made and no dedicated host is available.
