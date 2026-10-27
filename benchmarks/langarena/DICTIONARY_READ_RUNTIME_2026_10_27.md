# Dictionary read allocations — 2026-10-27

Runtime `bb0bd0e` passes the allocation, correctness and test-size checksum gates
below. This is a verified source change, not a new packaged release.

Successful interpreter dictionary reads previously copied the key and eagerly
formatted a diagnostic that was used only if the key was missing. This also
affected captured/imported helper execution reached through VM and JIT.

The candidate borrows string keys and builds missing-key diagnostics only on a
miss. Integer keys use the existing fixed stack-buffer decimal formatter. All
dictionary representations continue through `dictionary_get`, preserving present
null slots, canonical numeric strings, numeric limits and diagnostic spelling.

## Measured result

The comparator is the frozen `bdb232d` release. Matching probes execute identical
paths. A Kujo imported helper repeatedly reads a dictionary-contained array while
key length and loop iterations vary; snapshots and final values are asserted.

The extra 128 loop iterations, calculated from 256 minus 128 iterations, show:

| Key length | Baseline requested bytes | Candidate requested bytes | Candidate requests |
| --- | ---: | ---: | ---: |
| 8 bytes | 4,864 | 768 | 256 |
| 1,024 bytes | 526,464 | 768 | 256 |

VM and JIT results match. This removes key-length-dependent allocation from
successful reads; it does not claim zero allocation for the whole helper loop.
Initial setup and remaining loop work are included in full-run totals but removed
from the added-iteration calculation.

For 256 iterations with a 1,024-byte key, both modes fall from 5,664 requests and
1,844,244 bytes to 4,640 requests and 792,852 bytes. These are requested allocation
counts, not elapsed speed or peak memory. The laptop remains loaded; clean-host
comparisons with Goose/Rust remain pending.

The read/write gate passes all **16 allocation pairs and 24 semantic executions**.
It rejects the unchanged baseline because key-length allocation still grows.
The existing module matrix passes all 128 pairs and 192 semantic executions:
64 pairs reduce bytes, 64 match, and none increase requests or bytes. The separate
12-pair numeric-key gate also passes without regressions.

## Scope and remaining costs

Nested writes still copy the key in assignment validation and mutation. After
this read fix, 128 extra nested writes request 45,184 bytes with an eight-byte key
versus 305,280 with a 1,024-byte key. The 260,096-byte difference is exactly
128 × 2 × 1,016, consistent with the two remaining assignment key copies.
Those writes are a control and a next optimization target, not claimed fixed here.

Nested row/struct publication, captured-name/local setup, broader dictionary
write patterns and general capture traversal remain open. This change does not
close checklist sections 3/4 or establish production-size/platform acceptance.

## Verification

- 1,163 library tests pass; eight manual profiling tests remain ignored.
- 328 release integrations pass; one manual byte-growth test remains ignored.
- The 21 indexed-publication and three new dictionary-lookup contracts pass on
  the matching candidate. Lookup contracts also pass on the baseline and cover
  long/empty/null keys, canonical strings, integer limits, errors and recovery.
- Unit coverage preserves escaped missing-key diagnostics and unsupported-index
  errors. The new lookup contracts run in VM, interpreter and JIT and are also
  included in the broader integration count.
- All 36 cold allocation comparisons and 54 cold semantic controls pass. Eight
  append pairs have identical requests/bytes, and three publication-boundary
  controls pass, one per runtime.
- All **150 test-size LangArena checksums** pass, 50 each in VM, interpreter and
  JIT, against the exact matching release candidate.
- Formatting, diff checks and source/binary/library/probe identity checks pass.

[Machine evidence](dictionary-read-runtime-2026-10-27.json) includes compiler
arguments, source identities, allocation pairs, raw checksum results, ignored-test
names, the negative baseline control and disk-recovery evidence.

The reusable targeted gate is
`probes/dictionary-read-allocations.rb BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR`,
run with Ruby from the repository root. Its workloads are Kujo; Ruby maintains
paired process/allocation evidence.

`indexed-publication-allocations.rb` adds explicit `control` mode for the complete
no-regression module matrix when another targeted gate proves the improvement.
Its default `array` and optional `dict` width-reduction gates remain unchanged.
Control mode passes the unchanged baseline with zero reductions; the read gate
independently rejects that same baseline, so acceptance still requires a measured
targeted improvement.

An integration link initially failed because the laptop ran out of disk space.
Only rebuildable Kujo incremental caches and failed temporary object files were
removed. The verified release and evidence were retained, and the integration
stage was retried successfully without runtime changes or repeating the release
build. No Kostya contact or upstream submission occurred.
