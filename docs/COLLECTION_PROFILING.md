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
  increasing prefixes. This remains quadratic in copied elements.
- **index:** N writes into a preallocated N-element local array. Setup excluded;
  the local in-place path scales approximately with the number of writes.
- **queue:** N dequeue/enqueue pairs on a queue of N values. `Queue` currently
  owns a `VecDeque<Value>`; argument evaluation and functional helpers copy it.
  This remains quadratic in copied elements. Size/peek calls also clone a queue
  through ordinary argument evaluation even though their handler only reads it.
- **nested:** N writes into one N-element row. Snapshot-based assignment keeps
  failed writes atomic but copies the path's shared row. This remains quadratic
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

Validation: 1,218 selected Rust tests passed (eight existing library ignores;
one profiling test ignored in the normal run), plus the explicit profiling run.
VM and dual CLI fixture gates each passed 150/150 runnable fixtures with 11 policy
skips; dual used no interpreter fallback. `cargo check`, formatting, and diff
whitespace checks passed. The full unfiltered Cargo suite was not run.

## Remaining performance decisions

Functional append, queue copies, and nested-row snapshot writes remain measured
limitations, not fixed by this patch. Improving them requires a separately scoped
ownership-transfer or persistent-collection design with explicit tests for builtin
shadowing, argument evaluation order, retained aliases, callbacks, failed writes,
closure snapshots, and depth enforcement. Do not infer full-suite feasibility from
these microprofiles. These are Kujo implementation decisions; no Kostya input is
needed to fix them. Benchmark algorithm/representation acceptance remains outside
this task.
