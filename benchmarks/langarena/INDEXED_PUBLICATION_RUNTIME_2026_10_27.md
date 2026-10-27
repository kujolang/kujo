# Sparse captured-array publication — 2026-10-27

Runtime `d337cfa` removes repeated width-dependent copies for eligible sparse
captured-global array updates. This is a measured allocation improvement, not a
claim about elapsed speed, peak memory, all collections, or competitive ranking.
The laptop remains loaded; clean-server comparisons are pending.

## Measured result

The comparator is the frozen `aee15a2` release from the local-map-reserve checkpoint.
Both probes execute identical fixture paths. The matrix covers arrays,
struct-contained arrays, dictionary-contained arrays and dictionaries; widths
128/2,048; one/eight updates per call; 32/64 calls; and retained-once/fresh snapshots.

Of **128 paired allocation comparisons**, 16 reduce requested bytes and 112 match.
None increase allocation requests or requested bytes. All 192 corresponding
semantic executions pass across VM, interpreter and JIT.

For the 2,048-element array, 64 calls, one update per call, and a retained original
snapshot:

| Mode | Baseline requests | Candidate requests | Baseline bytes | Candidate bytes |
| --- | ---: | ---: | ---: | ---: |
| VM | 9,057 | 8,931 | 19,425,482 | 1,875,698 |
| JIT | 9,147 | 9,021 | 19,435,647 | 1,885,863 |

The extra 32 calls expose the scaling difference independently of initial setup:

| Array width | Baseline added bytes | Candidate added bytes |
| --- | ---: | ---: |
| 128 | 847,648 | 289,312 |
| 2,048 | 9,203,488 | 289,312 |

The same flat-width result holds with eight updates per call, in both modes.
The first shared copy and other call/setup allocations remain. Fresh snapshots,
nested containers and dictionaries retain their existing allocation behavior.

## Semantics and implementation

A bounded private journal holds up to two distinct integer, float, Boolean or null
updates. It takes the existing private map key without cloning it. Admission
requires a mutable, unshadowed global array in a synchronous, nonrecursive,
global-only capture, with exactly two base handles at admission. Publication
checks actual owner identity and uses normal copy-on-write for retained observers.

The public owner remains a real unchanged array until publication. At return,
the invocation drops its base reference before using ordinary `Arc::make_mut` on
the matching public owner. A new observer still forces a copy. If the owner was
replaced, publication uses the invocation's original base and completed writes.

Whole-value reads, calls, snapshots, unsupported operations and scope transfers
materialize ordinary values as needed. Pure integer literal/identifier indexed
reads can consult pending writes. Dense/nonscalar writes fall back. Task journals
remain separate, rollback releases pending values, and existing pool budgets are
unchanged. No public `Value` variant or environment/frame field was added. The
layout test compares the actual capture enum with its pre-integration shape.

## Verification

- 1,157 library tests pass; eight manual profiling tests remain ignored.
- 316 release integration tests pass; one manual byte-growth test remains ignored.
- All 15 indexed-publication contracts pass across VM/interpreter/JIT, covering
  index/root replacement, snapshots, errors, shadowing, native calls and captures.
- Three additional publication-boundary controls pass, one per runtime.
- All **150 test-size LangArena checksums** pass, 50 per runtime.
- All 36 cold allocation comparisons and 54 cold semantic controls pass.
- Eight paired append comparisons have identical allocation counts and bytes.
- Formatting, diff checks and frozen-source verification pass.

[Machine evidence](indexed-publication-runtime-2026-10-27.json) includes runtime,
library/probe identities, source hashes, compiler arguments, all allocation pairs,
raw checksum results, ignored-test names and boundary outputs. Large generated
cold fixtures are represented by their hashes and committed generator rather than
duplicated source text. Build-tool failures were repaired without runtime changes:
Rust thin LTO is required for these release bitcode libraries on the current Apple
linker; an append harness also needed its imported fixture copied alongside it.

The reusable gate is
`probes/indexed-publication-allocations.rb BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR`,
run with Ruby from the repository root. It requires a reproduced baseline width
cost, flat candidate added-call allocation across widths, and no regression in
the complete matrix. Its fixture programs remain Kujo; Ruby only maintains the
paired process/allocation evidence.

## Still open

Dictionary and nested-row/struct publication costs, dense or escaping update
patterns, retained captured-local/name copying and general capture traversal remain
open. This change does not close sections 3/4 as a whole. Production-size acceptance,
compiled-path expansion, platform checks and clean-host Goose/Rust comparisons
remain separate checklist items. No Kostya contact or upstream submission occurred.
