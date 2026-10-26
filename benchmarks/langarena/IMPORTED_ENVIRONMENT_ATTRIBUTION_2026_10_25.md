# Imported environment clone attribution — 2026-10-25

Continuation of checklist section 3, following runtime `0d0f365` and evidence
`8ebb34f`. This pass measures the remaining cost without changing runtime semantics.
It does not close the captured-environment optimization checkbox.

The standalone Rust probe `probes/import_environment_allocations.rs` loads an
actual module through `ModuleLoader`, obtains its exported function's captured
`Environment`, and counts 64 clones after warming the accounting path. Module
loading, parsing, setup and output are outside the measured interval. Counters
measure allocation requests and requested bytes on the current thread, not
retained memory, CPU time or end-to-end language speed.

Four fixtures vary extra module bindings (0/128/2,048) independently from captured
array width (128/2,048). The probe measures the complete environment, the global
map, binding-name clones, native-value-name clones, and other value clones.
Separate all-runtime controls check that exported mutable state persists while
previously returned arrays remain snapshots.

| Extra bindings / captured array width | Environment requests | Requested bytes |
| --- | ---: | ---: |
| 0 / 128 | 60,480 | 11,226,432 |
| 0 / 2,048 | 60,480 | 11,226,432 |
| 128 / 128 | 68,672 | 11,293,120 |
| 2,048 / 128 | 191,552 | 44,120,000 |

All counts cover 64 snapshots. The base fixture has 472 global bindings, of
which 465 are native functions. Its global-map clone costs 60,352 requests /
11,220,288 bytes. Binding-name clones contribute 30,208 / 333,184; native value
names contribute 29,760 / 332,032; other values contribute 320 / 2,752.
Subtracting those isolated components leaves **64 map allocations requesting
10,552,320 bytes**, approximately 94% of global-map requested bytes.

Thus shared native-name strings alone would reduce request count but leave most
requested bytes intact. A safe environment-storage optimization is the next
priority. Captured array width adds no clone slope in this fixture; user binding
count does. These are exact component measurements for this fixture, not additive
CPU attribution or a numerical decomposition of the earlier differently shaped
end-to-end imported-call fixture.


The filtered non-native map is a diagnostic counterfactual, **not** an executable
replacement environment: dropping native bindings would break user programs.
Pruning entries based only on this small fixture is not an acceptable optimization.

## Ownership constraints for the next fix

- `enter_captured_environment` currently snapshots the complete capture, executes
  against that snapshot, then stores completed state back. Recursion, callbacks,
  async write journals, errors and immutable-binding checks depend on this model.
  Moving the shared environment out and leaving an empty placeholder would change
  reentrant behavior.
- `Environment.scopes` is publicly accessible as `Vec<HashMap<String, Value>>`.
  Changing its element type is also a Rust embedding API change; language parity
  alone would not establish compatibility.
- Native VM paths take mutable pointers to those maps. `VM::native_context_views`
  refreshes views after callbacks; existing generator tests force scope relocation
  and verify pointer replacement. Adding shared map backing requires an explicit
  native-activation exclusivity/pinning design, including snapshots made while
  native code is active. Simply wrapping maps in `Arc` is not a reviewed solution.
- A candidate must preserve local-scope allocation behavior, imported mutation and
  recursion, captured callbacks, async merge semantics, public map access, and
  native-view lifetime tests. Measure read-only and write-heavy modules separately:
  a cheaper snapshot must not hide a larger first-write cost.

These are source-backed implementation constraints, not a claim of a newly
reproduced memory-safety defect in the current runtime. Current runtime behavior
and the previously recorded 150 test-size checksums are unchanged.

## Reproduction and evidence

Compile the standalone probe against the normal release library using Rust's
thin LTO (`-C lto=thin`); omitting it on this host sent Rust LLVM22 bitcode to
Apple LLVM17 and failed linking. The failed attempt executed no probe. Accepted
commands, executable/library/source hashes, generated sources and exact outputs
are recorded in `import-environment-attribution-2026-10-25.json`.

The four attribution controls and all **12 state/snapshot executions** (four
fixtures × VM/interpreter/JIT) passed. Rust formatting, runner syntax and diff
checks passed. No runtime source changed, so no new full-suite pass is claimed;
the prior final-runtime 150-checksum result remains historical evidence.

