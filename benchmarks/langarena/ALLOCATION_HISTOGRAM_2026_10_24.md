# QuickSort allocation attribution — 2026-10-24

Runtime: `16b96d5`, default-feature debug library, macOS x86_64.
This is structural allocation accounting, not a timing or peak-memory result.
The release verification job was running separately during this diagnostic.

`probes/quicksort_allocations.rs --histogram --expect-reuse` now emits exact
allocation-size counts below 2,048 bytes and one aggregate bucket at or above
2,048 bytes. Fixed thread-local counters require no accounting allocations.
Every trial asserts that histogram counts sum to total allocation requests.
Existing sorted-output, retained-snapshot, array-backing and reuse-budget
assertions still pass at sizes 128, 1,024 and 8,192, with VM and JIT enabled
separately and unique/shared inputs.

For the unique 8,192-element JIT trial, totals remain 124,380 requests and
28,608,056 cumulative requested bytes. There are zero array-sized allocations
and the original array backing is retained. The shared-input trial detaches
once and preserves its original snapshot.

| Request size | Count | Cumulative requested bytes |
| ---: | ---: | ---: |
| 660 | 14,633 | 9,657,780 |
| 1,304 | 7,316 | 9,540,064 |

These two buckets account for approximately 67% of requested bytes. A separate
diagnostic captured the first stack trace for each size in the unique JIT
8,192-element trial, temporarily disabling accounting while capturing the
trace. Both sampled stacks end at `jit_store_variable`, `src/jit.rs:2186`,
through `HashMap::insert` and hashbrown resize. Totals and ownership assertions
remained identical to the histogram-only run. The complete histogram and
sampled traces are retained in `quicksort-allocation-histogram-2026-10-24.json`.

The samples establish an allocation site, not attribution of every allocation
in either bucket. No runtime change or speedup is claimed here.

## Next implementation boundary

Whole-function native returns currently persist dirty SSA locals through
`persist_local_slots_to_vm`, even for VM call paths whose `func_locals` map is
discarded after extracting the return value. This is a candidate for removing
unnecessary map construction. It needs a controlled before/after allocation
experiment and semantic regression coverage before acceptance.

Do not disable `persist_local_slots` indiscriminately: loop exits and error
paths also use it to expose state to the VM. Preserve public standalone native
compilation behavior unless explicitly changing its contract. Audit named and
global assignments, object returns, recursive calls, exceptions and fallback
before selecting the narrower optimization.

Reproduce the histogram from the repository root, substituting the actual
default-feature library path:

```sh
rustc --edition=2021 benchmarks/langarena/probes/quicksort_allocations.rs \
  --extern kujo=target/debug/deps/libkujo-837bbf5b9bf890b9.rlib \
  -L dependency=target/debug/deps -o /tmp/kujo-allocation-histogram
/tmp/kujo-allocation-histogram --histogram --expect-reuse
```
