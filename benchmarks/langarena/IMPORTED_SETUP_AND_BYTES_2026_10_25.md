# Imported setup, parameter metadata and byte conversion — 2026-10-25

This round continues checklist sections 3–4. It reduces the reproduced imported
setup cost, removes repeated ordinary parameter-slot lookup, and avoids repeated
buffer growth in `bytes(array)`. Mutable value snapshots, errors, captures,
recursion, code-edit invalidation and iterator snapshots remain required contracts.
No Arena algorithm, configuration, checksum, or warmup changes; no Kostya contact.

Evidence: `imported-setup-and-bytes-2026-10-25.json`.
Tracker: [PERFORMANCE_CHECKLIST.md](PERFORMANCE_CHECKLIST.md).

## Baselines and scope

- First comparison: `787498d` runtime (documentation tip `61211fa`) → `c00515c`.
- Follow-up comparison: `c00515c` → `0d0f365` (byte reservation `133d4c2`), containing the byte
  reservation and parameter metadata changes. Their separate diagnostic kernels
  isolate the affected mechanisms; they share the final validation build.
- Standard `cargo build --locked --release --bin kujo --lib -j 2`, default features,
  Rust 1.96.0, thin LTO. Hashes, source identities, commands and logs are recorded.
- Frozen evidence/executables: `.audit-evidence/import-setup-2026-10-25/` and
  `.audit-evidence/bytes-slots-2026-10-25/`. `before` is the immediately preceding
  runtime in each comparison, not the original `f037705` performance baseline.
- This is local diagnosis. Clean-host, new Goose/Rust comparisons, production-size
  completion and platform acceptance remain pending.

## Imported setup: implicit mutable permissions

`Environment` already interprets a missing permission entry as mutable. Storing
those entries for native preloads and ordinary mutable bindings duplicated their
names during every captured-environment clone. `c00515c` stores only immutable
exceptions. Redefinition removes stale exceptions, and local helper-owner release
uses the same implicit-mutable rule. No value map, write journal or mutable
captured state is shared by this change.

The accepted implementation does not introduce a permission-map allocation when
entering an empty local scope. An allocation regression checks that warmed scope
push/pop still allocates nothing. Separate tests check immutable snapshots,
redefinition, caller isolation, imported state, recursion, errors and callbacks.

For 64 read-only imported calls, at both array widths 128 and 2,048:

| VM/JIT-enabled phase-4 traffic | Before | After |
| --- | ---: | ---: |
| Allocation requests | 92,129 | 61,977 |
| Requested bytes | 13,840,564 | 11,334,789 |

This removes **32.7% of requests and 18.1% of requested bytes**. No array-width
slope appears in either revision. These are main-thread allocation requests,
not retained memory. Interpreter behavior and elapsed time are checked separately;
the allocation probe instruments VM/JIT only.

Seven alternating pairs of a 4,096-call diagnostic, whole-process time:

| Runtime | Median after/before | Pair range |
| --- | ---: | ---: |
| VM | 0.722 | 0.610–0.759 |
| Interpreter | 0.672 | 0.552–0.801 |
| JIT-enabled | 0.697 | 0.643–0.802 |


All pairs improved locally. This is an imported-helper diagnostic, not a claim
that the whole language or the Arena suite is 30% faster. Host telemetry and
separate RSS observations are retained; clean-host acceptance stays open.

## Parameter lookup and cache lifetime

Ordinary VM frame setup previously searched `local_names` for every parameter on
every call and repeatedly checked named-alias metadata. The new cache resolves
slot indices and legacy alias flags once per immutable bytecode storage object.
It keeps the original first-match lookup behavior and unslotted/named fallbacks.

The cache belongs to the code object, not a function name or a process-global
map. Public bytecode mutation invalidates it through the existing copy-on-write
storage boundary. Tests cover changed parameter order, changed local slots,
changed named opcodes, independent callable snapshots and code equality.
Legacy receiver-field lookup is unchanged; no universal metadata-cost elimination
is claimed. The cache adds one bounded vector per called code object, proportional
to that function's parameter count.

| Arity | Runtime | Median after/before | Pair range |
| --- | --- | ---: | ---: |
| 0 | vm | 1.052 | 0.857–1.250 |
| 0 | interpreter | 0.881 | 0.799–1.028 |
| 0 | jit | 0.937 | 0.874–1.104 |
| 1 | vm | 1.017 | 0.855–1.106 |
| 1 | interpreter | 0.983 | 0.912–1.185 |
| 1 | jit | 1.006 | 0.809–1.169 |
| 8 | vm | 0.996 | 0.520–1.151 |
| 8 | interpreter | 1.048 | 0.956–1.109 |
| 8 | jit | 0.996 | 0.949–1.171 |
| 32 | vm | 0.834 | 0.650–0.925 |
| 32 | interpreter | 0.993 | 0.815–1.083 |
| 32 | jit | 1.044 | 0.932–1.389 |
| 128 | vm | 0.437 | 0.362–0.457 |
| 128 | interpreter | 1.025 | 0.823–1.246 |
| 128 | jit | 1.080 | 0.926–1.310 |

Seven alternating pairs per cell. VM 32- and 128-argument controls improved by
16.6% and 56.3% in median whole-process time. Small-arity improvements are not
established. The cache adds one 3,072-byte request for the 128-parameter VM kernel;
this is a scan-cost optimization, not an allocation reduction.

Initial zero-argument VM and 128-argument JIT medians were 5.2% and 8.0% slower.
Eleven additional alternating pairs, retaining all initial results, gave wall
ratios 0.981 and 0.998; user+system CPU ratios were 0.970 and 0.993. The unchanged
128-argument interpreter control gave wall 0.967 / CPU 0.958. Thus the slowdown
was not repeatable in this local check; absence of a clean-host regression is
not proven. Repeat ranges and all 66 observations are retained in JSON.

The JIT 128-argument allocation control recorded 19,901 native entries in both
revisions; this cache primarily affects VM frame setup, so broad native-speed
improvement is not claimed. Native boundary allocation remains outside this fix.


## Byte conversion and iterator review

`bytes(array)` knows the output length but previously grew its vector repeatedly.
It now reserves that length once and retains the same ordered integer/type/range
validation. Tests cover empty input, byte boundaries, invalid values, unchanged
source arrays, output snapshots and Unicode round-tripping in all runtimes.

For 64 conversions, VM phase-4 requests at width128 fall **1,236 → 980** and
requested bytes **45,055 → 37,375**; at width2048, requests fall **1,492 → 980**
and bytes **290,815 → 160,255**. Exact VM/JIT/interpreter observations are in JSON.
A direct native regression requires the same allocation count at widths8/128/2048.

Seven-pair whole-process byte timing ratios: VM0.967 (range0.515–1.239),
interpreter0.853 (0.774–1.078), JIT0.942 (0.697–1.589). Wide ranges prevent a
precise speed claim. The deterministic allocation reduction is the stronger result.


The representative controls also cover UTF-8 encoding/decoding, byte indexing,
and iteration while mutating the source. UTF-8 materialization scales with output
size; byte-index allocation does not scale with container width. The iterator
snapshot remains observable and requires a first detachment when the source is
mutated. Its measured traffic is linear in these controls. No iterator owner was
removed merely to improve a synthetic allocation count.

These controls use generated diagnostic inputs, not reduced or substituted Arena
workloads. The Kujo template is `probes/representation_costs.kujo`; raw generated
sources and hashes are preserved in evidence.

## Verification

- Final normal release build passed; all eight recorded source hashes match.
- **150/150 unchanged test-size checksums**: 50 each VM, interpreter and JIT.
- **201 final integration/source-unit tests passed**, zero failed, one ignored.
  Includes six exact-final bytecode source unit tests, ownership, imported helpers,
  callbacks, errors, byte validation and edited/legacy parameter contracts.
- First-fix gate: **1,109 library tests passed**, zero failed, eight ignored;
  **191 integration tests passed**, zero failed, one ignored.
- **56 imported-setup observations**, **356 final representation/slot/timing/resource
  observations**, and **66 targeted repeat observations** passed exact assertions.
- Formatting, Ruby syntax, source hash and diff checks passed.


The full 1,109-pass library run applies to `c00515c`, before the two follow-up
changes. Final-source bytecode unit tests are also compiled from the unchanged
source module into the integration harness, which links the final release library
and invokes the final CLI. Exact build commands and harness source are preserved.
Do not describe the earlier full-library run as an exact-final full-library run.

## Still open

- Imported helpers still clone their captured value environments; the remaining
  fixed cost is substantial. Removing redundant permission names reduces it but
  does not eliminate environment materialization. Further sharing must preserve
  captures, writes, recursion and JIT pointer lifetimes.
- Legacy receiver-field lookup and specialized collection paths are not covered
  by a claim that every metadata/access cost has been eliminated.
- The pre-existing direct-global reentrant-index difference between VM/JIT and
  interpreter is unchanged. Changing its result requires a separate semantic
  decision; it was not silently altered in a performance patch.
- Precise generated-native/fallback CPU attribution, clean-host comparisons,
  production-size and platform acceptance remain open. Test-size checksum passes
  do not substitute for those gates.
