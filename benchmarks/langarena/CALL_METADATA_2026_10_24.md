# Immutable call-dispatch metadata — 2026-10-24

Runtime commit: `16b96d5`. Previous runtime: `09c8944` (preparation-only HEAD
`1d55e8e`). This continues the performance plan while clean-server access is
pending; it does not complete cloud, production or platform acceptance.

Native callbacks repeatedly scanned instructions for closure creation, exception
regions and backward jumps, even after their target had been compiled. These
facts now live in a lazy `OnceLock` attached to immutable bytecode storage.
Clones share the cache; every mutable bytecode access clears it after any
copy-on-write detach. Cache state is excluded from code equality. No function
name is used as the key for these facts. Async/generator/capture/lexical-self
flags are included and invalidate through the same mutation mechanism.

The first metadata request still scans the body once. This is not direct
native-to-native calling, a new array representation, or removal of error,
ownership or capability checks. Indexed writes were inspected and already
borrow their binding name; no speculative name-allocation fix was made.

## Reproduction and evidence

`probes/native_call_metadata.rs` constructs functions with identical executed
work but 0, 1,000 or 10,000 unreachable Nop instructions after Return. It compiles
each function before timing and asserts the result of every callback. Four
10,000-call rounds per size use the debug library. Before and after executables
were separately linked; raw logs and identities are retained in
`native-call-metadata-2026-10-24.json`.

| Unreachable instructions | Before: seconds per 10,000 calls | After |
| ---: | ---: | ---: |
| 0 | 0.049–0.053 | 0.047–0.051 |
| 1,000 | 0.280–0.293 | 0.047–0.048 |
| 10,000 | 2.100–2.179 | 0.048–0.049 |

This isolates body-length-dependent dispatch overhead. It does **not** imply
the same speedup for QuickSort, production workloads, release binaries or other
languages. The trials are diagnostic, not randomized controlled-host timings;
compilation and the initial metadata population are outside their timed region.

Example build, substituting the actual default-feature debug rlib path:

```sh
rustc --edition=2021 benchmarks/langarena/probes/native_call_metadata.rs \
  --extern kujo=target/debug/deps/libkujo-837bbf5b9bf890b9.rlib \
  -L dependency=target/debug/deps -o /tmp/kujo-call-metadata-probe
/tmp/kujo-call-metadata-probe
```

## Verification boundary

- 1,089 library tests pass; 8 intentionally ignored.
- 29 targeted integration tests pass, including VM/interpreter/JIT exception,
  closure, loop, snapshot, recursion-depth and owned-argument behavior.
- A new storage regression covers shared cache identity, same-named distinct
  code, instruction edits, unique edits and async/generator/capture/lexical-self
  flag changes. Equality ignores whether the cache was initialized.
- `cargo check --no-default-features`, `cargo fmt --check` and `git diff --check`
  pass. Existing dependency/no-default warnings remain.

## Frozen release verification

`call-metadata-release-2026-10-24.json` records **150/150 passed checksums**:
all 50 unchanged test-size cases in VM, interpreter and JIT. The binary was
copied after source-diff checks against `16b96d5` before and after its build.
Registry names, per-case configuration, exit codes and checksum output were
verified. The running copy remained frozen during later source changes.

Seven rotating before/after VM/JIT QuickSort rounds passed all 28 checksums.
The baseline is `09c8944`:

| Runtime | Before median seconds | After median seconds | Median paired speedup | Pairs favoring after |
| --- | ---: | ---: | ---: | ---: |
| VM | 9.3500 | 9.0495 | 1.0251x | 5/7 |
| JIT | 3.0775 | 2.8981 | 1.0551x | 4/7 |

CPU speed limits changed from 51% to 68%. Mixed pair directions and changing
host limits make this timing evidence inconclusive; the dispatch-scan diagnostic
above is the evidence isolating the eliminated mechanism. No new compilation
tests ran during these timing samples.

After timings finished, integration linking exhausted disk space. The checksum
sweep stopped with 50 VM and 16 interpreter rows safely retained. Cargo cleanup
of this checkout's disposable debug artifacts recovered space. The remaining
34 interpreter cases and all 50 JIT cases were then run using the same frozen
binary and unchanged inputs. The interruption and resumed results are retained;
no failed or missing case was counted as passing.

This release gate predates the subsequent dead-local-map candidate. Production
completion, remaining allocation/indexing costs, peak-memory remeasurement and
supported-platform verification remain open.
