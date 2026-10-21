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

Field/index append assignments now evaluate the callable and its arguments in the
runtime's original order, then evaluate assignment indices once in their original
position. Native array push/append can reuse the destination only when it still
holds the evaluated source allocation. Callback replacement, aliases, shadowed
functions, and errors preserve ordinary behavior. Dynamic indexed paths are
supported; this is not limited to literal fields. VM guards retain normal calls
for other functions and participate in optimizer branch relocation/JIT fallback.

## LIFO storage and consuming array pop

Native `Stack` now uses persistent links, sharing unchanged storage across values.
Push, pop, size and peek avoid whole-stack copies, and depth metadata avoids scans
when a stack is returned inside a result pair. Iteration/array conversion remains
linear and preserves bottom-to-top order. Removed compound items retain ordinary
value semantics. Long unique chains are destroyed iteratively.

An adjacent `let pair := pop(items); items = pair[0]` can reuse array storage when
`pop` resolves to the native builtin and `items` is mutable. Both runtimes retain
aliases, popped values, empty-pop results, shadowing, and ordinary error behavior.
The VM recognizes this sequence in function-local statement blocks and releases
the dead pair slot from a previous loop iteration before checking ownership.
Standalone functional `pop(items)` still preserves `items` and therefore copies;
non-adjacent statements and other spellings use the ordinary path. Persistent
`Stack` provides cheap functional LIFO updates without that syntactic restriction.

### State/LIFO follow-up verification (2026-10-21)

Baseline: `5710de50e2294b23b409e11b0dd043b040ac3694`. Same macOS x86-64
host and debug builds as above, setup excluded, medians of three samples. The
first four baseline cases overlapped compilation; final samples ran after the
verification builds. Absolute timings and ratios are diagnostic, not rankings.

Milliseconds at N=4,000:

| Workload | VM before | VM after | Interpreter before | Interpreter after |
|---|---:|---:|---:|---:|
| Struct-owned row writes | 1924.34 | 82.85 | 34.69 | 42.96 |
| Dictionary-owned row writes | 1904.40 | 67.04 | 38.88 | 44.84 |
| Field append | 4093.31 | 77.36 | 1735.10 | 66.50 |
| Adjacent array pop/rebind | 994.17 | 37.12 | 1033.38 | 62.31 |
| Stack pop/push pairs | 9372.63 | 270.75 | 8652.94 | 161.41 |

The stack baseline was rerun with the preserved baseline executable; its VM
2,000-element sample was noisier and slower than its 4,000-element sample. Do not
infer precise scaling ratios from those absolute baseline timings.

Final 1,000 → 4,000 element samples grow approximately with the amount of work.
Interpreter nested writes were already efficient; their small timing differences
are not evidence of improvement. The structural tests cover row/append/pop buffer
identity, retained aliases, cached depth work, stack spine sharing and destruction,
plus both runtimes' evaluation order, errors, closures and immutable bindings.
VM host-value tests cover over-deep siblings and repair without partial writes.

Validation: 1,241 selected Rust tests passed (eight existing library ignores),
including library, native representation, runtime parity, CLI/JSON, diagnostics,
documentation and affected API contracts. Both VM and dual CLI gates passed
150/150 runnable fixtures, 11 policy skips, with zero dual fallback. Formatting,
`cargo check`, and diff whitespace checks passed. All five manual profile cases
passed their assertions. The full unfiltered Cargo integration suite was not run;
validation focused on the broad runtime/CLI and affected-surface gate above.
No benchmark port, release artifact, or maintainer communication is included.

## Helper-call ownership (2026-10-21)

Baseline: `229d808bf5fcb79f5c542f4b5f562ad56ff205cb`. A function-local loop
`x = edit(x, i)`, with `edit(a, i) { mut b := a; b[i] = 7; return b }`, retained
both the caller's old array and redundant callee parameter references. Repeated
updates consequently copied the whole array. The implementation now:

- Evaluates all call operands in the runtime's existing order, then releases a
  mutable caller-local destination only if its storage still matches argument one.
- Retains the old value when a handler in that invocation could observe it after
  failure. Globals/captured bindings and async/generator calls use ordinary paths.
- Moves owned argument vectors into callee bindings; VM slot parameters no longer
  carry a redundant named mirror unless legacy bytecode actually uses that name.
- Consumes an initial parameter alias only when lexical analysis proves the source
  is dead afterward. Other aliases, nested closures, and repeated arguments still
  retain their snapshots and trigger copy-on-write.
- At this milestone, moved VM-to-AST arguments through normalization, retaining
  unique array/dictionary buffers. The traversal and specialized-map conversion
  cost was subsequently removed by the imported collection work below.

Internal tests assert the original 10,000-element array buffer survives 64 helper
updates in the interpreter, VM, and VM-to-AST bridge. Integration contracts run
both runtimes and cover aliases, closures, recursive updates, bytes, dictionaries,
struct rows, immutable bindings, repeated arguments, argument effects, imports,
and catch recovery. No timing threshold is a test assertion.

Imported-recursion reproduction used an exported `expression` calling private
`term`/`factor` functions and recurring back into `expression`. Baseline VM imports
and the interpreter rejected `expression(10)` at the old 32-call limit. Both now
support `expression(60)` (183 ordinary calls), preserve module-private bindings,
and recover after excessive recursion. The ordinary interpreter limit is 256,
with guarded host-stack growth; the separate callback-bridge limit remains 32.
Small-stack embedding tests cover AST calls. The broader generator suite also
exposed a host-stack overflow in nested VM dispatch; guarding the dispatch entry
allows all 27 generator continuation contracts to pass on default test stacks.

Assertion-backed debug measurements on macOS x86-64, median of three runs per
size, timed inside the loop workload (setup and process startup excluded). All
verification builds completed before these samples. Times are milliseconds and
are diagnostic scaling evidence, not release-throughput claims.

| N | VM before | VM after | Interpreter before | Interpreter after |
|---:|---:|---:|---:|---:|
| 1000 | 194.291 | 74.707 | 211.608 | 112.874 |
| 2000 | 734.668 | 170.978 | 646.189 | 211.447 |
| 4000 | 2492.025 | 379.896 | 3822.707 | 358.025 |

Final validation: 1,264 selected Rust tests passed, with eight existing library
ignores. VM and dual CLI fixture gates each passed 150/150 runnable cases and
11 policy skips; dual used zero interpreter fallback. `cargo check`,
`cargo fmt --check`, and diff whitespace checks passed. The unfiltered Cargo
integration suite was not run. All profiling assertions passed. The helper
failure contracts also cover the baseline VM defect where array index-write
errors bypassed active handlers; both affected dispatch paths now unwind normally.
No LangArena port, maintainer contact, or release artifact is included.

## Imported collection boundaries (2026-10-21)

Baseline: `0c45eabaf44dd6b420255d03feb921d1d58a0e08`. Imported reads and
updates recursively normalized every collection argument. Retained arrays were
copied, shared subgraphs were revisited, and specialized dictionaries were rebuilt
as ordinary maps. The normalizer also omitted the full dense integer dictionary:
`mut d := {}; d[0] = 7` followed by an imported `peek(d)` returning `d[0]`
failed with `Invalid index operation` in the VM while passing in the interpreter.

The interpreter now reads every dictionary representation directly, including
indexing, iteration, spread, destructuring, optional access and truthiness.
Canonical decimal keys and explicit null entries retain their meaning. The
VM-to-AST bridge, VM callback results and await cache no longer normalize the
collection graph. Mutations retain copy-on-write isolation. Specialized map
writes can still materialize an ordinary dictionary, spread/rest explicitly copy
entries, and struct/fixed-dictionary cloning can copy immediate fields/values.
These changes remove unconditional normalization, not all function-call costs.

Structural regressions assert shared array/dictionary identity across the actual
AST bridge, including a 12-level shared graph and a struct containing a
10,000-element row. Direct representation tests cover all six dictionary shapes.
Integration regressions exercise imports in both runtimes, nested writes, aliases,
closure snapshots, struct rows, native serialization, VM callbacks, and repeated
awaits whose cached values must remain unchanged after a returned copy is edited.
No elapsed-time threshold is used in tests.

Measurements below use the baseline and updated debug binaries on macOS x86-64,
median of three assertion-backed runs. Setup and process startup are excluded;
builds and test runs finished before profiling. The read cases perform 256
imported calls returning one element. The edit case performs N imported calls
`x = edit(x, i)`, each assigning one array element. Map reads use a dense integer
map with an explicit null at key zero. Interpreter runs are controls: their
ordinary calls did not use the VM-to-AST normalizer. Timings are diagnostic,
not release-throughput promises.

| Workload | N | VM before (ms) | VM after (ms) | Interpreter before (ms) | Interpreter after (ms) |
|---|---:|---:|---:|---:|---:|
| read | 1000 | 177.293 | 118.3 | 101.928 | 110.036 |
| read | 4000 | 263.22 | 115.881 | 99.172 | 108.561 |
| edit | 1000 | 442.464 | 408.278 | 353.692 | 393.143 |
| edit | 4000 | 2669.201 | 1629.058 | 1599.006 | 1579.502 |
| map_read | 1000 | 540.35 | 118.567 | 132.566 | 113.463 |
| map_read | 4000 | 2852.142 | 130.648 | 117.838 | 102.584 |

The baseline was remeasured after the updated binary, under idle conditions;
these final baseline samples are the table's comparison. An earlier baseline
batch was substantially slower, including interpreter controls, so exact speedup
ratios should not be generalized. Fixed-count VM reads no longer scale with the
input collection size; N edits scale approximately with the number of calls.

Final validation: 1,279 selected Rust tests passed, with eight existing library
ignores. This includes library, imported collection/recursion/callback, helper and
collection ownership, closure, assignment, native representation, task, generator,
VM/interpreter parity, CLI/JSON, diagnostic, README and docs/example contracts.
VM and dual CLI gates each passed 150/150 runnable fixtures with 11 policy skips;
dual used zero interpreter fallback. `cargo check`, `cargo fmt --check` and
`git diff --check` passed. All profiling assertions passed. The full unfiltered
Cargo integration suite was not run. No LangArena port, maintainer contact or
release artifact is included.

## Dictionary deletion and field/indexed pop (2026-10-21)

Baseline: `c72ee562f8e1905cd01b26d04576e373f220bdf1`. Repeated dictionary
removal rebuilt the remaining map, and field/indexed pop copied the shrinking
array. Even after storage reuse, an ordinary field write-back would rescan its
payload for depth validation. The consuming adjacent pair now fuses that
write-back, preserves the validated depth bound, and refreshes cache identities
for ordinary and specialized dictionary parents.

The runtime checks the native callable, binding mutability and destination
storage identity after argument evaluation. Retained aliases, closures, result
pairs and parent snapshots still detach on mutation. Effectful index expressions,
shadowed index names, nonadjacent write-back and other callables retain their
ordinary evaluation. VM optimization is limited to function scopes. Specialized
map deletion may convert representation once; retaining each intermediate result
still requires copies. These are bounded ownership optimizations, not reference
semantics or a universal constant-time deletion guarantee.

Measurements: macOS x86-64 debug binaries, median of three runs, N removals/pops
from N entries. Setup and process startup are excluded. Builds and tests finished
before these measurements. All workload assertions passed. The cases are retained
in `tests/collection_profile.rs`; select `dictionary_remove`, `field_pop`,
`indexed_pop`, `nested_dictionary_remove`, or the existing `pop` control with
`KUJO_PROFILE_CASE`. The harness also offers N=2000; the table reports the measured
1000/4000 endpoints. Absolute timings remain diagnostic, not release claims.

| Workload | N | VM before (ms) | VM after (ms) | Interpreter before (ms) | Interpreter after (ms) |
|---|---:|---:|---:|---:|---:|
| dictionary_remove | 1000 | 200.879 | 83.467 | 151.629 | 81.743 |
| dictionary_remove | 4000 | 2943.91 | 318.656 | 2181.048 | 245.885 |
| field_pop | 1000 | 294.576 | 25.224 | 104.096 | 26.394 |
| field_pop | 4000 | 2084.883 | 103.685 | 1244.156 | 111.081 |
| indexed_pop | 1000 | 113.459 | 22.195 | 103.058 | 30.217 |
| indexed_pop | 4000 | 1281.114 | 109.024 | 1214.798 | 99.694 |
| nested_dictionary_remove | 1000 | 238.622 | 87.409 | 161.878 | 54.796 |
| nested_dictionary_remove | 4000 | 2435.222 | 339.878 | 2213.697 | 221.375 |
| local_pop_control | 1000 | 11.58 | 11.84 | 25.813 | 18.522 |
| local_pop_control | 4000 | 48.52 | 45.533 | 149.054 | 81.014 |

Structural regressions assert allocation reuse, parent snapshot isolation and a
single payload depth scan across repeated pops, including fixed, sparse integer
and dense integer dictionary parents. Language regressions cover both runtimes,
null/missing/canonical keys, empty and negative-index pops, callable aliases,
key/index effects, immutable errors, retained pairs and closure snapshots. A
host-injected overdeep value verifies failed VM write-back leaves state intact.

Validation: 1,291 selected Rust tests passed, with eight existing library ignores.
The VM and dual CLI gates each passed 150/150 runnable fixtures, 11 policy skips,
and zero dual interpreter fallbacks. `cargo check`, `cargo fmt --check`,
`git diff --check` and the profiling harness build passed. The full unfiltered
Cargo integration suite was not run.

A separate pre-existing parity gap was identified: out-of-bounds indexed reads and
immutable indexed writes can bypass VM `except` handlers; the interpreter catches
them. Both baseline and updated binaries reproduce it (VM exit 4, interpreter
exit 0). This is recorded separately for follow-up, not fixed by the ownership
optimization. A subsequent exception-routing fix now makes these failures
catchable in the default VM; `tests/index_exception_contracts.rs` covers local
and general indexed reads, cross-frame recovery, rejected writes and unchanged
snapshots in both runtimes. Uncaught errors still fail. No benchmark port,
maintainer contact or release is included.

## Object-tree storage and full-depth prerequisite probe (2026-10-21)

Struct fields now use shared immutable storage with copy-on-write. Reads and
ordinary value clones no longer recursively duplicate subtrees. Field/indexed
writes, helper calls, closures, imports, task capture and errors preserve value
semantics. Depth validation and environment graph traversal recognize shared
struct allocations; a 40-level shared graph is visited once per relevant depth,
not expanded exponentially. The weak depth cache refreshes identities after
mutation. This changes runtime storage, not object identity or reference semantics.
Rust embedders constructing `Value::Struct` supply shared field storage (`.into()`
converts an owned field map); Kujo source syntax is unchanged.

The reproducible resource probe is
`cargo test --test struct_storage_contracts -- --ignored --nocapture`.
It constructs and counts a fresh depth-20 binary tree (2,097,151 distinct nodes),
using an explicit 600-second cooperative deadline per runtime. It passed in both
runtimes at commit `2101e1c`, on macOS x86-64 with the debug build:

| Runtime | Construction | Traversal |
|---|---:|---:|
| VM | 220.550 s | 87.582 s |
| Interpreter | 67.013 s | 69.596 s |

These are single-run resource-completion observations, not release performance
numbers. The VM process was observed near 4.5 GiB RSS during execution; this is
not an instrumented peak-memory figure. A prior depth-12 probe on `d7ddda3`
measured 1.973 s VM traversal and 1.344 s interpreter traversal with recursive
field-map cloning. The post-change VM depth-12 probe measured 0.320 s traversal.
Do not extrapolate these debug samples into official benchmark rankings.

The resource probe verifies one construction/traversal workload, not the complete
LangArena lifecycle, production iteration counts or checksums. The object-tree
port must still preserve fresh allocation and distinguish objects from arenas.

## Growing byte buffers

`concat` assignment now consumes the temporary source reference and reuses unique
byte capacity, including struct fields and indexed destinations. Actual aliases,
closure snapshots, and self-concatenation retain value semantics. A storage test
checks capacity reuse directly; a dual-runtime contract grows and edits 3 MiB.

The reproducible opt-in profile is:

```sh
cargo test --test byte_concat_contracts profile_byte_growth -- --ignored --nocapture
# Set KUJO_PROFILE_BINARY to a baseline binary to compare.
```

Local debug-build medians of three samples, in milliseconds, using 16-byte chunks:

| Runtime | Appends | Before (2101e1c) | After |
| --- | ---: | ---: | ---: |
| VM | 16,000 | 831.65 | 144.44 |
| VM | 64,000 | 5,113.64 | 585.88 |
| Interpreter | 16,000 | 602.27 | 214.70 |
| Interpreter | 64,000 | 4,128.48 | 851.90 |

Setup is excluded; length and final byte are asserted. These are local scaling
observations, not release-build LangArena results or timing thresholds. The new
measurements scale approximately fourfold for fourfold input; retained snapshots
can still require copying.

## Production text workloads: string ownership and indexing

The LangArena production sweep on 2026-10-21 exposed two independent text costs:

- `text = text + suffix` retained the evaluated left operand while allocating the
  replacement, even when no program alias existed. Both runtimes now discard
  only that temporary reference when the destination still has the same storage.
  Retained aliases, self-appends, RHS reassignment, and immutable bindings retain
  their ordinary value semantics. A storage-pointer unit test verifies capacity
  reuse independently of wall-clock timing.
- Repeated `len(text)`, string indexing, and `substring` scanned stable strings.
  A per-thread cache retains at most 64 weak allocation references and character
  counts. ASCII strings then support direct character indexing and slicing.
  Unicode strings retain character-based indexing and slicing. Unicode indexed
  lookup still walks the prefix; this is not a general Unicode random-access
  index. Weak references retain no string payload and `Arc::make_mut` invalidates
  allocation identity before mutation. Cache tests verify bounded size, reuse,
  alias isolation, and invalidation.

Production workload completion and timing evidence are in the
[LangArena production report](../benchmarks/langarena/PRODUCTION_VERIFICATION.md);
a passing reduced checksum configuration is not evidence
that the production configuration completed.

### Callable bytecode storage

Compiled chunks share their code and metadata across callable aliases, active
frames and snapshots. Mutating a chunk (compiler optimization, jump patching or
legacy generator marking) detaches its storage first. Capture cells and runtime
values retain their existing semantics; sharing code does not merge captures.
A 10,000-call probe with an unused 1,000-statement branch exposed repeated code
copies despite executing the same short path. Storage-identity tests cover cheap
clones and detached edits, including nested functions; dual-runtime contracts
cover independent captures, callable aliases, recursion and retained arrays.
Legacy named-parameter detection is cached with the shared code allocation.
Any mutable access invalidates that metadata, including edits to uniquely owned
code. Cached metadata does not affect bytecode equality.

Bytecode callback bridges attach the live global environment without immediately
cloning its bindings into the interpreter. Interpreter fallback and native paths
that need that environment materialize the snapshot before use. Each callback
still has its own VM, cancellation/depth checks and capability policy. A regex
probe with 2,000 matches and 0 versus 1,000 unrelated globals exposed this cost;
ownership tests verify the deferred copy, and both runtimes cover imported helpers,
nested callbacks, global updates, errors and restricted capabilities.
