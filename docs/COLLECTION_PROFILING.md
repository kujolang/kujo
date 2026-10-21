# Byte handling and collection profiling (2026-10-20)

Scope: Kujo runtime primitives only. No LangArena benchmark implementation or
maintainer contact. Baseline: `8a8e1b13a6db7030cec158c26fb600bff50b86de`.

## Method

`tests/collection_profile.rs` is an opt-in, assertion-backed Kujo workload harness:

```sh
cargo test --test collection_profile -- --ignored --nocapture
KUJO_PROFILE_BINARY=/absolute/path/to/baseline-kujo cargo test --test collection_profile -- --ignored --nocapture
```

`KUJO_PROFILE_CASE=bytes` selects one workload for a focused rerun.

Measurements are medians of three runs, using `time_ns()` inside each Kujo
function after setup. Both runtime modes execute the same source. Host: macOS
x86_64, Rust 1.96.0, unoptimized development build. Compilation and other local
activity overlapped portions of the baseline run; timings are diagnostic evidence,
not release-performance claims or timing thresholds. Structural regression tests
prove the fixes independently of machine speed. Rerun an optimized build on an
idle host before using absolute numbers for capacity planning.

## Baseline measurements

Milliseconds; sizes are 1,000 and 4,000. All workload assertions passed.

| Workload | VM 1k | VM 4k | Interpreter 1k | Interpreter 4k |
|---|---:|---:|---:|---:|
| append | 246.43 | 5285.70 | 188.48 | 3016.36 |
| index | 4.69 | 20.76 | 7.51 | 25.86 |
| queue | 698.91 | 13769.33 | 854.98 | 13621.84 |
| nested | 133.26 | 2063.13 | 93.31 | 1303.39 |
| tree | 5.10 | 23.33 | 4.32 | 21.05 |
| string | 53.82 | 580.38 | 82.52 | 619.71 |
| bytes | 743.84 | 909.87 | 392.29 | 406.35 |

- **append:** N functional `x = push(x, item)` operations. Existing arguments
  retain the old array while the helper returns a new one; repeated growth copies
  increasing prefixes. At this baseline, work was quadratic in copied elements.
- **index:** N writes into a preallocated N-element local array. Setup excluded;
  the local in-place path scales approximately with the number of writes.
- **queue:** N dequeue/enqueue pairs on a queue of N values. `Queue` at the baseline
  owned a `VecDeque<Value>`; argument evaluation and functional helpers copy it.
  At this baseline, work was quadratic in copied elements. Size/peek calls also cloned a queue
  through ordinary argument evaluation even though their handler only reads it.
- **nested:** N writes into one N-element row. Snapshot-based assignment keeps
  failed writes atomic but copies the path's shared row. At this baseline, work was quadratic
  for this shape, despite flat local array writes being efficient.
- **tree:** balanced struct construction by doubling an owned struct tree until
  approximately N leaves. Owned field maps deep-copy subtrees. The measured
  balanced build grows roughly with output size; it is not evidence that arbitrary
  tree shapes or retained snapshots are cheap. No reference semantics were added.
- **string:** N Unicode-scalar indexed reads of an N-character ASCII string.
  Character indexing scans the UTF-8 string. Use `utf8_encode` once and index the
  resulting bytes when the algorithm needs bytes; Unicode indexing is unchanged.
- **bytes:** 20,000 indexed reads of a 3*N-byte buffer. Small buffers and per-call
  assertions mask much of the old cloning cost; the structural storage test is
  the decisive proof that reads no longer clone the payload.

## Confirmed fixes and verification

- Runtime bytes now use `Arc<Vec<u8>>`. Clones share allocation regardless of
  length (`byte_storage_contracts` checks up to one million bytes); host APIs that
  require owned vectors explicitly copy at their boundary. Language byte equality,
  slicing, hashing, encoding, compression, and binary I/O retain their contracts.
- Byte assignment previously returned `Invalid index assignment` in both runtimes.
  Checked writes now support global, local, captured, and nested paths, negative
  integer indices, and integer values 0..255. Copy-on-write isolates aliases;
  invalid writes do not partially update values. Regression coverage includes
  catchable errors and immutable bindings.
- `utf8_encode` and strict `utf8_decode` eliminate the base64/file round trip for
  text/binary conversion. Empty inputs, NULs, non-ASCII text, malformed sequences,
  arity, types, and first-class builtin calls are covered in both runtimes.
- A shared array DAG of depth 40 previously implied trillions of redundant visits
  during VM depth validation. A fresh allocation/depth cache now revisits shared
  storage only when reached on a deeper path. A deterministic test requires exactly
  82 visits for depth 41. Separate tests check traversal-order independence and
  freshness after mutation; a dual-runtime test constructs and traverses the DAG
  while checking copy isolation. The depth limit is unchanged.

## Focused before/after checks

A separate fixed-work probe performed 20,000 reads plus `byte_length` calls on
786,432 bytes, with allocation outside the timer. Median of three runs:

| Runtime | Baseline | Shared bytes |
|---|---:|---:|
| VM | 4,187.62 ms | 1,217.85 ms |
| Interpreter | 2,806.52 ms | 569.06 ms |

Reproduction source (run once per sample with `--allow-clock`, then with
`--allow-clock --interpreter`):

```kujo
func work() {
    let x := decode_base64(repeat("AAAA", 262144))
    let start := time_ns()
    mut i := 0
    while i < 20000 {
        let y := x[i % byte_length(x)]
        i += 1
    }
    print(time_ns() - start)
}
work()
```

For small 192-byte buffers, interpreter timing varied from 551.05 ms before to
631.77 ms after; these noisy debug measurements do not imply every workload got
faster. The improvement is avoiding payload-size-dependent copying. Borrowed
native byte-length handling and runtime byte clones now do constant work.

A single diagnostic shared-DAG probe (start `[0]`, repeat `tree = [tree, tree]`
20 times inside the timer) measured VM 725.69 ms before and 1.07 ms after;
interpreter 0.73 ms before and 0.57 ms after. The visit-count test, rather than
those single timings, locks in the complexity improvement.

Initial byte/DAG milestone validation: 1,218 selected Rust tests passed (eight existing library ignores;
one profiling test ignored in the normal run), plus the explicit profiling run.
VM and dual CLI fixture gates each passed 150/150 runnable fixtures with 11 policy
skips; dual used no interpreter fallback. `cargo check`, formatting, and diff
whitespace checks passed. The full unfiltered Cargo suite was not run.

## Copying-cost follow-up: resolved

The follow-up uses `68b6e911dba00e95d5f833f607470e8a2296afb2` as its baseline.
The same assertion-backed harness, machine, debug profile, sizes, and three-sample
medians were used. Builds overlapped parts of profiling, so treat timings as
qualitative scaling evidence rather than stable throughput claims.

Milliseconds at N=4,000:

| Workload | VM before | VM after | Interpreter before | Interpreter after |
|---|---:|---:|---:|---:|
| Append rebinding | 2599.75 | 30.36 | 5053.81 | 32.37 |
| Queue dequeue/enqueue | 15004.86 | 230.29 | 14592.86 | 147.15 |
| Nested row writes | 3222.25 | 39.07 | 2672.31 | 44.24 |

The final samples were collected after the verification builds finished.
Measurements support removal of the copying bottlenecks; allocation-identity and
snapshot tests, not timing ratios, guard against recurrence. No timing threshold
is used as a test assertion.

- **Append rebinding:** `items = f(items, expression)` evaluates the original
  arguments and resolves the callable in each runtime's existing order. When
  `f` resolves to builtin `push` or `append`, the destination is mutable, and it
  still contains the first argument's exact array allocation, the temporary
  argument snapshot is released before copy-on-write append. Callback changes,
  shadowed functions, invalid values, and immutable bindings use the original
  call/store path. An alias holding the array still forces copy-on-write; appending
  an array to itself retains a snapshot rather than creating a cycle. The VM
  checks the inserted item's depth before taking the fast path. The conditional
  bytecode target participates in optimizer relocation and safe JIT fallback.
- **Nested writes:** validate every path component before mutation, then edit
  the actual binding. A unique row retains its allocation across repeated writes;
  aliased rows detach. Validation failures cannot partially modify ancestors.
  The VM fully validates an updated snapshot once, then caches a weak identity
  for a valid array root and checks the inserted subtree's depth on repeated
  writes. Copy-on-write dissociates weak identities, invalidating the cache when
  another operation changes the array. The cache is bounded and retains no array
  payloads. This preserves checks on native/host-created inputs, including the
  ability to repair an over-deep subtree, without rescanning unchanged siblings. RHS and index-expression order is retained.
- **Queues:** immutable shared front/rear spines replace owned deques. Clone,
  size, and peek no longer copy the queue. Enqueue creates one node; dequeue is
  amortized constant time along an update sequence, with occasional linear rear
  reversal. Reversal is cached for snapshots sharing a rear spine, and reversed
  nodes share element payloads. Per-node length and maximum nesting metadata
  avoid full-queue scans when an enclosing bytecode collection is depth-checked. Iteration/array conversion remains
  linear, and returning a popped compound value can require its own value clone.
  Unique spine destruction is iterative, avoiding stack growth per queued item.

Deterministic regression evidence includes array/row allocation identity, alias
isolation, cached reversal identity, constant-visit queue depth checks, FIFO order
through 10,000 updates, 100,000-node destruction, closure snapshots, failed writes,
callback reassignment, builtin aliases/shadowing, optimizer relocation, JIT fallback,
and the existing VM builder/nested-write depth checks without partial assignment. This fixes the three
profiled costs; it does not promise constant-time copies of all retained snapshots,
optimize arbitrary non-rebinding append expressions, or change Unicode string-index
semantics. No benchmark port, maintainer contact, or release-readiness claim is part
of this work.

Follow-up verification: 1,234 selected Rust tests passed, with eight existing
library ignores. VM and dual CLI fixture gates each passed 150/150 runnable cases,
11 policy skips, and zero dual fallback. All three before/after profiles passed
their assertions; formatting, `cargo check`, and diff checks passed. The full
unfiltered Cargo suite was not run. The first broad run exposed a pre-existing
parallel fixture filename collision; an atomic suffix fixed it and the final
123-test parity suite passed within the clean broad rerun.

## Benchmark-state containers

VM nested writes now cache conservative depth bounds for array and dictionary
allocations below struct fields as well as for array roots. Weak identities are
invalidated by copy-on-write, and caches retain no collection payloads. Paths
validate before mutation. A possibly excessive bound triggers an exact proposed
result check, so repairing an over-deep host value remains possible. Unique row
buffers are preserved; retained snapshots still detach. Structural tests check
buffer identity and absence of repeated scans for both struct and dictionary
roots, alongside dual-runtime alias, closure, mutability and error tests.
